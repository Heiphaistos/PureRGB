mod backends;
mod conflicts;
mod core;
mod engine;
mod settings;

use crate::backends::hid::HidBackend;
use crate::backends::openrgb::OpenRgbBackend;
use crate::backends::Backend;
use crate::core::registry::{DeviceRegistry, SharedRegistry};
use crate::core::DeviceInfo;
use crate::engine::effects::EffectConfig;
use crate::engine::EffectsEngine;
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
    state.registry.lock().scan_all()
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
fn check_conflicts() -> ConflictReport {
    let (conflicts, openrgb_running) = conflicts::scan();
    ConflictReport {
        conflicts,
        openrgb_running,
    }
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
    s.native_drivers_enabled = native_drivers_enabled;
    s.fps = fps;
    s.start_minimized = start_minimized;
    settings::save(&s).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let saved = settings::load();
    let backends: Vec<Box<dyn Backend>> = vec![
        Box::new(OpenRgbBackend::new(
            saved.openrgb_host.clone(),
            saved.openrgb_port,
        )),
        Box::new(HidBackend::new(saved.native_drivers_enabled)),
    ];
    let registry = DeviceRegistry::shared(backends);
    let engine = EffectsEngine::start(registry.clone());
    engine.set_fps(saved.fps);

    // Restaurer les effets sauvegardés après un premier scan.
    {
        let devices = registry.lock().scan_all();
        for d in &devices {
            if let Some(cfg) = saved.effects.get(&d.id) {
                engine.set_effect(d.id.clone(), cfg.clone(), d.led_count);
            }
        }
    }

    let state = AppState {
        registry,
        engine,
        settings: Mutex::new(saved),
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
        .manage(state)
        .setup(|app| {
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
                        app.state::<AppState>().engine.shutdown();
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
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PureRGB");
}
