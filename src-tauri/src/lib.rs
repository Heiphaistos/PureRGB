mod background;
mod backends;
mod boot;
mod commands;
mod conflicts;
mod device_feedback;
mod core;
mod engine;
mod diag_extra;
mod diag_report;
mod gamemode;
mod gamepad_remap;
mod hotplug;
mod inventory;
mod led_catalog;
mod selftest;
mod self_update;
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
mod xinput;

use crate::backends::hid::HidBackend;
use crate::backends::mobo::MoboFanBackend;
use crate::backends::openrgb::manager::OpenRgbManager;
use crate::backends::openrgb::OpenRgbBackend;
use crate::core::registry::SharedRegistry;
use crate::engine::curves::CurveEngine;
use crate::engine::EffectsEngine;
use crate::sensors::SensorHub;
use crate::settings::Settings;
use parking_lot::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
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
pub(crate) fn cleanup_before_exit(app: &tauri::AppHandle) {
    cleanup(app, background::Modules::NONE, true);
}

/// Nettoyage des chemins « Quitter » (menu, bouton, test) : comme `cleanup_before_exit`, mais laisse un processus
/// `--background` prendre le relais des modules cochés ET déjà actifs (écrans avec design, éclairage appliqué,
/// courbes de ventilateurs existantes, remappage manette activé).
/// Pas utilisé pour les mises à jour : l'exécutable en cours ne doit pas rester ouvert.
pub(crate) fn cleanup_for_quit(app: &tauri::AppHandle) {
    let sm = app.state::<std::sync::Arc<screens::manager::ScreenManager>>();
    let (chosen, facts) = {
        let s = app.state::<AppState>().settings.lock().clone();
        (
            s.background,
            background::Facts {
                screen_design_active: sm.has_active_design(),
                game_active: gamemode::active(),
                saved_screen_design: screens::state::load().values().any(|c| c.layout.is_some()),
                has_lighting: !s.effects.is_empty() || !s.hw_modes.is_empty(),
                has_fan_settings: !s.curves.is_empty(),
                gamepad_enabled: s.gamepad_remap_enabled,
            },
        )
    };
    cleanup(app, background::modules_to_keep(chosen, facts), true);
}

/// `keep` : modules confiés au processus d'arrière-plan. `restore_vendor` : relancer les logiciels constructeur
/// arrêtés au lancement (faux quand quelqu'un d'autre reprend la main : ils restent arrêtés, liste transmise).
pub(crate) fn cleanup(app: &tauri::AppHandle, keep: background::Modules, restore_vendor: bool) {
    let restore_vendor = restore_vendor && !keep.any();
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
    // Le processus écrans seuls démarre AVANT la libération de l'écran (il réessaie jusqu'à ce qu'il soit libre) :
    // coupure la plus courte possible. Sidecars déjà arrêtés ci-dessus, donc le job peut être relâché.
    if keep.any() {
        match background::spawn_daemon(keep) {
            Ok(()) => log::info!("arrière-plan lancé ({}) : ces modules restent actifs sans fenêtre", keep.to_list()),
            Err(e) => log::warn!("lancement de l'arrière-plan : {e}"),
        }
    }
    app.state::<std::sync::Arc<screens::manager::ScreenManager>>().shutdown();
    // ...puis, en dernier (peut être lent), redémarrer les logiciels constructeur arrêtés
    // automatiquement au lancement. Best-effort : l'échéance de `shutdown.rs` borne le tout.
    let families = state.auto_stopped.lock().clone();
    if !restore_vendor {
        background::handover_save(&families); // pris en charge par le processus suivant
        return;
    }
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
    self_update::cleanup_old();
    init_logging();
    let ctx = tauri::generate_context!();
    if let background::Mode::Background(mods) = background::parse_mode(std::env::args()) {
        background::run(ctx, mods);
        return;
    }
    // Un processus « écrans seuls » tient peut-être l'écran : on le libère avant toute détection.
    background::stop_running_daemon();

    let saved = settings::load();
    // Avant tout démarrage matériel : un jeu protégé déjà lancé empêche OpenRGB, sensord et les écrans de démarrer.
    let game_at_start = gamemode::precheck(&saved);
    // Créer settings.json dès le premier lancement (valeurs par défaut visibles).
    if let Err(e) = settings::save(&saved) {
        log::warn!("écriture settings initiale: {e:#}");
    }
    let mods = background::Modules::ALL;
    let (state, screen_mgr) = boot::build(&saved, mods);
    // Reprend la liste des logiciels constructeur arrêtés par un processus d'arrière-plan précédent.
    state.auto_stopped.lock().extend(background::handover_take());
    boot::start_hw_init(&state, mods, saved.auto_manage_conflicts);

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
            shutdown::schedule_autoquit(app.handle(), cleanup_for_quit);
            boot::set_resources(app.handle());
            boot::start_sensord(app.handle())?;
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
            let quit = MenuItem::with_id(app, "quit", "Quitter (garde l'arrière-plan coché)", true, None::<&str>)?;
            let quit_all = MenuItem::with_id(app, "quit_all", "Quitter complètement", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit, &quit_all])?;
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
                    "quit" => shutdown::quit_async(app.clone(), cleanup_for_quit),
                    "quit_all" => shutdown::quit_async(app.clone(), cleanup_before_exit),
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
            background::background_get,
            background::background_set,
            background::background_status,
            xinput::xinput_state,
            background::background_stop,
            self_update::update_is_portable,
            self_update::update_prepare,
            self_update::update_portable,
            commands::devices::scan_devices,
            commands::devices::list_devices,
            commands::devices::backend_status,
            commands::devices::apply_effect,
            commands::devices::apply_effect_all,
            commands::devices::set_fan_duty,
            commands::devices::set_device_setting,
            commands::conflicts_cmds::check_conflicts,
            commands::conflicts_cmds::conflict_stop,
            commands::conflicts_cmds::conflict_restore,
            commands::hardware::get_sensors,
            commands::hardware::get_ram_temp_status,
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
            gamepad_remap::commands::gamepad_remap_install_hidhide,
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
            commands::diagnostics::diagnostic_report,
            commands::diagnostics::diagnostic_report_save,
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
        .run(ctx)
        .expect("erreur au lancement de PureRGB");
}


