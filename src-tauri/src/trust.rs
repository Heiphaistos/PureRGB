//! Confiance des binaires lancés avec les droits administrateur : PureRGB est
//! `requireAdministrator`, donc tout enfant est élevé. Un exe situé dans un
//! dossier modifiable par l'utilisateur (%APPDATA%, %LOCALAPPDATA%) donnerait
//! admin sans UAC à tout processus non élevé. Règle : on ne lance élevé que
//! ce qui est sous %ProgramFiles% (ACL admin) ; les binaires gérés sont
//! installés dans `%ProgramFiles%\PureRGB\<nom>` après vérification SHA-256.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

/// Seuls hôtes/préfixes depuis lesquels un binaire peut être téléchargé.
const ALLOWED_URL_PREFIXES: &[&str] = &[
    "https://codeberg.org/OpenRGB/OpenRGB/releases/download/",
    "https://github.com/namazso/PawnIO.Setup/releases/download/",
    "https://github.com/Heiphaistos/PureRGB/releases/download/",
    "https://github.com/GyanD/codexffmpeg/releases/download/",
];

/// Hôtes où une redirection GitHub (releases) peut légitimement aboutir.
pub const GITHUB_REDIRECT_HOSTS: &[&str] =
    &["github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"];

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

/// Limites d'un téléchargement épinglé.
pub struct DlLimits<'a> {
    pub max_bytes: u64,
    pub max_secs: u32,
    /// Si présent : l'URL finale (après redirections) doit être https sur l'un de ces hôtes.
    pub redirect_hosts: Option<&'a [&'a str]>,
}

/// Hôte d'une URL https (`None` si ce n'est pas une URL https simple).
pub fn https_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains(['@', ':', '\\'])).then_some(host)
}

/// Télécharge `url` (liste blanche) vers `dest`, vérifie le SHA-256 puis
/// renomme : `dest` n'existe jamais avec un contenu non vérifié. Client :
/// curl.exe (arguments, aucun shell, https uniquement même après redirection).
pub fn download_verified(url: &str, sha256: &str, dest: &Path) -> Result<()> {
    download_pinned(url, sha256, dest, &DlLimits { max_bytes: 512 << 20, max_secs: 180, redirect_hosts: None }, &|_| {})
}

/// Comme `download_verified`, avec taille/durée maximales, hôtes de redirection autorisés et
/// progression (`progress(octets reçus)`, appelée ~4 fois par seconde).
pub fn download_pinned(url: &str, sha256: &str, dest: &Path, lim: &DlLimits, progress: &dyn Fn(u64)) -> Result<()> {
    validate_url(url)?;
    let tmp = dest.with_extension("download");
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-sSfL", "--proto", "=https", "--proto-redir", "=https", "--max-time"])
        .arg(lim.max_secs.to_string())
        .arg("--max-filesize")
        .arg(lim.max_bytes.to_string())
        .args(["-w", "%{url_effective}", "-o"])
        .arg(&tmp)
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let fail = |e: anyhow::Error| {
        let _ = std::fs::remove_file(&tmp);
        e
    };
    let mut child = cmd.spawn().context("lancement de curl.exe")?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                progress(std::fs::metadata(&tmp).map_or(0, |m| m.len()));
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(fail(anyhow::Error::new(e).context("attente de curl.exe")));
            }
        }
    };
    let out = child.wait_with_output().context("sortie de curl.exe")?;
    if !status.success() {
        return Err(fail(anyhow::anyhow!("téléchargement échoué : {}", String::from_utf8_lossy(&out.stderr).trim())));
    }
    if let Some(hosts) = lim.redirect_hosts {
        let fin = String::from_utf8_lossy(&out.stdout);
        if !https_host(fin.trim()).is_some_and(|h| hosts.contains(&h)) {
            return Err(fail(anyhow::anyhow!("redirection refusée vers un hôte hors liste blanche : {}", fin.trim())));
        }
    }
    if std::fs::metadata(&tmp).map_or(0, |m| m.len()) > lim.max_bytes {
        return Err(fail(anyhow::anyhow!("fichier téléchargé trop gros")));
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
    fn redirect_hosts() {
        assert_eq!(https_host("https://release-assets.githubusercontent.com/a/b?x=1"), Some("release-assets.githubusercontent.com"));
        for bad in ["http://github.com/x", "https://github.com@evil.com/x", "https://github.com:8443/x", "ftp://github.com/x", "https:///x"] {
            assert_eq!(https_host(bad), None, "{bad}");
        }
        assert!(validate_url("https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip").is_ok());
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
