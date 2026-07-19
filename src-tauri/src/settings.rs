//! Persistance des réglages + profils dans %APPDATA%/PureRGB.

use crate::engine::curves::CurveMap;
use crate::engine::effects::EffectConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub openrgb_host: String,
    pub openrgb_port: u16,
    /// Lance automatiquement l'OpenRGB embarqué si aucun serveur ne répond.
    pub auto_start_openrgb: bool,
    pub native_drivers_enabled: bool,
    pub fps: u32,
    pub start_minimized: bool,
    /// Effets par appareil, restaurés au démarrage.
    pub effects: HashMap<String, EffectConfig>,
    /// Services constructeur désactivés par PureRGB : nom service → mode de
    /// démarrage d'origine (Auto/Manual), pour pouvoir les réactiver à l'identique.
    pub disabled_services: HashMap<String, String>,
    /// Courbes ventilateurs : "<device_id>|<canal>" → configuration.
    pub curves: CurveMap,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            openrgb_host: "127.0.0.1".into(),
            openrgb_port: 6742,
            auto_start_openrgb: true,
            native_drivers_enabled: false,
            fps: 30,
            start_minimized: false,
            effects: HashMap::new(),
            disabled_services: HashMap::new(),
            curves: CurveMap::new(),
        }
    }
}

fn settings_path() -> Result<PathBuf> {
    let dir = dirs_dir().context("répertoire de config introuvable")?;
    std::fs::create_dir_all(&dir).context("création du dossier de config")?;
    Ok(dir.join("settings.json"))
}

fn dirs_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB"))
}

pub fn load() -> Settings {
    let path = match settings_path() {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            log::warn!("settings.json invalide ({e}), valeurs par défaut");
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

pub fn save(s: &Settings) -> Result<()> {
    let path = settings_path()?;
    let text = serde_json::to_string_pretty(s)?;
    std::fs::write(&path, text).context("écriture settings.json")
}
