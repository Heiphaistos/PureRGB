mod backends;
mod commands;
mod conflicts;
mod device_feedback;
mod core;
mod engine;
mod diag_extra;
mod gamemode;
mod gamepad_remap;
mod hotplug;
mod inventory;
mod led_catalog;
mod selftest;
mod job;
mod netdev;
mod product_commands;
mod product_images;
mod rgb_catalog;
mod screens;
mod sensors;
mod settings;
mod shutdown;
mod telemetry;
mod trust;
mod usbcapture;

use crate::backends::hid::HidBackend;
use crate::backends::liquidctl::LiquidctlBackend;
use crate::backends::mobo::MoboFanBackend;
use crate::backends::openrgb::manager::OpenRgbManager;
use crate::backends::openrgb::OpenRgbBackend;
use crate::backends::smbus::SmbusBackend;
use crate::backends::Backend;
use crate::core::registry::{DeviceRegistry, SharedRegistry};
use crate::engine::curves::CurveEngine;
use crate::engine::EffectsEngine;
use crate::sensors::SensorHub;
use crate::settings::Settings;
use parking_lot::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use commands::devices::scan_with_zone_sizes;
use commands::diagnostics::{diagnostics_json, rescan_after_plug, rescan_and_notify};
pub(crate) use commands::diagnostics::HardwareDiagnostics;

pub(crate) struct AppState {
    registry: SharedRegistry,
    engine: EffectsEngine,
    settings: std::sync::Arc<Mutex<Settings>>,
    openrgb_mgr: std::sync::Arc<OpenRgbManager>,
    sensors: std::sync::Arc<SensorHub>,
    curve_engine: std::sync::Arc<CurveEngine>,
    /// Familles de conflit arrêtées automatiquement au lancement cette
    /// session (auto_manage_conflicts) — à redémarrer à la fermeture.
    auto_stopped: std::sync::Arc<Mutex<Vec<String>>>,
    /// Capture USB en cours (panneau Diagnostic) — une seule à la fois.
    active_capture: Mutex<Option<usbcapture::CaptureSession>>,
}

























































/// Restaure effets (globaux + zones) et modes matériels sauvegardés.
/// Suppose un scan déjà fait (device_list non vide).
pub(crate) fn restore_saved_state(registry: &SharedRegistry, engine: &EffectsEngine, saved: &Settings) {
    let devices = registry.lock().device_list();
    for d in &devices {
        if let Some(cfg) = saved.effects.get(&d.id) {
            engine.set_effect(d.id.clone(), cfg.clone(), d.led_count);
        }
        for (z_idx, zi) in d.zones.iter().enumerate() {
            let key = format!("{}#z{}", d.id, z_idx);
            if let Some(cfg) = saved.effects.get(&key) {
                let offset: u32 = d.zones.iter().take(z_idx).map(|z| z.led_count).sum();
                engine.set_zone_effect(
                    d.id.clone(),
                    z_idx as u32,
                    cfg.clone(),
                    offset,
                    zi.led_count,
                    d.led_count,
                );
            }
        }
    }
    let mut reg = registry.lock();
    for (device_id, m) in &saved.hw_modes {
        let Some((backend, local)) = device_id.split_once(':') else {
            continue;
        };
        if backend != "openrgb" {
            continue;
        }
        for b in reg.backends_mut() {
            if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    if let Err(e) =
                        orgb.set_mode(local, m.mode_index, m.speed, m.direction, m.colors.clone())
                    {
                        log::warn!("restauration mode matériel {device_id}: {e:#}");
                    }
                }
            }
        }
    }
}

