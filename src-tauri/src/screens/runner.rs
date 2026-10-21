//! Un écran = trois threads en pipeline (rendu -> encodage -> envoi), chacun relié au suivant par
//! une boîte aux lettres à une image : si l'étage aval est en retard, l'image la plus ancienne est
//! jetée (jamais de file qui s'allonge). Le rendu suit des échéances absolues (`pacing::Pacer`).
//! Erreur d'envoi (câble débranché, logiciel constructeur ouvert…) : message exposé
//! à l'interface, nouvel essai toutes les 3 s, jamais d'arrêt silencieux.

use super::bindings::ValueStore;
use super::driver::{Frame, ScreenDriver, ScreenInfo};
use super::drivers::encode;
use super::layout::{Background, Layout, WidgetKind};
use super::media::MediaStore;
use super::pacing::{Mailbox, Pacer, Perf, PerfStats};
use super::pipeline::{self, Job};
use super::render;
use super::state::ScreenConfig;
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const RETRY_DELAY: Duration = Duration::from_secs(3);
/// Tranche maximale de sommeil (réactivité à l'arrêt et aux changements de design).
const SLICE: Duration = Duration::from_millis(10);
/// Dernière fenêtre attendue en cédant le processeur (les `sleep` système dépassent de 1 à 15 ms).
const SPIN: Duration = Duration::from_millis(2);
/// Cadence d'un design animé dont la cadence native est inconnue.
const ANIMATED_DEFAULT_FPS: f32 = 15.0;

pub struct Shared {
    pub cfg: Mutex<ScreenConfig>,
    pub error: Mutex<Option<String>>,
    /// Faux si l'écran n'accepte que des images fixes (voir `ScreenDriver::live_frames`).
    pub live: bool,
    pub perf: PerfStats,
    /// Demande d'envoi d'une image fixe mémorisée par l'écran.
    still: AtomicBool,
    pub(super) stop: AtomicBool,
    dirty: AtomicBool,
    /// Un étage a échoué : le rendu repart de zéro après `RETRY_DELAY`.
    pub(super) failed: AtomicBool,
    /// Période visée (µs), lue par l'encodeur pour adapter la qualité JPEG.
    pub(super) period_us: AtomicU64,
}

impl Shared {
    pub fn new(cfg: ScreenConfig, live: bool) -> Shared {
        Shared {
            cfg: Mutex::new(cfg),
            error: Mutex::new(None),
            live,
            perf: PerfStats::default(),
            still: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            dirty: AtomicBool::new(true),
            failed: AtomicBool::new(false),
            period_us: AtomicU64::new(100_000),
        }
    }

    /// Un étage signale une erreur : message pour l'interface, reprise à froid du rendu.
    pub(super) fn fail(&self, id: &str, e: &anyhow::Error) {
        log::warn!("écran {id} : {e:#}");
        *self.error.lock() = Some(format!("{e:#}"));
        self.failed.store(true, Ordering::Release);
    }
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
        let shared = Arc::new(Shared::new(cfg, driver.live_frames()));
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

    /// Mesures de cadence pour l'interface.
    pub fn perf(&self) -> Perf {
        self.shared.perf.snapshot(Instant::now())
    }

    /// Arrête le thread et libère le périphérique.
    pub fn stop(mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            // Un envoi USB bloqué ne doit jamais figer l'application : on attend au plus 3 s,
            // puis le thread est abandonné (il meurt avec le processus).
            let end = Instant::now() + Duration::from_secs(3);
            while !h.is_finished() && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(10));
            }
            if h.is_finished() {
                let _ = h.join();
            } else {
                log::warn!("écran {} : arrêt trop lent, thread abandonné", self.info.id);
            }
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

/// Cadence native la plus haute des GIF/vidéos du design (fond ou widget Image) ; `None` si aucun n'est animé.
pub fn animated_fps(layout: &Layout, media: &MediaStore) -> Option<f32> {
    let bg = match &layout.background {
        Background::Media { media_id, .. } => Some(media_id),
        _ => None,
    };
    let widgets = layout.widgets.iter().filter_map(|w| match &w.kind {
        WidgetKind::Image { media_id, .. } => Some(media_id),
        _ => None,
    });
    bg.into_iter()
        .chain(widgets)
        .filter(|id| media.is_animated(id))
        .map(|id| media.source_fps(id).unwrap_or(ANIMATED_DEFAULT_FPS))
        .reduce(f32::max)
}

