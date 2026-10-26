//! Thread dédié (jamais le thread d'interface) : lit la manette physique par XInput, applique le
//! profil, émet vers une manette Xbox 360 virtuelle ViGEmBus. La manette virtuelle est détruite
//! quand le réglage est coupé, en mode jeu, si la manette physique disparaît et à la fermeture.

use super::logic::{center_from_samples, pick_physical, transform};
use super::model::{PadState, Profile};
use crate::AppState;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

static STOP: AtomicBool = AtomicBool::new(false);
static VIRTUAL_UP: AtomicBool = AtomicBool::new(false);
/// Échantillons de calibration en cours (sticks au repos), `Some` = mesure demandée.
static CALIB: Mutex<Option<Vec<[i16; 4]>>> = Mutex::new(None);
static LIVE: Mutex<Live> = Mutex::new(Live::EMPTY);
const CALIB_SAMPLES: usize = 90;

#[derive(Debug, Clone, Serialize)]
pub struct Live {
    pub virtual_active: bool,
    /// Clé de la manette physique lue (`xinput<N>`), profils assignés par cette clé.
    pub pad_key: Option<String>,
    pub raw: PadState,
    pub out: PadState,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl Live {
    const EMPTY: Live = Live { virtual_active: false, pad_key: None, raw: pad0(), out: pad0(), error: None, message: None };
}

const fn pad0() -> PadState {
    PadState { buttons: 0, lt: 0, rt: 0, lx: 0, ly: 0, rx: 0, ry: 0 }
}

pub fn live() -> Live {
    LIVE.lock().clone()
}

/// Demande une mesure de dérive (1 s, sticks au repos) ; appliquée au profil de la manette lue.
pub fn request_calibration() {
    *CALIB.lock() = Some(Vec::with_capacity(CALIB_SAMPLES));
    LIVE.lock().message = Some("Mesure en cours : ne touchez pas aux sticks.".into());
}

/// Arrête le thread et attend (au plus 1,5 s) que la manette virtuelle soit détruite.
pub fn stop() {
    STOP.store(true, Ordering::Release);
    let t = Instant::now();
    while VIRTUAL_UP.load(Ordering::Acquire) && t.elapsed() < Duration::from_millis(1500) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Instantané des réglages relus ~4 fois par seconde.
struct Cfg {
    enabled: bool,
    profiles: BTreeMap<String, Profile>,
    assign: BTreeMap<String, String>,
}

fn read_cfg(app: &AppHandle) -> Cfg {
    let s = app.state::<AppState>();
    let s = s.settings.lock();
    Cfg { enabled: s.gamepad_remap_enabled, profiles: s.gamepad_remap_profiles.clone(), assign: s.gamepad_remap_assign.clone() }
}

/// Profil de la manette `key` (identité si aucun n'est assigné).
fn profile_for(cfg: &Cfg, key: &str) -> (String, Profile) {
    let name = cfg.assign.get(key).cloned().unwrap_or_else(|| crate::settings::DEFAULT_REMAP_PROFILE.to_string());
    let p = cfg.profiles.get(&name).cloned().unwrap_or_default();
    (name, p)
}

/// Enregistre le centre mesuré dans le profil assigné à `key` (créé au besoin).
fn store_center(app: &AppHandle, key: &str, samples: &[[i16; 4]]) -> String {
    let l: Vec<(i16, i16)> = samples.iter().map(|s| (s[0], s[1])).collect();
    let r: Vec<(i16, i16)> = samples.iter().map(|s| (s[2], s[3])).collect();
    let (Some(lc), Some(rc)) = (center_from_samples(&l), center_from_samples(&r)) else {
        return "Mesure refusée : un stick a bougé pendant la mesure. Recommencez sans y toucher.".into();
    };
    let state = app.state::<AppState>();
    let mut s = state.settings.lock();
    let name = s.gamepad_remap_assign.get(key).cloned().unwrap_or_else(|| crate::settings::DEFAULT_REMAP_PROFILE.to_string());
    let p = s.gamepad_remap_profiles.entry(name).or_default();
    (p.left_stick.center_x, p.left_stick.center_y) = lc;
    (p.right_stick.center_x, p.right_stick.center_y) = rc;
    p.sanitize();
    match crate::settings::save(&s) {
        Ok(()) => "Dérive mesurée et enregistrée dans le profil.".into(),
        Err(e) => format!("Dérive mesurée mais non enregistrée : {e:#}"),
    }
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    if let Err(e) = std::thread::Builder::new().name("gamepad-remap".into()).spawn(move || imp::run(&app)) {
        log::warn!("thread gamepad-remap : {e}");
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn run(_: &tauri::AppHandle) {}
}

#[cfg(windows)]
mod imp {
    use super::*;
    use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};
    use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};

    struct Active {
        target: Xbox360Wired<Client>,
        pslot: u32,
        last: Option<PadState>,
    }

    fn read_slot(slot: u32) -> Option<PadState> {
        let mut st = XINPUT_STATE::default();
        // SAFETY: pointeur vers une structure locale valide ; slot < 4.
        if unsafe { XInputGetState(slot, &mut st) } != 0 {
            return None;
        }
        let g = st.Gamepad;
        let s = PadState { buttons: g.wButtons.0, lt: g.bLeftTrigger, rt: g.bRightTrigger, lx: g.sThumbLX, ly: g.sThumbLY, rx: g.sThumbRX, ry: g.sThumbRY };
        Some(s)
    }

    fn connected() -> [bool; 4] {
        let mut c = [false; 4];
        for (i, v) in c.iter_mut().enumerate() {
            *v = read_slot(i as u32).is_some();
        }
        c
    }

    fn plug(pslot: u32) -> Result<Active, String> {
        let client = Client::connect().map_err(|e| format!("Pilote ViGEmBus indisponible ({e})"))?;
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin().map_err(|e| format!("Création de la manette virtuelle impossible ({e})"))?;
        target.wait_ready().map_err(|e| format!("Manette virtuelle non prête ({e})"))?;
        // La manette virtuelle ne doit jamais être relue comme manette physique.
        let vslot = target.get_user_index().map_err(|e| format!("Emplacement XInput de la manette virtuelle inconnu ({e})"))?;
        if vslot == pslot {
            return Err("Conflit d'emplacement XInput avec la manette physique".into());
        }
        Ok(Active { target, pslot, last: None })
    }

    fn emit(a: &mut Active, o: &PadState) -> Result<(), String> {
        let g = XGamepad { buttons: XButtons(o.buttons), left_trigger: o.lt, right_trigger: o.rt, thumb_lx: o.lx, thumb_ly: o.ly, thumb_rx: o.rx, thumb_ry: o.ry };
        a.target.update(&g).map_err(|e| format!("Envoi vers la manette virtuelle échoué ({e})"))
    }

    fn set_live(f: impl FnOnce(&mut Live)) {
        f(&mut LIVE.lock());
    }

    pub fn run(app: &AppHandle) {
        let mut cfg = read_cfg(app);
        let mut cfg_at = Instant::now();
        let mut active: Option<Active> = None;
        let mut timer = false;
        let mut retry_at = Instant::now();
        loop {
            if STOP.load(Ordering::Acquire) {
                break;
            }
            if cfg_at.elapsed() > Duration::from_millis(250) {
                cfg = read_cfg(app);
                cfg_at = Instant::now();
            }
            let allowed = cfg.enabled && !crate::gamemode::active();
            if !allowed {
                if active.take().is_some() {
                    VIRTUAL_UP.store(false, Ordering::Release);
                    set_live(|l| *l = Live::EMPTY);
                }
                if timer {
                    // SAFETY: appariement avec timeBeginPeriod(1).
                    unsafe { timeEndPeriod(1) };
                    timer = false;
                }
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            if !timer {
                // SAFETY: résolution d'horloge 1 ms, relâchée à l'arrêt.
                unsafe { timeBeginPeriod(1) };
                timer = true;
            }
            let Some(a) = active.as_mut() else {
                if Instant::now() >= retry_at {
                    retry_at = Instant::now() + Duration::from_secs(2);
                    match pick_physical(&connected(), None) {
                        None => set_live(|l| {
                            l.error = Some("Aucune manette physique XInput détectée (les manettes Sony/génériques ne sont pas lues).".into())
                        }),
                        Some(p) => match plug(p) {
                            Ok(a) => {
                                active = Some(a);
                                VIRTUAL_UP.store(true, Ordering::Release);
                                set_live(|l| {
                                    l.virtual_active = true;
                                    l.error = None;
                                });
                            }
                            Err(e) => set_live(|l| l.error = Some(e)),
                        },
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };
            let key = format!("xinput{}", a.pslot);
            let Some(raw) = read_slot(a.pslot) else {
                // Manette débranchée : la virtuelle disparaît aussi.
                active = None;
                VIRTUAL_UP.store(false, Ordering::Release);
                set_live(|l| *l = Live::EMPTY);
                continue;
            };
            {
                let mut done = None;
                if let Some(buf) = CALIB.lock().as_mut() {
                    buf.push([raw.lx, raw.ly, raw.rx, raw.ry]);
                    if buf.len() >= CALIB_SAMPLES {
                        done = Some(std::mem::take(buf));
                    }
                }
                if let Some(samples) = done {
                    *CALIB.lock() = None;
                    let m = store_center(app, &key, &samples);
                    set_live(|l| l.message = Some(m));
                    cfg = read_cfg(app);
                }
                let out = transform(&profile_for(&cfg, &key).1, &raw);
                if a.last != Some(out) {
                    if let Err(e) = emit(a, &out) {
                        log::warn!("gamepad-remap : {e}");
                        active = None;
                        VIRTUAL_UP.store(false, Ordering::Release);
                        set_live(|l| {
                            *l = Live::EMPTY;
                            l.error = Some(e);
                        });
                        continue;
                    }
                    a.last = Some(out);
                }
                set_live(|l| {
                    l.pad_key = Some(key);
                    l.raw = raw;
                    l.out = out;
                });
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // Fermeture : la manette virtuelle est débranchée (Drop de Xbox360Wired) avant de rendre la main.
        drop(active);
        VIRTUAL_UP.store(false, Ordering::Release);
        set_live(|l| *l = Live::EMPTY);
        if timer {
            // SAFETY: appariement avec timeBeginPeriod(1).
            unsafe { timeEndPeriod(1) };
        }
    }
}
