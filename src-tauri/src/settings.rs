//! Persistance des réglages + profils dans %APPDATA%/PureRGB.

use crate::engine::curves::CurveMap;
use crate::engine::effects::EffectConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    /// Modes matériels appliqués : device_id → réglages, restaurés au boot.
    pub hw_modes: HashMap<String, SavedHwMode>,
    /// Lancement au démarrage de Windows (tâche planifiée, sans UAC).
    pub autostart: bool,
    /// Tailles de zones ARGB choisies : "<nom appareil>|<nom zone>" → nb LEDs.
    /// Ré-appliquées après chaque scan (OpenRGB repart à 0 via le SDK).
    pub zone_sizes: HashMap<String, u32>,
    /// Appareils réseau / maison connectée, synchronisés vers OpenRGB.json.
    pub network_devices: Vec<crate::netdev::NetworkDevice>,
    /// Familles de conflit gardées : re-tuées en continu tant que l'app
    /// tourne (certains logiciels — Corsair.Service — se relancent seuls
    /// malgré service désactivé + tâche planifiée neutralisée).
    pub guarded_families: HashSet<String>,
    /// Arrête automatiquement les logiciels constructeur en conflit au
    /// lancement (arrêt réversible, non permanent) et les redémarre à la
    /// fermeture de l'app.
    pub auto_manage_conflicts: bool,
    /// Envoie un snapshot diagnostic matériel (VID/PID, état
    /// liquidctl/sensord/OpenRGB) à un service opt-in pour aider à
    /// identifier le matériel non reconnu. Aucune donnée personnelle.
    pub telemetry_opt_in: bool,
}

/// Mode matériel choisi par l'utilisateur (surcharges du mode d'usine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHwMode {
    pub mode_index: u32,
    pub speed: Option<u32>,
    pub direction: Option<u32>,
    pub colors: Option<Vec<crate::core::Color>>,
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
            hw_modes: HashMap::new(),
            autostart: false,
            zone_sizes: HashMap::new(),
            network_devices: Vec::new(),
            guarded_families: HashSet::new(),
            auto_manage_conflicts: true,
            telemetry_opt_in: false,
        }
    }
}

fn settings_path() -> Result<PathBuf> {
    let dir = dirs_dir().context("répertoire de config introuvable")?;
    std::fs::create_dir_all(&dir).context("création du dossier de config")?;
    Ok(dir.join("settings.json"))
}

pub(crate) fn dirs_dir() -> Option<PathBuf> {
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
