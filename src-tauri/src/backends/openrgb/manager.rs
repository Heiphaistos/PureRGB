//! Gestion du serveur OpenRGB embarqué : localisation, installation,
//! lancement en arrière-plan et arrêt propre.
//!
//! Ordre de recherche de OpenRGB.exe :
//! 1. Ressources de l'app (bundle NSIS : `resources/openrgb/`)
//! 2. `%ProgramFiles%\PureRGB\openrgb\` (installé par l'exe portable)
//! 3. Installations standard dans `%ProgramFiles%`
//!
//! Tout est lancé avec les droits admin : seuls des emplacements à ACL admin
//! (`%ProgramFiles%`) sont acceptés, jamais %APPDATA%/%LOCALAPPDATA%.
//! Si introuvable : téléchargement de la version épinglée (URL en liste
//! blanche), vérifiée par SHA-256 avant extraction.

use crate::trust;
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
    /// Dernière raison pour laquelle OpenRGB n'a pas pu démarrer (vide si tout va bien).
    pub last_error: Option<String>,
}

pub struct OpenRgbManager {
    child: Mutex<Option<Child>>,
    last_error: Mutex<Option<String>>,
    /// Dossier ressources du bundle (résolu par Tauri au démarrage).
    resource_dir: Mutex<Option<PathBuf>>,
}

impl OpenRgbManager {
    pub fn new() -> Self {
        OpenRgbManager {
            child: Mutex::new(None),
            last_error: Mutex::new(None),
            resource_dir: Mutex::new(None),
        }
    }

    pub fn set_resource_dir(&self, dir: PathBuf) {
        *self.resource_dir.lock() = Some(dir);
    }

