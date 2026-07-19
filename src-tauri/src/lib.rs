mod backends;
mod conflicts;
mod core;
mod engine;
mod netdev;
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

/// Ré-applique les tailles de zones ARGB sauvegardées (clé nom appareil +
/// nom zone : stable entre les redémarrages, contrairement à l'index).
/// Retourne true si au moins un resize a été envoyé (=> re-scan nécessaire).
fn apply_saved_zone_sizes(
    reg: &mut crate::core::registry::DeviceRegistry,
    sizes: &std::collections::HashMap<String, u32>,
) -> bool {
    if sizes.is_empty() {
        return false;
    }
    let devices = reg.device_list();
    let mut resized = false;
    for d in devices.iter().filter(|d| d.backend == "openrgb") {
        let Some(local) = d.id.strip_prefix("openrgb:") else {
            continue;
        };
        for (z_idx, z) in d.zones.iter().enumerate() {
            let key = format!("{}|{}", d.name, z.name);
            let Some(&wanted) = sizes.get(&key) else {
                continue;
            };
            if wanted == z.led_count || !z.resizable() {
                continue;
            }
            let clamped = wanted.clamp(z.leds_min, z.leds_max);
            for b in reg.backends_mut() {
                if b.name() == "openrgb" {
                    if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                        match orgb.resize_zone(local, z_idx as u32, clamped) {
                            Ok(()) => resized = true,
                            Err(e) => log::warn!("resize zone {key}: {e:#}"),
                        }
                    }
                }
            }
        }
    }
    resized
}

/// Scan complet + ré-application des tailles de zones sauvegardées.
fn scan_with_zone_sizes(
    reg: &mut crate::core::registry::DeviceRegistry,
    sizes: &std::collections::HashMap<String, u32>,
) -> Vec<DeviceInfo> {
    let devices = reg.scan_all();
    if apply_saved_zone_sizes(reg, sizes) {
        return reg.scan_all();
    }
    devices
}

#[tauri::command]
fn scan_devices(state: State<AppState>) -> Vec<DeviceInfo> {
    let sizes = state.settings.lock().zone_sizes.clone();
    let devices = scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
    // Le matériel a pu être réinitialisé : forcer la ré-application des effets.
    state.engine.invalidate();
    devices
}

/// Redimensionne une zone ARGB (nombre de LEDs branchées sur un connecteur
/// carte mère ou un canal de hub) et persiste le choix.
#[tauri::command(async)]
fn resize_zone(
    state: State<AppState>,
    device_id: String,
    zone: u32,
    new_size: u32,
) -> Result<(), String> {
    let (backend, local) = device_id
        .split_once(':')
        .ok_or_else(|| format!("id invalide: {device_id}"))?;
    if backend != "openrgb" {
        return Err("zones redimensionnables uniquement via OpenRGB".into());
    }
    let local = local.to_string();
    let key = {
        let reg = state.registry.lock();
        let d = reg
            .get(&device_id)
            .ok_or_else(|| format!("appareil inconnu: {device_id}"))?;
        let z = d
            .zones
            .get(zone as usize)
            .ok_or_else(|| format!("zone {zone} inconnue"))?;
        if !z.resizable() {
            return Err(format!("la zone « {} » n'est pas redimensionnable", z.name));
        }
        if new_size < z.leds_min || new_size > z.leds_max {
            return Err(format!(
                "taille hors bornes ({}-{})",
                z.leds_min, z.leds_max
            ));
        }
        format!("{}|{}", d.name, z.name)
    };
    {
        let mut reg = state.registry.lock();
        let mut done = false;
        for b in reg.backends_mut() {
            if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    orgb.resize_zone(&local, zone, new_size)
                        .map_err(|e| format!("{e:#}"))?;
                    done = true;
                }
            }
        }
        if !done {
            return Err("backend openrgb indisponible".into());
        }
        reg.scan_all();
    }
    state.engine.invalidate();
    let mut s = state.settings.lock();
    s.zone_sizes.insert(key, new_size);
    settings::save(&s).map_err(|e| e.to_string())
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

