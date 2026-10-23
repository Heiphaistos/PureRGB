//! Serveur OpenRGB embarqué, appareils réseau (Hue, Nanoleaf) et pilote PawnIO.

use crate::backends::openrgb::manager::{OpenRgbManager, OpenRgbStatus};
use tauri::State;
use crate::commands::devices::*;
use crate::{netdev, settings};
use crate::AppState;

#[tauri::command]
pub(crate) fn openrgb_status(state: State<AppState>) -> OpenRgbStatus {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    state.openrgb_mgr.status(&host, port)
}

/// Lance (ou installe puis lance) l'OpenRGB embarqué. Bloquant : exécuté
/// hors du thread principal par Tauri (commande async côté JS).
#[tauri::command(async)]
pub(crate) fn openrgb_start(state: State<AppState>) -> Result<bool, String> {
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
pub(crate) fn openrgb_stop(state: State<AppState>) {
    state.openrgb_mgr.stop();
}

/// Redémarre l'OpenRGB géré (après arrêt d'un logiciel en conflit, les
/// contrôleurs libérés ne sont vus qu'après une nouvelle détection).
#[tauri::command(async)]
pub(crate) fn openrgb_restart(state: State<AppState>) -> Result<bool, String> {
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
pub(crate) fn netdev_sync_and_reload(state: &State<AppState>) -> Result<(), String> {
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
pub(crate) fn netdev_list(state: State<AppState>) -> Vec<netdev::NetworkDevice> {
    state.settings.lock().network_devices.clone()
}

/// Ajoute un appareil réseau, écrit la config OpenRGB et relance le serveur.
#[tauri::command(async)]
pub(crate) fn netdev_add(state: State<AppState>, device: netdev::NetworkDevice) -> Result<(), String> {
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
pub(crate) fn netdev_remove(state: State<AppState>, index: usize) -> Result<(), String> {
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
pub(crate) fn hue_pair(ip: String) -> Result<String, String> {
    netdev::hue_fetch_mac(&ip).map_err(|e| format!("{e:#}"))
}

/// Appairage Nanoleaf : récupère l'auth_token (appareil en mode appairage).
#[tauri::command(async)]
pub(crate) fn nanoleaf_pair(ip: String, port: u16) -> Result<String, String> {
    netdev::nanoleaf_request_token(&ip, port).map_err(|e| format!("{e:#}"))
}

/// Installe le driver PawnIO (SMBus : RAM, carte mère). Admin requis.
#[tauri::command(async)]
pub(crate) fn pawnio_install() -> Result<(), String> {
    OpenRgbManager::pawnio_install().map_err(|e| format!("{e:#}"))
}
