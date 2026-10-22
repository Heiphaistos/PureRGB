//! Installation du décodeur vidéo de secours (ffmpeg) quand Media Foundation ne sait pas lire un fichier.
//!
//! Même politique de confiance que les autres binaires gérés (`trust.rs`) : archive épinglée
//! (URL en liste blanche, taille et SHA-256 vérifiés AVANT toute extraction), redirections limitées à
//! GitHub, extraction d'UNE seule entrée connue vers `%ProgramFiles%\PureRGB\tools\ffmpeg.exe`
//! (jamais de chemin tiré de l'archive). PureRGB étant élevé, l'exe n'est lancé que s'il est sous
//! Program Files et que son SHA-256 (posé à l'installation dans `ffmpeg.exe.sha256`) correspond.

use crate::trust;
use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const FFMPEG_VERSION: &str = "9.0.2";
pub const FFMPEG_URL: &str = "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip";
pub const FFMPEG_ZIP_BYTES: u64 = 114_768_076;
pub const FFMPEG_ZIP_SHA256: &str = "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba";
pub const FFMPEG_ENTRY: &str = "ffmpeg-9.0.2-essentials_build/bin/ffmpeg.exe";
/// Taille maximale de l'exe extrait (protège contre une bombe de décompression).
pub const MAX_EXE_BYTES: u64 = 200 * 1024 * 1024;
const MAX_SECS: u32 = 900;

#[derive(Debug, Clone, Serialize)]
pub struct FfmpegStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

/// `phase` : `idle` | `download` | `extract` | `done` | `error`.
#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub phase: String,
    pub done: u64,
    pub total: u64,
    pub error: Option<String>,
}

static PROGRESS: Mutex<InstallProgress> = Mutex::new(InstallProgress { phase: String::new(), done: 0, total: 0, error: None });
/// (exe, taille, date) déjà vérifiés : évite de re-hasher 100 Mo à chaque lancement.
static VERIFIED: Mutex<Option<(PathBuf, u64, SystemTime)>> = Mutex::new(None);

fn set_progress(phase: &str, done: u64, error: Option<String>) {
    *PROGRESS.lock() = InstallProgress { phase: phase.into(), done, total: FFMPEG_ZIP_BYTES, error };
}

pub fn progress() -> InstallProgress {
    let p = PROGRESS.lock().clone();
    if p.phase.is_empty() {
        InstallProgress { phase: "idle".into(), ..p }
    } else {
        p
    }
}

fn marker_of(exe: &Path) -> PathBuf {
    exe.with_extension("exe.sha256")
}

/// L'exe existe et son SHA-256 est celui noté à l'installation (re-hash seulement s'il a changé).
pub fn check_installed(exe: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(exe) else { return false };
    let key = (exe.to_path_buf(), meta.len(), meta.modified().unwrap_or(SystemTime::UNIX_EPOCH));
    if VERIFIED.lock().as_ref() == Some(&key) {
        return true;
    }
    let Ok(expected) = std::fs::read_to_string(marker_of(exe)) else { return false };
    let ok = trust::sha256_hex(exe).is_ok_and(|h| h.eq_ignore_ascii_case(expected.trim()));
    if ok {
        *VERIFIED.lock() = Some(key);
    }
    ok
}

/// Exe ffmpeg de confiance (sous Program Files, hash vérifié), s'il est installé.
pub fn locate() -> Option<PathBuf> {
    let exe = trust::managed_dir("tools")?.join("ffmpeg.exe");
    (trust::is_trusted(&exe) && check_installed(&exe)).then_some(exe)
}

pub fn status() -> FfmpegStatus {
    let path = locate();
    FfmpegStatus { installed: path.is_some(), version: path.as_ref().map(|_| FFMPEG_VERSION.into()), path: path.map(|p| p.display().to_string()) }
}

