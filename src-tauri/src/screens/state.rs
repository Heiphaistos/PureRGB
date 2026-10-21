//! Réglages persistés par écran : `%APPDATA%\PureRGB\screens.json`.

use super::layout::Layout;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenConfig {
    pub layout: Option<Layout>,
    pub brightness: u8,
    pub rotation: u16,
    pub fps: u8,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        ScreenConfig { layout: None, brightness: 100, rotation: 0, fps: 5 }
    }
}

impl ScreenConfig {
    /// Ramène des valeurs importées/reçues de l'interface dans les bornes valides.
    pub fn clamped(mut self) -> Self {
        self.brightness = self.brightness.min(100);
        self.rotation = match self.rotation {
            90 | 180 | 270 => self.rotation,
            _ => 0,
        };
        self.fps = self.fps.clamp(1, 30);
        self
    }
}

pub type ConfigMap = HashMap<String, ScreenConfig>;

/// Dossier racine des données des écrans (designs utilisateur, médias).
pub fn data_dir() -> Option<PathBuf> {
    crate::settings::dirs_dir().map(|d| d.join("screens"))
}

fn file() -> Option<PathBuf> {
    crate::settings::dirs_dir().map(|d| d.join("screens.json"))
}

pub fn load() -> ConfigMap {
    let Some(path) = file() else { return ConfigMap::new() };
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!("screens.json illisible, réglages ignorés: {e}");
            ConfigMap::new()
        }),
        Err(_) => ConfigMap::new(),
    }
}

pub fn save(map: &ConfigMap) -> Result<()> {
    let path = file().context("dossier de données introuvable")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(map)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_out_of_range_options() {
        let c = ScreenConfig { layout: None, brightness: 250, rotation: 45, fps: 0 }.clamped();
        assert_eq!((c.brightness, c.rotation, c.fps), (100, 0, 1));
        let c = ScreenConfig { layout: None, brightness: 10, rotation: 270, fps: 99 }.clamped();
        assert_eq!((c.brightness, c.rotation, c.fps), (10, 270, 30));
    }

    #[test]
    fn missing_fields_use_defaults() {
        let c: ScreenConfig = serde_json::from_str("{}").unwrap();
        assert_eq!((c.brightness, c.fps), (100, 5));
    }
}
