//! Gestion du serveur OpenRGB embarqué : localisation, installation,
//! lancement en arrière-plan et arrêt propre.
//!
//! Ordre de recherche de OpenRGB.exe :
//! 1. Ressources de l'app (bundle NSIS : `resources/openrgb/`)
//! 2. `%APPDATA%\PureRGB\openrgb\` (installé par l'exe portable)
//! 3. Installations standard + PATH
//!
//! Si introuvable : téléchargement de la release officielle 0.9 depuis
//! openrgb.org, vérifiée par SHA-256 avant extraction.

use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const OPENRGB_URL: &str =
    "https://codeberg.org/OpenRGB/OpenRGB/releases/download/release_candidate_1.0rc3/OpenRGB_1.0rc3_Windows_64_6fbcf62.zip";
const OPENRGB_SHA256: &str = "a6bb0fbcb7b6eb84214287e3808fadae2777c902efb3dd6cd1e2976f14271c8c";
/// PawnIO : driver noyau signé requis par OpenRGB 1.0rc pour l'accès SMBus
/// (RAM RGB, cartes mères). Installé une fois au niveau système.
const PAWNIO_URL: &str =
    "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";
const PAWNIO_SHA256: &str = "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
pub struct OpenRgbStatus {
    /// Chemin de l'exe trouvé, None si absent partout.
    pub exe_path: Option<String>,
    /// true si un serveur répond sur le port configuré.
    pub server_reachable: bool,
    /// true si le processus a été lancé par PureRGB.
    pub managed: bool,
    /// true si le service driver PawnIO est enregistré.
    pub pawnio_installed: bool,
    /// true si le device \\.\PawnIO répond — seule preuve fiable : le devnode
    /// PnP peut manquer même service enregistré (SwDeviceCreate asynchrone du
    /// setup, parfois incomplet en install silencieuse).
    pub pawnio_ready: bool,
}

pub struct OpenRgbManager {
    child: Mutex<Option<Child>>,
    /// Dossier ressources du bundle (résolu par Tauri au démarrage).
    resource_dir: Mutex<Option<PathBuf>>,
}

impl OpenRgbManager {
    pub fn new() -> Self {
        OpenRgbManager {
            child: Mutex::new(None),
            resource_dir: Mutex::new(None),
        }
    }

    pub fn set_resource_dir(&self, dir: PathBuf) {
        *self.resource_dir.lock() = Some(dir);
    }

