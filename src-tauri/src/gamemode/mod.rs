//! Mode jeu : quand un jeu protégé par un anti-cheat tourne, PureRGB cesse tout accès matériel bas niveau
//! (serveur OpenRGB, SMBus/RAM, capteurs et ventilateurs carte mère via sensord/PawnIO, mini-écrans USB,
//! pilotes HID natifs) et reprend seul 10 s après la fermeture du jeu.
//!
//! Limites assumées : on n'injecte rien dans les jeux et on ne désinstalle JAMAIS le pilote PawnIO (modification
//! système) : on arrête seulement les processus qui l'utilisent. Le mode jeu réduit le risque ; seule la fermeture
//! de PureRGB le ramène à zéro. Décision pure et machine à états : `detect.rs`.

pub mod detect;

use crate::backends::hid::HidBackend;
use crate::backends::mobo::MoboFanBackend;
use crate::backends::openrgb::manager::OpenRgbManager;
use crate::backends::smbus::SmbusBackend;
use crate::commands::devices::scan_with_zone_sizes;
use crate::{restore_saved_state, screens, AppState};
use detect::{Phase, Rules, Transition};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, Manager, State};

/// Intervalle de relevé des processus (un relevé « noms seuls » coûte quelques ms).
const POLL: Duration = Duration::from_secs(4);
const MAX_GAMES: usize = 100;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SUSPENDED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static STATUS: Mutex<Status> = Mutex::new(Status { game: None, grace: false, external_openrgb: false });

#[derive(Debug, Clone, PartialEq, Serialize)]
struct Status {
    game: Option<String>,
    /// Jeu fermé : reprise dans quelques secondes.
    grace: bool,
    /// Un OpenRGB lancé par l'utilisateur (pas par PureRGB) répond encore : on ne peut pas l'arrêter.
    external_openrgb: bool,
}

/// Le mode jeu est-il en vigueur (accès matériel suspendu) ?
pub fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

/// Ce backend doit-il rester au repos ? (`DeviceRegistry` : aucun scan ni envoi pendant le mode jeu.)
pub fn backend_suspended(name: &str) -> bool {
    active() && SUSPENDED.lock().contains(&name)
}

/// Jeu détecté et reprise imminente (lecture seule, pour le diagnostic).
pub fn current_game() -> (Option<String>, bool) {
    let st = STATUS.lock();
    (st.game.clone(), st.grace)
}

fn set_gate(on: bool, keep_hid: bool) {
    *SUSPENDED.lock() = if on { detect::suspended_backends(keep_hid) } else { Vec::new() };
    ACTIVE.store(on, Ordering::Release);
}

fn running_names(sys: &mut System) -> Vec<String> {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().map(|p| p.name().to_string_lossy().to_string()).collect()
}

fn detect_now(sys: &mut System, rules: &Rules) -> Option<String> {
    let names = running_names(sys);
    rules.detect(names.iter().map(String::as_str))
}

/// Premier relevé, avant tout démarrage matériel : un jeu déjà lancé empêche OpenRGB/sensord/écrans de démarrer.
pub fn precheck(s: &crate::settings::Settings) -> Option<String> {
    if !s.game_mode {
        return None;
    }
    let game = detect_now(&mut System::new(), &Rules::load(&s.game_mode_games))?;
    set_gate(true, s.game_mode_keep_hid);
    STATUS.lock().game = Some(game.clone());
    log::info!("mode jeu actif dès le démarrage : {game}");
    Some(game)
}

fn publish(app: &AppHandle, f: impl FnOnce(&mut Status)) {
    let new = {
        let mut st = STATUS.lock();
        let mut n = st.clone();
        f(&mut n);
        if n == *st {
            return;
        }
        *st = n.clone();
        n
    };
    if let Err(e) = app.emit("game-mode-changed", &new) {
        log::debug!("émission game-mode-changed : {e:#}");
    }
}

fn notify(app: &AppHandle, body: String) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(e) = app.notification().builder().title("PureRGB").body(body).show() {
        log::debug!("notification mode jeu : {e:#}");
    }
}

fn enter(app: &AppHandle, game: &str, keep_hid: bool) {
    log::info!("mode jeu : {game} détecté, accès matériel suspendu");
    set_gate(true, keep_hid);
    let state = app.state::<AppState>();
    {
        let mut reg = state.registry.lock();
        for b in reg.backends_mut() {
            if let Some(m) = b.as_any_mut().downcast_mut::<MoboFanBackend>() {
                m.release_all(); // ventilateurs rendus au BIOS avant de couper sensord
            } else if let Some(sm) = b.as_any_mut().downcast_mut::<SmbusBackend>() {
                sm.set_native_enabled(false); // ferme le handle PawnIO
            } else if let (false, Some(hid)) = (keep_hid, b.as_any_mut().downcast_mut::<HidBackend>()) {
                hid.set_native_enabled(false);
            }
        }
    }
    state.sensors.stop();
    state.openrgb_mgr.stop(); // ne tue que l'OpenRGB lancé par PureRGB
    app.state::<Arc<screens::manager::ScreenManager>>().shutdown();
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    let external = OpenRgbManager::serves_openrgb(&host, port);
    publish(app, |s| {
        s.game = Some(game.to_string());
        s.grace = false;
        s.external_openrgb = external;
    });
    notify(app, format!("Mode jeu actif : accès matériel suspendu pour {game}"));
}

