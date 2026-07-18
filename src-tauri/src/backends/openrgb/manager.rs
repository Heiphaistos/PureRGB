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
    "https://openrgb.org/releases/release_0.9/OpenRGB_0.9_Windows_64_b5f46e3.zip";
const OPENRGB_SHA256: &str = "4a42df973bf9e0694268993478f03a71dafbf2ddbcb1512835b4bbabdc6dc6de";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
pub struct OpenRgbStatus {
    /// Chemin de l'exe trouvé, None si absent partout.
    pub exe_path: Option<String>,
    /// true si un serveur répond sur le port configuré.
    pub server_reachable: bool,
    /// true si le processus a été lancé par PureRGB.
    pub managed: bool,
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
    pub fn locate(&self) -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(res) = self.resource_dir.lock().clone() {
            candidates.push(res.join("openrgb").join("OpenRGB.exe"));
        }
        if let Some(app) = Self::appdata_dir() {
            candidates.push(app.join("OpenRGB.exe"));
        }
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
        }
    }

    /// Installe OpenRGB dans %APPDATA%\PureRGB\openrgb (téléchargement officiel
    /// + vérification SHA-256 + extraction). Utilisé par l'exe portable.
    pub fn install(&self) -> Result<PathBuf> {
        let target = Self::appdata_dir().context("APPDATA introuvable")?;
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