/// Nettoyage avant sortie (exécuté dans un thread borné, voir `shutdown.rs`) : jamais sur le thread d'interface.
fn cleanup_before_exit(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    // D'abord ce qui est rapide et qui libère les processus et le matériel...
    state.engine.shutdown();
    state.curve_engine.shutdown();
    // Rendre les headers PWM au BIOS avant de couper sensord.
    {
        let mut reg = state.registry.lock();
        for b in reg.backends_mut() {
            if let Some(m) = b.as_any_mut().downcast_mut::<MoboFanBackend>() {
                m.release_all();
            } else if let Some(h) = b.as_any_mut().downcast_mut::<HidBackend>() {
                h.release_fans_all(); // hubs natifs : synchro carte mère / valeur d'avant ; rien si non réglés
            }
        }
    }
    gamepad_remap::stop(); // manette virtuelle détruite avant tout
    state.sensors.stop();
    state.openrgb_mgr.stop();
    // Arrêter une éventuelle capture USB en cours (pas de USBPcapCMD.exe orphelin).
    if let Some(session) = state.active_capture.lock().take() {
        log::info!("capture USB en cours arrêtée à la fermeture de l'app");
        usbcapture::stop_capture(session);
    }
    app.state::<std::sync::Arc<screens::manager::ScreenManager>>().shutdown();
    // ...puis, en dernier (peut être lent), redémarrer les logiciels constructeur arrêtés
    // automatiquement au lancement. Best-effort : l'échéance de `shutdown.rs` borne le tout.
    let families = state.auto_stopped.lock().clone();
    for key in families {
        if let Err(e) = conflicts::restore_family(&key, &std::collections::HashMap::new()) {
            log::warn!("redémarrage auto {key}: {e:#}");
        }
    }
}

