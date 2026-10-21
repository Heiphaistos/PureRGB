//! Lecteur vidéo de secours par ffmpeg (processus enfant), pour les PC dont Windows ne sait pas
//! décoder le fichier (édition « N », extensions vidéo absentes). L'exe vient de `ffmpeg_install`
//! (Program Files, hash vérifié). Sonde = parse de `ffmpeg -i` (stderr, pas besoin de ffprobe) ;
//! lecture = images BGRA brutes sur un pipe, une taille de trame connue, une boucle par relance
//! du processus. Rien n'est jamais passé par un shell ; le processus est tué à l'arrêt/au Drop.

use super::video::{bgrx_to_pixmap, Decoded, FrameSource, VideoInfo, MAX_OUT, MAX_SOURCE_SIDE, VIDEO_EXTS};
use anyhow::{anyhow, bail, Context, Result};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

/// Attente maximale d'une image (démarrage de ffmpeg compris) : au-delà, le processus est tué.
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);
/// Sortie de sonde lue au plus (un fichier piégé ne remplit pas la mémoire).
const MAX_PROBE_TEXT: u64 = 256 * 1024;
/// Images lues au plus par passage (1 h à 60 i/s) : au-delà, la boucle repart.
const MAX_FRAMES_PER_PASS: u64 = 216_000;
const DEFAULT_FPS: f32 = 30.0;
const MAX_FPS: f32 = 60.0;

/// Arrête un processus (idempotent).
pub trait Killer: Send {
    fn kill(&mut self);
}

/// Un processus lancé : son flux de sortie + de quoi le tuer.
pub struct Spawned {
    pub out: Box<dyn Read + Send>,
    pub killer: Box<dyn Killer>,
}

/// Lance ffmpeg avec ces arguments (tableau, jamais de shell) ; mockable dans les tests.
pub trait Spawner: Send + Sync {
    fn spawn(&self, args: &[OsString]) -> std::io::Result<Spawned>;
}

struct ChildKiller(Child);

