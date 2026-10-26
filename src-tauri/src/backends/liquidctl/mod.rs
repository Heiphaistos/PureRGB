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
    /// liquidctl écrirait des consignes sur les canaux que l'utilisateur n'a pas réglés (voir `writes_speeds_on_init`) :
    /// le réglage manuel est refusé pour ne jamais forcer un ventilateur ou une pompe.
    speed_locked: bool,
}

/// Pilotes liquidctl dont `initialize` (ou chaque `set`) réécrit TOUS les canaux ventilateur/pompe, ceux non réglés
/// retombant à 100 % (documenté dans le code de liquidctl : « Unconfigured fan channels may default to 100% duty »),
/// ou change le mode de pompe, ou réinitialise les profils :
/// Hydro Platinum / Pro XT (hydro_platinum.py), Hydro Pro (asetek_pro.py), H110i GT (coolit.py), Kraken X2 / M22 (kraken2.py,
/// profils remis à zéro), Asetek 690LC « legacy » (asetek.py), MSI CoreLiquid (msi.py), Lian Li Uni (lianli_uni.py, synchro PWM).
fn writes_speeds_on_init(description: &str) -> bool {
    let d = description.to_lowercase();
    (d.contains("hydro h") && (d.contains("platinum") || d.contains(" pro")))
        || ["h110i gt", "kraken x (x42", "kraken m22", "assuming nzxt kraken x", "coreliquid", "lian li uni"]
            .iter()
            .any(|k| d.contains(k))
}

/// Asetek 690LC legacy : `status` lui-même réécrit pompe et ventilateur (à 100 % par défaut) -> jamais appelé.
fn writes_speeds_on_status(description: &str) -> bool {
    description.to_lowercase().contains("assuming nzxt kraken x")
}

fn device_key(bus: &str, address: &str) -> String {
    format!("{bus}|{address}")
}

pub struct LiquidctlBackend {
    exe: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
    entries: Vec<Entry>,
    /// Appareils (`bus|adresse`) déjà initialisés cette session.
    initialized: std::collections::HashSet<String>,
    last_install_failure: Option<(std::time::Instant, String)>,
}

impl LiquidctlBackend {
    pub fn new() -> Self {
        LiquidctlBackend {
            exe: None,
            resource_dir: None,
            entries: Vec::new(),
            initialized: Default::default(),
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
        if let Some(dir) = crate::trust::managed_dir("liquidctl") {
            candidates.push(dir.join("liquidctl.exe"));
        }
        // Lancé élevé : jamais depuis un dossier modifiable par l'utilisateur.
        candidates.retain(|p| crate::trust::is_trusted(p));
        candidates.into_iter().find(|p| p.is_file())
    }

    /// Télécharge liquidctl.exe (SHA-256 pinné) vers %ProgramFiles%\PureRGB\liquidctl\.
    /// Onefile PyInstaller statique : pas de DLL runtime à copier séparément.
    /// Atomique : télécharge vers .download, vérifie SHA, ne déplace que si OK.
    fn install() -> Result<PathBuf> {
        let dir = crate::trust::managed_dir("liquidctl").context("ProgramFiles introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let exe = dir.join("liquidctl.exe");
        crate::trust::download_verified(LIQUIDCTL_URL, LIQUIDCTL_SHA256, &exe)
            .context("téléchargement liquidctl")?;
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
                version: Err("binaire liquidctl.exe introuvable (resources/, %ProgramFiles%)".into()),
                list: Err("—".into()),
                initialize: Err("—".into()),
                status: Err("—".into()),
            };
        };
        let version = Self::run_diag(&exe, &["--version"]);
        let list = Self::run_diag(&exe, &["list", "--json"]);
        let listed: Vec<ListedDevice> = list.as_deref().ok().and_then(|l| serde_json::from_str(l.trim()).ok()).unwrap_or_default();
        let initialize = match self.init_safe(&listed) {
            (done, 0) => Ok(format!("{done} appareil(s) initialisé(s)")),
            (done, skipped) => Ok(format!("{done} initialisé(s), {skipped} laissé(s) intact(s) : leur initialisation écrirait des vitesses")),
        };
        let status = if listed.iter().any(|d| writes_speeds_on_status(&d.description)) {
            Ok("non lu : le statut de cet appareil réécrit pompe et ventilateur".to_string())
        } else {
            Self::run_diag(&exe, &["status", "--json"])
        };
        LiquidctlDiag {
            exe_path: Some(exe.display().to_string()),
            version,
            list,
            initialize,
            status,
        }
    }

    /// `initialize` appareil par appareil, jamais `initialize all` : les pilotes qui écrivent des vitesses sont laissés intacts.
    /// Rend (initialisés maintenant, laissés intacts).
    fn init_safe(&mut self, listed: &[ListedDevice]) -> (usize, usize) {
        let (mut done, mut skipped) = (0, 0);
        for d in listed {
            let (Some(bus), Some(address)) = (&d.bus, &d.address) else { continue };
            if writes_speeds_on_init(&d.description) {
                skipped += 1;
                continue;
            }
            let key = device_key(bus, address);
            if self.initialized.contains(&key) {
                continue;
            }
            match self.run(&["--bus", bus.as_str(), "--address", address.as_str(), "initialize"]) {
                Ok(_) => done += 1,
                Err(e) => log::warn!("liquidctl initialize {}: {e:#}", d.description),
            }
            self.initialized.insert(key); // une seule tentative par session, même en échec
        }
        (done, skipped)
    }