/// Extrait l'entrée `name` (nom exact) de l'archive vers `dest`, au plus `max` octets.
/// Le nom de l'entrée ne sert qu'à la retrouver : la destination est imposée par l'appelant.
pub fn extract_entry(zip_path: &Path, name: &str, dest: &Path, max: u64) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(zip_path).context("ouverture de l'archive")?).context("archive illisible")?;
    let mut entry = zip.by_name(name).map_err(|_| anyhow::anyhow!("l'archive ne contient pas {name}"))?;
    if !entry.is_file() || entry.size() > max {
        bail!("entrée {name} invalide ou trop grosse");
    }
    let res = (|| -> Result<()> {
        let mut out = std::fs::File::create(dest).context("création de ffmpeg.exe")?;
        let n = std::io::copy(&mut (&mut entry).take(max + 1), &mut out).context("extraction de ffmpeg.exe")?;
        if n > max {
            bail!("ffmpeg.exe dépasse {max} octets");
        }
        out.flush().context("écriture de ffmpeg.exe")
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    res
}

/// Taille exacte puis SHA-256 ; l'archive est supprimée si l'un des deux ne correspond pas.
pub fn verify_archive(zip_path: &Path, bytes: u64, sha256: &str) -> Result<()> {
    if std::fs::metadata(zip_path).context("archive absente")?.len() != bytes {
        let _ = std::fs::remove_file(zip_path);
        bail!("taille de l'archive inattendue, fichier supprimé");
    }
    trust::verify_sha256(zip_path, sha256)
}

/// Archive épinglée vérifiée (taille + SHA-256) puis `dir/ffmpeg.exe` + marqueur de hash.
pub fn install_from_archive(zip_path: &Path, dir: &Path) -> Result<PathBuf> {
    verify_archive(zip_path, FFMPEG_ZIP_BYTES, FFMPEG_ZIP_SHA256)?;
    install_entry(zip_path, FFMPEG_ENTRY, dir)
}

fn install_entry(zip_path: &Path, entry: &str, dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).context("création du dossier tools")?;
    let (exe, tmp) = (dir.join("ffmpeg.exe"), dir.join("ffmpeg.exe.tmp"));
    extract_entry(zip_path, entry, &tmp, MAX_EXE_BYTES)?;
    let hash = trust::sha256_hex(&tmp)?;
    let _ = std::fs::remove_file(marker_of(&exe));
    let _ = std::fs::remove_file(&exe);
    std::fs::rename(&tmp, &exe).context("installation de ffmpeg.exe")?;
    std::fs::write(marker_of(&exe), hash).context("marqueur de vérification")?;
    Ok(exe)
}

