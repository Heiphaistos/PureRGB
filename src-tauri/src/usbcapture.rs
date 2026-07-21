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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formate_le_chemin_du_hub_racine() {
        assert_eq!(root_hub_path(1), r"\\.\USBPcap1");
        assert_eq!(root_hub_path(8), r"\\.\USBPcap8");
    }
}