/// Cadence effective : design fixe = réglage de l'utilisateur ; design animé = max(réglage, cadence
/// native de la source plafonnée à 30). Dans tous les cas plafonnée par le matériel (`cap`), 1 i/s au minimum.
pub fn effective_fps(user_fps: u8, source_fps: Option<f32>, cap: u8) -> u8 {
    let want = match source_fps {
        Some(s) => user_fps.max(s.round().clamp(1.0, 30.0) as u8),
        None => user_fps,
    };
    want.min(cap.max(1)).max(1)
}

fn hash_frame(f: &Frame) -> u64 {
    let mut h = DefaultHasher::new();
    f.rgba.hash(&mut h);
    h.finish()
}

fn period_of(fps: u8) -> Duration {
    Duration::from_micros(1_000_000 / u64::from(fps.max(1)))
}

/// Dort jusqu'à `due` : tranches de 10 ms, puis les 2 dernières ms en cédant le processeur.
/// Sort dès qu'un arrêt ou un changement est demandé.
fn sleep_until(shared: &Shared, due: Instant) {
    loop {
        if shared.stop.load(Ordering::Acquire) || shared.dirty.load(Ordering::Acquire) {
            return;
        }
        let left = due.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        if left > SPIN {
            std::thread::sleep((left - SPIN).min(SLICE));
        } else {
            std::thread::yield_now();
        }
    }
}

fn run_loop(driver: Box<dyn ScreenDriver>, shared: Arc<Shared>, store: Arc<ValueStore>, media: Arc<MediaStore>) {
    let info = driver.info().clone();
    let cap = driver.max_fps();
    let preparer = driver.preparer();
    let driver: pipeline::SharedDriver = Arc::new(Mutex::new(driver));
    let (to_enc, to_send) = (Arc::new(Mailbox::new()), Arc::new(Mailbox::new()));
    let enc = {
        let (s, id, a, b) = (shared.clone(), info.id.clone(), to_enc.clone(), to_send.clone());
        std::thread::Builder::new().name(format!("encode-{}", info.id)).spawn(move || pipeline::encode_stage(&s, &id, preparer, &a, &b))
    };
    let snd = {
        let (s, id, d, a) = (shared.clone(), info.id.clone(), driver.clone(), to_send.clone());
        std::thread::Builder::new().name(format!("send-{}", info.id)).spawn(move || pipeline::send_stage(&s, &id, &d, &a))
    };
    if enc.is_ok() && snd.is_ok() {
        render_loop(&driver, &shared, &info, cap, &store, &media, &to_enc);
    } else {
        log::error!("threads du pipeline d'écran {} : création impossible", info.id);
        *shared.error.lock() = Some("impossible de démarrer l'affichage (threads)".into());
    }
    to_enc.close();
    to_send.close();
    for h in [enc, snd].into_iter().flatten() {
        let _ = h.join();
    }
}

