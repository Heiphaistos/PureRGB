//! Médiathèque des écrans : images (PNG/JPG/BMP/WebP) et GIF animés copiés dans
//! `<racine>/<id>.<ext>` (+ fiche `<id>.json`). L'id est dérivé du contenu
//! (SHA-256 tronqué) : réimporter le même fichier ne duplique rien.
//! Les vidéos (mp4, avi, wmv...) sont copiées de même ; leur lecture passe par un
//! `VideoPlayer` par média (Media Foundation de Windows, voir `video.rs`).

use anyhow::{bail, Context, Result};
use image::{AnimationDecoder, ImageFormat};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use super::video::{self, VideoPlayer, VIDEO_EXTS};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tiny_skia::Pixmap;

pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 8192;
/// Mémoire maximale d'un média décodé (GIF : toutes les images) et du cache.
const MAX_DECODED_BYTES: usize = 512 * 1024 * 1024;
const CACHE_BYTES: usize = 256 * 1024 * 1024;
pub const EXTS: [&str; 5] = ["png", "jpg", "bmp", "webp", "gif"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Gif,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub name: String,
    pub kind: MediaKind,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub duration_ms: u32,
}

struct Anim {
    frames: Vec<(Arc<Pixmap>, u32)>,
    total_ms: u64,
    bytes: usize,
}

pub struct MediaStore {
    root: PathBuf,
    /// LRU : le plus récemment utilisé en dernier.
    cache: Mutex<Vec<(String, Arc<Anim>)>>,
    /// Un lecteur par vidéo, partagé entre l'aperçu et les écrans réels. `None` = ouverture
    /// échouée (mémorisée pour ne pas retenter à chaque image).
    players: Mutex<HashMap<String, Option<Arc<VideoPlayer>>>>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn premultiplied(w: u32, h: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut pm = Pixmap::new(w, h)?;
    for (dst, src) in pm.data_mut().chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
        let a = src[3] as u32;
        for i in 0..3 {
            dst[i] = ((src[i] as u32 * a + 127) / 255) as u8;
        }
        dst[3] = src[3];
    }
    Some(pm)
}

/// Décode toutes les images (une seule hors GIF). Garde-fous mémoire inclus.
fn decode(bytes: &[u8], format: ImageFormat) -> Result<Vec<(Arc<Pixmap>, u32)>> {
    let mut out = Vec::new();
    let mut total = 0usize;
    let mut push = |rgba: &image::RgbaImage, ms: u32| -> Result<()> {
        total += rgba.as_raw().len();
        if total > MAX_DECODED_BYTES {
            bail!("média trop lourd une fois décodé (animation trop longue ou trop grande)");
        }
        let pm = premultiplied(rgba.width(), rgba.height(), rgba.as_raw()).context("image vide")?;
        out.push((Arc::new(pm), ms));
        Ok(())
    };
    if format == ImageFormat::Gif {
        let dec = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).context("GIF illisible")?;
        for frame in dec.into_frames() {
            let frame = frame.context("image GIF illisible")?;
            let (n, d) = frame.delay().numer_denom_ms();
            let ms = if d == 0 { 100 } else { n / d };
            push(frame.buffer(), if ms < 20 { 100 } else { ms })?;
        }
    } else {
        let img = image::load_from_memory_with_format(bytes, format).context("image illisible")?;
        push(&img.to_rgba8(), 0)?;
    }
    if out.is_empty() {
        bail!("aucune image dans le fichier");
    }
    Ok(out)
}

impl MediaStore {
    /// `root` : en production `%APPDATA%\PureRGB\screens\media` ; injectable pour les tests.
    pub fn new(root: PathBuf) -> MediaStore {
        MediaStore { root, cache: Mutex::new(Vec::new()), players: Mutex::new(HashMap::new()) }
    }

