//! Construction et démarrage des moteurs matériel, partagés par l'application complète et le processus
//! d'arrière-plan (`--background`). `Modules` décide ce qui est créé : un module décoché n'a ni backend,
//! ni moteur de courbes, ni processus (pas d'OpenRGB sans « éclairage », pas de courbes sans « ventilateurs »…).

use crate::backends::hid::HidBackend;
use crate::backends::liquidctl::LiquidctlBackend;
use crate::backends::mobo::MoboFanBackend;
use crate::backends::openrgb::manager::OpenRgbManager;
use crate::backends::openrgb::OpenRgbBackend;
use crate::backends::smbus::SmbusBackend;
use crate::backends::Backend;
use crate::background::Modules;
use crate::commands::devices::scan_with_zone_sizes;
use crate::core::registry::DeviceRegistry;
use crate::engine::curves::{CurveEngine, CurveMap};
use crate::engine::EffectsEngine;
use crate::screens::manager::ScreenManager;
use crate::sensors::SensorHub;
use crate::settings::{self, Settings};
use crate::{conflicts, netdev, restore_saved_state, AppState};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Backends nécessaires aux modules choisis (ordre identique à l'application complète).
fn backends_for(saved: &Settings, mods: Modules, sensors: &Arc<SensorHub>) -> Vec<Box<dyn Backend>> {
    let mut v: Vec<Box<dyn Backend>> = Vec::new();
    if mods.lighting {
        v.push(Box::new(OpenRgbBackend::new(saved.openrgb_host.clone(), saved.openrgb_port)));
    }
    if mods.lighting || mods.fans {
        v.push(Box::new(HidBackend::new(saved.native_drivers_enabled)));
    }
    if mods.lighting {
        v.push(Box::new(SmbusBackend::new(saved.native_drivers_enabled, saved.smbus_writes_enabled)));
    }
    if mods.lighting || mods.fans {
        v.push(Box::new(LiquidctlBackend::new()));
    }
    if mods.fans {
        v.push(Box::new(MoboFanBackend::new(sensors.clone())));
    }
    v
}

pub(crate) fn build(saved: &Settings, mods: Modules) -> (AppState, Arc<ScreenManager>) {
    let settings_shared: Arc<Mutex<Settings>> = Arc::new(Mutex::new(saved.clone()));
    let sensors = SensorHub::new();
    let registry = DeviceRegistry::shared(backends_for(saved, mods, &sensors));
    let engine = EffectsEngine::start(registry.clone());
    engine.set_fps(saved.fps);
    let openrgb_mgr = Arc::new(OpenRgbManager::new());
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
            if let Some(mut reg) = registry.try_lock_for(Duration::from_secs(5)) {
                for b in reg.backends_mut() {
                    if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                        orgb.set_endpoint(host.clone(), port);
                    }
                }
            }
        }));
    }
    // Courbes : seulement avec le module « ventilateurs » (jamais d'écriture sans courbe déjà réglée).
    let curves = if mods.fans { saved.curves.clone() } else { CurveMap::new() };
    let curve_engine = CurveEngine::start(registry.clone(), sensors.clone(), curves);
    let screen_mgr = ScreenManager::new(sensors.clone());
    let state = AppState {
        registry,
        engine,
        settings: settings_shared,
        openrgb_mgr,
        sensors,
        curve_engine,
        auto_stopped: Arc::new(Mutex::new(Vec::new())),
        active_capture: Mutex::new(None),
    };
    (state, screen_mgr)
}

/// Démarrage matériel en arrière-plan : (application complète) arrêt des logiciels constructeur en conflit,
/// OpenRGB embarqué, scan, puis restauration des effets/modes sauvegardés, limités aux modules choisis.
pub(crate) fn start_hw_init(state: &AppState, mods: Modules, manage_conflicts: bool) {
    if !(mods.lighting || mods.fans) {
        return;
    }
    let registry = state.registry.clone();
    let engine = state.engine.clone();
    let mgr = state.openrgb_mgr.clone();
    let settings_shared = state.settings.clone();
    let auto_stopped = state.auto_stopped.clone();
    let spawned = std::thread::Builder::new().name("hw-init".into()).spawn(move || {
        let mut saved = settings_shared.lock().clone();
        if manage_conflicts && saved.auto_manage_conflicts {
            // AVANT le scan matériel : libère les handles HID pour qu'OpenRGB détecte tout. Réversible.
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
        if mods.lighting {
            // Config réseau à jour AVANT le démarrage du serveur : les détecteurs réseau ne lisent OpenRGB.json qu'au boot.
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
        } else {
            // Ventilateurs seuls : aucun effet d'éclairage ne doit être rejoué par les pilotes HID.
            saved.effects.clear();
            saved.hw_modes.clear();
        }
        scan_with_zone_sizes(&mut registry.lock(), &saved.zone_sizes);
        restore_saved_state(&registry, &engine, &saved);
    });
    if let Err(e) = spawned {
        log::error!("thread hw-init: {e}");
    }
}

/// Dossier ressources du bundle : openrgb/, liquidctl/, sensord/.
pub(crate) fn set_resources(app: &AppHandle) {
    let Ok(res) = app.path().resource_dir() else { return };
    let state = app.state::<AppState>();
    state.openrgb_mgr.set_resource_dir(res.clone());
    state.sensors.set_resource_dir(res.clone());
    crate::gamepad_remap::driver::set_resource_dir(res.clone());
    let mut reg = state.registry.lock();
    for b in reg.backends_mut() {
        if let Some(lc) = b.as_any_mut().downcast_mut::<LiquidctlBackend>() {
            lc.set_resource_dir(res.clone());
        }
    }
}

/// Capteurs : démarrage différé hors du thread UI (init LHM ~2-5 s), puis nouveau scan pour faire apparaître
/// les ventilateurs carte mère (backend mobo).
pub(crate) fn start_sensord(app: &AppHandle) -> std::io::Result<()> {
    let state = app.state::<AppState>();
    let sensors = state.sensors.clone();
    let registry = state.registry.clone();
    std::thread::Builder::new().name("sensord-start".into()).spawn(move || match sensors.start() {
        Ok(true) => {
            log::info!("sensord démarré");
            std::thread::sleep(Duration::from_secs(6));
            registry.lock().scan_all();
        }
        Ok(false) => log::info!("sensord absent"),
        Err(e) => log::warn!("sensord: {e:#}"),
    })?;
    Ok(())
}