    /// Statut de chaque appareil. Un appareil dont le statut écrit des vitesses est sauté (aucun canal, lecture seule vide).
    fn read_statuses(&self, listed: &[ListedDevice]) -> Result<Vec<DeviceStatus>> {
        if !listed.iter().any(|d| writes_speeds_on_status(&d.description)) {
            return serde_json::from_str(self.run(&["status", "--json"])?.trim()).context("parse liquidctl status");
        }
        let mut all = Vec::new();
        for d in listed.iter().filter(|d| !writes_speeds_on_status(&d.description)) {
            let (Some(bus), Some(address)) = (&d.bus, &d.address) else { continue };
            match self.run(&["--bus", bus.as_str(), "--address", address.as_str(), "status", "--json"]) {
                Ok(out) => match serde_json::from_str::<Vec<DeviceStatus>>(out.trim()) {
                    Ok(mut v) => all.append(&mut v),
                    Err(e) => log::warn!("parse statut {}: {e}", d.description),
                },
                Err(e) => log::warn!("statut {}: {e:#}", d.description),
            }
        }
        Ok(all)
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

        // initialize appareil par appareil (requis après boot pour la plupart : active le rapport de status), une fois par
        // session, SAUF les pilotes qui écriraient des vitesses (voir `writes_speeds_on_init`) : rien n'est écrit tant que
        // l'utilisateur n'a rien réglé.
        self.init_safe(&listed);

        let statuses = self.read_statuses(&listed)?;

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

            let speed_locked = writes_speeds_on_init(&d.description);
            let idx = self.entries.len();
            self.entries.push(Entry {
                bus: bus.clone(),
                address: address.clone(),
                description: d.description.clone(),
                channels: channels.clone(),
                has_lcd,
                speed_locked,
            });

            devices.push(DeviceInfo {
                id: idx.to_string(),
                name: d.description.clone(),
                vendor: String::new(),
                backend: String::new(),
                device_type: dtype,
                zones: Vec::new(),
                led_count: 0,
                settings: Vec::new(),
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
                controllable: !channels.is_empty() && !speed_locked,
                has_lcd,
                modes: Vec::new(),
                active_mode: -1,
                note: if speed_locked {
                    "ventilateurs/pompe en lecture seule : liquidctl réécrit tous les canaux (non réglés = 100 %) à chaque consigne, \
                     PureRGB ne les touche donc pas — RGB via OpenRGB"
                        .into()
                } else if has_lcd {
                    "ventilateurs/pompe + écran LCD via liquidctl (garde le dernier réglage appliqué, rien n'est restauré) — RGB via OpenRGB".into()
                } else {
                    "ventilateurs/pompe via liquidctl (garde le dernier réglage appliqué, rien n'est restauré) — RGB via OpenRGB".into()
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
        if entry.speed_locked {
            bail!("{} : réglage manuel désactivé, liquidctl forcerait à 100 % les canaux non réglés", entry.description);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(desc: &str, n: u8) -> ListedDevice {
        ListedDevice { description: desc.into(), bus: Some("hid".into()), address: Some(format!("addr{n}")) }
    }

    #[test]
    fn drivers_that_force_speeds_are_recognised_and_benign_ones_are_not() {
        for d in [
            "Corsair Hydro H100i Platinum",
            "Corsair Hydro H115i Pro XT",
            "Corsair Hydro H150i Pro",
            "Corsair H110i GT",
            "NZXT Kraken X (X42, X52, X62 or X72)",
            "NZXT Kraken M22",
            "Asetek 690LC (assuming NZXT Kraken X)",
            "MSI MPG Coreliquid K360",
            "Lian Li Uni SL",
        ] {
            assert!(writes_speeds_on_init(d), "{d}");
        }
        for d in ["NZXT Kraken X (X53, X63 or X73)", "NZXT Kraken Z (Z53, Z63 or Z73)", "Corsair Hydro H100i v2", "Corsair H115i RGB Platinum", "NZXT Smart Device V2", "Corsair Commander Pro"] {
            assert!(!writes_speeds_on_init(d), "{d}");
        }
        assert!(writes_speeds_on_status("Asetek 690LC (assuming NZXT Kraken X)"));
        assert!(!writes_speeds_on_status("Corsair Hydro H100i Platinum"));
    }

    #[test]
    fn speed_locked_device_refuses_manual_duty_without_running_liquidctl() {
        let mut b = LiquidctlBackend::new();
        b.entries.push(Entry {
            bus: "hid".into(),
            address: "a".into(),
            description: "Corsair Hydro H100i Platinum".into(),
            channels: vec![Channel { lc_name: "fan1".into(), display: "Fan 1".into(), rpm: None }],
            has_lcd: false,
            speed_locked: true,
        });
        let e = b.set_fan_duty("0", 0, 50).unwrap_err().to_string();
        assert!(e.contains("100 %"), "{e}");
    }

    #[test]
    fn init_safe_never_initializes_a_speed_writing_driver_and_never_uses_initialize_all() {
        // Sans liquidctl.exe, `run` échoue : on vérifie seulement quels appareils sont tentés (clé marquée) ou sautés.
        let mut b = LiquidctlBackend::new();
        let list = [listed("Corsair Hydro H100i Platinum", 1), listed("NZXT Smart Device V2", 2), listed("Asetek 690LC (assuming NZXT Kraken X)", 3)];
        let (done, skipped) = b.init_safe(&list);
        assert_eq!((done, skipped), (0, 2), "2 sautés, le Smart Device tenté (échec faute d'exe)");
        assert_eq!(b.initialized.len(), 1);
        assert!(b.initialized.contains("hid|addr2"));
    }
}