fn exit(app: &AppHandle) {
    log::info!("mode jeu : jeu fermé, reprise de l'accès matériel");
    set_gate(false, false);
    let state = app.state::<AppState>();
    let saved = state.settings.lock().clone();
    {
        let mut reg = state.registry.lock();
        for b in reg.backends_mut() {
            if let Some(sm) = b.as_any_mut().downcast_mut::<SmbusBackend>() {
                sm.set_native_enabled(saved.native_drivers_enabled);
            } else if let Some(hid) = b.as_any_mut().downcast_mut::<HidBackend>() {
                hid.set_native_enabled(saved.native_drivers_enabled);
            }
        }
    }
    let sensors_started = match state.sensors.start() {
        Ok(started) => started,
        Err(e) => {
            log::warn!("sensord après mode jeu : {e:#}");
            false
        }
    };
    if saved.auto_start_openrgb {
        if let Err(e) = state.openrgb_mgr.ensure_running(&saved.openrgb_host, saved.openrgb_port) {
            log::warn!("OpenRGB après mode jeu : {e:#}");
        }
    }
    if sensors_started {
        std::thread::sleep(Duration::from_secs(6)); // premier relevé sensord avant de détecter les ventilateurs carte mère
    }
    let devices = scan_with_zone_sizes(&mut state.registry.lock(), &saved.zone_sizes);
    restore_saved_state(&state.registry, &state.engine, &saved);
    state.engine.invalidate();
    let sm = app.state::<Arc<screens::manager::ScreenManager>>();
    let screens_found = sm.scan(std::env::var_os("PURERGB_SIM_SCREEN").is_some()).len();
    for (event, n) in [("devices-changed", devices.len()), ("screens-changed", screens_found)] {
        if let Err(e) = app.emit(event, n) {
            log::debug!("émission {event} : {e:#}");
        }
    }
    publish(app, |s| *s = Status { game: None, grace: false, external_openrgb: false });
    notify(app, "Mode jeu terminé : accès matériel rétabli".to_string());
}

/// Lance la surveillance (thread dédié, jamais le thread d'interface). `initial` : jeu vu par `precheck`.
pub fn start(app: &AppHandle, initial: Option<String>) {
    let app = app.clone();
    let spawned = std::thread::Builder::new().name("gamemode".into()).spawn(move || {
        let mut phase = initial.map_or(Phase::Idle, Phase::Active);
        let mut sys = System::new();
        let mut rules_for: Vec<String> = Vec::new();
        let mut rules = Rules::load(&rules_for);
        loop {
            std::thread::sleep(POLL);
            let (enabled, keep_hid, games) = {
                let state = app.state::<AppState>();
                let s = state.settings.lock();
                (s.game_mode, s.game_mode_keep_hid, s.game_mode_games.clone())
            };
            if games != rules_for {
                rules = Rules::load(&games);
                rules_for = games;
            }
            let detected = if enabled { detect_now(&mut sys, &rules) } else { None };
            match phase.step(detected, enabled, Instant::now()) {
                Transition::Enter(g) => enter(&app, &g, keep_hid),
                Transition::Exit => exit(&app),
                Transition::None => {}
            }
            let grace = matches!(phase, Phase::Grace(..));
            if let Some(g) = phase.game() {
                publish(&app, |s| {
                    s.game = Some(g.to_string());
                    s.grace = grace;
                });
            }
        }
    });
    if let Err(e) = spawned {
        log::warn!("thread mode jeu : {e}");
    }
}

#[derive(Serialize)]
pub(crate) struct View {
    enabled: bool,
    keep_hid: bool,
    games: Vec<String>,
    active: bool,
    #[serde(flatten)]
    status: Status,
}

#[tauri::command]
pub(crate) fn game_mode_status(state: State<AppState>) -> View {
    let s = state.settings.lock();
    View {
        enabled: s.game_mode,
        keep_hid: s.game_mode_keep_hid,
        games: s.game_mode_games.clone(),
        active: active(),
        status: STATUS.lock().clone(),
    }
}

/// Noms d'exécutables saisis par l'utilisateur : nettoyés, dédoublonnés, sans chemin ni caractère interdit.
fn clean_games(games: Vec<String>) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for g in games.into_iter().map(|g| g.trim().to_string()).filter(|g| !g.is_empty()) {
        if g.len() > 128 || g.chars().any(|c| c.is_control() || "\\/:*?\"<>|".contains(c)) {
            return Err(format!("Nom d'exécutable invalide : « {g} » (nom seul, sans chemin)"));
        }
        if !out.iter().any(|o| detect::norm(o) == detect::norm(&g)) {
            out.push(g);
        }
    }
    if out.len() > MAX_GAMES {
        return Err(format!("Trop d'entrées (maximum {MAX_GAMES})"));
    }
    Ok(out)
}

#[tauri::command]
pub(crate) fn set_game_mode(state: State<AppState>, enabled: bool, keep_hid: bool, games: Vec<String>) -> Result<(), String> {
    let games = clean_games(games)?;
    let mut s = state.settings.lock();
    s.game_mode = enabled;
    s.game_mode_keep_hid = keep_hid;
    s.game_mode_games = games;
    crate::settings::save(&s).map_err(|e| e.to_string())
}

/// « Quitter PureRGB proprement » : même chemin que le menu du tray (nettoyage borné hors thread UI).
#[tauri::command]
pub(crate) fn quit_app(app: AppHandle) {
    crate::shutdown::quit_async(app, crate::cleanup_before_exit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_list_input_is_validated() {
        let ok = clean_games(vec![" a.exe ".into(), "A.EXE".into(), "".into(), "b".into()]).unwrap();
        assert_eq!(ok, vec!["a.exe", "b"]);
        assert!(clean_games(vec!["C:\\x\\jeu.exe".into()]).is_err());
        assert!(clean_games(vec!["a\nb".into()]).is_err());
        assert!(clean_games((0..101).map(|i| format!("g{i}")).collect()).is_err());
    }
}