    fn appdata_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB").join("openrgb"))
    }

    /// Cherche OpenRGB.exe sans rien installer.
    /// Nos copies (ressources, APPDATA) doivent être en 1.0rc (PawnIOLib.dll
    /// présente) — une 0.9 restée d'une version précédente est ignorée pour
    /// déclencher la réinstallation. Les installations de l'utilisateur sont
    /// acceptées telles quelles.
    pub fn locate(&self) -> Option<PathBuf> {
        let ours_ok = |exe: &PathBuf| {
            exe.is_file() && exe.with_file_name("PawnIOLib.dll").is_file()
        };
        if let Some(res) = self.resource_dir.lock().clone() {
            // Selon le bundling, resource_dir() est le dossier d'install (les
            // fichiers sont sous resources/) ou directement resources/.
            for base in [res.join("resources"), res] {
                let exe = base.join("openrgb").join("OpenRGB.exe");
                if ours_ok(&exe) {
                    return Some(exe);
                }
            }
        }
        if let Some(app) = Self::appdata_dir() {
            let exe = app.join("OpenRGB.exe");
            if ours_ok(&exe) {
                return Some(exe);
            }
        }
        let mut candidates: Vec<PathBuf> = Vec::new();
        for base in ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"] {
            if let Some(p) = std::env::var_os(base) {
                candidates.push(PathBuf::from(&p).join("OpenRGB").join("OpenRGB.exe"));
                candidates.push(
                    PathBuf::from(&p)
                        .join("Programs")
                        .join("OpenRGB")
                        .join("OpenRGB.exe"),
                );
            }
        }
        candidates.into_iter().find(|p| p.is_file())
    }

    pub fn server_reachable(host: &str, port: u16) -> bool {
        format!("{host}:{port}")
            .parse()
            .ok()
            .and_then(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok())
            .is_some()
    }

    pub fn status(&self, host: &str, port: u16) -> OpenRgbStatus {
        OpenRgbStatus {
            exe_path: self.locate().map(|p| p.display().to_string()),
            server_reachable: Self::server_reachable(host, port),
            managed: self.child.lock().is_some(),
            pawnio_installed: Self::pawnio_installed(),
            pawnio_ready: Self::pawnio_ready(),
        }
    }

    /// Le driver PawnIO est enregistré comme service noyau `PawnIO`.
    pub fn pawnio_installed() -> bool {
        Command::new("sc.exe")
            .args(["query", "PawnIO"])
            .creation_flags_no_window()
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Le device est réellement utilisable (devnode PnP présent, driver chargé).
    pub fn pawnio_ready() -> bool {
        std::fs::OpenOptions::new()
            .read(true)
            .open(r"\\.\PawnIO")
            .is_ok()
    }

    /// Télécharge (SHA-256 pinné) puis installe PawnIO en silencieux.
    /// Ré-exécute le setup si le device manque (répare un devnode absent —
    /// cas observé après une première install silencieuse).
    /// Nécessite les droits administrateur (PureRGB les demande au lancement).
    pub fn pawnio_install() -> Result<()> {
        if Self::pawnio_ready() {
            return Ok(());
        }
        let dir = Self::appdata_dir().context("APPDATA introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let setup = dir.join("PawnIO_setup.exe");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{exe}' -UseBasicParsing; \
             $h = (Get-FileHash '{exe}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{exe}' -Force; throw \"hash mismatch: $h\" }}",
            url = PAWNIO_URL,
            exe = setup.display(),
            sha = PAWNIO_SHA256,
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags_no_window()
            .output()
            .context("téléchargement PawnIO")?;
        if !output.status.success() {
            bail!(
                "téléchargement PawnIO échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // Jusqu'à 2 passes : la création du devnode (SwDeviceCreate) est
        // asynchrone et échoue parfois à la première install silencieuse ;
        // relancer le setup (uninstallPrevious + install) la répare.
        let mut last_err = String::new();
        for attempt in 1..=2u8 {
            let status = Command::new(&setup)
                .args(["-install", "-silent"])
                .creation_flags_no_window()
                .status()
                .context("lancement PawnIO_setup.exe")?;
            if !status.success() {
                last_err = format!("setup code {status}");
                continue;
            }
            // Laisser le temps au devnode d'apparaître.
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if Self::pawnio_ready() {
                    let _ = std::fs::remove_file(&setup);
                    log::info!("PawnIO opérationnel (tentative {attempt})");
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            last_err = "device \\\\.\\PawnIO absent".into();
        }
        let _ = std::fs::remove_file(&setup);
        bail!(
            "PawnIO installé mais device inactif ({last_err}) — un redémarrage \
             de Windows peut être nécessaire"
        );
    }

    /// Installe OpenRGB dans %APPDATA%\PureRGB\openrgb (téléchargement officiel
    /// + vérification SHA-256 + extraction). Utilisé par l'exe portable.
    pub fn install(&self) -> Result<PathBuf> {
        let target = Self::appdata_dir().context("APPDATA introuvable")?;
        // Purger une ancienne version (0.9 sans PawnIOLib.dll) avant extraction.
        if target.join("OpenRGB.exe").is_file() && !target.join("PawnIOLib.dll").is_file() {
            let _ = std::fs::remove_dir_all(&target);
        }
        std::fs::create_dir_all(&target)?;
        let zip_path = target.join("openrgb_download.zip");

        // Téléchargement + hash + extraction via PowerShell (pas de dépendance
        // HTTP/zip côté Rust, Windows-only assumé).
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{zip}' -UseBasicParsing; \
             $h = (Get-FileHash '{zip}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{zip}' -Force; throw \"hash mismatch: $h\" }}; \
             Expand-Archive '{zip}' '{dir}' -Force; \
             Remove-Item '{zip}' -Force",
            url = OPENRGB_URL,
            zip = zip_path.display(),
            sha = OPENRGB_SHA256,
            dir = target.display(),
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags_no_window()
            .output()
            .context("lancement PowerShell pour téléchargement OpenRGB")?;
        if !output.status.success() {
            bail!(
                "téléchargement OpenRGB échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Le zip contient un sous-dossier "OpenRGB Windows 64-bit" : aplatir.
        let nested = target.join("OpenRGB Windows 64-bit");
        if nested.is_dir() {
            for entry in std::fs::read_dir(&nested)? {
                let entry = entry?;
                let dest = target.join(entry.file_name());
                if dest.exists() {
                    if dest.is_dir() {
                        std::fs::remove_dir_all(&dest)?;
                    } else {
                        std::fs::remove_file(&dest)?;
                    }
                }
                std::fs::rename(entry.path(), dest)?;
            }
            std::fs::remove_dir_all(&nested)?;
        }

        let exe = target.join("OpenRGB.exe");
        if !exe.is_file() {
            bail!("OpenRGB.exe absent après extraction");
        }

        // VC++ runtime app-local : sans ces DLLs, OpenRGB (Qt/MSVC) reste
        // vivant mais mort-né (aucun port, aucune fenêtre, aucune erreur).
        // Sources par priorité : System32, ressources du bundle PureRGB.
        let mut dll_sources: Vec<PathBuf> = Vec::new();
        if let Some(win) = std::env::var_os("WINDIR") {
            dll_sources.push(PathBuf::from(win).join("System32"));
        }
        if let Some(res) = self.resource_dir.lock().clone() {
            dll_sources.push(res.join("openrgb"));
        }
        for dll in ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"] {
            let dest = target.join(dll);
            if dest.is_file() {
                continue;
            }
            match dll_sources.iter().map(|d| d.join(dll)).find(|p| p.is_file()) {
                Some(src) => {
                    if let Err(e) = std::fs::copy(&src, &dest) {
                        log::warn!("copie {dll}: {e}");
                    }
                }
                None => log::warn!(
                    "{dll} introuvable — OpenRGB pourrait ne pas démarrer (installer le runtime VC++ 2015-2022)"
                ),
            }
        }
        Ok(exe)
    }

    /// Lance le serveur OpenRGB en arrière-plan (fenêtre minimisée dans le tray).
    /// No-op si un serveur répond déjà (OpenRGB de l'utilisateur, par exemple).
    pub fn ensure_running(&self, host: &str, port: u16) -> Result<bool> {
        if Self::server_reachable(host, port) {
            return Ok(false); // déjà servi, rien à lancer
        }
        let exe = match self.locate() {
            Some(e) => e,
            None => self.install().context("installation OpenRGB")?,
        };
        // Best-effort : sans PawnIO actif, OpenRGB démarre mais ignore RAM/carte mère.
        if !Self::pawnio_ready() {
            if let Err(e) = Self::pawnio_install() {
                log::warn!("installation PawnIO: {e:#} — RAM et carte mère non détectables");
            }
        }
        let child = Command::new(&exe)
            .args([
                "--server",
                "--server-port",
                &port.to_string(),
                "--startminimized",
            ])
            .current_dir(exe.parent().context("dossier OpenRGB")?)
            .spawn()
            .context("lancement OpenRGB")?;
        *self.child.lock() = Some(child);

        // Attendre que le serveur écoute (init matériel : jusqu'à 20 s).
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if Self::server_reachable(host, port) {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        bail!("OpenRGB lancé mais le serveur SDK ne répond pas après 20 s")
    }

    /// Arrête OpenRGB uniquement s'il a été lancé par PureRGB.
    pub fn stop(&self) {
        if let Some(mut child) = self.child.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for OpenRgbManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Extension : CREATE_NO_WINDOW sur Windows, no-op ailleurs.
trait CreationFlagsExt {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}

impl CreationFlagsExt for Command {
    #[cfg(windows)]
    fn creation_flags_no_window(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn creation_flags_no_window(&mut self) -> &mut Self {
        self
    }
}
