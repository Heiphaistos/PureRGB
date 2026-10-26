//! Mode arrière-plan (`purergb.exe --background=<modules>`, ancien `--screens-only`) : le même exécutable, sans
//! fenêtre ni WebView2, qui ne lance QUE les modules cochés par l'utilisateur : écrans, éclairage appliqué,
//! ventilateurs (courbes/réglages manuels déjà enregistrés), mode manette. Rien d'autre (pas d'interface,
//! pas de télémétrie, pas de garde anti-conflit, pas de détection USB périodique).
//!
//! Garde-fous : aucun service, aucune élévation, aucun pilote ajouté, aucune injection. Le mode jeu s'applique ici
//! comme dans l'application complète (même code) : ce n'est PAS un moyen de contourner un anti-cheat, seulement
//! un moyen de ne garder ni fenêtre ni WebView2. Le processus est arrêté par l'application complète à son lancement
//! (fichier `background.stop`, puis arrêt du PID vérifié) : les ventilateurs sont alors rendus comme à la fermeture.

use crate::gamemode;
use crate::screens::layout::{Layout, WidgetKind};
use crate::screens::manager::ScreenManager;
use crate::sensors::SensorHub;
use crate::{boot, settings, shutdown, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

const STOP_WAIT: Duration = Duration::from_secs(6);
const PARENT_WAIT: Duration = Duration::from_secs(15);
/// Au démarrage, on réessaie vite la détection des écrans pendant ce temps.
const FAST_RETRY: Duration = Duration::from_secs(20);
const RESCAN_EVERY: Duration = Duration::from_secs(60);

static OPEN_FULL: AtomicBool = AtomicBool::new(false);
/// L'application complète reprend la main (ou le fichier d'arrêt l'a demandé) : ne pas relancer les logiciels constructeur.
static HAND_BACK: AtomicBool = AtomicBool::new(false);
static RESCAN: AtomicBool = AtomicBool::new(false);
static MODULES: OnceLock<Modules> = OnceLock::new();

/// Ce qui reste actif sans fenêtre. Mémorisé dans les réglages (`background`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Modules {
    /// Écrans LCD : design, thème, vidéo.
    pub screens: bool,
    /// Éclairage appliqué : couleurs, effets, thèmes (OpenRGB et pilotes natifs).
    pub lighting: bool,
    /// Ventilateurs : courbes et réglages manuels DÉJÀ actifs, jamais de réglage automatique.
    pub fans: bool,
    /// Mode manette : remappage via manette virtuelle (ViGEm).
    pub gamepad: bool,
}

impl Modules {
    pub const ALL: Modules = Modules { screens: true, lighting: true, fans: true, gamepad: true };
    pub const NONE: Modules = Modules { screens: false, lighting: false, fans: false, gamepad: false };

    pub fn any(self) -> bool {
        self != Self::NONE
    }

    /// Forme de ligne de commande : `screens,fans`.
    pub fn to_list(self) -> String {
        let mut v = Vec::new();
        for (on, name) in [(self.screens, "screens"), (self.lighting, "lighting"), (self.fans, "fans"), (self.gamepad, "gamepad")] {
            if on {
                v.push(name);
            }
        }
        v.join(",")
    }

    /// Noms inconnus ignorés.
    pub fn from_list(list: &str) -> Modules {
        let mut m = Modules::NONE;
        for n in list.split(',').map(str::trim) {
            match n {
                "screens" => m.screens = true,
                "lighting" => m.lighting = true,
                "fans" => m.fans = true,
                "gamepad" => m.gamepad = true,
                _ => {}
            }
        }
        m
    }
}

/// Modules actifs dans CE processus : tous dans l'application complète.
pub fn active_modules() -> Modules {
    MODULES.get().copied().unwrap_or(Modules::ALL)
}

/// Vrai dans le processus d'arrière-plan.
pub fn is_background() -> bool {
    MODULES.get().is_some()
}

/// Demande une nouvelle détection des écrans (fin du mode jeu côté arrière-plan).
pub fn request_rescan() {
    RESCAN.store(true, Ordering::Release);
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Full,
    Background(Modules),
}

/// `args` commence par le chemin de l'exécutable (comme `std::env::args`).
pub fn parse_mode<I: IntoIterator<Item = String>>(args: I) -> Mode {
    for a in args.into_iter().skip(1) {
        if a == "--screens-only" {
            return Mode::Background(Modules { screens: true, ..Modules::NONE });
        }
        if let Some(list) = a.strip_prefix("--background=") {
            let m = Modules::from_list(list);
            return if m.any() { Mode::Background(m) } else { Mode::Full };
        }
    }
    Mode::Full
}

