//! Backend liquidctl (GPLv3, binaire séparé — aucun code GPL dans PureRGB) :
//! AIO (Kraken, Hydro), hubs (Smart Device, Commander), pompes, et écran LCD
//! des Kraken Z / 2023. Le RGB de ces appareils reste piloté par OpenRGB pour
//! éviter le double-pilotage ; ici : ventilateurs, pompe, LCD.

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, FanChannel};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Résultat brut d'une étape liquidctl — capturé même en cas d'échec, pour
/// diagnostiquer précisément pourquoi le binaire "ne se charge pas" plutôt
/// que de l'avaler dans un log serveur invisible depuis l'UI.
#[derive(Debug, Clone, Serialize)]
pub struct LiquidctlDiag {
    pub exe_path: Option<String>,
    /// "--version" : sortie ou message d'erreur (échec de lancement du process).
    pub version: Result<String, String>,
    pub list: Result<String, String>,
    pub initialize: Result<String, String>,
    pub status: Result<String, String>,
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Filet de sécurité pour l'exe portable : NSIS copie resources/liquidctl/
/// à côté du binaire, mais le portable n'a pas de dossier resources/ du
/// tout (voir OpenRgbManager::install, même limitation déjà compensée pour
/// OpenRGB). Binaire hébergé sur nos propres releases GitHub (PyInstaller
/// onefile, aucune release Windows officielle liquidctl à épingler).
const LIQUIDCTL_URL: &str =
    "https://github.com/Heiphaistos/PureRGB/releases/download/sidecars-v1/liquidctl.exe";
const LIQUIDCTL_SHA256: &str = "efcde9918537997a47a4e965deec457c8fd30e42af8103a5c738d5ea86c5e0a2";

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
    last_install_failure: Option<(std::time::Instant, String)>,
}

impl LiquidctlBackend {
    pub fn new() -> Self {
        LiquidctlBackend {
            exe: None,
            resource_dir: None,
            entries: Vec::new(),
            initialized: false,
            last_install_failure: None,
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

    fn appdata_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB").join("liquidctl"))
    }

    /// Télécharge liquidctl.exe (SHA-256 pinné) vers %APPDATA%\PureRGB\liquidctl\.
    /// Onefile PyInstaller statique : pas de DLL runtime à copier séparément.
    /// Atomique : télécharge vers .download, vérifie SHA, ne déplace que si OK.
    fn install() -> Result<PathBuf> {
        let dir = Self::appdata_dir().context("APPDATA introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let exe = dir.join("liquidctl.exe");
        let tmp = dir.join("liquidctl.exe.download");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{tmp}' -UseBasicParsing; \
             $h = (Get-FileHash '{tmp}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{tmp}' -Force; throw \"hash mismatch: $h\" }}; \
             Move-Item '{tmp}' '{exe}' -Force",
            url = LIQUIDCTL_URL,
            tmp = tmp.display(),
            exe = exe.display(),
            sha = LIQUIDCTL_SHA256,
        );
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().context("téléchargement liquidctl")?;
        if !output.status.success() {
            bail!(
                "téléchargement liquidctl échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if !exe.is_file() {
            bail!("liquidctl.exe absent après téléchargement");
        }
        Ok(exe)
    }

    /// `locate()` puis, si absent, tentative de téléchargement — ne jamais
    /// laisser le backend mort silencieusement comme avant (portable sans
    /// resources/, cause confirmée d'un retour terrain "0 AIO/hub détecté").
    /// Retourne Result pour que le diagnostic puisse surfacer l'erreur réelle.
    fn locate_or_install(&mut self) -> Result<PathBuf, String> {
        if let Some(p) = self.locate() {
            return Ok(p);
        }
        const COOLDOWN: std::time::Duration = std::time::Duration::from_secs(30);
        if let Some((at, err)) = &self.last_install_failure {
            if at.elapsed() < COOLDOWN {
                return Err(err.clone());
            }
        }
        match Self::install() {
            Ok(p) => {
                self.last_install_failure = None;
                Ok(p)
            }
            Err(e) => {
                let msg = format!("{e:#}");
                self.last_install_failure = Some((std::time::Instant::now(), msg.clone()));
                Err(msg)
            }
        }
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

    /// Comme `run`, mais ne bascule jamais en erreur : capture le résultat
    /// exact (process introuvable / plante au lancement / stderr non-vide)
    /// pour le diagnostic. Ne jamais utiliser pour le pilotage normal.
    fn run_diag(exe: &PathBuf, args: &[&str]) -> Result<String, String> {
        let mut cmd = Command::new(exe);
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match cmd.output() {
            Ok(out) if out.status.success() => {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            Ok(out) => Err(format!(
                "code {} — {}",
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => Err(format!("lancement impossible : {e} (DLL VC++ manquante ?)")),
        }
    }

    /// Exécute chaque étape séparément et capture le résultat exact — pour
    /// comprendre pourquoi liquidctl "ne se charge pas" sur une machine
    /// donnée (binaire présent mais process mort-né, ou aucun appareil
    /// listé, ou initialize/status en échec).
    pub fn diagnose(&mut self) -> LiquidctlDiag {
        if self.exe.is_none() {
            match self.locate_or_install() {
                Ok(p) => self.exe = Some(p),
                Err(e) => {
                    return LiquidctlDiag {
                        exe_path: None,
                        version: Err(format!("binaire liquidctl.exe introuvable : {e}")),
                        list: Err("—".into()),
                        initialize: Err("—".into()),
                        status: Err("—".into()),
                    };
                }
            }
        }
        let Some(exe) = self.exe.clone() else {
            return LiquidctlDiag {
                exe_path: None,
                version: Err("binaire liquidctl.exe introuvable (resources/, %APPDATA%)".into()),
                list: Err("—".into()),
                initialize: Err("—".into()),
                status: Err("—".into()),
            };
        };
        let version = Self::run_diag(&exe, &["--version"]);
        let list = Self::run_diag(&exe, &["list", "--json"]);
        let initialize = if self.initialized {
            Ok("déjà initialisé cette session".to_string())
        } else {
            let r = Self::run_diag(&exe, &["initialize", "all"]);
            if r.is_ok() {
                self.initialized = true;
            }
            r
        };
        let status = Self::run_diag(&exe, &["status", "--json"]);
        LiquidctlDiag {
            exe_path: Some(exe.display().to_string()),
            version,
            list,
            initialize,
            status,
        }
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
            match self.locate_or_install() {
                Ok(p) => self.exe = Some(p),
                Err(e) => log::warn!("installation liquidctl: {e}"),
            }
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
                modes: Vec::new(),
                active_mode: -1,
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
