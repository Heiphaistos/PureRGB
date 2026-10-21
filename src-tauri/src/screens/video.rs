//! Lecture de vidéos pour les fonds et widgets image.
//!
//! `VideoPlayer` = un thread de décodage paresseux (démarre à la première demande
//! d'image, boucle à la fin, s'arrête seul après quelques secondes sans demande) qui garde
//! quelques images décodées d'avance, horodatées sur l'horloge de lecture. `frame()` sert
//! l'image qui correspond à l'instant de la demande (pas « la dernière publiée »).
//! Le décodage lui-même est derrière le trait `FrameSource` : Media Foundation sous
//! Windows (`video_mf.rs`), un faux lecteur dans les tests.

use anyhow::{anyhow, bail, Result};
use parking_lot::{Condvar, Mutex};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tiny_skia::Pixmap;

/// Extensions vidéo acceptées à l'import (le décodage dépend des codecs installés dans Windows).
pub const VIDEO_EXTS: [&str; 7] = ["mp4", "m4v", "mov", "avi", "wmv", "mkv", "webm"];
/// Résolution maximale décodée (au-delà, Media Foundation redimensionne).
pub const MAX_OUT: (u32, u32) = (1920, 1080);
/// Côté maximal accepté d'un fichier source (le décodage 8K écraserait le PC).
pub const MAX_SOURCE_SIDE: u32 = 4096;
pub const MSG_CODEC: &str =
    "format/codec non pris en charge par Windows (installez les extensions vidéo du Microsoft Store)";

const IDLE_STOP: Duration = Duration::from_secs(3);
const FIRST_FRAME_WAIT: Duration = Duration::from_millis(2500);
const RETRY_AFTER_ERROR: Duration = Duration::from_secs(3);
const SLICE: Duration = Duration::from_millis(10);
/// Retard (ms) au-delà duquel l'horloge de lecture est recalée (décodage trop lent, veille...).
const MAX_LAG_MS: u64 = 500;
/// Images décodées d'avance par rapport à l'instant de lecture.
const AHEAD: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u32,
    pub fps: f32,
}

/// Une image décodée et son horodatage (ms depuis le début de la vidéo).
pub struct Decoded {
    pub pixmap: Pixmap,
    pub ts_ms: u64,
}

/// Pas de `Send` : créé et utilisé sur le seul thread de décodage (objets COM).
pub trait FrameSource {
    /// Image suivante ; `Ok(None)` en fin de flux.
    fn next_frame(&mut self) -> Result<Option<Decoded>>;
    /// Retour au début (boucle).
    fn rewind(&mut self) -> Result<()>;
}

type Factory = Arc<dyn Fn() -> Result<Box<dyn FrameSource>> + Send + Sync>;

#[derive(Default)]
struct Slot {
    /// Images décodées, croissantes : (horodatage de lecture en ms depuis `origin`, image).
    frames: VecDeque<(u64, Arc<Pixmap>)>,
    /// Heure réelle de l'horodatage 0 de la lecture (recalée si le décodage prend du retard).
    origin: Option<Instant>,
    error: Option<String>,
    failed_at: Option<Instant>,
}

impl Slot {
    fn pos_ms(&self, now: Instant) -> u64 {
        self.origin.map_or(0, |o| now.saturating_duration_since(o).as_millis() as u64)
    }

    /// Image à afficher à `now` : la dernière dont l'horodatage est atteint (sinon la première,
    /// en attente) ; les images plus anciennes sont libérées.
    fn pick(&mut self, now: Instant) -> Option<Arc<Pixmap>> {
        let pos = self.pos_ms(now);
        while self.frames.len() > 1 && self.frames.get(1).is_some_and(|(ts, _)| *ts <= pos) {
            self.frames.pop_front();
        }
        self.frames.front().map(|(_, f)| f.clone())
    }

    fn ahead(&self, now: Instant) -> usize {
        let pos = self.pos_ms(now);
        self.frames.iter().filter(|(ts, _)| *ts > pos).count()
    }
}

struct Shared {
    slot: Mutex<Slot>,
    /// Signalé à chaque image décodée (réveille `frame()`) et à chaque consommation.
    ready: Condvar,
    last_req: Mutex<Instant>,
    running: AtomicBool,
    stop: AtomicBool,
}