fn parse_wait_pid<I: IntoIterator<Item = String>>(args: I) -> Option<u32> {
    args.into_iter().find_map(|a| a.strip_prefix("--wait-pid=").and_then(|v| v.parse().ok()))
}

/// Ce qui est réellement en cours à la fermeture (on ne garde que ce qui est DÉJÀ actif).
#[derive(Debug, Clone, Copy)]
pub struct Facts {
    pub screen_design_active: bool,
    pub game_active: bool,
    pub saved_screen_design: bool,
    pub has_lighting: bool,
    pub has_fan_settings: bool,
    pub gamepad_enabled: bool,
}

/// Modules à laisser tourner : ceux cochés ET déjà en service. Jamais de ventilateurs sans courbe/réglage existant.
pub fn modules_to_keep(chosen: Modules, f: Facts) -> Modules {
    Modules {
        screens: chosen.screens && (f.screen_design_active || (f.game_active && f.saved_screen_design)),
        lighting: chosen.lighting && f.has_lighting,
        fans: chosen.fans && f.has_fan_settings,
        gamepad: chosen.gamepad && f.gamepad_enabled,
    }
}

/// Sources hors horloge/système qui exigent sensord (`sysinfo` fournit le reste sans pilote).
fn needs_sensord_key(key: &str) -> bool {
    let system_only = key.starts_with("ram.")
        || key.starts_with("net.")
        || key.starts_with("disk.")
        || matches!(key, "uptime" | "time" | "date" | "cpu.load" | "cpu.clock");
    !system_only
}

pub fn needs_sensord(layouts: &[Layout]) -> bool {
    layouts.iter().flat_map(|l| &l.widgets).any(|w| {
        let own = match &w.kind {
            WidgetKind::Value { source, .. }
            | WidgetKind::GaugeArc { source, .. }
            | WidgetKind::Bar { source, .. }
            | WidgetKind::Graph { source, .. } => Some(source.as_str()),
            _ => None,
        };
        own.into_iter().chain(w.visible_if.as_deref()).any(needs_sensord_key)
    })
}

fn dir() -> Option<PathBuf> {
    settings::dirs_dir()
}

fn file(name: &str) -> Option<PathBuf> {
    dir().map(|d| d.join(name))
}

/// Familles de logiciels constructeur arrêtées par PureRGB : passées d'un processus à l'autre sans être relancées.
pub fn handover_save(families: &[String]) {
    if let (Some(p), Ok(text)) = (file("background-handover.json"), serde_json::to_string(families)) {
        if let Err(e) = std::fs::write(p, text) {
            log::warn!("passation des logiciels arrêtés : {e}");
        }
    }
}

pub fn handover_take() -> Vec<String> {
    let Some(p) = file("background-handover.json") else { return Vec::new() };
    let v = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let _ = std::fs::remove_file(p);
    v
}

fn same_exe_name(a: &str, b: &str) -> bool {
    !a.is_empty() && a.eq_ignore_ascii_case(b)
}

/// Le PID est-il bien un processus portant le même nom d'exécutable que nous (jamais tuer un PID réutilisé) ?
fn is_ours(pid: u32) -> bool {
    let Some(want) = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_string_lossy().into_owned())) else {
        return false;
    };
    let p = Pid::from_u32(pid);
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[p]), true, ProcessRefreshKind::nothing());
    sys.process(p).is_some_and(|pr| same_exe_name(&pr.name().to_string_lossy(), &want))
}

fn read_pid() -> Option<u32> {
    std::fs::read_to_string(file("background.pid")?).ok()?.trim().parse().ok()
}

/// PID d'un autre processus d'arrière-plan vivant, s'il y en a un.
fn live_daemon_pid() -> Option<u32> {
    read_pid().filter(|p| *p != std::process::id() && is_ours(*p))
}