/// Applique un effet logiciel. `zone: None` = appareil entier (remplace les
/// effets de zone) ; `Some(i)` = uniquement la zone i (superposée au global).
#[tauri::command]
fn apply_effect(
    state: State<AppState>,
    device_id: String,
    config: EffectConfig,
    zone: Option<u32>,
) -> Result<(), String> {
    let (led_count, zone_bounds) = {
        let reg = state.registry.lock();
        let d = reg
            .get(&device_id)
            .ok_or_else(|| format!("appareil inconnu: {device_id}"))?;
        let bounds = zone.map(|z| {
            let offset: u32 = d.zones.iter().take(z as usize).map(|zi| zi.led_count).sum();
            let len = d.zones.get(z as usize).map(|zi| zi.led_count).unwrap_or(0);
            (offset, len)
        });
        (d.led_count, bounds)
    };
    let mut s = state.settings.lock();
    match (zone, zone_bounds) {
        (Some(z), Some((offset, len))) => {
            if len == 0 {
                return Err(format!("zone {z} inconnue ou vide"));
            }
            state
                .engine
                .set_zone_effect(device_id.clone(), z, config.clone(), offset, len, led_count);
            s.effects.insert(format!("{device_id}#z{z}"), config);
        }
        _ => {
            state
                .engine
                .set_effect(device_id.clone(), config.clone(), led_count);
            // Effet global : purge les zones sauvegardées de cet appareil.
            s.effects.retain(|k, _| !k.starts_with(&format!("{device_id}#z")));
            s.effects.insert(device_id.clone(), config);
        }
    }
    // Reprendre la main sur un éventuel mode matériel.
    s.hw_modes.remove(&device_id);
    settings::save(&s).map_err(|e| e.to_string())
}

/// Applique un mode matériel natif OpenRGB (le firmware anime tout seul).
#[tauri::command]
fn set_hardware_mode(
    state: State<AppState>,
    device_id: String,
    mode_index: u32,
    speed: Option<u32>,
    direction: Option<u32>,
    colors: Option<Vec<crate::core::Color>>,
) -> Result<(), String> {
    let (backend, local) = device_id
        .split_once(':')
        .ok_or_else(|| format!("id invalide: {device_id}"))?;
    if backend != "openrgb" {
        return Err("modes matériels disponibles uniquement via OpenRGB".into());
    }
    let local = local.to_string();
    {
        let mut reg = state.registry.lock();
        let mut done = false;
        for b in reg.backends_mut() {
            if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    orgb.set_mode(&local, mode_index, speed, direction, colors.clone())
                        .map_err(|e| format!("{e:#}"))?;
                    done = true;
                }
            }
        }
        if !done {
            return Err("backend openrgb indisponible".into());
        }
    }
    // Le firmware pilote : retirer les effets logiciels de cet appareil.
    state.engine.remove_effect(&device_id);
    let mut s = state.settings.lock();
    s.effects
        .retain(|k, _| k != &device_id && !k.starts_with(&format!("{device_id}#z")));
    s.hw_modes.insert(
        device_id,
        settings::SavedHwMode {
            mode_index,
            speed,
            direction,
            colors,
        },
    );
    settings::save(&s).map_err(|e| e.to_string())
}

