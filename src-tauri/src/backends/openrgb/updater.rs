//! Auto-update de l'OpenRGB embarqué : vérifie la dernière release Codeberg
//! au lancement, télécharge et bascule les copies gérées par PureRGB
//! (bundle NSIS, `%APPDATA%\PureRGB\openrgb`) avec rollback automatique si
//! la nouvelle version ne démarre pas. Ne touche jamais une installation
//! OpenRGB indépendante de l'utilisateur.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::manager::{CreationFlagsExt, OpenRgbManager};

const RELEASES_API: &str =
    "https://codeberg.org/api/v1/repos/OpenRGB/OpenRGB/releases?limit=1";

/// Sélectionne l'asset Windows 64-bit dans la liste des assets d'une
/// release Codeberg. Le hash de commit dans le nom change à chaque
/// release (`OpenRGB_1.0rc3_Windows_64_6fbcf62.zip`) — sélection par motif,
/// exclut explicitement le `.msi`.
pub fn pick_windows_asset(assets: &[(String, String)]) -> Option<(String, String)> {
    assets
        .iter()
        .find(|(name, _)| {
            name.starts_with("OpenRGB_") && name.contains("_Windows_64_") && name.ends_with(".zip")
        })
        .cloned()
}

/// true si la version distante diffère de la version enregistrée (ou si
/// aucune version n'a jamais été enregistrée).
pub fn needs_update(latest_tag: &str, saved_version: &Option<String>) -> bool {
    saved_version.as_deref() != Some(latest_tag)
}

/// Interroge l'API Codeberg pour la dernière release OpenRGB publiée.
/// Retourne `(tag_name, download_url)` de l'asset Windows 64-bit.
/// Aucun fichier de checksums n'est publié par OpenRGB : seule garantie,
/// HTTPS vers `codeberg.org` (décision validée dans la spec).
pub fn check_latest_version() -> Result<(String, String)> {
    let body = crate::netdev::curl(&[RELEASES_API]).context("requête releases Codeberg")?;
    let releases: serde_json::Value =
        serde_json::from_str(&body).context("parsing réponse Codeberg")?;
    let release = releases
        .get(0)
        .context("aucune release Codeberg trouvée")?;
    let tag_name = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .context("tag_name absent de la réponse Codeberg")?
        .to_string();
    let assets: Vec<(String, String)> = release
        .get("assets")
        .and_then(|v| v.as_array())
        .context("assets absents de la réponse Codeberg")?
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            Some((name, url))
        })
        .collect();
    let (_, download_url) = pick_windows_asset(&assets)
        .context("aucun asset Windows 64-bit trouvé dans la release")?;
    Ok((tag_name, download_url))
}