/// Arrête proprement un processus d'arrière-plan en cours (appelé au lancement de l'application complète, avant
/// toute détection) : demande par fichier, puis arrêt du PID vérifié si elle n'est pas honorée.
pub fn stop_running_daemon() {
    let (Some(stop), Some(pidf)) = (file("background.stop"), file("background.pid")) else { return };
    let Some(pid) = live_daemon_pid() else {
        let _ = std::fs::remove_file(&stop);
        let _ = std::fs::remove_file(&pidf);
        return;
    };
    log::info!("processus d'arrière-plan (PID {pid}) en cours : arrêt demandé");
    let _ = std::fs::write(&stop, b"stop");
    let t = Instant::now();
    while is_ours(pid) && t.elapsed() < STOP_WAIT {
        std::thread::sleep(Duration::from_millis(100));
    }
    if is_ours(pid) {
        log::warn!("arrière-plan (PID {pid}) n'a pas répondu : arrêt forcé");
        let p = Pid::from_u32(pid);
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[p]), true, ProcessRefreshKind::nothing());
        if let Some(pr) = sys.process(p) {
            pr.kill();
        }
        let t = Instant::now();
        while is_ours(pid) && t.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let _ = std::fs::remove_file(&stop);
    let _ = std::fs::remove_file(&pidf);
}

fn claim() -> bool {
    if live_daemon_pid().is_some() {
        return false;
    }
    if let (Some(d), Some(p)) = (dir(), file("background.pid")) {
        let _ = std::fs::create_dir_all(d);
        if let Err(e) = std::fs::write(p, std::process::id().to_string()) {
            log::warn!("fichier PID arrière-plan : {e}");
        }
    }
    true
}

fn release_files() {
    if read_pid() == Some(std::process::id()) {
        if let Some(p) = file("background.pid") {
            let _ = std::fs::remove_file(p);
        }
    }
    if let Some(s) = file("background.stop") {
        let _ = std::fs::remove_file(s);
    }
}

fn spawn_self(args: &[String]) -> std::io::Result<()> {
    let mut c = Command::new(std::env::current_exe()?);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000 | 0x0000_0200); // sans console | nouveau groupe
    }
    c.spawn().map(|_| ())
}

/// Lance le processus d'arrière-plan avec `keep`, détaché du job de fermeture (qui le tuerait avec PureRGB).
/// Les sidecars doivent être déjà arrêtés. Il attend la fin du processus appelant avant de toucher au matériel.
pub fn spawn_daemon(keep: Modules) -> std::io::Result<()> {
    crate::job::release_children();
    spawn_self(&[format!("--background={}", keep.to_list()), format!("--wait-pid={}", std::process::id())])
}

/// Fermeture du processus d'arrière-plan : moteurs arrêtés, ventilateurs rendus au BIOS comme à la fermeture de
/// l'application complète, puis (selon le cas) logiciels constructeur relancés ou application complète lancée.
fn daemon_cleanup(app: &AppHandle) {
    let handback = HAND_BACK.load(Ordering::Acquire) || OPEN_FULL.load(Ordering::Acquire);
    crate::cleanup(app, Modules::NONE, !handback);
    release_files();
    if OPEN_FULL.load(Ordering::Acquire) {
        crate::job::release_children();
        if let Err(e) = spawn_self(&[]) {
            log::warn!("lancement de PureRGB : {e}");
        }
    }
}

fn rescan(mgr: &ScreenManager, hub: &Arc<SensorHub>, fans: bool, sim: bool) {
    let n = mgr.scan(sim).len();
    log::info!("arrière-plan : {n} écran(s) actif(s)");
    let need = fans || needs_sensord(&mgr.active_layouts());
    if need && !hub.running() {
        match hub.start() {
            Ok(started) => log::info!("sensord démarré : {started}"),
            Err(e) => log::warn!("sensord : {e:#}"),
        }
    } else if !need && hub.running() {
        hub.stop();
    }
}

/// Module écrans : détection (rapide au début, puis seulement si un écran du catalogue est branché), mode jeu via
/// `gamemode` (qui demande un `rescan` à sa fin).
fn supervise_screens(app: &AppHandle, mods: Modules) {
    let sim = std::env::var_os("PURERGB_SIM_SCREEN").is_some();
    let mgr = app.state::<Arc<ScreenManager>>().inner().clone();
    let hub = app.state::<AppState>().sensors.clone();
    let (started, mut last_scan) = (Instant::now(), None::<Instant>);
    loop {
        let fast = started.elapsed() < FAST_RETRY;
        std::thread::sleep(if fast { Duration::from_millis(500) } else { Duration::from_secs(1) });
        if gamemode::active() {
            continue;
        }
        let forced = RESCAN.swap(false, Ordering::AcqRel);
        if mgr.all_running() && !forced {
            continue;
        }
        let due = last_scan.map_or(true, |t| t.elapsed() >= if fast { Duration::from_secs(1) } else { RESCAN_EVERY });
        if forced || (due && (fast || last_scan.is_none() || !mgr.plugged_but_idle().is_empty() || !mgr.list().is_empty())) {
            last_scan = Some(Instant::now());
            rescan(&mgr, &hub, mods.fans, sim);
        }
    }
}