impl Killer for ChildKiller {
    fn kill(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for ChildKiller {
    fn drop(&mut self) {
        self.kill();
    }
}

fn command(exe: &Path, args: &[OsString]) -> Command {
    let mut c = Command::new(exe);
    c.args(args).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

pub struct ExeSpawner {
    exe: PathBuf,
}

impl ExeSpawner {
    pub fn new(exe: PathBuf) -> ExeSpawner {
        ExeSpawner { exe }
    }
}

impl Spawner for ExeSpawner {
    fn spawn(&self, args: &[OsString]) -> std::io::Result<Spawned> {
        let mut child = command(&self.exe, args).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let out = child.stdout.take().ok_or_else(|| std::io::Error::other("pas de sortie standard"))?;
        Ok(Spawned { out: Box::new(out), killer: Box::new(ChildKiller(child)) })
    }
}

/// Fichier vidéo acceptable : chemin absolu (jamais pris pour une option ni une URL), fichier réel, extension vidéo.
fn validate_path(path: &Path) -> Result<()> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let s = path.to_string_lossy();
    if !path.is_absolute() || s.starts_with(r"\\.\") || !VIDEO_EXTS.contains(&ext.as_str()) || !path.is_file() {
        bail!("fichier vidéo invalide");
    }
    Ok(())
}

fn parse_dim(tok: &str) -> Option<(u32, u32)> {
    let (w, h) = tok.trim_end_matches(',').split_once('x')?;
    let num = |s: &str| (!s.is_empty() && s.len() <= 5 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<u32>().ok()).flatten();
    let (w, h) = (num(w)?, num(h)?);
    (w > 0 && h > 0).then_some((w, h)) // « 0x31637661 » (tag hexadécimal) est refusé par w > 0
}

fn parse_duration_ms(line: &str) -> u32 {
    let Some(rest) = line.split("Duration:").nth(1) else { return 0 };
    let tok = rest.trim().split(',').next().unwrap_or("").trim();
    let mut it = tok.split(':');
    let (Some(h), Some(m), Some(s), None) = (it.next(), it.next(), it.next(), it.next()) else { return 0 }; // « N/A »
    let (Ok(h), Ok(m), Ok(s)) = (h.parse::<f64>(), m.parse::<f64>(), s.parse::<f64>()) else { return 0 };
    ((h * 3600.0 + m * 60.0 + s) * 1000.0).clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Extrait taille d'affichage (rotation appliquée : ffmpeg pivote seul), durée et cadence de la sortie de `ffmpeg -i`.
pub fn parse_probe(text: &str) -> Result<VideoInfo> {
    let duration_ms = text.lines().find(|l| l.trim_start().starts_with("Duration:")).map_or(0, parse_duration_ms);
    let lines: Vec<&str> = text.lines().collect();
    let Some(vi) = lines.iter().position(|l| l.trim_start().starts_with("Stream #") && l.contains("Video:") && !l.contains("attached pic")) else {
        if text.contains("Invalid data found") || text.contains("moov atom not found") {
            bail!("ffmpeg ne reconnaît pas ce fichier (corrompu ou incomplet)");
        }
        bail!("aucune piste vidéo dans ce fichier");
    };
    let toks: Vec<&str> = lines[vi].split("Video:").nth(1).unwrap_or("").split_whitespace().collect();
    let (w, h) = toks.iter().find_map(|t| parse_dim(t)).ok_or_else(|| anyhow!("taille de la vidéo introuvable"))?;
    let rate = |unit: &str| toks.windows(2).find(|p| p[1].trim_end_matches(',') == unit).and_then(|p| p[0].parse::<f32>().ok());
    let fps = rate("fps").or_else(|| rate("tbr")).filter(|f| f.is_finite() && *f > 0.0 && *f < 1000.0).unwrap_or(0.0);
    // Rotation : lignes de métadonnées / données annexes de CETTE piste (jusqu'à la piste suivante).
    let rot = lines[vi + 1..]
        .iter()
        .take_while(|l| !l.trim_start().starts_with("Stream #"))
        .find_map(|l| {
            let l = l.trim();
            let v = l.split("rotation of ").nth(1).or_else(|| l.strip_prefix("rotate").and_then(|r| r.split(':').nth(1)))?;
            v.trim().split(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.')).next()?.parse::<f32>().ok()
        })
        .unwrap_or(0.0);
    let quarter = (rot.abs() / 90.0).round() as i64 % 2 == 1;
    let (width, height) = if quarter { (h, w) } else { (w, h) };
    if width > MAX_SOURCE_SIDE || height > MAX_SOURCE_SIDE {
        bail!("vidéo trop grande ({width}×{height}, max {MAX_SOURCE_SIDE} px de côté)");
    }
    Ok(VideoInfo { width, height, duration_ms, fps })
}

/// Taille de sortie : celle de la source, réduite (ratio conservé) pour tenir dans `MAX_OUT`.
pub fn out_size(w: u32, h: u32) -> (u32, u32) {
    if w <= MAX_OUT.0 && h <= MAX_OUT.1 {
        return (w.max(1), h.max(1));
    }
    let k = (f64::from(MAX_OUT.0) / f64::from(w)).min(f64::from(MAX_OUT.1) / f64::from(h));
    (((f64::from(w) * k) as u32).max(1), ((f64::from(h) * k) as u32).max(1))
}

/// Cadence de sortie : celle de la vidéo, bornée (inconnue -> 30).
fn out_fps(fps: f32) -> f32 {
    if fps >= 1.0 { fps.min(MAX_FPS) } else { DEFAULT_FPS }
}

/// Sonde `path` avec l'exe ffmpeg `exe` (au plus `PROBE_TIMEOUT`, au plus `MAX_PROBE_TEXT` lus).
pub fn probe_file(exe: &Path, path: &Path) -> Result<VideoInfo> {
    validate_path(path)?;
    let args: Vec<OsString> = ["-hide_banner", "-nostdin", "-protocol_whitelist", "file", "-i"].iter().map(OsString::from).chain([path.as_os_str().to_owned()]).collect();
    let mut child = command(exe, &args).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().context("lancement de ffmpeg")?;
    let err = child.stderr.take().ok_or_else(|| anyhow!("pas de sortie d'erreur ffmpeg"))?;
    let mut killer = ChildKiller(child);
    let (tx, rx) = sync_channel::<String>(1);
    std::thread::Builder::new()
        .name("ffmpeg-probe".into())
        .spawn(move || {
            let mut buf = Vec::new();
            let _ = err.take(MAX_PROBE_TEXT).read_to_end(&mut buf);
            let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
        })
        .context("thread de sonde ffmpeg")?;
    let text = rx.recv_timeout(PROBE_TIMEOUT);
    killer.kill();
    parse_probe(&text.map_err(|_| anyhow!("ffmpeg ne répond pas (sonde)"))?)
}

struct Run {
    rx: Receiver<Vec<u8>>,
    killer: Box<dyn Killer>,
    idx: u64,
}

pub struct FfmpegSource {
    spawner: Arc<dyn Spawner>,
    args: Vec<OsString>,
    size: (u32, u32),
    fps: f32,
    run: Option<Run>,
    read_timeout: Duration,
    max_frames: u64,
}

impl FfmpegSource {
    /// `info` : résultat de `probe_file` (tailles déjà bornées).
    pub fn open(spawner: Arc<dyn Spawner>, path: &Path, info: VideoInfo) -> Result<FfmpegSource> {
        validate_path(path)?;
        let size = out_size(info.width, info.height);
        let fps = out_fps(info.fps);
        let s = |v: &str| OsString::from(v);
        let args = vec![
            s("-nostdin"), s("-hide_banner"), s("-v"), s("error"), s("-protocol_whitelist"), s("file"),
            s("-threads"), s("2"), s("-i"), path.as_os_str().to_owned(), s("-an"), s("-sn"), s("-dn"),
            s("-vf"), OsString::from(format!("scale={}:{}:flags=bilinear", size.0, size.1)),
            s("-r"), OsString::from(format!("{fps}")), s("-f"), s("rawvideo"), s("-pix_fmt"), s("bgra"), s("pipe:1"),
        ];
        Ok(FfmpegSource { spawner, args, size, fps, run: None, read_timeout: READ_TIMEOUT, max_frames: MAX_FRAMES_PER_PASS })
    }

    fn start(&self) -> Result<Run> {
        let Spawned { mut out, killer } = self.spawner.spawn(&self.args).map_err(|e| anyhow!("lancement de ffmpeg : {e}"))?;
        let bytes = self.size.0 as usize * self.size.1 as usize * 4;
        let (tx, rx) = sync_channel::<Vec<u8>>(2); // 2 images d'avance : le pipe se remplit, ffmpeg attend
        std::thread::Builder::new()
            .name("ffmpeg-read".into())
            .spawn(move || loop {
                let mut buf = vec![0u8; bytes];
                if out.read_exact(&mut buf).is_err() || tx.send(buf).is_err() {
                    return; // fin de flux, processus tué ou lecteur parti
                }
            })
            .map_err(|e| anyhow!("thread de lecture ffmpeg : {e}"))?;
        Ok(Run { rx, killer, idx: 0 })
    }

    fn stop(&mut self) {
        if let Some(mut r) = self.run.take() {
            r.killer.kill();
        }
    }
}

impl FrameSource for FfmpegSource {
    fn next_frame(&mut self) -> Result<Option<Decoded>> {
        if self.run.is_none() {
            self.run = Some(self.start()?);
        }
        let Some(run) = self.run.as_mut() else { return Ok(None) };
        if run.idx >= self.max_frames {
            self.stop();
            return Ok(None);
        }
        match run.rx.recv_timeout(self.read_timeout) {
            Ok(buf) => {
                let ts_ms = (run.idx as f64 * 1000.0 / f64::from(self.fps)) as u64;
                run.idx += 1;
                let (w, h) = self.size;
                let pixmap = bgrx_to_pixmap(w, h, (w * 4) as i32, &buf, (w, h)).ok_or_else(|| anyhow!("image ffmpeg de taille inattendue"))?;
                Ok(Some(Decoded { pixmap, ts_ms }))
            }
            Err(RecvTimeoutError::Timeout) => {
                self.stop();
                bail!("ffmpeg ne produit plus d'images (délai dépassé)")
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.stop();
                Ok(None)
            }
        }
    }

    /// Relance le processus à la demande (la boucle = nouveau passage).
    fn rewind(&mut self) -> Result<()> {
        self.stop();
        Ok(())
    }
}

impl Drop for FfmpegSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "ffmpeg_dec_tests.rs"]
mod tests;