/// Télécharge, vérifie et installe ffmpeg (bloquant : à lancer hors thread UI).
pub fn install() -> Result<PathBuf> {
    {
        let mut p = PROGRESS.lock();
        if matches!(p.phase.as_str(), "download" | "extract") {
            bail!("une installation est déjà en cours");
        }
        *p = InstallProgress { phase: "download".into(), done: 0, total: FFMPEG_ZIP_BYTES, error: None };
    }
    let res = (|| {
        let dir = trust::managed_dir("tools").context("ProgramFiles introuvable")?;
        std::fs::create_dir_all(&dir).context("création du dossier tools (droits administrateur requis)")?;
        let zip = dir.join("ffmpeg_download.zip");
        let lim = trust::DlLimits { max_bytes: FFMPEG_ZIP_BYTES, max_secs: MAX_SECS, redirect_hosts: Some(trust::GITHUB_REDIRECT_HOSTS) };
        trust::download_pinned(FFMPEG_URL, FFMPEG_ZIP_SHA256, &zip, &lim, &|n| set_progress("download", n, None)).context("téléchargement de ffmpeg")?;
        set_progress("extract", FFMPEG_ZIP_BYTES, None);
        let r = install_from_archive(&zip, &dir);
        let _ = std::fs::remove_file(&zip);
        r
    })();
    match &res {
        Ok(_) => set_progress("done", FFMPEG_ZIP_BYTES, None),
        Err(e) => set_progress("error", 0, Some(format!("{e:#}"))),
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("purergb_ffi_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (n, d) in entries {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(d).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn extracts_only_the_named_entry_to_the_imposed_path() {
        let d = tmpdir("ok");
        let z = d.join("a.zip");
        make_zip(&z, &[("../evil.txt", b"x"), ("x/../../pwn.exe", b"x"), (FFMPEG_ENTRY, b"MZfake"), ("doc/readme.txt", b"hi")]);
        let exe = install_entry(&z, FFMPEG_ENTRY, &d.join("tools")).unwrap();
        assert_eq!(exe, d.join("tools").join("ffmpeg.exe"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZfake");
        assert!(check_installed(&exe), "marqueur de hash posé et valide");
        let mut names: Vec<_> = std::fs::read_dir(d.join("tools")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["ffmpeg.exe", "ffmpeg.exe.sha256"], "rien d'autre, ni .tmp");
        assert!(!d.join("evil.txt").exists() && !d.join("pwn.exe").exists());
    }

    #[test]
    fn tampered_exe_is_not_trusted() {
        let d = tmpdir("tamper");
        let z = d.join("a.zip");
        make_zip(&z, &[(FFMPEG_ENTRY, b"MZfake")]);
        let exe = install_entry(&z, FFMPEG_ENTRY, &d.join("tools")).unwrap();
        assert!(check_installed(&exe));
        std::fs::write(&exe, b"MZevil!").unwrap(); // contenu changé
        assert!(!check_installed(&exe), "taille changée : re-hash, refusé");
        std::fs::write(&exe, b"MZfake").unwrap();
        // Même taille qu'à l'installation : sans date de modification différente, le cache de vérification
        // reconnaîtrait le fichier (les horloges de fichiers ont un grain de plusieurs ms).
        std::fs::File::options().write(true).open(&exe).unwrap().set_modified(SystemTime::now() + std::time::Duration::from_secs(30)).unwrap();
        std::fs::write(marker_of(&exe), "0".repeat(64)).unwrap();
        assert!(!check_installed(&exe), "marqueur faux : refusé");
        std::fs::remove_file(marker_of(&exe)).unwrap();
        assert!(!check_installed(&exe), "sans marqueur : refusé");
    }

    #[test]
    fn missing_or_oversized_entry_is_refused() {
        let d = tmpdir("bad");
        let z = d.join("a.zip");
        make_zip(&z, &[("autre/ffmpeg.exe", b"MZ"), (FFMPEG_ENTRY, &[7u8; 64])]);
        assert!(extract_entry(&z, "ffmpeg.exe", &d.join("o"), 1000).is_err(), "nom exact seulement");
        let e = extract_entry(&z, FFMPEG_ENTRY, &d.join("o"), 10).unwrap_err();
        assert!(format!("{e:#}").contains("trop grosse"), "{e:#}");
        assert!(!d.join("o").exists());
        let z2 = d.join("b.zip");
        make_zip(&z2, &[("seulement/doc.txt", b"x")]);
        assert!(install_entry(&z2, FFMPEG_ENTRY, &d.join("tools")).is_err());
        assert!(!d.join("tools").join("ffmpeg.exe").exists());
        std::fs::write(d.join("pas_un_zip.zip"), b"PK pas vraiment").unwrap();
        assert!(extract_entry(&d.join("pas_un_zip.zip"), FFMPEG_ENTRY, &d.join("o"), 10).is_err());
    }

    #[test]
    fn archive_size_and_hash_checked_before_extraction() {
        let d = tmpdir("arch");
        let z = d.join("a.zip");
        make_zip(&z, &[(FFMPEG_ENTRY, b"MZfake")]);
        // Archive piégée : ni la taille ni le hash épinglés -> refusée et supprimée, rien extrait.
        assert!(install_from_archive(&z, &d.join("tools")).is_err());
        assert!(!z.exists() && !d.join("tools").join("ffmpeg.exe").exists());
        make_zip(&z, &[(FFMPEG_ENTRY, b"MZfake")]);
        let len = std::fs::metadata(&z).unwrap().len();
        let e = verify_archive(&z, len, &"0".repeat(64)).unwrap_err();
        assert!(format!("{e:#}").contains("SHA-256"), "{e:#}");
        assert!(!z.exists(), "mauvais hash : fichier supprimé");
    }

    /// Vraie archive épinglée (téléchargée à part) : taille + SHA + extraction + exécution `-version`.
    #[test]
    #[ignore = "nécessite PURERGB_TEST_FFMPEG_ZIP (archive épinglée déjà téléchargée)"]
    fn real_pinned_archive_installs_and_runs() {
        let Some(z) = std::env::var_os("PURERGB_TEST_FFMPEG_ZIP") else { return };
        let d = tmpdir("real");
        let exe = install_from_archive(Path::new(&z), &d.join("tools")).expect("installation");
        assert!(check_installed(&exe));
        let out = std::process::Command::new(&exe).arg("-version").output().expect("lancement");
        let txt = String::from_utf8_lossy(&out.stdout);
        assert!(txt.contains("ffmpeg version 9.0.2"), "{}", txt.lines().next().unwrap_or(""));
    }
}