/// Attend la fin du processus qui nous a lancés (il libère l'écran et le matériel en quittant), au plus `PARENT_WAIT`.
fn wait_parent(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    let t = Instant::now();
    let p = Pid::from_u32(pid);
    let mut sys = System::new();
    loop {
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[p]), true, ProcessRefreshKind::nothing());
        if sys.process(p).is_none() || t.elapsed() > PARENT_WAIT {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Surveille le fichier d'arrêt posé par l'application complète.
fn watch_stop(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("bg-stop".into()).spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        if file("background.stop").is_some_and(|s| s.exists()) {
            log::info!("arrière-plan : arrêt demandé");
            HAND_BACK.store(true, Ordering::Release);
            shutdown::quit_async(app.clone(), daemon_cleanup);
            return;
        }
    });
    if let Err(e) = spawned {
        log::warn!("thread bg-stop : {e}");
    }
}

#[tauri::command]
pub(crate) fn background_get(state: tauri::State<AppState>) -> Modules {
    state.settings.lock().background
}

#[tauri::command]
pub(crate) fn background_set(state: tauri::State<AppState>, modules: Modules) -> Result<(), String> {
    let mut s = state.settings.lock();
    s.background = modules;
    settings::save(&s).map_err(|e| e.to_string())
}

/// Un processus d'arrière-plan (`--background`) tourne-t-il en ce moment ? (interface : état réel, pas seulement le réglage)
#[tauri::command]
pub(crate) fn background_status() -> bool {
    live_daemon_pid().is_some()
}

/// Arrête le processus d'arrière-plan s'il tourne (demande d'arrêt, puis forcé si besoin) ; hors thread d'interface.
#[tauri::command]
pub(crate) async fn background_stop() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(stop_running_daemon).await.map_err(|e| e.to_string())
}

