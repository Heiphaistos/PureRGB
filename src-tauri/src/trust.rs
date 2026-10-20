//! Confiance des binaires lancés avec les droits administrateur : PureRGB est
//! `requireAdministrator`, donc tout enfant est élevé. Un exe situé dans un
//! dossier modifiable par l'utilisateur (%APPDATA%, %LOCALAPPDATA%) donnerait
//! admin sans UAC à tout processus non élevé. Règle : on ne lance élevé que
//! ce qui est sous %ProgramFiles% (ACL admin) ; les binaires gérés sont
//! installés dans `%ProgramFiles%\PureRGB\<nom>` après vérification SHA-256.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Seuls hôtes/préfixes depuis lesquels un binaire peut être téléchargé.
const ALLOWED_URL_PREFIXES: &[&str] = &[
    "https://codeberg.org/OpenRGB/OpenRGB/releases/download/",
    "https://github.com/namazso/PawnIO.Setup/releases/download/",
    "https://github.com/Heiphaistos/PureRGB/releases/download/",
];

const PF_VARS: [&str; 3] = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"];

fn program_files_roots() -> Vec<PathBuf> {
    PF_VARS
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect()
}

fn lower(p: &Path) -> Vec<String> {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

/// `path` est sous l'une des `roots` (insensible à la casse, sans `..`).
fn is_under(path: &Path, roots: &[PathBuf]) -> bool {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let p = lower(path);
    roots
        .iter()
        .any(|r| !r.as_os_str().is_empty() && p.len() > lower(r).len() && p.starts_with(&lower(r)))
}

/// Vrai si `path` est sous %ProgramFiles% : seul emplacement dont on
/// accepte qu'un exe soit lancé élevé.
pub fn is_trusted(path: &Path) -> bool {
    is_under(path, &program_files_roots())
}

/// `%ProgramFiles%\PureRGB\<sub>` : dossier géré, à ACL admin.
pub fn managed_dir(sub: &str) -> Option<PathBuf> {
    std::env::var_os("ProgramFiles").map(|p| PathBuf::from(p).join("PureRGB").join(sub))
}

/// URL de téléchargement : https, préfixe de la liste blanche, aucun
/// caractère pouvant sortir du contexte (quote, espace, `@`, `..`, contrôle).
pub fn validate_url(url: &str) -> Result<()> {
    let ok_chars = url
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-._~/:%+".contains(c));
    if !ok_chars || url.contains("..") {
        bail!("URL de téléchargement refusée");
    }
    if !ALLOWED_URL_PREFIXES.iter().any(|p| url.starts_with(p)) {
        bail!("URL de téléchargement hors liste blanche : {url}");
    }
    Ok(())
}

pub fn sha256_hex(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path).context("ouverture du fichier à hasher")?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Hash différent : le fichier est supprimé et l'appelant reçoit une erreur.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_hex(path)?;
    if !actual.eq_ignore_ascii_case(expected) {
        let _ = std::fs::remove_file(path);
        bail!("SHA-256 inattendu ({actual}), fichier supprimé");
    }
    Ok(())
}

/// Télécharge `url` (liste blanche) vers `dest`, vérifie le SHA-256 puis
/// renomme : `dest` n'existe jamais avec un contenu non vérifié. Client :
/// curl.exe (arguments, aucun shell, https uniquement même après redirection).
pub fn download_verified(url: &str, sha256: &str, dest: &Path) -> Result<()> {
    validate_url(url)?;
    let tmp = dest.with_extension("download");
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-sSfL", "--proto", "=https", "--proto-redir", "=https", "--max-time", "180", "-o"])
        .arg(&tmp)
        .arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().context("lancement de curl.exe")?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("téléchargement échoué : {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    verify_sha256(&tmp, sha256)?;
    let _ = std::fs::remove_file(dest);
    std::fs::rename(&tmp, dest).context("installation du fichier vérifié")
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "https://codeberg.org/OpenRGB/OpenRGB/releases/download/release_candidate_1.0rc3/OpenRGB_1.0rc3_Windows_64_6fbcf62.zip";

    #[test]
    fn url_pinned_accepted() {
        assert!(validate_url(OK).is_ok());
    }

    #[test]
    fn url_rejected() {
        for bad in [
            "http://codeberg.org/OpenRGB/OpenRGB/releases/download/x.zip",
            "https://codeberg.org.evil.com/OpenRGB/OpenRGB/releases/download/x.zip",
            "https://codeberg.org@evil.com/OpenRGB/OpenRGB/releases/download/x.zip",
            "https://codeberg.org/OpenRGB/OpenRGB/releases/download/x'; calc; '.zip",
            "https://codeberg.org/OpenRGB/OpenRGB/releases/download/../../x.zip",
            "https://codeberg.org/Evil/OpenRGB/releases/download/x.zip",
            "https://codeberg.org/OpenRGB/OpenRGB/releases/download/x y.zip",
        ] {
            assert!(validate_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn trusted_only_under_program_files() {
        let roots = [PathBuf::from(r"C:\Program Files")];
        assert!(is_under(Path::new(r"c:\program files\PureRGB\purergb.exe"), &roots));
        assert!(!is_under(Path::new(r"C:\Program Files"), &roots));
        assert!(!is_under(Path::new(r"C:\Users\x\AppData\Local\PureRGB\purergb.exe"), &roots));
        assert!(!is_under(Path::new(r"C:\Program Files Evil\a.exe"), &roots));
        assert!(!is_under(Path::new(r"C:\Program Files\..\Users\a.exe"), &roots));
    }

    #[test]
    fn sha256_mismatch_deletes_file() {
        let p = std::env::temp_dir().join("purergb_trust_test.bin");
        std::fs::write(&p, b"abc").unwrap();
        let good = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_sha256(&p, good).is_ok());
        assert!(verify_sha256(&p, &"0".repeat(64)).is_err());
        assert!(!p.exists());
    }
}
