mod backends;
mod conflicts;
mod core;
mod engine;
mod sensors;
mod settings;

use crate::backends::hid::HidBackend;
use crate::backends::liquidctl::LiquidctlBackend;
use crate::backends::openrgb::manager::{OpenRgbManager, OpenRgbStatus};
use crate::backends::openrgb::OpenRgbBackend;
use crate::backends::Backend;
use crate::core::registry::{DeviceRegistry, SharedRegistry};
use crate::core::DeviceInfo;
use crate::engine::curves::{CurveConfig, CurveEngine};
use crate::engine::effects::EffectConfig;
use crate::engine::EffectsEngine;
use crate::sensors::{Sensor, SensorHub};
use crate::settings::Settings;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};

struct AppState {
    registry: SharedRegistry,
    engine: EffectsEngine,
    settings: Mutex<Settings>,
    openrgb_mgr: std::sync::Arc<OpenRgbManager>,
    sensors: std::sync::Arc<SensorHub>,
    curve_engine: std::sync::Arc<CurveEngine>,
}

#[derive(Serialize)]
struct BackendStatus {
    name: String,
    available: bool,
}

#[derive(Serialize)]
struct ConflictReport {
    conflicts: Vec<conflicts::ConflictingSoftware>,
    openrgb_running: bool,
}

#[tauri::command]
fn scan_devices(state: State<AppState>) -> Vec<DeviceInfo> {
    let devices = state.registry.lock().scan_all();
    // Le matériel a pu être réinitialisé : forcer la ré-application des effets.
    state.engine.invalidate();
    devices
}

#[tauri::command]
fn list_devices(state: State<AppState>) -> Vec<DeviceInfo> {
    state.registry.lock().device_list()
}

#[tauri::command]
fn backend_status(state: State<AppState>) -> Vec<BackendStatus> {
    state
        .registry
        .lock()
        .backend_status()
        .into_iter()
        .map(|(name, available)| BackendStatus { name, available })
        .collect()
}

