//! Capteurs, courbes de ventilateurs et écrans LCD des refroidisseurs.

use crate::backends::liquidctl::LiquidctlBackend;
use crate::engine::curves::CurveConfig;
use crate::backends::smbus::{RamTempStatus, SmbusBackend};
use crate::sensors::Sensor;
use tauri::State;
use crate::settings;
use crate::AppState;

/// Logiciel RGB qui tient le bus SMBus (processus en cours, pas un simple service installé).
fn smbus_conflict() -> Option<String> {
    const KEYS: &[&str] = &["corsair", "asus", "msi", "gigabyte", "asrock", "adata_xpg", "signalrgb"];
    let (found, _) = crate::conflicts::scan();
    found.into_iter().find(|c| KEYS.contains(&c.family.as_str()) && !c.processes.is_empty()).map(|c| c.name)
}

/// Relevé des températures de la RAM (lecture seule, mis en cache 10 s côté backend).
fn ram_temps(state: &AppState) -> (RamTempStatus, Vec<Sensor>) {
    let mut reg = state.registry.lock();
    for b in reg.backends_mut() {
        if let Some(sm) = b.as_any_mut().downcast_mut::<SmbusBackend>() {
            return sm.ram_temperatures(smbus_conflict);
        }
    }
    (RamTempStatus { state: "nobus".into(), message: "Backend SMBus indisponible.".into() }, Vec::new())
}

/// Dernier relevé capteurs (sensord) + températures de la RAM. Vide si le sidecar est absent.
#[tauri::command(async)]
pub(crate) fn get_sensors(state: State<AppState>) -> Vec<Sensor> {
    let mut all = state.sensors.snapshot();
    all.extend(ram_temps(&state).1);
    all
}

/// État de la lecture des températures de la RAM (message affiché dans Surveillance si non « ok »).
#[tauri::command(async)]
pub(crate) fn get_ram_temp_status(state: State<AppState>) -> RamTempStatus {
    ram_temps(&state).0
}

/// Définit ou supprime (config=None) la courbe d'un canal ventilateur.
#[tauri::command]
pub(crate) fn set_curve(
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
pub(crate) fn lcd_apply(
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