/// Vérifie, télécharge et bascule l'OpenRGB embarqué si une nouvelle
/// version est disponible. Ne fait jamais rien si un serveur répond déjà
/// sur `host:port` — ça ne peut arriver ici (appelé avant le premier
/// `ensure_running` du thread `hw-init`) que si l'utilisateur a sa propre
/// installation OpenRGB indépendante déjà lancée : jamais touchée.
/// Retourne `Some(nouveau_tag)` en cas de bascule réussie (à persister
/// dans `settings.json`), `None` sinon (rien à faire, ou échec — l'ancienne
/// version reste en place, nouvelle tentative au prochain lancement).
pub fn update_if_needed(
    mgr: &OpenRgbManager,
    host: &str,
    port: u16,
    saved_version: &Option<String>,
) -> Option<String> {
    if OpenRgbManager::server_reachable(host, port) {
        log::info!(
            "auto-update OpenRGB: serveur déjà joignable sur {host}:{port}, vérification différée"
        );
        return None;
    }

    let (tag_name, download_url) = match check_latest_version() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("auto-update OpenRGB: vérification version échouée: {e:#}");
            return None;
        }
    };

    if !needs_update(&tag_name, saved_version) {
        return None;
    }
    log::info!("auto-update OpenRGB: nouvelle version détectée ({tag_name})");

    let appdata = match std::env::var_os("APPDATA") {
        Some(a) => PathBuf::from(a).join("PureRGB"),
        None => {
            log::warn!("auto-update OpenRGB: APPDATA introuvable");
            return None;
        }
    };
    let staging = appdata.join("openrgb_update_staging");
    let _ = std::fs::remove_dir_all(&staging);
    if let Err(e) = std::fs::create_dir_all(&staging) {
        log::warn!("auto-update OpenRGB: création staging échouée: {e:#}");
        return None;
    }

    if let Err(e) = download_and_extract(&download_url, &staging) {
        log::warn!("auto-update OpenRGB: téléchargement/extraction échoués: {e:#}");
        let _ = std::fs::remove_dir_all(&staging);
        return None;
    }

    // Laisse une chance à resource_dir() de se peupler (hook .setup() de
    // Tauri, concurrent à ce thread) avant de chercher les emplacements à
    // mettre à jour — sinon la copie setup NSIS pourrait être ratée en
    // silence sur ce lancement (se rattrape au suivant, mais autant éviter).
    let _ = wait_for_resource_dir(mgr, Duration::from_secs(3));

    let targets = update_targets(mgr, &appdata);
    if targets.is_empty() {
        log::info!("auto-update OpenRGB: aucune copie gérée par PureRGB trouvée, rien à faire");
        let _ = std::fs::remove_dir_all(&staging);
        return None;
    }

    let resource_dir = mgr.resource_dir();
    let mut backups: Vec<(PathBuf, PathBuf)> = Vec::new();
    for target in &targets {
        let backup = target.with_file_name(format!(
            "{}_backup",
            target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "openrgb".to_string())
        ));
        let _ = std::fs::remove_dir_all(&backup);
        if target.is_dir() {
            if let Err(e) = std::fs::rename(target, &backup) {
                log::warn!("auto-update OpenRGB: sauvegarde {} échouée: {e:#}", target.display());
                restore_backups(&backups);
                let _ = std::fs::remove_dir_all(&staging);
                return None;
            }
        }
        backups.push((backup.clone(), target.clone()));
        if let Err(e) = copy_dir_all(&staging, target) {
            log::warn!(
                "auto-update OpenRGB: copie nouvelle version vers {} échouée: {e:#}",
                target.display()
            );
            restore_backups(&backups);
            let _ = std::fs::remove_dir_all(&staging);
            return None;
        }
        copy_vc_runtime_dlls(target, &resource_dir);
    }

    mgr.stop();
    let ok = match mgr.ensure_running(host, port) {
        Ok(_) => wait_for_port(host, port, Duration::from_secs(10)),
        Err(e) => {
            log::warn!("auto-update OpenRGB: relance après bascule échouée: {e:#}");
            false
        }
    };

    let _ = std::fs::remove_dir_all(&staging);

    if ok {
        for (backup, _) in &backups {
            let _ = std::fs::remove_dir_all(backup);
        }
        log::info!("auto-update OpenRGB: bascule vers {tag_name} réussie");
        Some(tag_name)
    } else {
        log::warn!("auto-update OpenRGB: la nouvelle version ne démarre pas, restauration");
        mgr.stop();
        restore_backups(&backups);
        if let Err(e) = mgr.ensure_running(host, port) {
            log::error!("auto-update OpenRGB: relance après restauration échouée: {e:#}");
        }
        None
    }
}