#[tauri::command]
fn apply_effect(
    state: State<AppState>,
    device_id: String,
    config: EffectConfig,
) -> Result<(), String> {
    let led_count = state
        .registry
        .lock()
        .get(&device_id)
        .map(|d| d.led_count)
        .ok_or_else(|| format!("appareil inconnu: {device_id}"))?;
    state
        .engine
        .set_effect(device_id.clone(), config.clone(), led_count);
    let mut s = state.settings.lock();
    s.effects.insert(device_id, config);
    settings::save(&s).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_effect_all(state: State<AppState>, config: EffectConfig) -> Result<u32, String> {
    let devices = state.registry.lock().device_list();
    let mut applied = 0u32;
    let mut s = state.settings.lock();
    for d in devices.iter().filter(|d| d.controllable && d.led_count > 0) {
        state
            .engine
            .set_effect(d.id.clone(), config.clone(), d.led_count);
        s.effects.insert(d.id.clone(), config.clone());
        applied += 1;
    }
    settings::save(&s).map_err(|e| e.to_string())?;
    Ok(applied)
}

#[tauri::command]
fn set_fan_duty(
    state: State<AppState>,
    device_id: String,
    channel: u8,
    percent: u8,
) -> Result<(), String> {
    state
        .registry
        .lock()
        .set_fan_duty(&device_id, channel, percent)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn openrgb_status(state: State<AppState>) -> OpenRgbStatus {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    state.openrgb_mgr.status(&host, port)
}

/// Lance (ou installe puis lance) l'OpenRGB embarqué. Bloquant : exécuté
/// hors du thread principal par Tauri (commande async côté JS).
#[tauri::command(async)]
fn openrgb_start(state: State<AppState>) -> Result<bool, String> {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    let launched = state
        .openrgb_mgr
        .ensure_running(&host, port)
        .map_err(|e| format!("{e:#}"))?;
    // Serveur prêt : re-scanner pour récupérer les contrôleurs.
    state.registry.lock().scan_all();
    Ok(launched)
}

#[tauri::command]
fn openrgb_stop(state: State<AppState>) {
    state.openrgb_mgr.stop();
}

/// Redémarre l'OpenRGB géré (après arrêt d'un logiciel en conflit, les
/// contrôleurs libérés ne sont vus qu'après une nouvelle détection).
#[tauri::command(async)]
fn openrgb_restart(state: State<AppState>) -> Result<bool, String> {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    state.openrgb_mgr.stop();
    let launched = state
        .openrgb_mgr
        .ensure_running(&host, port)
        .map_err(|e| format!("{e:#}"))?;
    state.registry.lock().scan_all();
    state.engine.invalidate();
    Ok(launched)
}

/// Installe le driver PawnIO (SMBus : RAM, carte mère). Admin requis.
#[tauri::command(async)]
fn pawnio_install() -> Result<(), String> {
    OpenRgbManager::pawnio_install().map_err(|e| format!("{e:#}"))
}

/// Énumère processus ET services : passe par PowerShell, donc async.
#[tauri::command(async)]
fn check_conflicts() -> ConflictReport {
    let (conflicts, openrgb_running) = conflicts::scan();
    ConflictReport {
        conflicts,
        openrgb_running,
    }
}

/// Stoppe une famille de logiciels en conflit (services + processus).
/// `disable` : désactive aussi le démarrage automatique des services
/// (mode d'origine sauvegardé pour `conflict_restore`).
#[tauri::command(async)]
fn conflict_stop(state: State<AppState>, family: String, disable: bool) -> Result<(), String> {
    let modes = conflicts::stop_family(&family, disable).map_err(|e| format!("{e:#}"))?;
    if !modes.is_empty() {
        let mut s = state.settings.lock();
        s.disabled_services.extend(modes);
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Dernier relevé capteurs (sensord). Vide si le sidecar est absent.
#[tauri::command]
fn get_sensors(state: State<AppState>) -> Vec<Sensor> {
    state.sensors.snapshot()
}

/// Définit ou supprime (config=None) la courbe d'un canal ventilateur.
#[tauri::command]
fn set_curve(
    state: State<AppState>,
    device_id: String,
    channel: u8,
    config: Option<CurveConfig>,
) -> Result<(), String> {
    let key = format!("{device_id}|{channel}");
    let mut s = state.settings.lock();
    match config {
        Some(cfg) => {
            cfg.validate()?;
            s.curves.insert(key, cfg);
        }
        None => {
            s.curves.remove(&key);
        }
    }
    state.curve_engine.set_curves(s.curves.clone());
    settings::save(&s).map_err(|e| e.to_string())
}

/// Commande écran LCD (Kraken Z / 2023 via liquidctl).
/// kind: liquid | static | gif | brightness | orientation. arg: chemin ou valeur.
#[tauri::command(async)]
fn lcd_apply(
    state: State<AppState>,
    device_id: String,
    kind: String,
    arg: Option<String>,
) -> Result<(), String> {
    let (backend, local) = device_id
        .split_once(':')
        .ok_or_else(|| format!("id invalide: {device_id}"))?;
    if backend != "liquidctl" {
        return Err("LCD disponible uniquement via liquidctl".into());
    }
    let local = local.to_string();
    let mut reg = state.registry.lock();
    for b in reg.backends_mut() {
        if b.name() == "liquidctl" {
            if let Some(lc) = b.as_any_mut().downcast_mut::<LiquidctlBackend>() {
                return lc
                    .lcd_apply(&local, &kind, arg.as_deref())
                    .map_err(|e| format!("{e:#}"));
            }
        }
    }
    Err("backend liquidctl indisponible".into())
}

/// Réactive une famille : StartupType restauré + services relancés.
#[tauri::command(async)]
fn conflict_restore(state: State<AppState>, family: String) -> Result<(), String> {
    let saved = state.settings.lock().disabled_services.clone();
    let restored = conflicts::restore_family(&family, &saved).map_err(|e| format!("{e:#}"))?;
    let mut s = state.settings.lock();
    for name in restored {
        s.disabled_services.remove(&name);
    }
    settings::save(&s).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn update_settings(
    state: State<AppState>,
    openrgb_host: String,
    openrgb_port: u16,
    auto_start_openrgb: bool,
    native_drivers_enabled: bool,
    fps: u32,
    start_minimized: bool,
) -> Result<(), String> {
    // Validation input : host non vide, fps borné.
    let host = openrgb_host.trim().to_string();
    if host.is_empty() || host.len() > 253 {
        return Err("hôte OpenRGB invalide".into());
    }
    let fps = fps.clamp(5, 60);

    {
        let mut reg = state.registry.lock();
        for b in reg.backends_mut() {
            if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    orgb.set_endpoint(host.clone(), openrgb_port);
                }
            } else if b.name() == "hid" {
                if let Some(hid) = b.as_any_mut().downcast_mut::<HidBackend>() {
                    hid.set_native_enabled(native_drivers_enabled);
                }
            }
        }
    }
    state.engine.set_fps(fps);

    let mut s = state.settings.lock();
    s.openrgb_host = host;
    s.openrgb_port = openrgb_port;
    s.auto_start_openrgb = auto_start_openrgb;
    s.native_drivers_enabled = native_drivers_enabled;
    s.fps = fps;
    s.start_minimized = start_minimized;
    settings::save(&s).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let saved = settings::load();
    // Créer settings.json dès le premier lancement (valeurs par défaut visibles).
    if let Err(e) = settings::save(&saved) {
        log::warn!("écriture settings initiale: {e:#}");
    }
    let backends: Vec<Box<dyn Backend>> = vec![
        Box::new(OpenRgbBackend::new(
            saved.openrgb_host.clone(),
            saved.openrgb_port,
        )),
        Box::new(HidBackend::new(saved.native_drivers_enabled)),
        Box::new(LiquidctlBackend::new()),
    ];
    let registry = DeviceRegistry::shared(backends);
    let engine = EffectsEngine::start(registry.clone());
    engine.set_fps(saved.fps);
    let openrgb_mgr = std::sync::Arc::new(OpenRgbManager::new());
    let sensors = SensorHub::new();
    let curve_engine = CurveEngine::start(registry.clone(), sensors.clone(), saved.curves.clone());

    // Démarrage matériel en arrière-plan : auto-start OpenRGB embarqué
    // (si activé et aucun serveur joignable), scan, restauration des effets.
    // Hors du thread UI — l'init OpenRGB peut prendre 20 s.
    {
        let registry = registry.clone();
        let engine = engine.clone();
        let mgr = openrgb_mgr.clone();
        let sensors_hub = sensors.clone();
        let saved = saved.clone();
        std::thread::Builder::new()
            .name("hw-init".into())
            .spawn(move || {
                let _ = sensors_hub; // démarré dans setup() une fois resource_dir connu
                if saved.auto_start_openrgb {
                    match mgr.ensure_running(&saved.openrgb_host, saved.openrgb_port) {
                        Ok(true) => log::info!("OpenRGB embarqué démarré"),
                        Ok(false) => log::info!("serveur OpenRGB déjà actif"),
                        Err(e) => log::warn!("auto-start OpenRGB: {e:#}"),
                    }
                }
                let devices = registry.lock().scan_all();
                for d in &devices {
                    if let Some(cfg) = saved.effects.get(&d.id) {
                        engine.set_effect(d.id.clone(), cfg.clone(), d.led_count);
                    }
                }
            })
            .expect("spawn hw-init");
    }

    let state = AppState {
        registry,
        engine,
        settings: Mutex::new(saved),
        openrgb_mgr,
        sensors,
        curve_engine,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Deuxième lancement : remettre la fenêtre au premier plan.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            // Dossier ressources du bundle : openrgb/, liquidctl/, sensord/.
            if let Ok(res) = app.path().resource_dir() {
                let state = app.state::<AppState>();
                state.openrgb_mgr.set_resource_dir(res.clone());
                state.sensors.set_resource_dir(res.clone());
                let mut reg = state.registry.lock();
                for b in reg.backends_mut() {
                    if b.name() == "liquidctl" {
                        if let Some(lc) = b.as_any_mut().downcast_mut::<LiquidctlBackend>() {
                            lc.set_resource_dir(res.clone());
                        }
                    }
                }
            }
            // Capteurs : démarrage différé hors du thread UI (init LHM ~2-5 s).
            {
                let sensors = app.state::<AppState>().sensors.clone();
                std::thread::Builder::new()
                    .name("sensord-start".into())
                    .spawn(move || match sensors.start() {
                        Ok(true) => log::info!("sensord démarré"),
                        Ok(false) => log::info!("sensord absent"),
                        Err(e) => log::warn!("sensord: {e:#}"),
                    })?;
            }
            let start_minimized = app.state::<AppState>().settings.lock().start_minimized;
            if start_minimized {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            let show = MenuItem::with_id(app, "show", "Afficher PureRGB", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("PureRGB")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        let state = app.state::<AppState>();
                        state.engine.shutdown();
                        state.curve_engine.shutdown();
                        state.sensors.stop();
                        state.openrgb_mgr.stop();
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Fermer = minimiser dans le tray (comportement RGB standard).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            list_devices,
            backend_status,
            apply_effect,
            apply_effect_all,
            set_fan_duty,
            check_conflicts,
            conflict_stop,
            conflict_restore,
            get_sensors,
            set_curve,
            lcd_apply,
            openrgb_status,
            openrgb_start,
            openrgb_stop,
            openrgb_restart,
            pawnio_install,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PureRGB");
}
