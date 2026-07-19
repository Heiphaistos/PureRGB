//! Backend liquidctl (GPLv3, binaire séparé — aucun code GPL dans PureRGB) :
//! AIO (Kraken, Hydro), hubs (Smart Device, Commander), pompes, et écran LCD
//! des Kraken Z / 2023. Le RGB de ces appareils reste piloté par OpenRGB pour
//! éviter le double-pilotage ; ici : ventilateurs, pompe, LCD.

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, FanChannel};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Deserialize)]
struct ListedDevice {
    description: String,
    bus: Option<String>,
    address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct StatusEntry {
    key: String,
    value: serde_json::Value,
    unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DeviceStatus {
    description: String,
    bus: Option<String>,
    address: Option<String>,
    status: Vec<StatusEntry>,
}

/// Canal pilotable dérivé du status ("Fan 1 speed" → canal liquidctl "fan1").
#[derive(Debug, Clone)]
struct Channel {
    lc_name: String,
    display: String,
    rpm: Option<u32>,
}

#[derive(Debug, Clone)]
struct Entry {
    bus: String,
    address: String,
    description: String,
    channels: Vec<Channel>,
    has_lcd: bool,
}

pub struct LiquidctlBackend {
    exe: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
    entries: Vec<Entry>,
    initialized: bool,
}

impl LiquidctlBackend {
    pub fn new() -> Self {
        LiquidctlBackend {
            exe: None,
            resource_dir: None,
            entries: Vec::new(),
            initialized: false,
        }
    }

    pub fn set_resource_dir(&mut self, dir: PathBuf) {
        self.resource_dir = Some(dir);
        self.exe = None; // re-localiser au prochain scan
    }

    fn locate(&self) -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(res) = &self.resource_dir {
            // resource_dir() = dossier d'install (fichiers sous resources/) ou resources/.
            candidates.push(res.join("resources").join("liquidctl").join("liquidctl.exe"));
            candidates.push(res.join("liquidctl").join("liquidctl.exe"));
        }
        if let Some(app) = std::env::var_os("APPDATA") {
            candidates.push(
                PathBuf::from(app)
                    .join("PureRGB")
                    .join("liquidctl")
                    .join("liquidctl.exe"),
            );
        }
        candidates.into_iter().find(|p| p.is_file())
    }

    fn run(&self, args: &[&str]) -> Result<String> {
        let exe = self.exe.as_ref().context("liquidctl.exe introuvable")?;
        let mut cmd = Command::new(exe);
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd.output().context("exécution liquidctl")?;
        if !out.status.success() {
            bail!(
                "liquidctl {args:?}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn device_args<'a>(entry: &'a Entry) -> [&'a str; 4] {
        ["--bus", &entry.bus, "--address", &entry.address]
    }

    /// "Fan 1 speed" → ("fan1", "Fan 1") ; "Pump speed" → ("pump", "Pompe").
    fn channel_from_key(key: &str, unit: Option<&str>) -> Option<(String, String)> {
        if unit != Some("rpm") {
            return None;
        }
        let base = key.trim_end_matches(" speed").trim();
        let low = base.to_lowercase();
        if low == "pump" {
            return Some(("pump".into(), "Pompe".into()));
        }
        if let Some(rest) = low.strip_prefix("fan") {
            let n = rest.trim();
            let lc = if n.is_empty() {
                "fan".to_string()
            } else {
                format!("fan{n}")
            };
            return Some((lc, base.to_string()));
        }
        None
    }

    fn entry_index(&self, local_id: &str) -> Result<usize> {
        local_id
            .parse::<usize>()
            .ok()
            .filter(|i| *i < self.entries.len())
            .with_context(|| format!("appareil liquidctl inconnu: {local_id}"))
    }

    /// Applique une commande LCD (Kraken Z / 2023 uniquement).
    /// `kind` : "liquid" (température), "static" (image), "gif", "brightness",
    /// "orientation". `arg` : chemin fichier ou valeur numérique.
    pub fn lcd_apply(&mut self, local_id: &str, kind: &str, arg: Option<&str>) -> Result<()> {
        let idx = self.entry_index(local_id)?;
        let entry = self.entries[idx].clone();
        if !entry.has_lcd {
            bail!("pas d'écran LCD sur {}", entry.description);
        }
        let dev = Self::device_args(&entry);
        match kind {
            "liquid" => {
                let mut args = vec![];
                args.extend_from_slice(&dev);
                args.extend_from_slice(&["set", "lcd", "screen", "liquid"]);
                self.run(&args)?;
            }
            "static" | "gif" => {
                let path = arg.context("chemin du fichier requis")?;
                if !std::path::Path::new(path).is_file() {
                    bail!("fichier introuvable: {path}");
                }
                let mut args = vec![];
                args.extend_from_slice(&dev);
                args.extend_from_slice(&["set", "lcd", "screen", kind, path]);
                self.run(&args)?;
            }
            "brightness" => {
                let v = arg.context("valeur 0-100 requise")?;
                let _: u8 = v.parse().ok().filter(|x| *x <= 100).context("0-100")?;
                let mut args = vec![];
                args.extend_from_slice(&dev);
                args.extend_from_slice(&["set", "lcd", "brightness", v]);
                self.run(&args)?;
            }
            "orientation" => {
                let v = arg.context("angle 0/90/180/270 requis")?;
                if !matches!(v, "0" | "90" | "180" | "270") {
                    bail!("angle invalide: {v}");
                }
                let mut args = vec![];
                args.extend_from_slice(&dev);
                args.extend_from_slice(&["set", "lcd", "orientation", v]);
                self.run(&args)?;
            }
            other => bail!("commande LCD inconnue: {other}"),
        }
        Ok(())
    }
}

impl Backend for LiquidctlBackend {
    fn name(&self) -> &'static str {
        "liquidctl"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        self.entries.clear();
        if self.exe.is_none() {
            self.exe = self.locate();
        }
        if self.exe.is_none() {
            return Ok(Vec::new()); // binaire absent : backend silencieux
        }

        let listed: Vec<ListedDevice> =
            serde_json::from_str(self.run(&["list", "--json"])?.trim())
                .context("parse liquidctl list")?;
        if listed.is_empty() {
            return Ok(Vec::new());
        }

        // initialize all : requis après boot pour la plupart des appareils
        // (active le rapport de status). Une seule fois par session.
        if !self.initialized {
            if let Err(e) = self.run(&["initialize", "all"]) {
                log::warn!("liquidctl initialize: {e:#}");
            }
            self.initialized = true;
        }

        let statuses: Vec<DeviceStatus> =
            serde_json::from_str(self.run(&["status", "--json"])?.trim())
                .context("parse liquidctl status")?;

        let mut devices = Vec::new();
        for d in &listed {
            let (Some(bus), Some(address)) = (&d.bus, &d.address) else {
                continue;
            };
            let status = statuses
                .iter()
                .find(|s| s.bus.as_deref() == Some(bus) && s.address.as_deref() == Some(address))
                .map(|s| s.status.as_slice())
                .unwrap_or(&[]);

            let mut channels = Vec::new();
            for e in status {
                if let Some((lc, display)) =
                    Self::channel_from_key(&e.key, e.unit.as_deref())
                {
                    let rpm = e.value.as_f64().map(|v| v.round() as u32);
                    channels.push(Channel { lc_name: lc, display, rpm });
                }
            }

            let desc_low = d.description.to_lowercase();
            let has_lcd = desc_low.contains("kraken z")
                || (desc_low.contains("kraken") && desc_low.contains("elite"))
                || desc_low.contains("kraken 2023");
            let dtype = if desc_low.contains("kraken") || desc_low.contains("hydro") {
                DeviceType::Aio
            } else {
                DeviceType::Hub
            };

            let idx = self.entries.len();
            self.entries.push(Entry {
                bus: bus.clone(),
                address: address.clone(),
                description: d.description.clone(),
                channels: channels.clone(),
                has_lcd,
            });

            devices.push(DeviceInfo {
                id: idx.to_string(),
                name: d.description.clone(),
                vendor: String::new(),
                backend: String::new(),
                device_type: dtype,
                zones: Vec::new(),
                led_count: 0,
                fan_channels: channels
                    .iter()
                    .enumerate()
                    .map(|(i, c)| FanChannel {
                        index: i as u8,
                        name: c.display.clone(),
                        duty_percent: None,
                        rpm: c.rpm,
                    })
                    .collect(),
                controllable: !channels.is_empty(),
                has_lcd,
                note: if has_lcd {
                    "ventilateurs/pompe + écran LCD via liquidctl — RGB via OpenRGB".into()
                } else {
                    "ventilateurs/pompe via liquidctl — RGB via OpenRGB".into()
                },
            });
        }
        Ok(devices)
    }

    fn set_colors(&mut self, _local_id: &str, _colors: &[Color]) -> Result<()> {
        bail!("RGB piloté via OpenRGB pour éviter les conflits")
    }

    fn set_fan_duty(&mut self, local_id: &str, channel: u8, percent: u8) -> Result<()> {
        let idx = self.entry_index(local_id)?;
        let entry = self.entries[idx].clone();
        let ch = entry
            .channels
            .get(channel as usize)
            .with_context(|| format!("canal {channel} inconnu"))?;
        let pct = percent.to_string();
        let dev = Self::device_args(&entry);
        let mut args = vec![];
        args.extend_from_slice(&dev);
        args.extend_from_slice(&["set", &ch.lc_name, "speed", &pct]);
        self.run(&args)?;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.exe.is_some()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
