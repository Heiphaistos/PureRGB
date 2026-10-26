//! Pilote ViGEmBus (manette virtuelle) : détection, et installation SUR CONFIRMATION de l'utilisateur.
//!
//! ViGEmBus (BSD-3, projet archivé par son auteur mais redistribuable) n'est JAMAIS téléchargé ni
//! installé automatiquement. L'installeur officiel (signé Nefarius Software Solutions e.U.) est
//! téléchargé depuis les releases GitHub officielles, son SHA-256 est vérifié par `trust.rs`
//! (valeur ci-dessous relevée le 2026-10-26 sur le fichier officiel v1.22.0), puis lancé tel quel
//! (interface de l'installeur + UAC : l'utilisateur clique lui-même ; aucun argument silencieux
//! non documenté n'est supposé).

use crate::trust;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;

pub const VIGEM_VERSION: &str = "1.22.0";
pub const VIGEM_URL: &str = "https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe";
pub const VIGEM_SHA256: &str = "89220a7865076b342892f98865f3499fb7c4cfd673159e89d352c360fd014c6a";
pub const HIDHIDE_URL: &str = "https://github.com/nefarius/HidHide";
const MAX_INSTALLER: u64 = 32 << 20;

#[derive(Debug, Clone, Serialize)]
pub struct DriverInfo {
    /// Le pilote répond (ouverture du périphérique réussie).
    pub vigem_ready: bool,
    /// HidHide (masquage de la manette physique, option avancée) est installé.
    pub hidhide_installed: bool,
    pub vigem_version: &'static str,
    pub vigem_sha256: &'static str,
    pub hidhide_url: &'static str,
}

/// Ouvre le périphérique du pilote ; ne crée aucune manette.
#[cfg(windows)]
pub fn vigem_ready() -> bool {
    vigem_client::Client::connect().is_ok()
}

#[cfg(not(windows))]
pub fn vigem_ready() -> bool {
    false
}

/// HidHide installé si son pilote est présent dans `<racine Windows>\System32\drivers`.
pub fn hidhide_installed_in(system_root: &Path) -> bool {
    system_root.join("System32").join("drivers").join("HidHide.sys").is_file()
}

pub fn info() -> DriverInfo {
    // Mode jeu : on n'ouvre même pas le périphérique du pilote.
    let ready = !crate::gamemode::active() && vigem_ready();
    let hidhide = std::env::var_os("SystemRoot").is_some_and(|r| hidhide_installed_in(Path::new(&r)));
    DriverInfo { vigem_ready: ready, hidhide_installed: hidhide, vigem_version: VIGEM_VERSION, vigem_sha256: VIGEM_SHA256, hidhide_url: HIDHIDE_URL }
}

/// Garde-fous de l'installation, sans effet de bord (testable).
pub fn check_install_allowed(confirmed: bool, already_ready: bool, game_mode: bool) -> Result<()> {
    if !confirmed {
        bail!("Installation refusée : confirmation explicite de l'utilisateur requise");
    }
    if game_mode {
        bail!("Installation impossible pendant le mode jeu");
    }
    if already_ready {
        bail!("Le pilote ViGEmBus est déjà installé");
    }
    Ok(())
}

/// Télécharge l'installeur officiel (hash épinglé) dans `%ProgramFiles%\PureRGB\vigembus` puis le
/// lance et attend sa fin. N'est appelé que par la commande confirmée par l'utilisateur.
pub fn install(confirmed: bool) -> Result<()> {
    check_install_allowed(confirmed, vigem_ready(), crate::gamemode::active())?;
    let dir = trust::managed_dir("vigembus").context("ProgramFiles introuvable")?;
    std::fs::create_dir_all(&dir)?;
    let setup = dir.join("ViGEmBus_setup.exe");
    let lim = trust::DlLimits { max_bytes: MAX_INSTALLER, max_secs: 120, redirect_hosts: Some(trust::GITHUB_REDIRECT_HOSTS) };
    trust::download_pinned(VIGEM_URL, VIGEM_SHA256, &setup, &lim, &|_| {}).context("téléchargement ViGEmBus")?;
    let status = std::process::Command::new(&setup).status().context("lancement de l'installeur ViGEmBus")?;
    let _ = std::fs::remove_file(&setup);
    if !status.success() {
        bail!("L'installeur ViGEmBus s'est terminé avec le code {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_url_is_whitelisted_and_hash_well_formed() {
        assert!(trust::validate_url(VIGEM_URL).is_ok());
        assert_eq!(VIGEM_SHA256.len(), 64);
        assert!(VIGEM_SHA256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        // un autre dépôt du même hôte reste refusé
        assert!(trust::validate_url("https://github.com/evil/ViGEmBus/releases/download/v1/x.exe").is_err());
    }

    #[test]
    fn install_needs_explicit_confirmation() {
        assert!(check_install_allowed(false, false, false).is_err());
        assert!(check_install_allowed(true, false, true).is_err());
        assert!(check_install_allowed(true, true, false).is_err());
        assert!(check_install_allowed(true, false, false).is_ok());
    }

    #[test]
    fn hidhide_detection_by_driver_file() {
        let root = std::env::temp_dir().join("purergb_hidhide_test");
        let drv = root.join("System32").join("drivers");
        std::fs::create_dir_all(&drv).unwrap();
        let _ = std::fs::remove_file(drv.join("HidHide.sys"));
        assert!(!hidhide_installed_in(&root));
        std::fs::write(drv.join("HidHide.sys"), b"x").unwrap();
        assert!(hidhide_installed_in(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
