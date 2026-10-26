//! Commandes des appareils : scan, tailles de zones, effets, modes matériels, ventilateurs.

use crate::backends::hid::HidBackend;
use crate::backends::openrgb::OpenRgbBackend;
use crate::core::DeviceInfo;
use crate::engine::effects::EffectConfig;
use serde::Serialize;
use tauri::State;
use crate::settings;
use crate::AppState;

#[derive(Serialize)]
pub(crate) struct BackendStatus {
    name: String,
    available: bool,
}

/// Nombre de LEDs appliqué à un connecteur ARGB dont le contrôleur n'annonce aucune LED : un connecteur ne
/// peut pas dire ce qui y est branché, on prend une taille courante plutôt que d'exiger un comptage
/// (les LEDs en trop n'existent pas, aucun effet néfaste ; modifiable à la main).
pub(crate) const AUTO_ARGB_LEDS: u32 = 60;

/// Taille voulue pour une zone : choix enregistré (même 0), sinon `AUTO_ARGB_LEDS` si la zone est vide et redimensionnable.
pub(crate) fn wanted_zone_size(sizes: &std::collections::HashMap<String, u32>, key: &str, z: &crate::core::ZoneInfo) -> Option<u32> {
    sizes.get(key).copied().or_else(|| (z.led_count == 0 && z.resizable()).then_some(AUTO_ARGB_LEDS))
}

/// Ré-applique les tailles de zones ARGB sauvegardées (clé nom appareil +
/// nom zone : stable entre les redémarrages, contrairement à l'index).
/// Retourne true si au moins un resize a été envoyé (=> re-scan nécessaire).
pub(crate) fn apply_saved_zone_sizes(
    reg: &mut crate::core::registry::DeviceRegistry,
    sizes: &std::collections::HashMap<String, u32>,
) -> bool {
    let devices = reg.device_list();
    let mut resized = false;
    for d in devices.iter().filter(|d| d.backend == "openrgb") {
        let Some(local) = d.id.strip_prefix("openrgb:") else {
            continue;
        };
        for (z_idx, z) in d.zones.iter().enumerate() {
            let key = format!("{}|{}", d.name, z.name);
            let Some(wanted) = wanted_zone_size(sizes, &key, z) else {
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

/// Transmet les tailles de zones enregistrées au backend HID natif (appliquées pendant son scan).
fn set_hid_zone_sizes(reg: &mut crate::core::registry::DeviceRegistry, sizes: &std::collections::HashMap<String, u32>) {
    for b in reg.backends_mut() {
        if let Some(hid) = b.as_any_mut().downcast_mut::<HidBackend>() {
            hid.set_zone_sizes(sizes.clone());
        }
    }
}

/// Scan complet + ré-application des tailles de zones sauvegardées.
pub(crate) fn scan_with_zone_sizes(
    reg: &mut crate::core::registry::DeviceRegistry,
    sizes: &std::collections::HashMap<String, u32>,
) -> Vec<DeviceInfo> {
    set_hid_zone_sizes(reg, sizes);
    let devices = reg.scan_all();
    if apply_saved_zone_sizes(reg, sizes) {
        return reg.scan_all();
    }
    devices
}

#[tauri::command]
pub(crate) fn scan_devices(state: State<AppState>) -> Vec<DeviceInfo> {
    let sizes = state.settings.lock().zone_sizes.clone();
    let devices = scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
    // Le matériel a pu être réinitialisé : forcer la ré-application des effets.
    state.engine.invalidate();
    devices
}

/// Redimensionne une zone ARGB (nombre de LEDs branchées sur un connecteur
/// carte mère ou un canal de hub) et persiste le choix.
#[tauri::command(async)]
pub(crate) fn resize_zone(
    state: State<AppState>,
    device_id: String,
    zone: u32,
    new_size: u32,
) -> Result<(), String> {
    let (backend, local) = device_id
        .split_once(':')
        .ok_or_else(|| format!("id invalide: {device_id}"))?;
    if backend != "openrgb" && backend != "hid" {
        return Err("zones redimensionnables uniquement via OpenRGB ou un driver natif".into());
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
            if backend == "hid" {
                if let Some(hid) = b.as_any_mut().downcast_mut::<HidBackend>() {
                    hid.set_zone_size(key.clone(), new_size);
                    done = true;
                }
            } else if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    orgb.resize_zone(&local, zone, new_size)
                        .map_err(|e| format!("{e:#}"))?;
                    done = true;
                }
            }
        }
        if !done {
            return Err("backend indisponible".into());
        }
        reg.scan_all();
    }
    state.engine.invalidate();
    let mut s = state.settings.lock();
    s.zone_sizes.insert(key, new_size);
    settings::save(&s).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn list_devices(state: State<AppState>) -> Vec<DeviceInfo> {
    state.registry.lock().device_list()
}

#[tauri::command]
pub(crate) fn backend_status(state: State<AppState>) -> Vec<BackendStatus> {
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
pub(crate) fn apply_effect(
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
            if led_count == 0 {
                return Err("appareil sans LED pilotable — RGB non disponible".into());
            }
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
pub(crate) fn set_hardware_mode(
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

#[tauri::command]
pub(crate) fn apply_effect_all(state: State<AppState>, config: EffectConfig) -> Result<u32, String> {
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
pub(crate) fn set_fan_duty(
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
pub(crate) fn set_device_setting(state: State<AppState>, device_id: String, setting: String, value: i32) -> Result<(), String> {
    state.registry.lock().set_device_setting(&device_id, &setting, value).map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod zone_size_tests {
    use super::wanted_zone_size;
    use crate::core::ZoneInfo;
    use std::collections::HashMap;

    fn argb(count: u32) -> ZoneInfo {
        ZoneInfo { name: "JRAINBOW1".into(), led_count: count, zone_type: 1, leds_min: 0, leds_max: 240 }
    }

    #[test]
    fn empty_argb_header_gets_an_automatic_size() {
        assert_eq!(wanted_zone_size(&HashMap::new(), "k", &argb(0)), Some(60));
    }

    #[test]
    fn saved_size_wins_even_zero_and_sized_or_fixed_zones_are_left_alone() {
        let saved: HashMap<String, u32> = [("k".to_string(), 0)].into();
        assert_eq!(wanted_zone_size(&saved, "k", &argb(0)), Some(0));
        assert_eq!(wanted_zone_size(&HashMap::new(), "k", &argb(24)), None);
        assert_eq!(wanted_zone_size(&HashMap::new(), "k", &ZoneInfo::fixed("Carte", 0)), None);
    }
}