pub struct VideoPlayer {
    shared: Arc<Shared>,
    factory: Factory,
    idle: Duration,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Convertit un tampon BGRX (RGB32 de Media Foundation, octet X indéfini) en Pixmap opaque.
/// `stride` négatif : image stockée de bas en haut. Alpha = 255 donc déjà « prémultiplié ».
/// Le tampon fait `w`×`h` (taille codée, souvent arrondie : 1088 pour 1080) ; on garde le
/// coin haut-gauche `crop` = taille d'affichage.
pub fn bgrx_to_pixmap(w: u32, h: u32, stride: i32, data: &[u8], crop: (u32, u32)) -> Option<Pixmap> {
    let (cw, ch) = (crop.0.min(w), crop.1.min(h));
    let row = (w as usize).checked_mul(4)?;
    let step = stride.unsigned_abs() as usize;
    if step < row || data.len() < step.checked_mul(h as usize)?.checked_sub(step - row)? {
        return None;
    }
    let mut pm = Pixmap::new(cw, ch)?;
    for (y, dst) in pm.data_mut().chunks_exact_mut(cw as usize * 4).enumerate() {
        let src_row = if stride < 0 { h as usize - 1 - y } else { y };
        let src = &data[src_row * step..src_row * step + cw as usize * 4];
        for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            d.copy_from_slice(&[s[2], s[1], s[0], 255]);
        }
    }
    Some(pm)
}

/// Sonde un fichier (taille, durée, fps) ; erreur française si Windows ne sait pas le lire.
pub fn probe(path: &Path) -> Result<VideoInfo> {
    #[cfg(windows)]
    {
        let p = path.to_path_buf();
        // Thread dédié : COM/MF initialisés et libérés proprement, sans toucher au thread appelant.
        std::thread::Builder::new()
            .name("video-probe".into())
            .spawn(move || super::video_mf::MfSource::open(&p).map(|(_, info)| info))
            .map_err(|e| anyhow!("thread de sonde vidéo : {e}"))?
            .join()
            .map_err(|_| anyhow!("la sonde vidéo a planté"))?
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        bail!("lecture vidéo disponible uniquement sous Windows")
    }
}

impl VideoPlayer {
    /// Ouvre un fichier vidéo (sonde synchrone ; le décodage démarre à la première image demandée).
    pub fn open(path: &Path) -> Result<VideoPlayer> {
        probe(path)?;
        let path: PathBuf = path.to_path_buf();
        let factory: Factory = Arc::new(move || {
            #[cfg(windows)]
            {
                let (src, _) = super::video_mf::MfSource::open(&path)?;
                Ok(Box::new(src) as Box<dyn FrameSource>)
            }
            #[cfg(not(windows))]
            {
                let _ = &path;
                bail!("lecture vidéo disponible uniquement sous Windows")
            }
        });
        Ok(Self::with_factory(factory, IDLE_STOP))
    }

    fn with_factory(factory: Factory, idle: Duration) -> VideoPlayer {
        VideoPlayer {
            shared: Arc::new(Shared {
                slot: Mutex::new(Slot::default()),
                ready: Condvar::new(),
                last_req: Mutex::new(Instant::now()),
                running: AtomicBool::new(false),
                stop: AtomicBool::new(false),
            }),
            factory,
            idle,
            handle: Mutex::new(None),
        }
    }

    /// Dernière erreur de décodage, s'il y en a une (diagnostic des tests ; le journal la contient aussi).
    #[cfg(test)]
    fn last_error(&self) -> Option<String> {
        self.shared.slot.lock().error.clone()
    }

    /// Image à afficher maintenant. Lance le décodage si besoin ; à l'éveil attend la première image (2,5 s max).
    pub fn frame(&self) -> Option<Arc<Pixmap>> {
        let now = Instant::now();
        *self.shared.last_req.lock() = now;
        self.ensure_running();
        let mut slot = self.shared.slot.lock();
        if slot.frames.is_empty() && self.shared.running.load(Ordering::Acquire) {
            self.shared.ready.wait_for(&mut slot, FIRST_FRAME_WAIT);
        }
        let f = slot.pick(Instant::now());
        drop(slot);
        self.shared.ready.notify_all(); // le décodeur peut reprendre : de la place s'est libérée
        f
    }

