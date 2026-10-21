//! Banc de mesure de la fluidité (tests `#[ignore]`, à lancer avec `--release --nocapture`).
//! `PURERGB_TEST_VIDEO=<fichier vidéo>` ajoute les mesures sur une vraie vidéo (lecture seule, copiée en temp).
//!   cargo test --release --lib perf_bench -- --ignored --nocapture --test-threads=1

use super::bindings::ValueStore;
use super::driver::{Frame, Protocol, ScreenDriver, ScreenInfo};
use super::drivers::catalog::Profile;
use super::drivers::{encode, wire};
use super::layout::Layout;
use super::media::MediaStore;
use super::render;
use super::runner::Runner;
use super::state::ScreenConfig;
use anyhow::Result;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const W: u32 = 1920;
const H: u32 = 462;

fn layout_with_bg(media_id: &str) -> Layout {
    serde_json::from_str(&format!(
        r##"{{"version":1,"name":"bench","width":{W},"height":{H},"background":{{"kind":"media","media_id":"{media_id}","fit":"cover"}},"widgets":[]}}"##
    ))
    .expect("layout")
}

/// GIF de remplacement quand aucune vidéo n'est fournie : 12 images à 10 i/s.
fn make_gif(dir: &std::path::Path) -> PathBuf {
    use image::{codecs::gif::GifEncoder, Delay, Frame as GF, Rgba, RgbaImage};
    let p = dir.join("bench.gif");
    let mut enc = GifEncoder::new(std::fs::File::create(&p).expect("gif"));
    for i in 0..12u32 {
        let img = RgbaImage::from_fn(640, 154, |x, y| Rgba([((x + i * 40) % 256) as u8, (y * 2 % 256) as u8, (i * 20) as u8, 255]));
        enc.encode_frame(GF::from_parts(img, 0, 0, Delay::from_numer_denom_ms(100, 1))).expect("frame");
    }
    p
}

struct Bench {
    media: Arc<MediaStore>,
    id: String,
    label: String,
}

fn setup(tag: &str) -> Bench {
    let dir = std::env::temp_dir().join(format!("purergb-bench-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dossier");
    let media = Arc::new(MediaStore::new(dir.join("media")));
    let (path, label) = match std::env::var("PURERGB_TEST_VIDEO").ok().map(PathBuf::from).filter(|p| p.is_file()) {
        Some(p) => (p, "vidéo PURERGB_TEST_VIDEO".to_string()),
        None => (make_gif(&dir), "GIF généré 640x154".to_string()),
    };
    let item = media.import(&path).expect("import");
    eprintln!("source : {label} ({}x{}, {} ms, {} images)", item.width, item.height, item.duration_ms, item.frames);
    Bench { media, id: item.id, label }
}

/// Pilote d'essai : enregistre l'instant de chaque envoi et simule le coût du transfert.
struct Probe {
    info: ScreenInfo,
    times: Arc<Mutex<Vec<Instant>>>,
    send: Duration,
}

impl ScreenDriver for Probe {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }
    fn push_frame(&mut self, _: &Frame) -> Result<()> {
        std::thread::sleep(self.send);
        self.times.lock().push(Instant::now());
        Ok(())
    }
}

/// Écran « LY » simulé : encodage JPEG q95 identique au vrai pilote + transfert simulé
/// (5 ms + taille / 30 Mo/s, hypothèse USB 2 réaliste ; non mesuré sur le matériel).
struct LyLike {
    info: ScreenInfo,
    profile: Profile,
    times: Arc<Mutex<Vec<Instant>>>,
}

fn ly_profile() -> Profile {
    Profile { width: W, height: H, jpeg: true, big_endian: false, rotate: false, wire_row_px: 0, pm_sub: None }
}

impl LyLike {
    fn transfer(&self, bytes: usize) {
        std::thread::sleep(Duration::from_micros(5_000 + bytes as u64 * 1_000_000 / 30_000_000));
        self.times.lock().push(Instant::now());
    }
}

impl ScreenDriver for LyLike {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }
    fn push_frame(&mut self, f: &Frame) -> Result<()> {
        let img = wire::encode_frame(&self.profile, f, 0, 95)?;
        self.transfer(img.data.len());
        Ok(())
    }
    fn max_fps(&self) -> u8 {
        16
    }
    fn preparer(&self) -> Option<Arc<dyn super::driver::FramePreparer>> {
        wire::jpeg_preparer(&self.profile, 0, 95)
    }
    fn send_prepared(&mut self, img: &wire::WireImage) -> Result<()> {
        self.transfer(img.data.len());
        Ok(())
    }
}

fn stats(ms: &mut [f64]) -> (f64, f64, f64) {
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = ms.len().max(1) as f64;
    (ms.iter().sum::<f64>() / n, ms[ms.len() / 2], ms[(ms.len() * 95 / 100).min(ms.len() - 1)])
}

