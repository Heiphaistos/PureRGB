//! Un thread par écran : rend le design à sa cadence et envoie l'image si elle a changé.
//! Erreur d'envoi (câble débranché, logiciel constructeur ouvert…) : message exposé
//! à l'interface, nouvel essai toutes les 3 s, jamais d'arrêt silencieux.

use super::bindings::ValueStore;
use super::driver::{Frame, ScreenDriver, ScreenInfo};
use super::drivers::encode;
use super::layout::Layout;
use super::media::MediaStore;
use super::render;
use super::state::ScreenConfig;
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const RETRY_DELAY: Duration = Duration::from_secs(3);
const SLICE: Duration = Duration::from_millis(50);

pub struct Shared {
    pub cfg: Mutex<ScreenConfig>,
    pub error: Mutex<Option<String>>,
    /// Faux si l'écran n'accepte que des images fixes (voir `ScreenDriver::live_frames`).
    pub live: bool,
    /// Demande d'envoi d'une image fixe mémorisée par l'écran.
    still: AtomicBool,
    stop: AtomicBool,
    dirty: AtomicBool,
}

pub struct Runner {
    pub info: ScreenInfo,
    pub shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn start(
        driver: Box<dyn ScreenDriver>,
        cfg: ScreenConfig,
        store: Arc<ValueStore>,
        media: Arc<MediaStore>,
    ) -> Runner {
        let info = driver.info().clone();
        let shared = Arc::new(Shared {
            cfg: Mutex::new(cfg),
            error: Mutex::new(None),
            live: driver.live_frames(),
            still: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            dirty: AtomicBool::new(true),
        });
        let s = shared.clone();
        let handle = std::thread::Builder::new()
            .name(format!("screen-{}", info.id))
            .spawn(move || run_loop(driver, s, store, media))
            .map_err(|e| log::error!("thread écran: {e}"))
            .ok();
        Runner { info, shared, handle }
    }

    /// Demande l'envoi du design courant comme image fixe mémorisée (écrans sans flux direct).
    pub fn request_still(&self) {
        self.shared.still.store(true, Ordering::Release);
        self.touch();
    }

    /// Demande un nouveau rendu immédiat (changement de design/options).
    pub fn touch(&self) {
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn is_running(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| !h.is_finished())
    }

    /// Arrête le thread et libère le périphérique.
    pub fn stop(mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Rend le design aux dimensions de l'écran, avec rotation utilisateur et luminosité logicielle.
pub fn compose(
    layout: Option<&Layout>,
    cfg: &ScreenConfig,
    info: &ScreenInfo,
    store: &ValueStore,
    media: &MediaStore,
    t_ms: u64,
    software_brightness: bool,
) -> Result<Frame> {
    let (w, h) = (info.width, info.height);
    let turned = cfg.rotation == 90 || cfg.rotation == 270;
    let (rw, rh) = if turned { (h, w) } else { (w, h) };
    let mut frame = match layout {
        Some(l) => render::render(l, store, media, rw, rh, t_ms),
        None => Frame::new(rw, rh),
    };
    if cfg.rotation != 0 {
        let (rgba, nw, nh) = encode::rotate_rgba(&frame.rgba, rw, rh, cfg.rotation)?;
        frame = Frame { width: nw, height: nh, rgba };
    }
    if software_brightness && cfg.brightness < 100 {
        encode::apply_brightness(&mut frame.rgba, cfg.brightness);
    }
    Ok(frame)
}

fn hash_frame(f: &Frame) -> u64 {
    let mut h = DefaultHasher::new();
    f.rgba.hash(&mut h);
    h.finish()
}

/// Dort `d` par tranches en sortant dès qu'un arrêt ou un changement est demandé.
fn nap(shared: &Shared, d: Duration) {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if shared.stop.load(Ordering::Acquire) || shared.dirty.load(Ordering::Acquire) {
            return;
        }
        std::thread::sleep(SLICE.min(end.saturating_duration_since(Instant::now())));
    }
}

fn run_loop(
    mut driver: Box<dyn ScreenDriver>,
    shared: Arc<Shared>,
    store: Arc<ValueStore>,
    media: Arc<MediaStore>,
) {
    let info = driver.info().clone();
    let start = Instant::now();
    let mut last_hash = 0u64;
    let mut hw_brightness: Option<(u8, bool)> = None; // (valeur envoyée, géré par le matériel)
    while !shared.stop.load(Ordering::Acquire) {
        shared.dirty.store(false, Ordering::Release);
        let cfg = shared.cfg.lock().clone();

        if hw_brightness.map(|(v, _)| v) != Some(cfg.brightness) {
            match driver.set_brightness(cfg.brightness) {
                Ok(handled) => hw_brightness = Some((cfg.brightness, handled)),
                Err(e) => {
                    *shared.error.lock() = Some(format!("{e:#}"));
                    nap(&shared, RETRY_DELAY);
                    continue;
                }
            }
        }
        if shared.still.swap(false, Ordering::AcqRel) {
            let sent = compose(cfg.layout.as_ref(), &cfg, &info, &store, &media, 0, true).and_then(|f| driver.upload_still(&f));
            *shared.error.lock() = sent.err().map(|e| format!("{e:#}"));
        }
        if !shared.live {
            nap(&shared, RETRY_DELAY);
            continue;
        }
        let software = !hw_brightness.is_some_and(|(_, handled)| handled);
        let t_ms = start.elapsed().as_millis() as u64;
        let sent = compose(cfg.layout.as_ref(), &cfg, &info, &store, &media, t_ms, software).and_then(|frame| {
            let h = hash_frame(&frame);
            if h == last_hash {
                return Ok(());
            }
            driver.push_frame(&frame)?;
            last_hash = h;
            Ok(())
        });
        match sent {
            Ok(()) => *shared.error.lock() = None,
            Err(e) => {
                log::warn!("écran {} : {e:#}", info.id);
                *shared.error.lock() = Some(format!("{e:#}"));
                last_hash = 0;
                hw_brightness = None;
                nap(&shared, RETRY_DELAY);
                continue;
            }
        }
        nap(&shared, Duration::from_millis(1000 / u64::from(cfg.fps.max(1))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::sim::SimScreen;

    fn solid(color: &str) -> Layout {
        serde_json::from_str(&format!(
            r##"{{"version":1,"name":"t","width":8,"height":8,"background":{{"kind":"solid","color":"{color}"}},"widgets":[]}}"##
        ))
        .unwrap()
    }

    #[test]
    fn runner_pushes_rotated_dimmed_frames_to_the_sim() {
        let store = Arc::new(ValueStore::empty());
        let media = Arc::new(MediaStore::new(std::env::temp_dir().join("purergb-runner-test")));
        let sim = SimScreen::new(8, 4);
        let last = sim.last.clone();
        let cfg = ScreenConfig { layout: Some(solid("#ffffff")), brightness: 50, rotation: 90, fps: 30 };
        let r = Runner::start(Box::new(sim), cfg, store, media);
        let deadline = Instant::now() + Duration::from_secs(3);
        while last.lock().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let f = last.lock().clone().expect("aucune image reçue");
        r.stop();
        assert_eq!((f.width, f.height), (8, 4), "rotation 90 doit rendre les dimensions de l'écran");
        assert!(f.rgba[0] < 200 && f.rgba[0] > 100, "luminosité 50 % attendue, obtenu {}", f.rgba[0]);
    }
}
