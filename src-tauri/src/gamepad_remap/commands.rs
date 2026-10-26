//! Commandes Tauri du remappage de manette.

use super::driver::{self, DriverInfo};
use super::model::{valid_profile_name, Profile};
use super::worker::{self, Live};
use crate::settings::{self, DEFAULT_REMAP_PROFILE};
use crate::AppState;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

const MAX_PROFILES: usize = 32;

#[derive(Serialize)]
pub(crate) struct View {
    enabled: bool,
    profiles: BTreeMap<String, Profile>,
    assign: BTreeMap<String, String>,
    live: Live,
    /// Mode jeu actif : le remappage est suspendu (certains anti-cheats refusent les pilotes de manette virtuelle).
    game_mode_active: bool,
}

#[tauri::command]
pub(crate) fn gamepad_remap_status(state: State<AppState>) -> View {
    let s = state.settings.lock();
    View {
        enabled: s.gamepad_remap_enabled,
        profiles: s.gamepad_remap_profiles.clone(),
        assign: s.gamepad_remap_assign.clone(),
        live: worker::live(),
        game_mode_active: crate::gamemode::active(),
    }
}

/// État des pilotes (ouvre le périphérique ViGEmBus : thread bloquant dédié, pas le thread UI).
#[tauri::command(async)]
pub(crate) fn gamepad_remap_drivers() -> DriverInfo {
    driver::info()
}

#[tauri::command]
pub(crate) fn gamepad_remap_enable(state: State<AppState>, enabled: bool) -> Result<(), String> {
    let mut s = state.settings.lock();
    s.gamepad_remap_enabled = enabled;
    settings::save(&s).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn gamepad_remap_save_profile(state: State<AppState>, name: String, mut profile: Profile) -> Result<(), String> {
    if !valid_profile_name(&name) {
        return Err("Nom de profil invalide (1 à 40 caractères, sans caractère de contrôle)".into());
    }
    profile.sanitize();
    let mut s = state.settings.lock();
    if !s.gamepad_remap_profiles.contains_key(name.trim()) && s.gamepad_remap_profiles.len() >= MAX_PROFILES {
        return Err(format!("Trop de profils (maximum {MAX_PROFILES})"));
    }
    s.gamepad_remap_profiles.insert(name.trim().to_string(), profile);
    settings::save(&s).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn gamepad_remap_delete_profile(state: State<AppState>, name: String) -> Result<(), String> {
    if name == DEFAULT_REMAP_PROFILE {
        return Err("Le profil par défaut ne peut pas être supprimé".into());
    }
    let mut s = state.settings.lock();
    s.gamepad_remap_profiles.remove(&name);
    s.gamepad_remap_assign.retain(|_, v| *v != name);
    settings::save(&s).map_err(|e| e.to_string())
}

/// Assigne un profil à une manette (`xinput0`..`xinput3`).
#[tauri::command]
pub(crate) fn gamepad_remap_assign(state: State<AppState>, pad_key: String, profile: String) -> Result<(), String> {
    let valid_key = pad_key.strip_prefix("xinput").is_some_and(|n| matches!(n, "0" | "1" | "2" | "3"));
    if !valid_key {
        return Err("Manette inconnue".into());
    }
    let mut s = state.settings.lock();
    if profile != DEFAULT_REMAP_PROFILE && !s.gamepad_remap_profiles.contains_key(&profile) {
        return Err("Profil inconnu".into());
    }
    s.gamepad_remap_assign.insert(pad_key, profile);
    settings::save(&s).map_err(|e| e.to_string())
}

/// Mesure de dérive des sticks au repos (1 s), enregistrée dans le profil de la manette lue.
#[tauri::command]
pub(crate) fn gamepad_remap_calibrate() {
    worker::request_calibration();
}

/// « Installer le pilote » : n'agit qu'avec `confirmed == true` (confirmation affichée par l'interface).
#[tauri::command(async)]
pub(crate) fn gamepad_remap_install_driver(confirmed: bool) -> Result<(), String> {
    driver::install(confirmed).map_err(|e| format!("{e:#}"))
}

/// « Installer HidHide » (option avancée) : n'agit qu'avec `confirmed == true`.
#[tauri::command(async)]
pub(crate) fn gamepad_remap_install_hidhide(confirmed: bool) -> Result<(), String> {
    driver::install_hidhide(confirmed).map_err(|e| format!("{e:#}"))
}