#[test]
#[ignore]
fn bench_render_and_jpeg() {
    let b = setup("render");
    let layout = layout_with_bg(&b.id);
    let store = ValueStore::empty();
    let _ = render::render(&layout, &store, &b.media, W, H, 0); // amorçage (1re image, cache)
    let mut render_ms = Vec::new();
    let mut last = Frame::new(W, H);
    for i in 0..60u64 {
        let t = Instant::now();
        last = render::render(&layout, &store, &b.media, W, H, i * 62);
        render_ms.push(t.elapsed().as_secs_f64() * 1e3);
        std::thread::sleep(Duration::from_millis(20));
    }
    let (avg, med, p95) = stats(&mut render_ms);
    eprintln!("RENDU {W}x{H} ({}) : moy {avg:.1} ms, méd {med:.1}, p95 {p95:.1}", b.label);
    for q in [95u8, 85, 75] {
        let mut ms = Vec::new();
        let mut size = 0;
        for _ in 0..20 {
            let t = Instant::now();
            let j = encode::rgba_to_jpeg(&last.rgba, W, H, q).expect("jpeg");
            ms.push(t.elapsed().as_secs_f64() * 1e3);
            size = j.len();
        }
        let (avg, med, _) = stats(&mut ms);
        eprintln!(
            "JPEG q{q} : moy {avg:.1} ms, méd {med:.1}, {} Ko, {:.1} Mo/s à 16 i/s ({:.1} Mo/s à 30 i/s)",
            size / 1000,
            size as f64 * 16.0 / 1e6,
            size as f64 * 30.0 / 1e6
        );
    }
    let t = Instant::now();
    let h = {
        use std::hash::{Hash, Hasher};
        let mut s = std::collections::hash_map::DefaultHasher::new();
        last.rgba.hash(&mut s);
        s.finish()
    };
    eprintln!("HACHAGE de trame : {:.1} ms (h={h})", t.elapsed().as_secs_f64() * 1e3);
}

fn info() -> ScreenInfo {
    ScreenInfo {
        id: "bench".into(),
        vendor: "b".into(),
        model: "b".into(),
        protocol: Protocol::Simulated,
        width: W,
        height: H,
        rotation: 0,
        resolution_confirmed: true,
        verified: true,
    }
}

/// Cadence réelle et gigue d'un écran pendant `secs` secondes, à `fps` demandés.
fn run_pacing(b: &Bench, label: &str, drv: Box<dyn ScreenDriver>, times: &Arc<Mutex<Vec<Instant>>>, fps: u8, secs: u64) {
    let cfg = ScreenConfig { layout: Some(layout_with_bg(&b.id)), brightness: 100, rotation: 0, fps };
    let r = Runner::start(drv, cfg, Arc::new(ValueStore::empty()), b.media.clone());
    std::thread::sleep(Duration::from_secs(secs));
    let pf = r.perf();
    let perf = format!("{} (visé {})", r.shared.cfg.lock().fps, pf.target_fps);
    r.stop();
    let t = times.lock().clone();
    let mut dt: Vec<f64> = t.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1e3).collect();
    if dt.len() < 3 {
        eprintln!("PACING {label} fps={fps} : trop peu d'images ({})", t.len());
        return;
    }
    let span = (t[t.len() - 1] - t[0]).as_secs_f64();
    let real = (t.len() - 1) as f64 / span;
    let mean = dt.iter().sum::<f64>() / dt.len() as f64;
    let sd = (dt.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / dt.len() as f64).sqrt();
    let (_, _, p95) = stats(&mut dt);
    let max = dt.iter().cloned().fold(0.0, f64::max);
    eprintln!(
        "PACING {label}, demandé {perf} i/s : réel {real:.2} i/s, intervalle moy {mean:.1} ms, écart-type {sd:.1}, p95 {p95:.1}, max {max:.1} | rendu {:.1} ms, encodage {:.1} ms, envoi {:.1} ms, abandonnées {}, q{}",
        pf.render_ms, pf.encode_ms, pf.send_ms, pf.dropped, pf.quality
    );
}

#[test]
#[ignore]
fn bench_runner_pacing() {
    let b = setup("pacing");
    for (fps, send) in [(16u8, 5u64), (16, 30), (30, 5), (30, 40)] {
        let times = Arc::new(Mutex::new(Vec::new()));
        let drv = Probe { info: info(), times: times.clone(), send: Duration::from_millis(send) };
        run_pacing(&b, &format!("écran générique (envoi {send} ms)"), Box::new(drv), &times, fps, 6);
    }
    for fps in [16u8, 30] {
        let times = Arc::new(Mutex::new(Vec::new()));
        let drv = LyLike { info: info(), profile: ly_profile(), times: times.clone() };
        run_pacing(&b, "LY simulé (JPEG q95 + 5 ms + 30 Mo/s)", Box::new(drv), &times, fps, 8);
    }
}

#[test]
#[ignore]
fn bench_media_sampling() {
    let b = setup("sample");
    let _ = b.media.frame_at(&b.id, 0);
    for hz in [16u64, 30] {
        let period = Duration::from_micros(1_000_000 / hz);
        let start = Instant::now();
        let mut due = start;
        let (mut n, mut dup) = (0u32, 0u32);
        let mut prev: Option<*const tiny_skia::Pixmap> = None;
        while start.elapsed() < Duration::from_secs(5) {
            let t = start.elapsed().as_millis() as u64;
            if let Some(f) = b.media.frame_at(&b.id, t) {
                let p = Arc::as_ptr(&f);
                if prev == Some(p) {
                    dup += 1;
                }
                prev = Some(p);
                n += 1;
            }
            due += period;
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
        }
        eprintln!(
            "ÉCHANTILLONNAGE {hz} Hz ({}) : {n} lectures, {dup} images identiques consécutives ({:.0} %)",
            b.label,
            dup as f64 * 100.0 / n.max(1) as f64
        );
    }
}
