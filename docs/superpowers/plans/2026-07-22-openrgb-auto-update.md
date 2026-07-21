# PureRGB — Auto-update OpenRGB embarqué — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** PureRGB détecte au lancement si une nouvelle version d'OpenRGB est publiée sur Codeberg, la télécharge, remplace silencieusement la copie embarquée (bundle NSIS et/ou `%APPDATA%\PureRGB\openrgb`), et revient en arrière automatiquement si la nouvelle version ne démarre pas.

**Architecture:** Nouveau module `src-tauri/src/backends/openrgb/updater.rs` avec deux fonctions pures testables (`pick_windows_asset`, `needs_update`) et une fonction d'orchestration (`update_if_needed`) qui télécharge vers un dossier de staging, bascule chaque emplacement géré par PureRGB avec backup/rollback, valide en relançant `OpenRgbManager::ensure_running`, et ne touche jamais un serveur déjà joignable (install indépendante de l'utilisateur). Appelé en tout début du thread `hw-init` dans `lib.rs`, avant la logique existante.

**Tech Stack:** Rust, `anyhow`, `serde_json` (déjà en dépendances), `curl.exe` via `crate::netdev::curl` (pas de nouvelle dépendance HTTP), PowerShell (`Invoke-WebRequest`/`Expand-Archive`) pour le téléchargement, mêmes patterns que `OpenRgbManager::install()`.

**Spec source:** `docs/superpowers/specs/2026-07-22-openrgb-auto-update-design.md`

**Note d'implémentation (précision ajoutée par ce plan, cohérente avec la spec) :** la spec ne précise pas la signature exacte d'`update_if_needed` — `OpenRgbManager::ensure_running`/`server_reachable` exigent déjà `(host, port)` explicites (le manager ne les stocke pas). Signature retenue : `update_if_needed(mgr: &OpenRgbManager, host: &str, port: u16, saved_version: &Option<String>) -> Option<String>`. Ajout de sécurité conforme à l'invariant déjà énoncé dans la spec ("l'auto-update ne touche que les copies que PureRGB gère lui-même") : si un serveur répond déjà sur `host:port` au moment de l'appel (ne peut arriver qu'avant le premier `ensure_running` du thread, donc seulement si un OpenRGB indépendant de l'utilisateur tourne déjà), la mise à jour est intégralement sautée cette session — jamais de bascule de fichiers ni de redémarrage d'un process qu'on ne contrôle pas.

---

### Task 1: Exposer `resource_dir()` et rendre `CreationFlagsExt` réutilisable

**Files:**
- Modify: `src-tauri/src/backends/openrgb/manager.rs:60-66` (ajout d'un getter)
- Modify: `src-tauri/src/backends/openrgb/manager.rs:352` (visibilité du trait)

- [ ] **Step 1: Ajouter un getter `resource_dir()` sur `OpenRgbManager`**

Dans `manager.rs`, juste après `set_resource_dir` (ligne 60-62) :

```rust
    pub fn set_resource_dir(&self, dir: PathBuf) {
        *self.resource_dir.lock() = Some(dir);
    }

    /// Dossier ressources du bundle, si résolu (utilisé par l'auto-update
    /// pour retrouver l'emplacement setup NSIS en plus d'APPDATA).
    pub(crate) fn resource_dir(&self) -> Option<PathBuf> {
        self.resource_dir.lock().clone()
    }
```

- [ ] **Step 2: Rendre `CreationFlagsExt` réutilisable par `updater.rs`**

Ligne 352, changer :

```rust
trait CreationFlagsExt {
```

en :

```rust
pub(crate) trait CreationFlagsExt {
```

- [ ] **Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès, aucune nouvelle erreur (le trait et le getter ne sont pas encore utilisés ailleurs, un warning "unused" est acceptable temporairement — il disparaîtra dès la Task 5).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/backends/openrgb/manager.rs
git commit -m "refactor(openrgb): expose resource_dir() and CreationFlagsExt for updater module"
```

---

### Task 2: Champ `openrgb_version` dans `Settings`

**Files:**
- Modify: `src-tauri/src/settings.rs:47` (champ)
- Modify: `src-tauri/src/settings.rs:77` (Default)

- [ ] **Step 1: Ajouter le champ**

Après `pub telemetry_opt_in: bool,` (ligne 47) :

```rust
    /// Envoie un snapshot diagnostic matériel (VID/PID, état
    /// liquidctl/sensord/OpenRGB) à un service opt-in pour aider à
    /// identifier le matériel non reconnu. Aucune donnée personnelle.
    pub telemetry_opt_in: bool,
    /// Tag de la version d'OpenRGB embarquée actuellement installée
    /// (`None` = jamais vérifié, traite toute version distante comme
    /// nouvelle). Mis à jour par `backends::openrgb::updater`.
    pub openrgb_version: Option<String>,
}
```

- [ ] **Step 2: Ajouter la valeur par défaut**

Après `telemetry_opt_in: false,` (ligne 77) :

```rust
            telemetry_opt_in: false,
            openrgb_version: None,
        }
    }
}
```

- [ ] **Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès (`#[serde(default)]` sur `Settings` gère la rétrocompatibilité des `settings.json` existants sans le champ).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat(settings): add openrgb_version field for auto-update tracking"
```

---

### Task 3: Fonctions pures testables (`pick_windows_asset`, `needs_update`)

**Files:**
- Create: `src-tauri/src/backends/openrgb/updater.rs`

- [ ] **Step 1: Écrire les tests d'abord**

Créer `src-tauri/src/backends/openrgb/updater.rs` avec :

```rust
//! Auto-update de l'OpenRGB embarqué : vérifie la dernière release Codeberg
//! au lancement, télécharge et bascule les copies gérées par PureRGB
//! (bundle NSIS, `%APPDATA%\PureRGB\openrgb`) avec rollback automatique si
//! la nouvelle version ne démarre pas. Ne touche jamais une installation
//! OpenRGB indépendante de l'utilisateur.