/// Télécharge l'asset vers `staging/openrgb_update.zip`, l'extrait, puis
/// aplatit le sous-dossier "OpenRGB Windows 64-bit" du zip (même structure
/// que `fetch-openrgb.ps1`/`OpenRgbManager::install()`). HTTPS seul comme
/// garantie (pas de checksum officiel publié par OpenRGB).
/// Lance `powershell.exe` borné par un timeout global — `-TimeoutSec` sur
/// Invoke-WebRequest ne couvre que la requête HTTP, jamais `Expand-Archive`
/// ni un blocage du process lui-même (zip corrompu/énorme, quirk OS) : sans
/// cette borne, un script qui ne rend jamais la main gèlerait `hw-init`
/// indéfiniment (violerait l'invariant "jamais bloquant" de l'auto-update).
fn run_powershell_with_timeout(script: &str, timeout: Duration) -> Result<std::process::Output> {
    use std::process::Stdio;
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags_no_window()
        .spawn()
        .context("lancement PowerShell (auto-update)")?;

    let start = Instant::now();
    loop {
        if child.try_wait().context("attente PowerShell (auto-update)")?.is_some() {
            break;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!("PowerShell (auto-update) n'a pas terminé sous {timeout:?}, processus arrêté");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    child
        .wait_with_output()
        .context("récupération sortie PowerShell (auto-update)")
}

fn download_and_extract(url: &str, staging: &Path) -> Result<()> {
    let zip_path = staging.join("openrgb_update.zip");
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -Uri '{url}' -OutFile '{zip}' -UseBasicParsing -TimeoutSec 20; \
         Expand-Archive '{zip}' '{dir}' -Force; \
         Remove-Item '{zip}' -Force",
        url = url,
        zip = zip_path.display(),
        dir = staging.display(),
    );
    let output = run_powershell_with_timeout(&script, Duration::from_secs(60))
        .context("téléchargement OpenRGB (auto-update)")?;
    if !output.status.success() {
        bail!(
            "téléchargement OpenRGB (auto-update) échoué: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let nested = staging.join("OpenRGB Windows 64-bit");
    if nested.is_dir() {
        for entry in std::fs::read_dir(&nested)? {
            let entry = entry?;
            let dest = staging.join(entry.file_name());
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
    if !staging.join("OpenRGB.exe").is_file() {
        bail!("OpenRGB.exe absent après extraction (auto-update)");
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Le zip OpenRGB ne contient pas les DLL runtime VC++ app-local : sans
/// elles, OpenRGB (Qt/MSVC) reste vivant mais mort-né (aucun port). Même
/// logique de sourcing que `OpenRgbManager::install()`.
fn copy_vc_runtime_dlls(target: &Path, resource_dir: &Option<PathBuf>) {
    let mut dll_sources: Vec<PathBuf> = Vec::new();
    if let Some(win) = std::env::var_os("WINDIR") {
        dll_sources.push(PathBuf::from(win).join("System32"));
    }
    if let Some(res) = resource_dir {
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
                    log::warn!("auto-update OpenRGB: copie {dll}: {e}");
                }
            }
            None => log::warn!(
                "auto-update OpenRGB: {dll} introuvable — le serveur basculé pourrait ne pas démarrer"
            ),
        }
    }
}

/// Emplacements OpenRGB gérés par PureRGB (jamais une install indépendante
/// de l'utilisateur) : bundle NSIS si présent, `%APPDATA%\PureRGB\openrgb`
/// si présent. Chacun validé par la présence de `PawnIOLib.dll` (marqueur
/// 1.0rc, même critère que `OpenRgbManager::locate()`).
fn update_targets(mgr: &OpenRgbManager, appdata: &Path) -> Vec<PathBuf> {
    let ours_ok = |dir: &Path| dir.join("OpenRGB.exe").is_file() && dir.join("PawnIOLib.dll").is_file();
    let mut targets = Vec::new();
    if let Some(res) = mgr.resource_dir() {
        for base in [res.join("resources"), res] {
            let dir = base.join("openrgb");
            if ours_ok(&dir) {
                targets.push(dir);
                break;
            }
        }
    }
    let app_dir = appdata.join("openrgb");
    if ours_ok(&app_dir) {
        targets.push(app_dir);
    }
    targets
}

fn restore_backups(backups: &[(PathBuf, PathBuf)]) {
    for (backup, target) in backups {
        if let Err(e) = std::fs::remove_dir_all(target) {
            if target.exists() {
                log::warn!(
                    "auto-update OpenRGB: restauration {} — suppression échouée: {e}",
                    target.display()
                );
            }
        }
        if let Err(e) = std::fs::rename(backup, target) {
            log::warn!(
                "auto-update OpenRGB: restauration {} échouée, sauvegarde restée en {}: {e}",
                target.display(),
                backup.display()
            );
        }
    }
}

fn wait_for_port(host: &str, port: u16, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if OpenRgbManager::server_reachable(host, port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

/// `resource_dir` n'est peuplé que par le hook `.setup()` de Tauri, qui ne
/// s'exécute qu'au tout dernier appel `.run()` de `lib.rs::run()` — le
/// thread `hw-init` (qui appelle `update_if_needed` en tout premier) peut
/// donc démarrer avant que `resource_dir` soit renseigné. Attente bornée
/// pour éviter de rater silencieusement la copie installée par le setup
/// NSIS lors de la bascule (l'auto-update doit mettre à jour les deux
/// emplacements gérés par PureRGB, pas seulement APPDATA).
fn wait_for_resource_dir(mgr: &OpenRgbManager, timeout: Duration) -> Option<PathBuf> {
    let start = Instant::now();
    loop {
        if let Some(dir) = mgr.resource_dir() {
            return Some(dir);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_powershell_with_timeout_returns_success_for_fast_script() {
        let result = run_powershell_with_timeout("exit 0", Duration::from_secs(10));
        assert!(result.is_ok());
        assert!(result.unwrap().status.success());
    }

    #[test]
    fn run_powershell_with_timeout_kills_and_errors_on_hang() {
        let start = Instant::now();
        let result = run_powershell_with_timeout("Start-Sleep -Seconds 60", Duration::from_secs(2));
        assert!(result.is_err());
        // Doit revenir peu après le timeout, pas attendre les 60s du script.
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn pick_windows_asset_selects_zip_by_pattern() {
        let assets = vec![
            ("OpenRGB_1.0rc3_Windows_64_6fbcf62.zip".to_string(), "https://a/zip".to_string()),
            ("OpenRGB_1.0rc3_Windows_64_6fbcf62.msi".to_string(), "https://a/msi".to_string()),
            ("OpenRGB_1.0rc3_Source_Code.zip".to_string(), "https://a/src".to_string()),
        ];
        let picked = pick_windows_asset(&assets);
        assert_eq!(
            picked,
            Some((
                "OpenRGB_1.0rc3_Windows_64_6fbcf62.zip".to_string(),
                "https://a/zip".to_string()
            ))
        );
    }

    #[test]
    fn pick_windows_asset_handles_varying_commit_hash() {
        let assets = vec![(
            "OpenRGB_1.0rc4_Windows_64_a1b2c3d.zip".to_string(),
            "https://a/other".to_string(),
        )];
        assert!(pick_windows_asset(&assets).is_some());
    }

    #[test]
    fn pick_windows_asset_none_when_missing() {
        let assets = vec![("OpenRGB_1.0rc3_Linux_x64.tar.gz".to_string(), "https://a/lin".to_string())];
        assert_eq!(pick_windows_asset(&assets), None);
    }

    #[test]
    fn needs_update_true_when_different() {
        assert!(needs_update("release_candidate_1.0rc4", &Some("release_candidate_1.0rc3".to_string())));
    }

    #[test]
    fn needs_update_false_when_same() {
        assert!(!needs_update("release_candidate_1.0rc3", &Some("release_candidate_1.0rc3".to_string())));
    }

    #[test]
    fn needs_update_true_when_never_checked() {
        assert!(needs_update("release_candidate_1.0rc3", &None));
    }
}