    pub fn import(&self, path: &Path) -> Result<MediaItem> {
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if VIDEO_EXTS.contains(&ext.as_str()) {
            return self.import_video(path, &ext);
        }
        let meta = std::fs::metadata(path).context("fichier introuvable")?;
        if !meta.is_file() {
            bail!("ce n'est pas un fichier");
        }
        if meta.len() > MAX_FILE_BYTES {
            bail!("fichier trop gros (max {} Mo)", MAX_FILE_BYTES / 1024 / 1024);
        }
        let bytes = std::fs::read(path).context("lecture du fichier")?;
        let format = image::guess_format(&bytes).map_err(|_| anyhow::anyhow!("format non supporté (PNG, JPG, BMP, WebP, GIF)"))?;
        let ext = match format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Bmp => "bmp",
            ImageFormat::WebP => "webp",
            ImageFormat::Gif => "gif",
            _ => bail!("format non supporté (PNG, JPG, BMP, WebP, GIF)"),
        };
        let (w, h) = image::ImageReader::with_format(Cursor::new(&bytes), format).into_dimensions().context("dimensions illisibles")?;
        if w == 0 || h == 0 || w > MAX_DIMENSION || h > MAX_DIMENSION {
            bail!("image trop grande (max {MAX_DIMENSION} px)");
        }
        let frames = decode(&bytes, format)?;
        let duration_ms = frames.iter().map(|f| f.1).sum();
        let id: String = Sha256::digest(&bytes).iter().take(8).map(|b| format!("{b:02x}")).collect();
        let name: String = path
            .file_stem()
            .map(|s| s.to_string_lossy().chars().filter(|c| !c.is_control()).take(80).collect())
            .unwrap_or_default();
        let item = MediaItem {
            id: id.clone(),
            name,
            kind: if frames.len() > 1 { MediaKind::Gif } else { MediaKind::Image },
            width: w,
            height: h,
            frames: frames.len() as u32,
            duration_ms,
        };
        if let Some(existing) = self.read_item(&id) {
            return Ok(existing);
        }
        std::fs::create_dir_all(&self.root).context("création du dossier médias")?;
        std::fs::write(self.root.join(format!("{id}.{ext}")), &bytes).context("copie du média")?;
        let json = serde_json::to_vec(&item).context("fiche média")?;
        std::fs::write(self.root.join(format!("{id}.json")), json).context("écriture de la fiche")?;
        Ok(item)
    }

    /// Installe un média connu d'avance (intégré à l'application) sans le sonder ni le décoder.
    pub fn install_known(&self, item: &MediaItem, ext: &str, bytes: &[u8]) -> Result<()> {
        if !valid_id(&item.id) || !EXTS.iter().chain(VIDEO_EXTS.iter()).any(|e| *e == ext) {
            bail!("média intégré invalide");
        }
        std::fs::create_dir_all(&self.root).context("création du dossier médias")?;
        std::fs::write(self.root.join(format!("{}.{ext}", item.id)), bytes).context("copie du média")?;
        std::fs::write(self.root.join(format!("{}.json", item.id)), serde_json::to_vec(item).context("fiche média")?)
            .context("écriture de la fiche")
    }

    /// Fichier média `<id>.<ext>` de la médiathèque (export de thème).
    pub fn file_of(&self, id: &str) -> Option<PathBuf> {
        if !valid_id(id) {
            return None;
        }
        EXTS.iter().chain(VIDEO_EXTS.iter()).map(|e| self.root.join(format!("{id}.{e}"))).find(|p| p.is_file())
    }

    /// Vidéo : sondée par Windows avant la copie (un fichier illisible n'entre pas dans la médiathèque).
    fn import_video(&self, path: &Path, ext: &str) -> Result<MediaItem> {
        let ext = VIDEO_EXTS.iter().find(|e| **e == ext).context("extension vidéo inconnue")?;
        let meta = std::fs::metadata(path).context("fichier introuvable")?;
        if !meta.is_file() {
            bail!("ce n'est pas un fichier");
        }
        if meta.len() > MAX_FILE_BYTES {
            bail!("fichier trop gros (max {} Mo)", MAX_FILE_BYTES / 1024 / 1024);
        }
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path).context("lecture du fichier")?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = file.read(&mut buf).context("lecture du fichier")?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let id: String = hasher.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
        if let Some(existing) = self.read_item(&id) {
            return Ok(existing);
        }
        let info = video::probe(path)?;
        let name: String = path
            .file_stem()
            .map(|s| s.to_string_lossy().chars().filter(|c| !c.is_control()).take(80).collect())
            .unwrap_or_default();
        let item = MediaItem { id: id.clone(), name, kind: MediaKind::Video, width: info.width, height: info.height, frames: (info.fps * info.duration_ms as f32 / 1000.0).round() as u32, duration_ms: info.duration_ms };
        std::fs::create_dir_all(&self.root).context("création du dossier médias")?;
        let dest = self.root.join(format!("{id}.{ext}"));
        std::fs::copy(path, &dest).context("copie du média")?;
        let json = serde_json::to_vec(&item).context("fiche média")?;
        if let Err(e) = std::fs::write(self.root.join(format!("{id}.json")), json) {
            let _ = std::fs::remove_file(&dest);
            return Err(anyhow::Error::new(e).context("écriture de la fiche"));
        }
        Ok(item)
    }

    /// Lecteur de la vidéo `id` (créé à la demande) s'il s'agit bien d'une vidéo de la médiathèque.
    fn player(&self, id: &str) -> Option<Arc<VideoPlayer>> {
        let mut players = self.players.lock();
        if let Some(p) = players.get(id) {
            return p.clone();
        }
        let path = VIDEO_EXTS.iter().map(|e| self.root.join(format!("{id}.{e}"))).find(|p| p.is_file())?;
        let player = match VideoPlayer::open(&path) {
            Ok(p) => Some(Arc::new(p)),
            Err(e) => {
                log::warn!("vidéo {id} illisible: {e:#}");
                None
            }
        };
        players.insert(id.to_string(), player.clone());
        player
    }

    fn read_item(&self, id: &str) -> Option<MediaItem> {
        let raw = std::fs::read(self.root.join(format!("{id}.json"))).ok()?;
        serde_json::from_slice(&raw).ok()
    }

    pub fn list(&self) -> Vec<MediaItem> {
        let Ok(dir) = std::fs::read_dir(&self.root) else { return Vec::new() };
        let mut items: Vec<MediaItem> = dir
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                let id = p.file_stem()?.to_str()?;
                (p.extension()? == "json" && valid_id(id)).then(|| self.read_item(id)).flatten()
            })
            .collect();
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
        items
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        if !valid_id(id) {
            bail!("identifiant de média invalide");
        }
        if self.read_item(id).is_none() {
            bail!("média introuvable");
        }
        self.cache.lock().retain(|(k, _)| k != id);
        // Le lecteur tient le fichier ouvert : l'arrêter avant de le supprimer.
        if let Some(Some(p)) = self.players.lock().remove(id) {
            p.shutdown();
        }
        let _ = std::fs::remove_file(self.root.join(format!("{id}.json")));
        for ext in EXTS.iter().chain(VIDEO_EXTS.iter()) {
            let p = self.root.join(format!("{id}.{ext}"));
            if p.exists() {
                std::fs::remove_file(&p).with_context(|| format!("suppression de {}", p.display()))?;
            }
        }
        Ok(())
    }

    /// Vrai si le média est animé (GIF ou vidéo) : l'écran doit alors tourner à 15 i/s au moins.
    pub fn is_animated(&self, id: &str) -> bool {
        valid_id(id) && self.read_item(id).is_some_and(|m| matches!(m.kind, MediaKind::Gif | MediaKind::Video))
    }

    /// Cadence native (i/s) d'un GIF ou d'une vidéo ; `None` pour une image fixe ou un média inconnu.
    /// Les vidéos importées avant l'enregistrement du nombre d'images (frames = 0) comptent pour 24 i/s.
    pub fn source_fps(&self, id: &str) -> Option<f32> {
        let m = if valid_id(id) { self.read_item(id)? } else { return None };
        match m.kind {
            MediaKind::Gif | MediaKind::Video if m.frames > 0 && m.duration_ms > 0 => Some(m.frames as f32 * 1000.0 / m.duration_ms as f32),
            MediaKind::Video => Some(24.0),
            MediaKind::Gif => Some(10.0),
            _ => None,
        }
    }

    fn load(&self, id: &str) -> Option<Arc<Anim>> {
        {
            let mut c = self.cache.lock();
            if let Some(pos) = c.iter().position(|(k, _)| k == id) {
                let entry = c.remove(pos);
                let anim = entry.1.clone();
                c.push(entry);
                return Some(anim);
            }
        }
        let (path, format) = EXTS.iter().find_map(|ext| {
            let p = self.root.join(format!("{id}.{ext}"));
            let f = ImageFormat::from_extension(ext)?;
            p.is_file().then_some((p, f))
        })?;
        let bytes = std::fs::read(&path).ok()?;
        let frames = match decode(&bytes, format) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("média {id} illisible: {e:#}");
                return None;
            }
        };
        let bytes_used = frames.iter().map(|(p, _)| p.data().len()).sum();
        let total_ms = frames.iter().map(|f| f.1 as u64).sum();
        let anim = Arc::new(Anim { frames, total_ms, bytes: bytes_used });
        let mut c = self.cache.lock();
        c.push((id.to_string(), anim.clone()));
        while c.len() > 1 && c.iter().map(|(_, a)| a.bytes).sum::<usize>() > CACHE_BYTES {
            c.remove(0);
        }
        Some(anim)
    }

    /// Image du média à l'instant `t_ms` (GIF bouclé selon ses délais). Pour une vidéo,
    /// `t_ms` est ignoré : le temps de lecture est celui du lecteur (partagé entre aperçu et écrans).
    pub fn frame_at(&self, id: &str, t_ms: u64) -> Option<Arc<Pixmap>> {
        if !valid_id(id) {
            return None;
        }
        let is_image = self.cache.lock().iter().any(|(k, _)| k == id);
        if !is_image {
            if let Some(p) = self.player(id) {
                return p.frame();
            }
        }
        let anim = self.load(id)?;
        if anim.frames.len() == 1 || anim.total_ms == 0 {
            return anim.frames.first().map(|f| f.0.clone());
        }
        let mut t = t_ms % anim.total_ms;
        for (pm, ms) in &anim.frames {
            if t < *ms as u64 {
                return Some(pm.clone());
            }
            t -= *ms as u64;
        }
        anim.frames.last().map(|f| f.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::gif::GifEncoder, Delay, Frame, Rgba, RgbaImage};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("purergb-media-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn imports_png_and_dedups() {
        let dir = tmp("png");
        let src = dir.join("logo.png");
        RgbaImage::from_pixel(8, 4, Rgba([255, 0, 0, 255])).save(&src).unwrap();
        let store = MediaStore::new(dir.join("media"));
        let a = store.import(&src).unwrap();
        let b = store.import(&src).unwrap();
        assert_eq!((a.kind, a.width, a.height, a.frames), (MediaKind::Image, 8, 4, 1));
        assert_eq!(a.id, b.id);
        assert_eq!(store.list().len(), 1);
        let f = store.frame_at(&a.id, 0).unwrap();
        assert_eq!(f.pixel(0, 0).map(|p| p.red()), Some(255));
        store.delete(&a.id).unwrap();
        assert!(store.list().is_empty());
        assert!(store.frame_at(&a.id, 0).is_none());
    }

    #[test]
    fn gif_loops_by_delay() {
        let dir = tmp("gif");
        let src = dir.join("anim.gif");
        {
            let mut enc = GifEncoder::new(std::fs::File::create(&src).unwrap());
            for c in [[255, 0, 0, 255], [0, 0, 255, 255]] {
                let f = Frame::from_parts(RgbaImage::from_pixel(4, 4, Rgba(c)), 0, 0, Delay::from_numer_denom_ms(100, 1));
                enc.encode_frame(f).unwrap();
            }
        }
        let store = MediaStore::new(dir.join("media"));
        let item = store.import(&src).unwrap();
        assert_eq!((item.kind, item.frames, item.duration_ms), (MediaKind::Gif, 2, 200));
        let red = |t| store.frame_at(&item.id, t).and_then(|f| f.pixel(1, 1)).map(|p| p.red());
        assert_eq!(red(0), Some(255));
        assert_eq!(red(150), Some(0));
        assert_eq!(red(250), Some(255)); // boucle
    }

    #[cfg(windows)]
    #[test]
    fn video_import_play_and_delete() {
        let dir = tmp("video");
        let src = dir.join("mini.avi");
        std::fs::write(&src, crate::screens::video::tests::tiny_avi()).unwrap();
        let store = MediaStore::new(dir.join("media"));
        let item = store.import(&src).unwrap();
        assert_eq!((item.kind, item.width, item.height, item.frames), (MediaKind::Video, 32, 24, 5));
        assert!(item.duration_ms > 0);
        assert_eq!(store.import(&src).unwrap().id, item.id, "dédoublonné");
        assert_eq!(store.list().len(), 1);
        assert!(store.is_animated(&item.id));
        let f = store.frame_at(&item.id, 123_456).expect("première image");
        assert_eq!((f.width(), f.height()), (32, 24));
        // Même lecteur pour deux demandes (partagé entre aperçu et écran).
        assert!(Arc::ptr_eq(&store.player(&item.id).unwrap(), &store.player(&item.id).unwrap()));
        store.delete(&item.id).unwrap();
        assert!(store.list().is_empty());
        assert!(store.frame_at(&item.id, 0).is_none());
        assert!(!dir.join("media").join(format!("{}.avi", item.id)).exists(), "fichier copié supprimé");
    }

    #[test]
    fn unreadable_video_is_refused_without_copy() {
        let dir = tmp("badvideo");
        let store = MediaStore::new(dir.join("media"));
        let fake = dir.join("faux.mp4");
        std::fs::write(&fake, b"pas une video").unwrap();
        assert!(store.import(&fake).is_err());
        assert!(store.list().is_empty());
        let copies = std::fs::read_dir(dir.join("media")).map(|d| d.count()).unwrap_or(0);
        assert_eq!(copies, 0, "rien copié");
    }

    #[test]
    fn rejects_hostile_input() {
        let dir = tmp("bad");
        let store = MediaStore::new(dir.join("media"));
        let junk = dir.join("junk.png");
        std::fs::write(&junk, b"not an image at all").unwrap();
        assert!(store.import(&junk).is_err());
        assert!(store.import(&dir.join("absent.png")).is_err());
        assert!(store.import(&dir).is_err());
        let wide = dir.join("wide.png");
        RgbaImage::from_pixel(MAX_DIMENSION + 1, 1, Rgba([0, 0, 0, 255])).save(&wide).unwrap();
        assert!(store.import(&wide).is_err());
        for id in ["../x", "a/b", "..", "", "a\\b", "x.png"] {
            assert!(store.frame_at(id, 0).is_none(), "{id}");
            assert!(store.delete(id).is_err(), "{id}");
        }
    }
}