    /// Cherche OpenRGB.exe sans rien installer, uniquement sous %ProgramFiles%
    /// (lancé élevé). Nos copies doivent être en 1.0rc (PawnIOLib.dll présente)
    /// — une 0.9 restée d'une version précédente est ignorée pour déclencher la
    /// réinstallation.
    pub fn locate(&self) -> Option<PathBuf> {
        let ours_ok = |exe: &PathBuf| {
            trust::is_trusted(exe) && exe.is_file() && exe.with_file_name("PawnIOLib.dll").is_file()
        };
        if let Some(res) = self.resource_dir.lock().clone() {
            // Selon le bundling, resource_dir() est le dossier d'install (les
            // fichiers sous resources/) ou directement resources/.
            for base in [res.join("resources"), res] {
                let exe = base.join("openrgb").join("OpenRGB.exe");
                if ours_ok(&exe) {
                    return Some(exe);
                }
            }
        }
        if let Some(dir) = trust::managed_dir("openrgb") {
            let exe = dir.join("OpenRGB.exe");
            if ours_ok(&exe) {
                return Some(exe);
            }
        }
        ["ProgramFiles", "ProgramFiles(x86)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(|p| PathBuf::from(p).join("OpenRGB").join("OpenRGB.exe"))
            .find(|p| p.is_file())
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
            last_error: self.last_error.lock().clone(),
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
        let dir = trust::managed_dir("pawnio").context("ProgramFiles introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let setup = dir.join("PawnIO_setup.exe");
        // Hash vérifié dans un dossier à ACL admin, juste avant le lancement.
        trust::download_verified(PAWNIO_URL, PAWNIO_SHA256, &setup)
            .context("téléchargement PawnIO")?;
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

    /// Installe OpenRGB dans %ProgramFiles%\PureRGB\openrgb (téléchargement
    /// de la version épinglée + vérification SHA-256 + extraction par tar.exe,
    /// sans shell). Utilisé par l'exe portable et les installs hors Program Files.
    pub fn install(&self) -> Result<PathBuf> {
        let target = trust::managed_dir("openrgb").context("ProgramFiles introuvable")?;
        // Purger toute copie précédente (0.9 sans PawnIOLib.dll, ou altérée).
        let _ = std::fs::remove_dir_all(&target);
        std::fs::create_dir_all(&target)?;
        let zip_path = target.join("openrgb_download.zip");
        trust::download_verified(OPENRGB_URL, OPENRGB_SHA256, &zip_path)
            .context("téléchargement OpenRGB")?;
        let output = Command::new("tar.exe")
            .arg("-xf")
            .arg(&zip_path)
            .arg("-C")
            .arg(&target)
            .creation_flags_no_window()
            .output()
            .context("extraction OpenRGB (tar.exe)")?;
        let _ = std::fs::remove_file(&zip_path);
        if !output.status.success() {
            bail!(
                "extraction OpenRGB échouée: {}",
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
        // Le dossier bundle n'est une source que s'il est sous %ProgramFiles%.
        if let Some(res) = self.resource_dir.lock().clone().filter(|r| trust::is_trusted(r)) {
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
    /// En cas d'échec, la cause (avec diagnostic) est gardée pour l'interface.
    pub fn ensure_running(&self, host: &str, port: u16) -> Result<bool> {
        let r = self.start_server(host, port);
        *self.last_error.lock() = r.as_ref().err().map(|e| format!("{e:#}"));
        r
    }

    /// Le serveur qui écoute répond-il au protocole OpenRGB ?
    pub fn serves_openrgb(host: &str, port: u16) -> bool {
        Self::server_reachable(host, port) && speaks_openrgb(host, port)
    }

    /// Efface l'erreur mémorisée ; true s'il y en avait une.
    pub fn take_error(&self) -> bool {
        self.last_error.lock().take().is_some()
    }

    fn start_server(&self, host: &str, port: u16) -> Result<bool> {
        if Self::server_reachable(host, port) {
            if speaks_openrgb(host, port) {
                return Ok(false); // déjà servi, rien à lancer
            }
            let who = port_owner(port).map(|(pid, name)| format!(" par « {name} » (PID {pid})")).unwrap_or_default();
            bail!(
                "Le port {port} est déjà utilisé{who} et ce n'est pas un serveur OpenRGB. Fermez ce programme ou changez le port d'OpenRGB dans Réglages."
            );
        }
        // Fichiers de notre copie manquants (antivirus, suppression) : on la réinstalle (SHA-256 vérifié).
        let exe = match self.locate() {
            Some(e) if missing_files(&e).is_empty() => e,
            _ => self.install().context("installation OpenRGB")?,
        };
        // Best-effort : sans PawnIO actif, OpenRGB démarre mais ignore RAM/carte mère.
        if !Self::pawnio_ready() {
            if let Err(e) = Self::pawnio_install() {
                log::warn!("installation PawnIO: {e:#} — RAM et carte mère non détectables");
            }
        }
        let mut child = Command::new(&exe)
            .args([
                "--server",
                "--server-host",
                "127.0.0.1",
                "--server-port",
                &port.to_string(),
                "--startminimized",
            ])
            .current_dir(exe.parent().context("dossier OpenRGB")?)
            .spawn()
            .context("lancement OpenRGB")?;

        // Attendre que le serveur écoute (init matériel : jusqu'à 20 s). Si le processus
        // s'arrête avant, inutile d'attendre : on rapporte son code de sortie.
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if Self::server_reachable(host, port) {
                *self.child.lock() = Some(child);
                return Ok(true);
            }
            if let Ok(Some(status)) = child.try_wait() {
                bail!("OpenRGB s'est arrêté tout seul ({status}). {}", diagnose(port, child.id()));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let child_id = child.id();
        *self.child.lock() = Some(child);
        bail!("OpenRGB lancé mais le serveur SDK ne répond pas après 20 s. {}", diagnose(port, child_id))
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

/// Fichiers indispensables de notre copie d'OpenRGB (vide si ce n'en est pas une, ou si tout est là).
fn missing_files(exe: &std::path::Path) -> Vec<&'static str> {
    let Some(dir) = exe.parent().filter(|d| d.join("PawnIOLib.dll").is_file()) else { return Vec::new() };
    ["Qt5Core.dll", "Qt5Gui.dll", "Qt5Widgets.dll", "hidapi.dll"].into_iter().filter(|f| !dir.join(f).is_file()).collect()
}

/// Le serveur répond-il au protocole OpenRGB (en-tête « ORGB » en réponse à une demande de version) ?
fn speaks_openrgb(host: &str, port: u16) -> bool {
    use std::io::{Read, Write};
    let Ok(addr) = format!("{host}:{port}").parse() else { return false };
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(800)) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let mut req = super::protocol::header(0, super::protocol::REQUEST_PROTOCOL_VERSION, 4).to_vec();
    req.extend_from_slice(&super::protocol::PROTOCOL_VERSION.to_le_bytes());
    let mut head = [0u8; 4];
    s.write_all(&req).is_ok() && s.read_exact(&mut head).is_ok() && &head == super::protocol::MAGIC
}

/// Programme qui écoute sur ce port TCP (PID, nom), d'après `netstat`.
fn port_owner(port: u16) -> Option<(u32, String)> {
    let out = Command::new("netstat.exe").args(["-ano", "-p", "TCP"]).creation_flags_no_window().output().ok()?;
    let suffix = format!(":{port}");
    let pid = String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.contains("LISTENING")).find_map(|l| {
        let cols: Vec<&str> = l.split_whitespace().collect();
        if cols.get(1)?.ends_with(&suffix) { cols.last()?.parse::<u32>().ok() } else { None }
    })?;
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    );
    let name = sys.process(sysinfo::Pid::from_u32(pid)).map(|p| p.name().to_string_lossy().into_owned());
    Some((pid, name.unwrap_or_else(|| "inconnu".into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn serve(reply: &'static [u8]) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut c, _)) = l.accept() {
                let mut buf = [0u8; 20];
                let _ = c.read(&mut buf);
                let _ = c.write_all(reply);
                std::thread::sleep(Duration::from_millis(300));
            }
        });
        port
    }

    #[test]
    fn recognizes_an_openrgb_server() {
        assert!(speaks_openrgb("127.0.0.1", serve(b"ORGB\0\0\0\0")));
    }

    #[test]
    fn rejects_a_foreign_server() {
        assert!(!speaks_openrgb("127.0.0.1", serve(b"HTTP/1.1 400")));
    }

    #[test]
    fn rejects_a_silent_server() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(!speaks_openrgb("127.0.0.1", l.local_addr().unwrap().port()));
    }
}

/// Pistes quand OpenRGB ne démarre pas : un autre OpenRGB ou un logiciel RGB déjà actif,
/// ou un programme qui occupe déjà le port du serveur.
fn diagnose(port: u16, own_pid: u32) -> String {
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    );
    let mut hints = Vec::new();
    let (active, _) = crate::conflicts::scan();
    let other_orgb = sys.processes().iter().any(|(pid, p)| {
        pid.as_u32() != own_pid && p.name().to_string_lossy().to_lowercase().starts_with("openrgb")
    });
    if other_orgb {
        hints.push("un autre OpenRGB est déjà ouvert (le fermer)".to_string());
    }
    for c in active.iter().filter(|c| c.active) {
        hints.push(format!("{} est actif et peut verrouiller les appareils", c.name));
    }
    if let Some((pid, name)) = port_owner(port) {
        hints.push(format!("le port {port} est déjà utilisé par « {name} » (PID {pid})"));
    }
    if hints.is_empty() {
        "Aucun conflit repéré : antivirus qui bloque OpenRGB.exe, ou pilote manquant ? (voir purergb.log dans le dossier PureRGB de APPDATA)".into()
    } else {
        format!("Pistes : {}.", hints.join(" ; "))
    }
}

/// Extension : CREATE_NO_WINDOW sur Windows, no-op ailleurs.
pub(crate) trait CreationFlagsExt {
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