/// Point d'entrée du processus d'arrière-plan. Aucune fenêtre n'est créée (donc pas de WebView2).
pub fn run(mut ctx: tauri::Context, mods: Modules) {
    if !claim() {
        log::info!("arrière-plan déjà en cours : sortie");
        return;
    }
    let _ = MODULES.set(mods);
    log::info!("arrière-plan : démarrage (PID {}), modules : {}", std::process::id(), mods.to_list());
    let saved = settings::load();
    let game = gamemode::precheck(&saved);
    let (state, mgr) = boot::build(&saved, mods);
    state.auto_stopped.lock().extend(handover_take());
    let wait = parse_wait_pid(std::env::args());
    ctx.config_mut().app.windows.clear();
    let built = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .manage(mgr)
        .setup(move |app| {
            boot::set_resources(app.handle());
            let open = MenuItem::with_id(app, "open", "Ouvrir PureRGB", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter complètement (arrête tout)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip(format!("PureRGB en arrière-plan ({})", mods.to_list()))
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "open" {
                        OPEN_FULL.store(true, Ordering::Release);
                    }
                    shutdown::quit_async(app.clone(), daemon_cleanup);
                })
                .build(app)?;
            watch_stop(app.handle().clone());
            let handle = app.handle().clone();
            std::thread::Builder::new().name("bg-main".into()).spawn(move || {
                wait_parent(wait); // l'application complète libère écran et matériel avant que nous y touchions
                let state = handle.state::<AppState>();
                gamemode::start(&handle, game);
                boot::start_hw_init(&state, mods, false);
                if mods.fans {
                    if let Err(e) = boot::start_sensord(&handle) {
                        log::warn!("sensord : {e}");
                    }
                }
                if mods.gamepad {
                    crate::gamepad_remap::start(&handle);
                }
                if mods.screens {
                    supervise_screens(&handle, mods);
                }
            })?;
            Ok(())
        })
        .build(ctx);
    match built {
        Ok(app) => app.run(|_, ev| {
            // Sans fenêtre, seule une sortie explicite (menu, fichier d'arrêt) doit terminer le processus.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = ev {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        }),
        Err(e) => {
            log::error!("arrière-plan : {e}");
            release_files();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    fn m(s: bool, l: bool, f: bool, g: bool) -> Modules {
        Modules { screens: s, lighting: l, fans: f, gamepad: g }
    }

    #[test]
    fn mode_comes_only_from_flags_after_the_exe_path() {
        assert_eq!(parse_mode(args(&["purergb.exe"])), Mode::Full);
        assert_eq!(parse_mode(args(&["purergb.exe", "--screens-only"])), Mode::Background(m(true, false, false, false)));
        assert_eq!(
            parse_mode(args(&["purergb.exe", "--x", "--background=fans,screens,bogus"])),
            Mode::Background(m(true, false, true, false))
        );
        assert_eq!(parse_mode(args(&["purergb.exe", "--background="])), Mode::Full, "liste vide : pas d'arrière-plan");
        assert_eq!(parse_mode(args(&["purergb.exe", "--background=bogus"])), Mode::Full);
        assert_eq!(parse_mode(args(&["--screens-only"])), Mode::Full, "le premier élément est le chemin de l'exe");
        assert_eq!(parse_mode(args(&["purergb.exe", "--screens-only2"])), Mode::Full);
    }

    #[test]
    fn modules_round_trip_and_wait_pid() {
        assert_eq!(Modules::from_list(&Modules::ALL.to_list()), Modules::ALL);
        assert_eq!(Modules::NONE.to_list(), "");
        assert_eq!(parse_wait_pid(args(&["x", "--wait-pid=42"])), Some(42));
        assert_eq!(parse_wait_pid(args(&["x", "--wait-pid=abc"])), None);
    }

    #[test]
    fn only_what_is_already_active_is_kept() {
        let none = Facts { screen_design_active: false, game_active: false, saved_screen_design: false, has_lighting: false, has_fan_settings: false, gamepad_enabled: false };
        assert_eq!(modules_to_keep(Modules::ALL, none), Modules::NONE, "rien d'actif : rien à garder");
        let all = Facts { screen_design_active: true, game_active: false, saved_screen_design: true, has_lighting: true, has_fan_settings: true, gamepad_enabled: true };
        assert_eq!(modules_to_keep(Modules::ALL, all), Modules::ALL);
        assert_eq!(modules_to_keep(Modules::NONE, all), Modules::NONE, "rien de coché");
        assert_eq!(modules_to_keep(m(false, false, true, false), all), m(false, false, true, false));
        // Ventilateurs : jamais sans courbe/réglage manuel existant, même coché.
        let no_fans = Facts { has_fan_settings: false, ..all };
        assert!(!modules_to_keep(Modules::ALL, no_fans).fans);
        // Jeu en cours : les écrans reviennent après la partie si un design est enregistré.
        let game = Facts { screen_design_active: false, game_active: true, ..all };
        assert!(modules_to_keep(Modules::ALL, game).screens);
        assert!(!modules_to_keep(Modules::ALL, Facts { saved_screen_design: false, ..game }).screens);
    }

    #[test]
    fn sensord_only_for_hardware_sensors() {
        let l = |src: &str, vis: Option<&str>| -> Layout {
            serde_json::from_value(serde_json::json!({
                "version": 1, "name": "t", "width": 100, "height": 100,
                "background": {"kind": "solid", "color": "#000"},
                "widgets": [{"id": "a", "x": 0, "y": 0, "w": 10, "h": 10, "visible_if": vis,
                    "type": "value", "source": src, "format": "{:.0}",
                    "style": {"font": "x", "size": 10, "color": "#fff"}}]
            }))
            .unwrap()
        };
        assert!(!needs_sensord(&[l("time", None)]));
        assert!(!needs_sensord(&[l("ram.used_pct", None), l("net.down", None), l("disk.C.used_pct", None)]));
        assert!(needs_sensord(&[l("cpu.temp", None)]));
        assert!(needs_sensord(&[l("time", Some("gpu.temp"))]));
        assert!(!needs_sensord(&[]));
    }

    #[test]
    fn exe_name_comparison_and_pids() {
        assert!(same_exe_name("PureRGB.exe", "purergb.EXE"));
        assert!(!same_exe_name("chrome.exe", "purergb.exe"));
        assert!(!same_exe_name("", ""));
        assert!(!is_ours(0));
        assert!(!is_ours(u32::MAX - 1));
        assert!(is_ours(std::process::id()));
    }

    #[test]
    fn handover_round_trip_is_consumed_once() {
        let d = std::env::temp_dir().join(format!("purergb-handover-{}", std::process::id()));
        std::fs::create_dir_all(d.join("PureRGB")).unwrap();
        // APPDATA est global au processus de test : ce test est le seul à le modifier.
        std::env::set_var("APPDATA", &d);
        assert!(handover_take().is_empty());
        handover_save(&["corsair".to_string(), "msi".to_string()]);
        assert_eq!(handover_take(), vec!["corsair", "msi"]);
        assert!(handover_take().is_empty(), "consommé une seule fois");
        let _ = std::fs::remove_dir_all(&d);
    }
}