    fn ensure_running(&self) {
        if self.shared.stop.load(Ordering::Acquire) {
            return;
        }
        let mut handle = self.handle.lock();
        if self.shared.running.load(Ordering::Acquire) {
            return;
        }
        if self.shared.slot.lock().failed_at.is_some_and(|t| t.elapsed() < RETRY_AFTER_ERROR) {
            return;
        }
        if let Some(old) = handle.take() {
            let _ = old.join(); // thread précédent déjà terminé (running == false)
        }
        self.shared.running.store(true, Ordering::Release);
        let (shared, factory, idle) = (self.shared.clone(), self.factory.clone(), self.idle);
        match std::thread::Builder::new().name("video-decode".into()).spawn(move || {
            if let Err(e) = decode_loop(&shared, &factory, idle) {
                log::warn!("vidéo : {e:#}");
                let mut s = shared.slot.lock();
                s.error = Some(format!("{e:#}"));
                s.failed_at = Some(Instant::now());
            }
            {
                let mut s = shared.slot.lock();
                s.frames.clear(); // libère les images ; la reprise repartira du début
                s.origin = None;
            }
            shared.running.store(false, Ordering::Release);
            shared.ready.notify_all();
        }) {
            Ok(h) => *handle = Some(h),
            Err(e) => {
                log::error!("thread vidéo : {e}");
                self.shared.running.store(false, Ordering::Release);
            }
        }
    }

    /// Arrête le décodage et libère le fichier (indispensable avant de le supprimer).
    pub fn shutdown(&self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.lock().take() {
            let _ = h.join();
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Attend (par tranches) qu'une place se libère dans l'avance de décodage ; faux si arrêt ou inactivité.
fn wait_room(shared: &Shared, idle: Duration) -> bool {
    loop {
        if shared.stop.load(Ordering::Acquire) || shared.last_req.lock().elapsed() > idle {
            return false;
        }
        let mut slot = shared.slot.lock();
        if slot.ahead(Instant::now()) < AHEAD {
            return true;
        }
        shared.ready.wait_for(&mut slot, SLICE);
    }
}

fn decode_loop(shared: &Shared, factory: &Factory, idle: Duration) -> Result<()> {
    let mut src = factory()?;
    // Horodatage de lecture = base (cumul des boucles passées) + ts de l'image - ts de la 1re image de la boucle.
    let (mut base, mut ts0): (u64, Option<u64>) = (0, None);
    let (mut since_rewind, mut prev_ts, mut last_dur) = (0u32, 0u64, 33u64);
    loop {
        if !wait_room(shared, idle) {
            return Ok(());
        }
        let Some(f) = src.next_frame()? else {
            if since_rewind == 0 {
                bail!("la vidéo ne contient aucune image");
            }
            // La dernière image garde sa durée normale avant de reboucler (boucle sans à-coup).
            base += prev_ts.saturating_sub(ts0.unwrap_or(0)) + last_dur;
            src.rewind()?;
            (since_rewind, ts0) = (0, None);
            continue;
        };
        if since_rewind > 0 && f.ts_ms > prev_ts {
            last_dur = (f.ts_ms - prev_ts).min(1000);
        }
        (since_rewind, prev_ts) = (since_rewind + 1, f.ts_ms);
        let abs = base + f.ts_ms.saturating_sub(*ts0.get_or_insert(f.ts_ms));
        let now = Instant::now();
        let mut slot = shared.slot.lock();
        // Première image, ou décodage en retard de plus de MAX_LAG_MS (veille, 4K...) : on recale l'horloge.
        if slot.origin.is_none() || slot.pos_ms(now) > abs + MAX_LAG_MS {
            slot.origin = Some(now.checked_sub(Duration::from_millis(abs)).unwrap_or(now));
            slot.frames.clear(); // l'ancien contenu est dans un autre repère de temps
        }
        slot.frames.push_back((abs, Arc::new(f.pixmap)));
        drop(slot);
        shared.ready.notify_all();
    }
}

#[cfg(test)]
#[path = "video_tests.rs"]
pub(crate) mod tests;
