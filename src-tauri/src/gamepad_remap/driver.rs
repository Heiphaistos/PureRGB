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
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Dossier des ressources de l'application (appelé une fois au démarrage).
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}

/// Installeur embarqué (`resources/drivers/<nom>`), s'il existe dans l'installation.
fn bundled(name: &str) -> Option<PathBuf> {
    let res = RESOURCE_DIR.get()?;
    [res.join("resources").join("drivers").join(name), res.join("drivers").join(name)].into_iter().find(|p| p.is_file())
}

pub const VIGEM_VERSION: &str = "1.22.0";
pub const VIGEM_URL: &str = "https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe";
pub const VIGEM_SHA256: &str = "89220a7865076b342892f98865f3499fb7c4cfd673159e89d352c360fd014c6a";
pub const VIGEM_FILE: &str = "ViGEmBus_1.22.0_x64_x86_arm64.exe";
pub const HIDHIDE_URL: &str = "https://github.com/nefarius/HidHide";
pub const HIDHIDE_VERSION: &str = "1.5.230";
pub const HIDHIDE_FILE: &str = "HidHide_1.5.230_x64.exe";
/// SHA-256 de l'installeur officiel HidHide v1.5.230.0 (signé Nefarius), relevé le 2026-10-26.
pub const HIDHIDE_SHA256: &str = "f4bbbcb82e6258641b887c74bc81c4c5f66e4aa811808dfc304347687b7605f6";
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
    pub hidhide_version: &'static str,
    pub hidhide_sha256: &'static str,
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
    DriverInfo { vigem_ready: ready, hidhide_installed: hidhide, vigem_version: VIGEM_VERSION, vigem_sha256: VIGEM_SHA256, hidhide_url: HIDHIDE_URL, hidhide_version: HIDHIDE_VERSION, hidhide_sha256: HIDHIDE_SHA256 }
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

/// Copie l'installeur EMBARQUÉ dans `%ProgramFiles%\PureRGB\<sub>` (dossier protégé : pas de
/// substitution possible), vérifie son SHA-256 épinglé, le lance et attend sa fin. Si la copie
/// embarquée manque (ViGEmBus seulement), repli sur le téléchargement officiel à hash épinglé.
fn run_installer(sub: &str, file: &str, sha256: &str, fallback_url: Option<&str>) -> Result<()> {
    let dir = trust::managed_dir(sub).context("ProgramFiles introuvable")?;
    std::fs::create_dir_all(&dir)?;
    let setup = dir.join(file);
    match bundled(file) {
        Some(src) => {
            std::fs::copy(&src, &setup).context("copie de l'installeur embarqué")?;
            trust::verify_sha256(&setup, sha256).inspect_err(|_| {
                let _ = std::fs::remove_file(&setup);
            })?;
        }
        None => {
            let url = fallback_url.context("installeur absent de cette installation")?;
            let lim = trust::DlLimits { max_bytes: MAX_INSTALLER, max_secs: 120, redirect_hosts: Some(trust::GITHUB_REDIRECT_HOSTS) };
            trust::download_pinned(url, sha256, &setup, &lim, &|_| {}).context("téléchargement")?;
        }
    }
    let status = std::process::Command::new(&setup).status().context("lancement de l'installeur")?;
    let _ = std::fs::remove_file(&setup);
    if !status.success() {
        bail!("L'installeur s'est terminé avec le code {status}");
    }
    Ok(())
}

/// ViGEmBus. N'est appelé que par la commande confirmée par l'utilisateur.
pub fn install(confirmed: bool) -> Result<()> {
    check_install_allowed(confirmed, vigem_ready(), crate::gamemode::active())?;
    run_installer("vigembus", VIGEM_FILE, VIGEM_SHA256, Some(VIGEM_URL))
}

/// HidHide (masque la manette physique derrière la virtuelle ; redémarrage requis, réglé par l'utilisateur
/// dans HidHide Configuration Client : PureRGB ne modifie JAMAIS sa liste de périphériques masqués).
pub fn install_hidhide(confirmed: bool) -> Result<()> {
    let already = std::env::var_os("SystemRoot").is_some_and(|r| hidhide_installed_in(Path::new(&r)));
    if already {
        bail!("HidHide est déjà installé");
    }
    check_install_allowed(confirmed, false, crate::gamemode::active())?;
    run_installer("hidhide", HIDHIDE_FILE, HIDHIDE_SHA256, None)
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
    fn hidhide_hash_well_formed_and_refused_unconfirmed() {
        assert_eq!(HIDHIDE_SHA256.len(), 64);
        assert!(install_hidhide(false).is_err());
    }

    #[test]
    fn bundled_installers_match_pinned_hashes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("drivers");
        for (f, h) in [(VIGEM_FILE, VIGEM_SHA256), (HIDHIDE_FILE, HIDHIDE_SHA256)] {
            trust::verify_sha256(&dir.join(f), h).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
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