/// Journal dans %APPDATA%\PureRGB\purergb.log (l'exe n'a pas de console) ; repli stderr. Repart de zéro au-delà de 1 Mo.
fn init_logging() {
    let mut b = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Some(dir) = settings::dirs_dir() {
        let path = dir.join("purergb.log");
        let big = std::fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20);
        let _ = std::fs::create_dir_all(&dir);
        let file = std::fs::OpenOptions::new().create(true).write(true).append(!big).truncate(big).open(&path);
        if let Ok(f) = file {
            b.target(env_logger::Target::Pipe(Box::new(f)));
        }
    }
    b.init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Tous les processus enfants meurent avec PureRGB (même en cas de plantage).
    job::kill_children_on_exit();
    init_logging();

    let saved = settings::load();
    // Avant tout démarrage matériel : un jeu protégé déjà lancé empêche OpenRGB, sensord et les écrans de démarrer.
    let game_at_start = gamemode::precheck(&saved);
    // Créer settings.json dès le premier lancement (valeurs par défaut visibles).
    if let Err(e) = settings::save(&saved) {
        log::warn!("écriture settings initiale: {e:#}");
    }
    let settings_shared: std::sync::Arc<Mutex<Settings>> =
        std::sync::Arc::new(Mutex::new(saved.clone()));
    let sensors = SensorHub::new();
    let backends: Vec<Box<dyn Backend>> = vec![
        Box::new(OpenRgbBackend::new(
            saved.openrgb_host.clone(),
            saved.openrgb_port,
        )),
        Box::new(HidBackend::new(saved.native_drivers_enabled)),
        Box::new(SmbusBackend::new(saved.native_drivers_enabled, saved.smbus_writes_enabled)),
        Box::new(LiquidctlBackend::new()),
        Box::new(MoboFanBackend::new(sensors.clone())),
    ];
    let registry = DeviceRegistry::shared(backends);
    let engine = EffectsEngine::start(registry.clone());
    engine.set_fps(saved.fps);
    let openrgb_mgr = std::sync::Arc::new(OpenRgbManager::new());
    // Port d'OpenRGB changé automatiquement (le port réglé était pris) : on l'enregistre et on réoriente le client.
    {
        let (registry, settings_shared) = (registry.clone(), settings_shared.clone());
        openrgb_mgr.set_port_hook(Box::new(move |port| {
            let host = {
                let mut s = settings_shared.lock();
                s.openrgb_port = port;
                if let Err(e) = settings::save(&s) {
                    log::warn!("enregistrement du nouveau port OpenRGB: {e:#}");
                }
                s.openrgb_host.clone()
            };
            if let Some(mut reg) = registry.try_lock_for(std::time::Duration::from_secs(5)) {
                for b in reg.backends_mut() {
                    if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                        orgb.set_endpoint(host.clone(), port);
                    }
                }
            }
        }));
    }
    let curve_engine = CurveEngine::start(registry.clone(), sensors.clone(), saved.curves.clone());

    let auto_stopped: std::sync::Arc<Mutex<Vec<String>>> = std::sync::Arc::new(Mutex::new(Vec::new()));

    // Démarrage matériel en arrière-plan : auto-start OpenRGB embarqué
    // (si activé et aucun serveur joignable), scan, restauration des effets.
    // Hors du thread UI — l'init OpenRGB peut prendre 20 s.
    {
        let registry = registry.clone();
        let engine = engine.clone();
        let mgr = openrgb_mgr.clone();
        let settings_shared = settings_shared.clone();
        let auto_stopped = auto_stopped.clone();
        std::thread::Builder::new()
            .name("hw-init".into())
            .spawn(move || {
                let saved = settings_shared.lock().clone();
                // Arrêt auto des logiciels constructeur en conflit AVANT le scan
                // matériel : libère les handles HID pour qu'OpenRGB détecte tout.
                // Réversible (disable=false), redémarrés au "Quitter" du tray.
                if saved.auto_manage_conflicts {
                    let (found, _) = conflicts::scan();
                    let mut stopped = auto_stopped.lock();
                    for fam in found.iter().filter(|f| f.active) {
                        match conflicts::stop_family(&fam.family, false) {
                            Ok(_) => {
                                log::info!("conflit auto-arrêté au lancement: {}", fam.name);
                                stopped.push(fam.family.clone());
                            }
                            Err(e) => log::warn!("auto-arrêt {}: {e:#}", fam.name),
                        }
                    }
                }
                // Config réseau à jour AVANT le démarrage du serveur : les
                // détecteurs réseau ne lisent OpenRGB.json qu'au boot.
                if !saved.network_devices.is_empty() {
                    match netdev::openrgb_config_path() {
                        Ok(path) => {
                            if let Err(e) = netdev::sync_openrgb_config(&saved.network_devices, &path) {
                                log::warn!("synchro appareils réseau: {e:#}");
                            }
                        }
                        Err(e) => log::warn!("chemin OpenRGB.json: {e:#}"),
                    }
                }
                if saved.auto_start_openrgb {
                    match mgr.ensure_running(&saved.openrgb_host, saved.openrgb_port) {
                        Ok(true) => log::info!("OpenRGB embarqué démarré"),
                        Ok(false) => log::info!("serveur OpenRGB déjà actif"),
                        Err(e) => log::warn!("auto-start OpenRGB: {e:#}"),
                    }
                }
                scan_with_zone_sizes(&mut registry.lock(), &saved.zone_sizes);

                restore_saved_state(&registry, &engine, &saved);
            })
            .expect("spawn hw-init");
    }

    // Télémétrie de démarrage : entièrement séparée de `hw-init` pour ne
    // jamais retarder scan_with_zone_sizes()/restore_saved_state() (le curl
    // 3 s de refresh_known_devices() + le curl 5 s de maybe_send_report()
    // bloqueraient sinon la restauration visible des effets RGB, même pour
    // les utilisateurs hors-ligne ou non opt-in). Tourne en parallèle.
    {
        std::thread::Builder::new()
            .name("telemetry-init".into())
            .spawn(move || {
                // Table de reconnaissance distante (diagnostic uniquement,
                // jamais utilisée pour le pilotage réel — voir known_remote.rs).
                telemetry::refresh_known_devices();
            })
            .expect("spawn telemetry-init");
    }

    // Mini-écrans : détection en arrière-plan (ne retarde ni la fenêtre ni le RGB).
    let screen_mgr = screens::manager::ScreenManager::new(sensors.clone());
    {
        let m = screen_mgr.clone();
        if let Err(e) = std::thread::Builder::new().name("screens-init".into()).spawn(move || {
            // PURERGB_SIM_SCREEN : écran simulé au démarrage (tests d'acceptation de fermeture).
            let n = m.scan(std::env::var_os("PURERGB_SIM_SCREEN").is_some()).len();
            log::info!("mini-écrans détectés au démarrage : {n}");
        }) {
            log::warn!("thread screens-init: {e}");
        }
    }

    let state = AppState {
        registry,
        engine,
        settings: settings_shared,
        openrgb_mgr,
        sensors,
        curve_engine,
        auto_stopped,
        active_capture: Mutex::new(None),
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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state)
        .manage(screen_mgr)
        .setup(move |app| {
            gamemode::start(app.handle(), game_at_start);
            gamepad_remap::start(app.handle());
            shutdown::schedule_autoquit(app.handle(), cleanup_before_exit);
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
                let state = app.state::<AppState>();
                let sensors = state.sensors.clone();
                let registry = state.registry.clone();
                std::thread::Builder::new()
                    .name("sensord-start".into())
                    .spawn(move || match sensors.start() {
                        Ok(true) => {
                            log::info!("sensord démarré");
                            // Premier relevé après ~2 s : re-scanner pour faire
                            // apparaître les ventilateurs carte mère (backend mobo).
                            std::thread::sleep(std::time::Duration::from_secs(6));
                            registry.lock().scan_all();
                        }
                        Ok(false) => log::info!("sensord absent"),
                        Err(e) => log::warn!("sensord: {e:#}"),
                    })?;
            }
            // Garde anti-relance : certains logiciels (Corsair.Service) se
            // relancent seuls malgré service désactivé + tâche neutralisée.
            // Balayage périodique tant que l'app tourne, familles opt-in.
            {
                let app_handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("conflict-guard".into())
                    .spawn(move || loop {
                        std::thread::sleep(std::time::Duration::from_secs(12));
                        let state = app_handle.state::<AppState>();
                        let families: Vec<String> =
                            state.settings.lock().guarded_families.iter().cloned().collect();
                        drop(state);
                        for key in families {
                            if let Err(e) = conflicts::stop_family(&key, false) {
                                log::debug!("garde conflit {key}: {e:#}");
                            }
                        }
                    })?;
            }
            // Sondage périodique des nouveaux appareils USB non reconnus —
            // propose un diagnostic sans attendre que l'utilisateur remarque
            // lui-même qu'un appareil ne fonctionne pas. Premier sondage =
            // référence silencieuse (tout ce qui est déjà branché n'est pas
            // "nouveau"). Voir hotplug::diff_new_unrecognized.
            {
                let app_handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("usb-hotplug".into())
                    .spawn(move || {
                        let mut previous: std::collections::HashSet<(String, String)> =
                            std::collections::HashSet::new();
                        let mut first = true;
                        let mut last_rescan: Option<std::time::Instant> = None;
                        let mut orgb_was_up = false;
                        let mut screen_try: Option<(Vec<String>, std::time::Instant)> = None;
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                            if gamemode::active() {
                                continue; // mode jeu : ni re-détection ni relance de pilote pendant la partie
                            }
                            let state = app_handle.state::<AppState>();
                            let current: Vec<crate::backends::hid::RawHidDevice> = {
                                let mut reg = state.registry.lock();
                                reg.backends_mut()
                                    .iter_mut()
                                    .find(|b| b.name() == "hid")
                                    .and_then(|b| b.as_any_mut().downcast_mut::<HidBackend>())
                                    .and_then(|hid| hid.list_raw().ok())
                                    .unwrap_or_default()
                            };
                            drop(state);
                            let current_keys: std::collections::HashSet<(String, String)> = current
                                .iter()
                                .map(|d| (d.vid.clone(), d.pid.clone()))
                                .collect();
                            let (host, port) = {
                                let st = app_handle.state::<AppState>();
                                let g = st.settings.lock();
                                (g.openrgb_host.clone(), g.openrgb_port)
                            };
                            let orgb_up = backends::openrgb::manager::OpenRgbManager::serves_openrgb(&host, port);
                            // OpenRGB a fini par démarrer après un échec (init lente) : on re-scanne sans attendre un clic.
                            if orgb_up && !orgb_was_up && !first && app_handle.state::<AppState>().openrgb_mgr.take_error() {
                                rescan_and_notify(&app_handle, "OpenRGB démarré tardivement");
                            }
                            orgb_was_up = orgb_up;
                            // Mini-écran branché mais sans pilote actif : on retente (changement d'état, sinon 1 fois par minute).
                            let sm = app_handle.state::<std::sync::Arc<screens::manager::ScreenManager>>();
                            let idle = sm.plugged_but_idle();
                            if !first && !idle.is_empty() && screen_try.as_ref().map_or(true, |(set, t)| *set != idle || t.elapsed() > std::time::Duration::from_secs(60)) {
                                screen_try = Some((idle, std::time::Instant::now()));
                                let n = sm.scan(false).len();
                                log::info!("écran USB sans pilote actif : nouvelle détection, {n} écran(s)");
                                if let Err(e) = app_handle.emit("screens-changed", n) {
                                    log::debug!("émission screens-changed : {e:#}");
                                }
                            }
                            if first {
                                previous = current_keys;
                                first = false;
                                continue;
                            }
                            let new_unrecognized = hotplug::diff_new_unrecognized(&previous, &current);
                            let new_rgb = hotplug::diff_new_rgb(&previous, &current);
                            previous = current_keys;
                            // Au plus une re-détection par 2 min : un appareil qui se réénumère en boucle
                            // ne doit jamais relancer OpenRGB sans cesse.
                            let calm = last_rescan.map_or(true, |t| t.elapsed() > std::time::Duration::from_secs(120));
                            if !new_rgb.is_empty() && calm {
                                last_rescan = Some(std::time::Instant::now());
                                rescan_after_plug(&app_handle, &new_rgb);
                            }
                            if new_unrecognized.is_empty() {
                                continue;
                            }
                            log::info!(
                                "hotplug: {} nouvel(x) appareil(s) non reconnu(s)",
                                new_unrecognized.len()
                            );
                            if let Err(e) = app_handle.emit("unknown-device-detected", &new_unrecognized) {
                                log::debug!("émission événement hotplug: {e:#}");
                            }
                            let body = if new_unrecognized.len() == 1 {
                                format!(
                                    "{} {} ({}:{})",
                                    new_unrecognized[0].manufacturer,
                                    new_unrecognized[0].product,
                                    new_unrecognized[0].vid,
                                    new_unrecognized[0].pid
                                )
                            } else {
                                format!("{} nouveaux appareils non reconnus", new_unrecognized.len())
                            };
                            use tauri_plugin_notification::NotificationExt;
                            if let Err(e) =
                                app_handle.notification().builder().title("PureRGB").body(body).show()
                            {
                                log::debug!("notification OS hotplug: {e:#}");
                            }
                        }
                    })?;
            }

            // Télémétrie opt-in : un premier rapport quand le démarrage a eu le temps de finir (OpenRGB, écrans), puis
            // un rapport toutes les 30 min si quelque chose a changé.
            {
                let app_handle = app.handle().clone();
                std::thread::Builder::new().name("telemetry-report".into()).spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(90));
                    loop {
                        if app_handle.state::<AppState>().settings.lock().telemetry_opt_in {
                            match diagnostics_json(&app_handle) {
                                Ok(json) => match telemetry::maybe_send_report(&json, env!("CARGO_PKG_VERSION")) {
                                    Ok(true) => log::info!("rapport télémétrie envoyé"),
                                    Ok(false) => log::debug!("rapport télémétrie inchangé"),
                                    Err(e) => log::warn!("envoi télémétrie: {e:#}"),
                                },
                                Err(e) => log::warn!("sérialisation diagnostic télémétrie: {e:#}"),
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_secs(1800));
                    }
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
                    "quit" => shutdown::quit_async(app.clone(), cleanup_before_exit),
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
            commands::devices::scan_devices,
            commands::devices::list_devices,
            commands::devices::backend_status,
            commands::devices::apply_effect,
            commands::devices::apply_effect_all,
            commands::devices::set_fan_duty,
            commands::conflicts_cmds::check_conflicts,
            commands::conflicts_cmds::conflict_stop,
            commands::conflicts_cmds::conflict_restore,
            commands::hardware::get_sensors,
            commands::hardware::set_curve,
            commands::hardware::lcd_apply,
            commands::devices::set_hardware_mode,
            commands::profiles::set_autostart,
            commands::profiles::profile_export,
            commands::profiles::profile_import,
            commands::openrgb_cmds::openrgb_status,
            commands::openrgb_cmds::openrgb_start,
            commands::openrgb_cmds::openrgb_stop,
            commands::openrgb_cmds::openrgb_restart,
            commands::openrgb_cmds::pawnio_install,
            gamemode::game_mode_status,
            gamemode::set_game_mode,
            gamemode::quit_app,
            gamepad_remap::commands::gamepad_remap_status,
            gamepad_remap::commands::gamepad_remap_drivers,
            gamepad_remap::commands::gamepad_remap_enable,
            gamepad_remap::commands::gamepad_remap_save_profile,
            gamepad_remap::commands::gamepad_remap_delete_profile,
            gamepad_remap::commands::gamepad_remap_assign,
            gamepad_remap::commands::gamepad_remap_calibrate,
            gamepad_remap::commands::gamepad_remap_install_driver,
            commands::profiles::get_settings,
            commands::profiles::update_settings,
            product_commands::product_lookup,
            product_commands::product_image,
            product_commands::product_open_source,
            product_commands::set_product_photos,
            commands::devices::resize_zone,
            commands::openrgb_cmds::netdev_list,
            commands::openrgb_cmds::netdev_add,
            commands::openrgb_cmds::netdev_remove,
            commands::openrgb_cmds::hue_pair,
            commands::openrgb_cmds::nanoleaf_pair,
            commands::conflicts_cmds::conflict_guard_set,
            commands::diagnostics::hardware_diagnostics,
            commands::diagnostics::rgb_inventory,
            commands::diagnostics::selftest_scan,
            commands::diagnostics::selftest_apply,
            commands::diagnostics::send_telemetry_report,
            commands::feedback::device_feedback_preview,
            commands::feedback::device_feedback_send,
            commands::feedback::community_feedback_list,
            commands::diagnostics::usb_capture_ready,
            commands::diagnostics::usb_capture_install,
            commands::diagnostics::usb_capture_start,
            commands::diagnostics::usb_capture_stop,
            commands::diagnostics::usb_capture_upload,
            screens::commands::screens_list,
            screens::commands::screens_scan,
            screens::commands::screens_errors,
            screens::commands::screen_state,
            screens::commands::screen_set_layout,
            screens::commands::screen_set_options,
            screens::commands::screen_clear,
            screens::commands::screen_send_still,
            screens::commands::screen_preview,
            screens::commands::screen_values,
            screens::commands::screen_sensors,
            screens::commands::designs_list,
            screens::commands::design_get,
            screens::commands::design_user_save,
            screens::commands::design_user_delete,
            screens::commands::design_export,
            screens::commands::design_import,
            screens::commands::media_list,
            screens::commands::media_import,
            screens::commands::media_delete,
            screens::commands::ffmpeg_status,
            screens::commands::ffmpeg_install,
            screens::commands::ffmpeg_install_progress,
            screens::commands::theme_export,
            screens::commands::theme_import
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PureRGB");
}