fn render_loop(
    driver: &pipeline::SharedDriver,
    shared: &Shared,
    info: &ScreenInfo,
    cap: u8,
    store: &ValueStore,
    media: &MediaStore,
    to_enc: &Mailbox<Job>,
) {
    let start = Instant::now();
    let mut pacer = Pacer::new(period_of(15), start);
    let (mut last_hash, mut fps_used) = (0u64, 0u8);
    let mut hw_brightness: Option<(u8, bool)> = None; // (valeur envoyée, géré par le matériel)
    let mut anim_fps: Option<f32> = None;
    while !shared.stop.load(Ordering::Acquire) {
        let changed = shared.dirty.swap(false, Ordering::AcqRel);
        let cfg = shared.cfg.lock().clone();
        if changed {
            anim_fps = cfg.layout.as_ref().and_then(|l| animated_fps(l, media));
        }
        let fps = effective_fps(cfg.fps, anim_fps, cap);
        if changed || fps != fps_used {
            fps_used = fps;
            pacer.reset(period_of(fps), Instant::now());
            shared.period_us.store(period_of(fps).as_micros() as u64, Ordering::Relaxed);
            shared.perf.set_target(f32::from(fps));
        }
        if shared.failed.swap(false, Ordering::AcqRel) {
            last_hash = 0;
            hw_brightness = None;
            sleep_until(shared, Instant::now() + RETRY_DELAY);
            pacer.reset(period_of(fps), Instant::now());
            continue;
        }

        if hw_brightness.map(|(v, _)| v) != Some(cfg.brightness) {
            let r = driver.lock().set_brightness(cfg.brightness);
            match r {
                Ok(handled) => hw_brightness = Some((cfg.brightness, handled)),
                Err(e) => {
                    shared.fail(&info.id, &e);
                    continue;
                }
            }
        }
        if shared.still.swap(false, Ordering::AcqRel) {
            let sent = compose(cfg.layout.as_ref(), &cfg, info, store, media, 0, true).and_then(|f| driver.lock().upload_still(&f));
            *shared.error.lock() = sent.err().map(|e| format!("{e:#}"));
        }
        if !shared.live {
            sleep_until(shared, Instant::now() + RETRY_DELAY);
            continue;
        }

        sleep_until(shared, pacer.due());
        if shared.stop.load(Ordering::Acquire) || shared.dirty.load(Ordering::Acquire) {
            continue;
        }
        let software = !hw_brightness.is_some_and(|(_, handled)| handled);
        // Instant d'affichage visé (échéance planifiée) : les GIF avancent sans gigue.
        let t_ms = pacer.due().saturating_duration_since(start).as_millis() as u64;
        let t = Instant::now();
        match compose(cfg.layout.as_ref(), &cfg, info, store, media, t_ms, software) {
            Ok(frame) => {
                shared.perf.render(t.elapsed());
                let animated = anim_fps.is_some();
                // Un design animé change presque à chaque image : inutile de hacher 3,5 Mo pour rien.
                let unchanged = !animated && {
                    let h = hash_frame(&frame);
                    std::mem::replace(&mut last_hash, h) == h
                };
                if !unchanged && to_enc.put(Job { frame, animated }) {
                    shared.perf.dropped(1);
                }
            }
            Err(e) => shared.fail(&info.id, &e),
        }
        let skipped = pacer.advance(Instant::now());
        if skipped > 0 {
            shared.perf.dropped(u64::from(skipped));
        }
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

    /// Médiathèque d'essai : un GIF de 2 images à 100 ms (10 i/s) et une image fixe.
    fn media_with_gif(tag: &str) -> (Arc<MediaStore>, String, String, std::path::PathBuf) {
        use image::{codecs::gif::GifEncoder, Delay, Frame, Rgba, RgbaImage};
        let dir = std::env::temp_dir().join(format!("purergb-runner-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dossier");
        let gif = dir.join("a.gif");
        {
            let mut enc = GifEncoder::new(std::fs::File::create(&gif).expect("gif"));
            for c in [[255, 0, 0, 255], [0, 0, 255, 255]] {
                let f = Frame::from_parts(RgbaImage::from_pixel(4, 4, Rgba(c)), 0, 0, Delay::from_numer_denom_ms(100, 1));
                enc.encode_frame(f).expect("frame");
            }
        }
        let png = dir.join("b.png");
        RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255])).save(&png).expect("png");
        let media = MediaStore::new(dir.join("media"));
        let (g, p) = (media.import(&gif).expect("import gif").id, media.import(&png).expect("import png").id);
        (Arc::new(media), g, p, dir)
    }

    fn gif_bg_layout(id: &str) -> Layout {
        serde_json::from_str(&format!(
            r##"{{"version":1,"name":"t","width":8,"height":8,"background":{{"kind":"media","media_id":"{id}","fit":"cover"}},"widgets":[]}}"##
        ))
        .expect("layout")
    }

    #[test]
    fn animated_media_sets_the_frame_rate() {
        let (media, g, p, dir) = media_with_gif("anim");
        let with = |bg: &str, widget: &str| -> Layout {
            serde_json::from_str(&format!(
                r##"{{"version":1,"name":"t","width":8,"height":8,"background":{bg},"widgets":[{widget}]}}"##
            ))
            .expect("layout")
        };
        let solid = r##"{"kind":"solid","color":"#000000"}"##;
        let bg = |id: &str| format!(r##"{{"kind":"media","media_id":"{id}","fit":"cover"}}"##);
        let img = |id: &str| format!(r##"{{"id":"i","x":0,"y":0,"w":4,"h":4,"type":"image","media_id":"{id}","fit":"cover"}}"##);
        assert_eq!(animated_fps(&with(solid, ""), &media), None);
        assert_eq!(animated_fps(&with(&bg(&p), &img(&p)), &media), None, "image fixe");
        assert_eq!(animated_fps(&with(&bg(&g), ""), &media).map(f32::round), Some(10.0), "GIF 10 i/s en fond");
        assert!(animated_fps(&with(solid, &img(&g)), &media).is_some(), "GIF en widget");
        assert_eq!(animated_fps(&with(&bg("absent"), ""), &media), None, "média inconnu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn effective_rate_follows_source_user_and_hardware_cap() {
        assert_eq!(effective_fps(5, None, 30), 5, "design fixe : réglage utilisateur");
        assert_eq!(effective_fps(5, Some(24.0), 30), 24, "animé : cadence de la source");
        assert_eq!(effective_fps(20, Some(10.0), 30), 20, "l'utilisateur peut demander plus");
        assert_eq!(effective_fps(5, Some(60.0), 30), 30, "source plafonnée à 30");
        assert_eq!(effective_fps(30, Some(60.0), 16), 16, "plafond du matériel (LY)");
        assert_eq!(effective_fps(0, None, 0), 1, "jamais 0");
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

    /// Écran qui compte les images reçues et expose ses instants d'envoi.
    struct Counter {
        sim: SimScreen,
        times: Arc<Mutex<Vec<Instant>>>,
    }

    impl ScreenDriver for Counter {
        fn info(&self) -> &ScreenInfo {
            self.sim.info()
        }
        fn push_frame(&mut self, f: &Frame) -> Result<()> {
            self.times.lock().push(Instant::now());
            self.sim.push_frame(f)
        }
    }

    #[test]
    fn animated_design_is_sent_at_the_requested_rate_without_bursts() {
        let (media, g, _, dir) = media_with_gif("rate");
        let times = Arc::new(Mutex::new(Vec::new()));
        let drv = Counter { sim: SimScreen::new(8, 8), times: times.clone() };
        let cfg = ScreenConfig { layout: Some(gif_bg_layout(&g)), brightness: 100, rotation: 0, fps: 20 };
        let r = Runner::start(Box::new(drv), cfg, Arc::new(ValueStore::empty()), media);
        std::thread::sleep(Duration::from_millis(2500));
        let perf = r.perf();
        r.stop();
        let t = times.lock().clone();
        // 20 i/s pendant ~2,5 s : ~50 images. Bornes larges (machine chargée), mais pas de rafale ni de sous-cadence nette.
        assert!((35..=53).contains(&t.len()), "{} images en 2,5 s à 20 i/s", t.len());
        let gaps: Vec<u128> = t.windows(2).map(|w| (w[1] - w[0]).as_millis()).collect();
        assert!(gaps.iter().filter(|g| **g < 25).count() <= 3, "rafales : {gaps:?}");
        assert!(perf.fps > 15.0 && perf.fps < 22.0, "cadence mesurée {}", perf.fps);
        assert_eq!(perf.target_fps, 20.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hardware_cap_limits_the_rate() {
        struct Capped(Counter);
        impl ScreenDriver for Capped {
            fn info(&self) -> &ScreenInfo {
                self.0.info()
            }
            fn max_fps(&self) -> u8 {
                8
            }
            fn push_frame(&mut self, f: &Frame) -> Result<()> {
                self.0.push_frame(f)
            }
        }
        let (media, g, _, dir) = media_with_gif("cap");
        let times = Arc::new(Mutex::new(Vec::new()));
        let drv = Capped(Counter { sim: SimScreen::new(8, 8), times: times.clone() });
        let cfg = ScreenConfig { layout: Some(gif_bg_layout(&g)), brightness: 100, rotation: 0, fps: 30 };
        let r = Runner::start(Box::new(drv), cfg, Arc::new(ValueStore::empty()), media);
        std::thread::sleep(Duration::from_millis(2000));
        r.stop();
        let n = times.lock().len();
        assert!((10..=17).contains(&n), "{n} images en 2 s à 8 i/s max");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
