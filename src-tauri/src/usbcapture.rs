//! Capture du trafic USB brut (via USBPcap) pour aider à rétro-ingénierer
//! le protocole d'un appareil non reconnu. Installation à la demande,
//! toujours déclenchée par une action manuelle explicite — jamais liée
//! au driver de télémétrie automatique, jamais silencieuse.
//!
//! USBPcapCMD.exe ne filtre que par hub racine (`\\.\USBPcapN`), jamais
//! par appareil précis (limite confirmée du projet upstream) : on capture
//! tous les hubs racine détectés en parallèle, le filtrage par VID/PID se
//! fait après coup à l'analyse, pas ici.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const USBPCAP_URL: &str =
    "https://github.com/desowin/usbpcap/releases/download/1.5.4.0/USBPcapSetup-1.5.4.0.exe";
const USBPCAP_SHA256: &str = "87a7edf9bbbcf07b5f4373d9a192a6770d2ff3add7aa1e276e82e38582ccb622";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_ROOT_HUBS: u32 = 8;

fn root_hub_path(n: u32) -> String {
    format!(r"\\.\USBPcap{n}")
}

/// Tente d'ouvrir chaque \\.\USBPcapN (1 à 8) — ceux qui s'ouvrent sont
/// des hubs racine capturables. Ne capture rien, ouverture immédiatement
/// refermée (RAII, `OpenOptions::open` retourne un `File` qui se ferme
/// en sortant de portée).
pub fn enumerate_root_hubs() -> Vec<u32> {
    (1..=MAX_ROOT_HUBS)
        .filter(|n| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(root_hub_path(*n))
                .is_ok()
        })
        .collect()
}

/// Vrai si le driver USBPcap est déjà installé (au moins un hub racine
/// ouvrable).
pub fn usbpcap_ready() -> bool {
    !enumerate_root_hubs().is_empty()
}

fn usbpcap_setup_dir() -> Result<PathBuf> {
    crate::settings::dirs_dir().context("répertoire de config introuvable")
}

/// Télécharge (SHA-256 pinné) puis installe USBPcap en silencieux.
/// No-op si déjà installé. Nécessite les droits administrateur (PureRGB
/// les demande déjà au lancement pour PawnIO/OpenRGB).
pub fn usbpcap_install() -> Result<()> {
    if usbpcap_ready() {
        return Ok(());
    }
    let dir = usbpcap_setup_dir()?;
    std::fs::create_dir_all(&dir)?;
    let setup = dir.join("USBPcapSetup-1.5.4.0.exe");
    // Les apostrophes sont valides dans un nom de compte Windows (ex: O'Brien) ;
    // on les double pour rester une chaîne PowerShell à guillemets simples valide.
    let exe_str = setup.display().to_string().replace('\'', "''");
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -Uri '{url}' -OutFile '{exe}' -UseBasicParsing; \
         $h = (Get-FileHash '{exe}' -Algorithm SHA256).Hash.ToLower(); \
         if ($h -ne '{sha}') {{ Remove-Item '{exe}' -Force; throw \"hash mismatch: $h\" }}",
        url = USBPCAP_URL,
        exe = exe_str,
        sha = USBPCAP_SHA256,
    );
    let mut dl_cmd = Command::new("powershell.exe");
    dl_cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        dl_cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = dl_cmd.output().context("téléchargement USBPcap")?;
    if !output.status.success() {
        bail!(
            "téléchargement USBPcap échoué: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut install_cmd = Command::new(&setup);
    install_cmd.args(["/S"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        install_cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let status = install_cmd.status().context("lancement USBPcapSetup")?;
    if !status.success() {
        bail!("installation USBPcap échouée (code {status})");
    }
    let _ = std::fs::remove_file(&setup);
    if !usbpcap_ready() {
        bail!(
            "USBPcap installé mais aucun hub racine détecté — un \
             redémarrage de Windows peut être nécessaire"
        );
    }
    Ok(())
}

pub struct CaptureFile {
    pub path: PathBuf,
    pub hub: u32,
    pub size_bytes: u64,
}

pub struct CaptureSession {
    dir: PathBuf,
    children: Vec<Child>,
}

fn find_usbpcapcmd() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from(r"C:\Program Files\USBPcap\USBPcapCMD.exe"),
        PathBuf::from(r"C:\Program Files (x86)\USBPcap\USBPcapCMD.exe"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .context("USBPcapCMD.exe introuvable après installation")
}

/// Scanne un dossier de capture et retourne les fichiers .pcap non vides,
/// triés par numéro de hub.
fn collect_capture_files(dir: &Path) -> Vec<CaptureFile> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.len() == 0 {
            continue;
        }
        let hub = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("usbcapture_hub"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        files.push(CaptureFile {
            path,
            hub,
            size_bytes: meta.len(),
        });
    }
    files.sort_by_key(|f| f.hub);
    files
}

/// Démarre une capture sur tous les hubs racine détectés — un process
/// `USBPcapCMD.exe` par hub, chacun écrivant son propre fichier.
pub fn start_capture() -> Result<CaptureSession> {
    let hubs = enumerate_root_hubs();
    if hubs.is_empty() {
        bail!("aucun hub racine USBPcap détecté");
    }
    let cmd_path = find_usbpcapcmd()?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = usbpcap_setup_dir()?
        .join("captures")
        .join(timestamp.to_string());
    std::fs::create_dir_all(&dir)?;

    let mut children = Vec::new();
    for hub in hubs {
        let out = dir.join(format!("usbcapture_hub{hub}.pcap"));
        let mut cmd = Command::new(&cmd_path);
        cmd.arg("-d").arg(root_hub_path(hub)).arg("-o").arg(&out);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match cmd.spawn() {
            Ok(child) => children.push(child),
            Err(e) => {
                // Un hub précédent a peut-être déjà démarré : Child::drop ne tue
                // pas le process (comportement std documenté), donc sans ce
                // nettoyage explicite USBPcapCMD.exe continuerait de tourner et
                // d'écrire indéfiniment, orphelin et hors de portée.
                for mut c in children {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                return Err(e).with_context(|| format!("lancement capture hub {hub}"));
            }
        }
    }
    Ok(CaptureSession { dir, children })
}

/// Arrête tous les process de capture, retourne les fichiers non vides
/// produits (les hubs sans trafic ne produisent rien d'exploitable).
pub fn stop_capture(mut session: CaptureSession) -> Vec<CaptureFile> {
    for child in &mut session.children {
        let _ = child.kill();
        let _ = child.wait();
    }
    collect_capture_files(&session.dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formate_le_chemin_du_hub_racine() {
        assert_eq!(root_hub_path(1), r"\\.\USBPcap1");
        assert_eq!(root_hub_path(8), r"\\.\USBPcap8");
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "purergb_usbcapture_test_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ignore_les_fichiers_vides_et_trie_par_hub() {
        let dir = unique_test_dir("filter");
        std::fs::write(dir.join("usbcapture_hub2.pcap"), b"data").unwrap();
        std::fs::write(dir.join("usbcapture_hub1.pcap"), b"more data").unwrap();
        std::fs::write(dir.join("usbcapture_hub3.pcap"), b"").unwrap();

        let files = collect_capture_files(&dir);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].hub, 1);
        assert_eq!(files[1].hub, 2);
        assert!(files.iter().all(|f| f.size_bytes > 0));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dossier_absent_retourne_vide() {
        let dir = std::env::temp_dir().join("purergb_usbcapture_test_does_not_exist_xyz");
        let files = collect_capture_files(&dir);
        assert!(files.is_empty());
    }
}