/// Active/désactive le lancement au démarrage de Windows via une tâche
/// planifiée niveau Highest (pas de prompt UAC à chaque boot).
#[tauri::command(async)]
fn set_autostart(state: State<AppState>, enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let run = |args: &[&str]| -> Result<std::process::Output, String> {
        let mut cmd = std::process::Command::new("schtasks.exe");
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.output().map_err(|e| e.to_string())
    };
    if enabled {
        let tr = format!("\"{}\"", exe.display());
        let out = run(&[
            "/Create", "/TN", "PureRGB Autostart", "/TR", &tr, "/SC", "ONLOGON",
            "/RL", "HIGHEST", "/F",
        ])?;
        if !out.status.success() {
            return Err(format!(
                "schtasks: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    } else {
        let _ = run(&["/Delete", "/TN", "PureRGB Autostart", "/F"]);
    }
    let mut s = state.settings.lock();
    s.autostart = enabled;
    settings::save(&s).map_err(|e| e.to_string())
}

/// Exporte la configuration complète (effets, courbes, modes, réglages).
#[tauri::command]
fn profile_export(state: State<AppState>, path: String) -> Result<(), String> {
    let s = state.settings.lock();
    let json = serde_json::to_string_pretty(&*s).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Importe un profil et l'applique immédiatement.
#[tauri::command(async)]
fn profile_import(state: State<AppState>, path: String) -> Result<(), String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: Settings = serde_json::from_str(&text).map_err(|e| format!("profil invalide: {e}"))?;
    {
        let mut s = state.settings.lock();
        *s = imported.clone();
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    state.engine.set_fps(imported.fps);
    state.curve_engine.set_curves(imported.curves.clone());
    restore_saved_state(&state.registry, &state.engine, &imported);
    Ok(())
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
    let sizes = state.settings.lock().zone_sizes.clone();
    scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
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
    let sizes = state.settings.lock().zone_sizes.clone();
    scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
    state.engine.invalidate();
    Ok(launched)
}

/// Synchronise les appareils réseau vers OpenRGB.json puis relance le
/// serveur géré pour re-détecter (les détecteurs réseau ne lisent la config
/// qu'au démarrage).
fn netdev_sync_and_reload(state: &State<AppState>) -> Result<(), String> {
    let (devices, host, port) = {
        let s = state.settings.lock();
        (s.network_devices.clone(), s.openrgb_host.clone(), s.openrgb_port)
    };
    let path = netdev::openrgb_config_path().map_err(|e| e.to_string())?;
    netdev::sync_openrgb_config(&devices, &path).map_err(|e| format!("{e:#}"))?;
    let status = state.openrgb_mgr.status(&host, port);
    if status.managed {
        state.openrgb_mgr.stop();
        state
            .openrgb_mgr
            .ensure_running(&host, port)
            .map_err(|e| format!("{e:#}"))?;
    }
    let sizes = state.settings.lock().zone_sizes.clone();
    scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
    state.engine.invalidate();
    Ok(())
}

#[tauri::command]
fn netdev_list(state: State<AppState>) -> Vec<netdev::NetworkDevice> {
    state.settings.lock().network_devices.clone()
}

/// Ajoute un appareil réseau, écrit la config OpenRGB et relance le serveur.
#[tauri::command(async)]
fn netdev_add(state: State<AppState>, device: netdev::NetworkDevice) -> Result<(), String> {
    device.validate().map_err(|e| e.to_string())?;
    {
        let mut s = state.settings.lock();
        // Doublon exact (même kind + même IP) : remplacer au lieu d'empiler.
        s.network_devices.retain(|d| {
            !(d.ip() == device.ip()
                && std::mem::discriminant(d) == std::mem::discriminant(&device))
        });
        s.network_devices.push(device);
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    netdev_sync_and_reload(&state)
}

#[tauri::command(async)]
fn netdev_remove(state: State<AppState>, index: usize) -> Result<(), String> {
    {
        let mut s = state.settings.lock();
        if index >= s.network_devices.len() {
            return Err("index invalide".into());
        }
        s.network_devices.remove(index);
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    netdev_sync_and_reload(&state)
}

/// Étape 1 de l'appairage Hue : récupère la MAC du pont. L'utilisateur doit
/// ensuite appuyer sur le bouton du pont — OpenRGB crée le username au
/// redémarrage suivant.
#[tauri::command(async)]
fn hue_pair(ip: String) -> Result<String, String> {
    netdev::hue_fetch_mac(&ip).map_err(|e| format!("{e:#}"))
}

/// Appairage Nanoleaf : récupère l'auth_token (appareil en mode appairage).
#[tauri::command(async)]
fn nanoleaf_pair(ip: String, port: u16) -> Result<String, String> {
    netdev::nanoleaf_request_token(&ip, port).map_err(|e| format!("{e:#}"))
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

/// Restaure effets (globaux + zones) et modes matériels sauvegardés.
/// Suppose un scan déjà fait (device_list non vide).
fn restore_saved_state(registry: &SharedRegistry, engine: &EffectsEngine, saved: &Settings) {
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
            set_hardware_mode,
            set_autostart,
            profile_export,
            profile_import,
            openrgb_status,
            openrgb_start,
            openrgb_stop,
            openrgb_restart,
            pawnio_install,
            get_settings,
            update_settings,
            resize_zone,
            netdev_list,
            netdev_add,
            netdev_remove,
            hue_pair,
            nanoleaf_pair
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PureRGB");
}
