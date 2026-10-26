//! Mise à jour de la version portable (l'installeur NSIS ne la remplace pas) et préparation
//! de la mise à jour installée. Le fichier téléchargé est vérifié avec la même clé minisign que
//! le manifeste (`plugins.updater.pubkey`) avant de remplacer quoi que ce soit.

use anyhow::{bail, Context, Result};
use base64::Engine;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::AppHandle;

const BASE: &str = "https://github.com/Heiphaistos/PureRGB/releases/download";
const MAX_BYTES: u64 = 200 << 20;

fn exe() -> Result<PathBuf> {
    std::env::current_exe().context("chemin de l'exécutable")
}

/// Portable = pas de `uninstall.exe` à côté de l'exécutable (l'installeur NSIS en dépose un).
fn is_portable_at(exe: &Path) -> bool {
    exe.parent().is_some_and(|d| !d.join("uninstall.exe").exists())
}

fn b64(s: &str) -> Result<String> {
    let raw = base64::engine::general_purpose::STANDARD.decode(s.trim()).context("base64 invalide")?;
    String::from_utf8(raw).context("texte invalide")
}

/// Vérifie `data` contre la signature `sig_b64` (contenu d'un fichier .sig de Tauri) avec `pubkey_b64`.
fn verify(data: &[u8], sig_b64: &str, pubkey_b64: &str) -> Result<()> {
    let pk = minisign_verify::PublicKey::decode(&b64(pubkey_b64)?).context("clé publique")?;
    let sig = minisign_verify::Signature::decode(&b64(sig_b64)?).context("signature")?;
    pk.verify(data, &sig, true).map_err(|e| anyhow::anyhow!("signature refusée : {e}"))
}

fn pubkey() -> Result<String> {
    let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))?;
    conf["plugins"]["updater"]["pubkey"].as_str().map(str::to_owned).context("clé publique absente")
}

fn curl(url: &str, dest: &Path) -> Result<()> {
    let mut c = Command::new("curl.exe");
    c.args(["-sSfL", "--proto", "=https", "--proto-redir", "=https", "--max-time", "300", "--max-filesize"])
        .arg(MAX_BYTES.to_string())
        .arg("-o")
        .arg(dest)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let out = c.output().context("lancement de curl.exe")?;
    if !out.status.success() {
        bail!("téléchargement échoué : {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// Télécharge, vérifie et met en place la version `version` à côté de l'actuelle ; renvoie l'exe à relancer.
fn stage(version: &str) -> Result<PathBuf> {
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        bail!("numéro de version invalide");
    }
    let cur = exe()?;
    let new = cur.with_extension("exe.new");
    let old = cur.with_extension("exe.old");
    let url = format!("{BASE}/v{version}/PureRGB-portable.exe");
    let sig_file = cur.with_extension("exe.new.sig");
    let r = (|| {
        curl(&url, &new)?;
        curl(&format!("{url}.sig"), &sig_file)?;
        let data = std::fs::read(&new)?;
        verify(&data, &std::fs::read_to_string(&sig_file)?, &pubkey()?)?;
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&cur, &old).context("l'exécutable actuel ne peut pas être déplacé (dossier protégé ?)")?;
        if let Err(e) = std::fs::rename(&new, &cur) {
            let _ = std::fs::rename(&old, &cur);
            return Err(anyhow::Error::new(e).context("mise en place de la nouvelle version"));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&sig_file);
    if r.is_err() {
        let _ = std::fs::remove_file(&new);
    }
    r.map(|_| cur)
}

/// Relance `exe` après 3 s (le temps que PureRGB quitte : instance unique), hors du job de fermeture.
fn relaunch_later(exe: &Path) -> Result<()> {
    let mut c = Command::new("cmd.exe");
    c.arg("/C").arg(format!("ping -n 4 127.0.0.1 >nul & start \"\" \"{}\"", exe.display()));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000 | 0x0000_0200); // sans console | nouveau groupe
    }
    c.spawn().context("relance")?;
    Ok(())
}

/// Nettoie avant qu'un installeur (ou la nouvelle version) prenne la place : matériel rendu,
/// sidecars arrêtés, puis le job ne tue plus ce qui sera lancé ensuite.
fn prepare(app: &AppHandle) {
    crate::cleanup_before_exit(app);
    crate::job::release_children();
}

/// Supprime l'ancien exécutable laissé par une mise à jour portable (best-effort, au démarrage).
pub fn cleanup_old() {
    if let Ok(e) = exe() {
        let _ = std::fs::remove_file(e.with_extension("exe.old"));
    }
}

#[tauri::command]
pub fn update_is_portable() -> bool {
    exe().is_ok_and(|e| is_portable_at(&e))
}

/// Mise à jour installée : à appeler juste avant `Update::install()` (le plugin quitte ensuite).
#[tauri::command]
pub async fn update_prepare(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || prepare(&app)).await;
}

/// Mise à jour portable : télécharge, vérifie, remplace l'exécutable, quitte et relance.
#[tauri::command]
pub async fn update_portable(app: AppHandle, version: String) -> Result<(), String> {
    let cur = tauri::async_runtime::spawn_blocking(move || stage(&version))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))?;
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || prepare(&a)).await.map_err(|e| e.to_string())?;
    relaunch_later(&cur).map_err(|e| format!("{e:#}"))?;
    // Nettoyage déjà fait : sortie directe (le filet de shutdown.rs rejouerait cleanup inutilement).
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_detection() {
        let d = std::env::temp_dir().join("purergb-portable-test");
        let _ = std::fs::create_dir_all(&d);
        let exe = d.join("purergb.exe");
        let _ = std::fs::remove_file(d.join("uninstall.exe"));
        assert!(is_portable_at(&exe));
        std::fs::write(d.join("uninstall.exe"), b"").unwrap();
        assert!(!is_portable_at(&exe));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bad_signature_refused() {
        let pk = pubkey().unwrap();
        let junk = base64::engine::general_purpose::STANDARD.encode("untrusted comment: x\nAAAA\ntrusted comment: y\nAAAA");
        assert!(verify(b"data", &junk, &pk).is_err());
        assert!(verify(b"data", "pas du base64 !!", &pk).is_err());
    }

    #[test]
    fn version_validated() {
        assert!(stage("1.0/../x").is_err());
        assert!(stage("").is_err());
    }
}