#[cfg(test)]
mod tests {
    use super::*;

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
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent (fonctions absentes)**

Run: `cd src-tauri && cargo test --lib updater::`
Expected: FAIL avec `cannot find function 'pick_windows_asset'` (et `needs_update`) — les fonctions n'existent pas encore.

- [ ] **Step 3: Implémenter les deux fonctions**

Ajouter avant le `#[cfg(test)] mod tests` :

```rust
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
```

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cd src-tauri && cargo test --lib updater::`
Expected: PASS, 6/6 tests verts.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/backends/openrgb/updater.rs
git commit -m "feat(openrgb): add asset-selection and version-comparison logic for auto-update"
```

---

### Task 4: `check_latest_version()` — appel API Codeberg

**Files:**
- Modify: `src-tauri/src/backends/openrgb/updater.rs`

- [ ] **Step 1: Ajouter les imports et la constante**

En tête de fichier, avant le doc-comment existant reste, ajouter juste après :

```rust
use anyhow::{bail, Context, Result};

const RELEASES_API: &str =
    "https://codeberg.org/api/v1/repos/OpenRGB/OpenRGB/releases?limit=1";
```

- [ ] **Step 2: Implémenter `check_latest_version()`**

Ajouter après `needs_update` :

```rust
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
```

- [ ] **Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès.

- [ ] **Step 4: Vérification manuelle (pas unitaire — dépend du réseau)**

Ajouter temporairement dans `main()` ou un test ignoré `#[ignore]` un appel à `check_latest_version()` et logguer le résultat, ou vérifier directement en PowerShell :

Run: `curl.exe -s "https://codeberg.org/api/v1/repos/OpenRGB/OpenRGB/releases?limit=1"`
Expected: JSON contenant `"tag_name"` et un tableau `"assets"` avec un `.zip` `Windows_64`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/backends/openrgb/updater.rs
git commit -m "feat(openrgb): fetch latest OpenRGB release from Codeberg API"
```

---

### Task 5: Helpers de bascule (téléchargement, copie, DLL, rollback)

**Files:**
- Modify: `src-tauri/src/backends/openrgb/updater.rs`

- [ ] **Step 1: Imports supplémentaires**

Compléter la ligne d'imports :

```rust
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::manager::{CreationFlagsExt, OpenRgbManager};
```

- [ ] **Step 2: `download_and_extract` — télécharge et aplatit le zip vers le staging**

```rust
/// Télécharge l'asset vers `staging/openrgb_update.zip`, l'extrait, puis
/// aplatit le sous-dossier "OpenRGB Windows 64-bit" du zip (même structure
/// que `fetch-openrgb.ps1`/`OpenRgbManager::install()`). HTTPS seul comme
/// garantie (pas de checksum officiel publié par OpenRGB).
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
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags_no_window()
        .output()
        .context("lancement PowerShell pour téléchargement OpenRGB (auto-update)")?;
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
```

- [ ] **Step 3: `copy_dir_all` — copie récursive staging vers cible**

```rust
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
```

- [ ] **Step 4: `copy_vc_runtime_dlls` — même logique que `OpenRgbManager::install()`**

```rust
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
```

- [ ] **Step 5: `update_targets` — emplacements gérés par PureRGB (bundle + APPDATA)**

```rust
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
```

- [ ] **Step 6: `restore_backups` et `wait_for_port`**

```rust
fn restore_backups(backups: &[(PathBuf, PathBuf)]) {
    for (backup, target) in backups {
        let _ = std::fs::remove_dir_all(target);
        let _ = std::fs::rename(backup, target);
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
```

- [ ] **Step 7: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès (warnings "fonction jamais utilisée" acceptables — assemblées en Task 6).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/backends/openrgb/updater.rs
git commit -m "feat(openrgb): add download/copy/rollback helpers for auto-update"
```

---

### Task 6: `update_if_needed()` — orchestration complète

**Files:**
- Modify: `src-tauri/src/backends/openrgb/updater.rs`

- [ ] **Step 1: Implémenter l'orchestration**

Ajouter après `check_latest_version` (ou en fin de fichier, avant `#[cfg(test)]`) :

```rust
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
            target.file_name().unwrap().to_string_lossy()
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
        backups.push((backup, target.clone()));
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
        let _ = mgr.ensure_running(host, port);
        None
    }
}
```

- [ ] **Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès, plus aucun warning "fonction jamais utilisée" pour les helpers de la Task 5.

- [ ] **Step 3: Lancer toute la suite de tests du module**

Run: `cd src-tauri && cargo test --lib updater::`
Expected: PASS, 6/6 (les tests réseau/bascule ne sont pas unitaires — cf. Step 4).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/backends/openrgb/updater.rs
git commit -m "feat(openrgb): implement update_if_needed orchestration with rollback"
```

---

### Task 7: Déclarer le module

**Files:**
- Modify: `src-tauri/src/backends/openrgb/mod.rs:4-5`

- [ ] **Step 1: Ajouter la déclaration**

```rust
pub mod manager;
pub mod protocol;
pub mod updater;
```

- [ ] **Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/backends/openrgb/mod.rs
git commit -m "feat(openrgb): register updater module"
```

---

### Task 8: Câblage dans le thread `hw-init`

**Files:**
- Modify: `src-tauri/src/lib.rs:891-899` (capture `saved` en `mut`)
- Modify: `src-tauri/src/lib.rs:895-899` (insertion avant `auto_manage_conflicts`)
- Modify: `src-tauri/src/lib.rs` imports (ajout `settings`)

- [ ] **Step 1: Vérifier l'import de `settings`**

`lib.rs:23` importe déjà `use crate::settings::Settings;`. Le module `settings` lui-même (pour appeler `settings::save`) doit être accessible par chemin complet `crate::settings::save` — pas d'import supplémentaire nécessaire si `mod settings;` existe déjà en tête de fichier (à vérifier : `grep -n "^mod settings"  src-tauri/src/lib.rs` — si absent, l'ajouter à côté de `mod backends;` ligne 1).

Run: `grep -n "mod settings" src-tauri/src/lib.rs`
Expected: une ligne `mod settings;` (ou équivalent) déjà présente — sinon l'ajouter juste après `mod backends;` (ligne 1) :

```rust
mod backends;
mod settings;
```

(Ne pas dupliquer si déjà présent.)

- [ ] **Step 2: Rendre `saved` mutable dans le thread `hw-init`**

Dans `lib.rs`, bloc du thread `hw-init` (repérer par `.name("hw-init".into())`), ligne juste avant le `std::thread::Builder::new()` :

```rust
        let registry = registry.clone();
        let engine = engine.clone();
        let mgr = openrgb_mgr.clone();
        let saved = saved.clone();
        let auto_stopped = auto_stopped.clone();
```

Changer la ligne `let saved = saved.clone();` en :

```rust
        let mut saved = saved.clone();
```

- [ ] **Step 3: Insérer le check auto-update tout en haut du corps du thread**

Le corps du thread commence actuellement par :

```rust
            .spawn(move || {
                // Arrêt auto des logiciels constructeur en conflit AVANT le scan
                // matériel : libère les handles HID pour qu'OpenRGB détecte tout.
                // Réversible (disable=false), redémarrés au "Quitter" du tray.
                if saved.auto_manage_conflicts {
```

Remplacer par :

```rust
            .spawn(move || {
                // Auto-update OpenRGB AVANT tout le reste : doit précéder
                // ensure_running/scan/restore_saved_state pour que le scan
                // matériel se fasse contre la version définitive de la
                // session, jamais contre une version sur le point d'être
                // remplacée. Ne touche jamais un serveur déjà joignable
                // (install OpenRGB indépendante de l'utilisateur).
                if saved.auto_start_openrgb {
                    if let Some(new_version) = crate::backends::openrgb::updater::update_if_needed(
                        &mgr,
                        &saved.openrgb_host,
                        saved.openrgb_port,
                        &saved.openrgb_version,
                    ) {
                        saved.openrgb_version = Some(new_version);
                        if let Err(e) = crate::settings::save(&saved) {
                            log::warn!("sauvegarde version OpenRGB: {e:#}");
                        }
                    }
                }
                // Arrêt auto des logiciels constructeur en conflit AVANT le scan
                // matériel : libère les handles HID pour qu'OpenRGB détecte tout.
                // Réversible (disable=false), redémarrés au "Quitter" du tray.
                if saved.auto_manage_conflicts {
```

- [ ] **Step 4: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès. Si erreur `cannot borrow 'mgr' as ...` (mgr est `Arc<OpenRgbManager>`, la fonction attend `&OpenRgbManager`) : la coercion Deref s'applique automatiquement pour `&mgr` — si le compilateur se plaint malgré tout, remplacer `&mgr` par `mgr.as_ref()` dans l'appel.

- [ ] **Step 5: Build complet**

Run: `cd src-tauri && cargo build`
Expected: succès.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(openrgb): run auto-update check before hardware scan in hw-init"
```

---

### Task 9: Vérification manuelle du flux réel (non testable unitairement)

**Files:** aucun (vérification uniquement)

- [ ] **Step 1: Build de dev complet**

Run: `cd src-tauri && cargo build` puis `npm run build` (racine du repo)
Expected: les deux verts.

- [ ] **Step 2: Test manuel de la bascule**

1. Lancer l'app une première fois normalement pour avoir une copie OpenRGB installée (`%APPDATA%\PureRGB\openrgb` ou bundle NSIS selon le mode de lancement).
2. Fermer l'app.
3. Éditer `%APPDATA%\PureRGB\settings.json`, mettre `"openrgb_version": "version_perimee_test"`.
4. Relancer l'app (`npm run tauri dev` ou l'exe buildé), observer les logs (`.logs/` ou stdout selon config `env_logger`).

Expected dans les logs : `auto-update OpenRGB: nouvelle version détectée (release_candidate_1.0rc3)` (ou plus récent si Codeberg a publié depuis), puis `auto-update OpenRGB: bascule vers ... réussie`. Le serveur OpenRGB démarre normalement ensuite (mêmes appareils détectés qu'avant).
5. Revérifier `settings.json` : `openrgb_version` doit contenir le tag fraîchement basculé.
6. Relancer l'app une seconde fois : aucune re-bascule ne doit avoir lieu (log absent ou `needs_update` retourne `false` silencieusement — c'est le comportement normal, pas de log dans ce cas).

- [ ] **Step 3: Test manuel du rollback (optionnel mais recommandé)**

Simuler un échec : renommer temporairement `OpenRGB.exe` en `OpenRGB.exe.bak` dans le dossier de staging avant l'étape de validation n'est pas pratique à déclencher manuellement sans modifier le code — à défaut, vérifier par lecture de code que `wait_for_port` avec `false` déclenche bien `restore_backups` (relecture Task 6, Step 1 déjà fait). Si un doute subsiste, noter comme limite de vérification manuelle, à valider par Momo en conditions réelles au prochain vrai cycle de release OpenRGB.

- [ ] **Step 4: Ne pas committer `settings.json`**

`settings.json` est dans `%APPDATA%`, hors du repo — rien à nettoyer côté git.

---

### Task 10: Bump de version

**Files:**
- Modify: `src-tauri/Cargo.toml:3`
- Modify: `package.json:4`
- Modify: `src-tauri/tauri.conf.json:4`

- [ ] **Step 1: Bump 0.15.0 → 0.16.0 dans les trois fichiers**

`src-tauri/Cargo.toml:3` : `version = "0.16.0"`
`package.json:4` : `"version": "0.16.0",`
`src-tauri/tauri.conf.json:4` : `"version": "0.16.0",`

- [ ] **Step 2: Vérifier la compilation finale**

Run: `cd src-tauri && cargo check` puis `npm run build`
Expected: les deux verts.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml package.json src-tauri/tauri.conf.json
git commit -m "chore: bump version to 0.16.0"
```
