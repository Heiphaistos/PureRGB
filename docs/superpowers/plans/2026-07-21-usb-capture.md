# Capture USB intégrée — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Intégrer dans le panneau Diagnostic de PureRGB un bouton "Capturer le protocole USB" par appareil non reconnu — installe USBPcap à la demande, capture tous les hubs racine en parallèle, laisse l'utilisateur envoyer les fichiers vers un nouvel endpoint du service télémétrie pour analyse manuelle ultérieure.

**Architecture:** Deux dépôts. (1) `PureRGB` (Tauri/Rust + Vue) : nouveau module `usbcapture.rs` (installation USBPcap signée SHA-256 pinné, énumération des hubs racine, spawn/kill de `USBPcapCMD.exe` par hub, upload via `curl.exe` déjà utilisé ailleurs) + 5 commandes Tauri + UI dans `SettingsPanel.vue`. (2) `PureRGB-Telemetry` (Hono) : nouvelle table `capture_uploads`, endpoint `POST /capture-upload` protégé par un token statique partagé (pas le login dashboard — c'est l'app qui pousse, pas un navigateur), page dashboard listant/téléchargeant les captures reçues.

**Tech Stack:** Rust (aucune nouvelle dépendance Cargo), Vue 3/TypeScript, Hono/better-sqlite3 (aucune nouvelle dépendance npm — `crypto.randomUUID()`/`crypto.timingSafeEqual` sont natifs Node).

---

## Repères de contexte (déjà vérifiés, ne pas re-découvrir)

- **USBPcap 1.5.4.0** — URL et SHA-256 réels, vérifiés en téléchargeant le fichier cette session (pas supposés) :
  `https://github.com/desowin/usbpcap/releases/download/1.5.4.0/USBPcapSetup-1.5.4.0.exe`
  SHA-256 : `87a7edf9bbbcf07b5f4373d9a192a6770d2ff3add7aa1e276e82e38582ccb622`
- Licences : `USBPcapDriver` GPLv2 (binaire tiers non modifié, même traitement qu'OpenRGB dans ce projet), `USBPcapCMD` BSD 2-Clause. Release signée par un vrai certificat commercial — pas besoin d'activer `TESTSIGNING`.
- **`USBPcapCMD.exe` filtre uniquement par hub racine** (`\\.\USBPcapN`), jamais par appareil précis (confirmé : issue GitHub desowin/usbpcap#64). Décision de conception : capturer **tous** les hubs racine détectés en parallèle, filtrer par VID/PID après coup à l'analyse (pas de corrélation appareil→hub côté app).
- Syntaxe CLI confirmée (tour officiel desowin.org) : `USBPcapCMD.exe -d \\.\USBPcap2 -o sortie.pcap` démarre une capture ; tuer le process l'arrête proprement (ferme le fichier).
- **Simplification retenue par rapport à la spec** : l'arrêt automatique à 5 minutes ("anti-oubli") est géré côté **frontend** (timer JS dans `SettingsPanel.vue`, appelle `usb_capture_stop` tout seul après 300s), pas par un thread Rust séparé — évite un état partagé/thread de fond supplémentaire pour un besoin que le frontend peut satisfaire seul puisque la fenêtre de l'app doit de toute façon rester ouverte pendant la capture. Le module Rust n'a donc pas besoin de suivre le temps écoulé lui-même.
- `crate::netdev::curl(args: &[&str]) -> Result<String>` existe déjà (`pub(crate)`, réutilisé par `telemetry.rs`) — invoque `curl.exe` directement via `Command::new` (pas de shell, donc pas de risque d'injection même avec des noms d'appareil contenant des espaces/caractères spéciaux).
- Le token upload ne doit **jamais** être stocké dans `%APPDATA%\PureRGB\settings.json`, ni exposé par une commande Tauri lisible du frontend, ni **jamais écrit en clair dans un fichier suivi par git** (`PureRGB` est un repo **public** — contrairement à `TELEMETRY_BASE_URL`, qui n'est pas un secret, ce token gate l'upload et doit rester réellement secret). Injecté à la compilation via `option_env!("CAPTURE_UPLOAD_TOKEN")`, lu depuis un secret GitHub Actions (`gh secret set CAPTURE_UPLOAD_TOKEN --repo Heiphaistos/PureRGB`) au moment du build CI, jamais commité tel quel.
- **Token upload réel déjà généré et posé en secret GitHub Actions cette session** (ne pas en régénérer un autre, ne jamais le réécrire en clair dans un fichier — le récupérer via `gh secret list --repo Heiphaistos/PureRGB` pour confirmer sa présence, jamais sa valeur, `gh` ne l'affiche jamais en clair une fois posé).
- `AppState` actuel (`src-tauri/src/lib.rs:29-39`) : `registry, engine, settings: Mutex<Settings>, openrgb_mgr, sensors, curve_engine, auto_stopped`.
- `generate_handler![...]` se termine actuellement par `hardware_diagnostics, send_telemetry_report` (`src-tauri/src/lib.rs:1087-1088`).
- Table HID brute dans `SettingsPanel.vue` (~ligne 351-361) : une ligne par `RawHidDevice` (`vid, pid, manufacturer, product, recognized, has_native_driver`), colonne "État" affichant "non reconnu" quand `!d.recognized`.
- Volume Docker persistant du service VPS : `/app/data` (déjà utilisé pour `telemetry.db`) — les captures iront dans `/app/data/captures/`.
- Spec source : `docs/superpowers/specs/2026-07-21-usb-capture-design.md`.

---

## PARTIE A — App PureRGB (Rust + Vue)

### Task 1: Rendre `TELEMETRY_BASE_URL` réutilisable

**Files:**
- Modify: `src-tauri/src/telemetry.rs`

- [ ] **Step 1: Changer la visibilité de la constante**

Dans `src-tauri/src/telemetry.rs`, changer :
```rust
const TELEMETRY_BASE_URL: &str = "https://telemetry-purergb.heiphaistos.org";
```
en :
```rust
pub(crate) const TELEMETRY_BASE_URL: &str = "https://telemetry-purergb.heiphaistos.org";
```

- [ ] **Step 2: Vérifier la compilation**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check`
Expected: vert (aucun changement de comportement, juste la visibilité).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/telemetry.rs
git commit -m "refactor: make TELEMETRY_BASE_URL reusable by usbcapture module"
```

---

### Task 2: `usbcapture.rs` — installation USBPcap + énumération des hubs racine (TDD)

**Files:**
- Create: `src-tauri/src/usbcapture.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Créer `src-tauri/src/usbcapture.rs` avec les constantes + `root_hub_path` + son test**

```rust
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formate_le_chemin_du_hub_racine() {
        assert_eq!(root_hub_path(1), r"\\.\USBPcap1");
        assert_eq!(root_hub_path(8), r"\\.\USBPcap8");
    }
}
```

- [ ] **Step 2: Lancer le test**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo test usbcapture`
Expected: PASS (1 test) — `usbpcap_ready`/`enumerate_root_hubs` ne sont pas testés unitairement ici (dépendent du driver réellement installé sur la machine, vérifiés manuellement plus tard).

- [ ] **Step 3: Déclarer le module dans `src-tauri/src/lib.rs`**

Après `mod telemetry;` (ordre alphabétique) :
```rust
mod telemetry;
mod usbcapture;
```

- [ ] **Step 4: Ajouter la fonction d'installation**

Ajouter dans `src-tauri/src/usbcapture.rs`, après `usbpcap_ready()` :
```rust
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
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -Uri '{url}' -OutFile '{exe}' -UseBasicParsing; \
         $h = (Get-FileHash '{exe}' -Algorithm SHA256).Hash.ToLower(); \
         if ($h -ne '{sha}') {{ Remove-Item '{exe}' -Force; throw \"hash mismatch: $h\" }}",
        url = USBPCAP_URL,
        exe = setup.display(),
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
    install_cmd.args(["-install", "-silent"]);
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
```

Ce pattern (`#[cfg(windows)] { use ...; cmd.creation_flags(...); }` local à chaque spawn, plutôt qu'un `use` global en tête de fichier) reproduit exactement celui déjà en place dans `netdev.rs:314-319` (`curl()`) — garde le crate type-checkable hors Windows même si l'app ne tourne qu'sur Windows en pratique. Ne pas dévier vers un import global unconditionnel.

- [ ] **Step 5: Compiler**

Run: `cargo check`
Expected: vert.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/usbcapture.rs src-tauri/src/lib.rs
git commit -m "feat: usbcapture module — USBPcap install and root hub enumeration"
```

---

### Task 3: `usbcapture.rs` — démarrage/arrêt de capture + filtrage des fichiers (TDD)

**Files:**
- Modify: `src-tauri/src/usbcapture.rs`

- [ ] **Step 1: Écrire le test de filtrage des fichiers de capture (échoue — fonction inexistante)**

Ajouter dans `src-tauri/src/usbcapture.rs`, dans le module `tests` existant :
```rust
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
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `cargo test usbcapture`
Expected: FAIL — `cannot find function collect_capture_files`

- [ ] **Step 3: Implémenter `CaptureFile`, `CaptureSession`, `collect_capture_files`, `start_capture`, `stop_capture`**

Ajouter dans `src-tauri/src/usbcapture.rs`, avant le module `tests` :
```rust
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
        let child = cmd
            .spawn()
            .with_context(|| format!("lancement capture hub {hub}"))?;
        children.push(child);
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
```

- [ ] **Step 4: Relancer le test**

Run: `cargo test usbcapture`
Expected: PASS (3 tests : `formate_le_chemin_du_hub_racine`, `ignore_les_fichiers_vides_et_trie_par_hub`, `dossier_absent_retourne_vide`).

- [ ] **Step 5: Compiler**

Run: `cargo check`
Expected: vert.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/usbcapture.rs
git commit -m "feat: start/stop capture across all root hubs, filter empty files"
```

---

### Task 4: `usbcapture.rs` — upload vers le service télémétrie

**Files:**
- Modify: `src-tauri/src/usbcapture.rs`

- [ ] **Step 1: Ajouter la fonction d'upload (token injecté à la compilation, jamais commité)**

Ajouter à la fin du fichier, après `stop_capture` :
```rust
/// Envoie un fichier de capture vers le service télémétrie pour analyse
/// manuelle. Best-effort, jamais automatique — appelé uniquement suite à
/// un clic explicite "Envoyer pour analyse" côté UI.
///
/// Le token vient de `CAPTURE_UPLOAD_TOKEN`, une variable d'environnement
/// résolue à la COMPILATION via `option_env!` — jamais un `const` en dur
/// dans le source, `PureRGB` étant un repo public. Injecté par un secret
/// GitHub Actions du même nom pendant le build CI ; vide en dev local si
/// la variable n'est pas définie, auquel cas la fonction refuse d'envoyer
/// plutôt que de poser un header d'auth vide.
pub fn upload_capture(path: &Path, vid: &str, pid: &str, device_name: &str) -> Result<()> {
    let token = option_env!("CAPTURE_UPLOAD_TOKEN").unwrap_or("");
    if token.is_empty() {
        bail!("CAPTURE_UPLOAD_TOKEN non configuré au build — impossible d'envoyer la capture");
    }
    let url = format!("{}/capture-upload", crate::telemetry::TELEMETRY_BASE_URL);
    let auth = format!("Authorization: Bearer {token}");
    let file_field = format!("file=@{}", path.display());
    crate::netdev::curl(&[
        "-X",
        "POST",
        "-H",
        &auth,
        "--max-time",
        "60",
        "-F",
        &format!("vid={vid}"),
        "-F",
        &format!("pid={pid}"),
        "-F",
        &format!("device_name={device_name}"),
        "-F",
        &file_field,
        &url,
    ])
    .map(|_| ())
}
```

- [ ] **Step 2: Compiler**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check`
Expected: vert.

- [ ] **Step 3: Lancer tous les tests du crate**

Run: `cargo test`
Expected: PASS, aucune régression (33 tests : 30 précédents + 3 nouveaux `usbcapture`).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/usbcapture.rs
git commit -m "feat: upload captured pcap files to telemetry service"
```

---

### Task 5: Câblage `lib.rs` — commandes Tauri

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Ajouter le champ à `AppState`**

Dans `src-tauri/src/lib.rs`, remplacer (~ligne 29-39) :
```rust
struct AppState {
    registry: SharedRegistry,
    engine: EffectsEngine,
    settings: Mutex<Settings>,
    openrgb_mgr: std::sync::Arc<OpenRgbManager>,
    sensors: std::sync::Arc<SensorHub>,
    curve_engine: std::sync::Arc<CurveEngine>,
    /// Familles de conflit arrêtées automatiquement au lancement cette
    /// session (auto_manage_conflicts) — à redémarrer à la fermeture.
    auto_stopped: std::sync::Arc<Mutex<Vec<String>>>,
}
```
par :
```rust
struct AppState {
    registry: SharedRegistry,
    engine: EffectsEngine,
    settings: Mutex<Settings>,
    openrgb_mgr: std::sync::Arc<OpenRgbManager>,
    sensors: std::sync::Arc<SensorHub>,
    curve_engine: std::sync::Arc<CurveEngine>,
    /// Familles de conflit arrêtées automatiquement au lancement cette
    /// session (auto_manage_conflicts) — à redémarrer à la fermeture.
    auto_stopped: std::sync::Arc<Mutex<Vec<String>>>,
    /// Capture USB en cours (panneau Diagnostic) — une seule à la fois.
    active_capture: Mutex<Option<usbcapture::CaptureSession>>,
}
```

- [ ] **Step 2: Initialiser le nouveau champ**

Remplacer (~ligne 920-928) :
```rust
    let state = AppState {
        registry,
        engine,
        settings: Mutex::new(saved),
        openrgb_mgr,
        sensors,
        curve_engine,
        auto_stopped,
    };
```
par :
```rust
    let state = AppState {
        registry,
        engine,
        settings: Mutex::new(saved),
        openrgb_mgr,
        sensors,
        curve_engine,
        auto_stopped,
        active_capture: Mutex::new(None),
    };
```

- [ ] **Step 3: Ajouter les commandes Tauri**

Ajouter avant la fonction `hardware_diagnostics` (ou n'importe où au niveau module, avant `generate_handler!`) :
```rust
#[derive(Serialize)]
struct CaptureFileInfo {
    path: String,
    hub: u32,
    size_bytes: u64,
}

#[tauri::command(async)]
fn usb_capture_ready() -> bool {
    usbcapture::usbpcap_ready()
}

#[tauri::command(async)]
fn usb_capture_install() -> Result<(), String> {
    usbcapture::usbpcap_install().map_err(|e| format!("{e:#}"))
}

#[tauri::command(async)]
fn usb_capture_start(state: State<AppState>) -> Result<(), String> {
    let mut active = state.active_capture.lock();
    if active.is_some() {
        return Err("Une capture est déjà en cours".into());
    }
    let session = usbcapture::start_capture().map_err(|e| format!("{e:#}"))?;
    *active = Some(session);
    Ok(())
}

#[tauri::command(async)]
fn usb_capture_stop(state: State<AppState>) -> Result<Vec<CaptureFileInfo>, String> {
    let session = {
        let mut active = state.active_capture.lock();
        active.take()
    };
    let Some(session) = session else {
        return Err("Aucune capture en cours".into());
    };
    let files = usbcapture::stop_capture(session);
    Ok(files
        .into_iter()
        .map(|f| CaptureFileInfo {
            path: f.path.display().to_string(),
            hub: f.hub,
            size_bytes: f.size_bytes,
        })
        .collect())
}

#[tauri::command(async)]
fn usb_capture_upload(vid: String, pid: String, device_name: String, path: String) -> Result<(), String> {
    usbcapture::upload_capture(std::path::Path::new(&path), &vid, &pid, &device_name)
        .map_err(|e| format!("{e:#}"))
}
```

- [ ] **Step 4: Enregistrer les commandes dans `generate_handler!`**

Remplacer (~ligne 1087-1088) :
```rust
            hardware_diagnostics,
            send_telemetry_report
        ])
```
par :
```rust
            hardware_diagnostics,
            send_telemetry_report,
            usb_capture_ready,
            usb_capture_install,
            usb_capture_start,
            usb_capture_stop,
            usb_capture_upload
        ])
```

- [ ] **Step 5: Compiler**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check`
Expected: vert.

- [ ] **Step 6: Lancer tous les tests**

Run: `cargo test`
Expected: PASS, 33/33, aucune régression.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: expose usb capture as tauri commands"
```

---

### Task 6: Types frontend

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Ajouter le type `CaptureFileInfo`**

Ajouter dans `src/types.ts`, après `export interface RawHidDevice { ... }` :
```typescript
export interface CaptureFileInfo {
  path: string;
  hub: number;
  size_bytes: number;
}
```

- [ ] **Step 2: Vérifier**

Run: `cd "C:\Users\Momo\Desktop\PureRGB" && npx vue-tsc --noEmit`
Expected: vert.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add CaptureFileInfo type"
```

---

### Task 7: UI — bouton capture + modal + écran récapitulatif

**Files:**
- Modify: `src/components/SettingsPanel.vue`

- [ ] **Step 1: Ajouter l'état réactif et les fonctions, après `sendTelemetryNow`**

```typescript
import type { CaptureFileInfo } from "../types";

const captureTargetDevice = ref<{ vid: string; pid: string; manufacturer: string; product: string } | null>(null);
const captureStep = ref<"idle" | "warning" | "installing" | "running" | "summary">("idle");
const captureFiles = ref<CaptureFileInfo[]>([]);
const captureMsg = ref("");
const captureStartedAt = ref(0);
const captureElapsed = ref(0);
let captureTimerHandle: ReturnType<typeof setInterval> | null = null;

const CAPTURE_MAX_SECONDS = 300;

function openCaptureWarning(d: { vid: string; pid: string; manufacturer: string; product: string }) {
  captureTargetDevice.value = d;
  captureStep.value = "warning";
  captureMsg.value = "";
}

function closeCaptureFlow() {
  if (captureTimerHandle) {
    clearInterval(captureTimerHandle);
    captureTimerHandle = null;
  }
  captureStep.value = "idle";
  captureTargetDevice.value = null;
  captureFiles.value = [];
}

async function beginCapture() {
  captureStep.value = "installing";
  captureMsg.value = "";
  try {
    const ready = await invoke<boolean>("usb_capture_ready");
    if (!ready) {
      await invoke("usb_capture_install");
    }
    await invoke("usb_capture_start");
    captureStep.value = "running";
    captureStartedAt.value = Date.now();
    captureElapsed.value = 0;
    captureTimerHandle = setInterval(() => {
      captureElapsed.value = Math.floor((Date.now() - captureStartedAt.value) / 1000);
      if (captureElapsed.value >= CAPTURE_MAX_SECONDS) {
        stopCapture();
      }
    }, 1000);
  } catch (e) {
    captureMsg.value = `Installation : ${e}`;
    captureStep.value = "warning";
  }
}

async function stopCapture() {
  if (captureTimerHandle) {
    clearInterval(captureTimerHandle);
    captureTimerHandle = null;
  }
  try {
    captureFiles.value = await invoke<CaptureFileInfo[]>("usb_capture_stop");
    captureStep.value = "summary";
  } catch (e) {
    captureMsg.value = `Arrêt : ${e}`;
  }
}

const captureUploading = ref(false);

async function uploadCaptureFiles() {
  if (!captureTargetDevice.value) return;
  captureUploading.value = true;
  captureMsg.value = "";
  try {
    for (const f of captureFiles.value) {
      await invoke("usb_capture_upload", {
        vid: captureTargetDevice.value.vid,
        pid: captureTargetDevice.value.pid,
        deviceName: captureTargetDevice.value.product || captureTargetDevice.value.manufacturer || "inconnu",
        path: f.path,
      });
    }
    captureMsg.value = "Fichiers envoyés.";
  } catch (e) {
    captureMsg.value = `Envoi : ${e}`;
  } finally {
    captureUploading.value = false;
  }
}
```

- [ ] **Step 2: Ajouter le bouton dans le tableau des périphériques HID bruts**

Remplacer (~ligne 351-361) :
```html
        <table class="diag-table hid-table">
          <tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>État</th></tr>
          <tr v-for="d in hidRows()" :key="`${d.vid}:${d.pid}`">
            <td>{{ d.vid }}:{{ d.pid }}</td>
            <td>{{ d.manufacturer || "—" }}</td>
            <td>{{ d.product || "—" }}</td>
            <td :class="{ ok: d.recognized, fail: !d.recognized }">
              {{ d.recognized ? (d.has_native_driver ? "driver natif" : "reconnu") : "non reconnu" }}
            </td>
          </tr>
        </table>
```
par :
```html
        <table class="diag-table hid-table">
          <tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>État</th><th></th></tr>
          <tr v-for="d in hidRows()" :key="`${d.vid}:${d.pid}`">
            <td>{{ d.vid }}:{{ d.pid }}</td>
            <td>{{ d.manufacturer || "—" }}</td>
            <td>{{ d.product || "—" }}</td>
            <td :class="{ ok: d.recognized, fail: !d.recognized }">
              {{ d.recognized ? (d.has_native_driver ? "driver natif" : "reconnu") : "non reconnu" }}
            </td>
            <td>
              <button v-if="!d.recognized" @click="openCaptureWarning(d)">Capturer le protocole USB</button>
            </td>
          </tr>
        </table>
```

- [ ] **Step 3: Ajouter les écrans du flux de capture, après le `</table>` du tableau HID (avant la fermeture `</div>` du `diag-out`)**

```html
        <div v-if="captureStep !== 'idle'" class="capture-modal">
          <div class="capture-modal-inner">
            <template v-if="captureStep === 'warning'">
              <h4>Capturer le protocole USB — {{ captureTargetDevice?.product || captureTargetDevice?.manufacturer }}</h4>
              <p class="hint">
                Cette capture enregistre TOUT le trafic USB de cet ordinateur pendant la
                fenêtre, pas seulement cet appareil — d'autres périphériques branchés sur
                le même port apparaîtront aussi. Si un clavier est branché, vos frappes
                peuvent être incluses dans la capture. <strong>Ne tapez rien de sensible</strong>
                (mots de passe, etc.) pendant que la capture est active. Le fichier reste
                en local — vous choisirez ensuite de l'envoyer ou non.
              </p>
              <p v-if="captureMsg" class="hint" style="color: #c00">{{ captureMsg }}</p>
              <div class="inline" style="gap: 10px">
                <button @click="beginCapture">Démarrer</button>
                <button @click="closeCaptureFlow">Annuler</button>
              </div>
            </template>
            <template v-else-if="captureStep === 'installing'">
              <p>Installation d'USBPcap si nécessaire…</p>
            </template>
            <template v-else-if="captureStep === 'running'">
              <h4>Capture en cours — {{ captureElapsed }}s / {{ CAPTURE_MAX_SECONDS }}s</h4>
              <p class="hint">
                Ouvrez maintenant le logiciel officiel de cet appareil et changez une
                couleur ou un effet, puis cliquez Arrêter.
              </p>
              <button @click="stopCapture">Arrêter</button>
            </template>
            <template v-else-if="captureStep === 'summary'">
              <h4>Capture terminée ({{ captureFiles.length }} fichier(s))</h4>
              <table class="diag-table">
                <tr><th>Hub</th><th>Taille</th></tr>
                <tr v-for="f in captureFiles" :key="f.path">
                  <td>{{ f.hub }}</td>
                  <td>{{ (f.size_bytes / 1024).toFixed(1) }} Ko</td>
                </tr>
              </table>
              <p v-if="captureFiles.length === 0" class="hint">
                Aucun trafic capturé — réessayez en changeant bien une couleur pendant la fenêtre.
              </p>
              <p v-if="captureMsg" class="hint">{{ captureMsg }}</p>
              <div class="inline" style="gap: 10px">
                <button :disabled="captureUploading || captureFiles.length === 0" @click="uploadCaptureFiles">
                  {{ captureUploading ? "Envoi…" : "Envoyer pour analyse" }}
                </button>
                <button @click="closeCaptureFlow">Garder en local seulement</button>
              </div>
            </template>
          </div>
        </div>
```

- [ ] **Step 4: Ajouter le style minimal du modal**

Ajouter dans le bloc `<style>` de `SettingsPanel.vue` (à la fin) :
```css
.capture-modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.capture-modal-inner {
  background: #1a1a1a;
  padding: 24px;
  border-radius: 8px;
  max-width: 480px;
  width: 90%;
}
```

- [ ] **Step 5: Build frontend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB" && npm run build`
Expected: `vue-tsc --noEmit && vite build` vert.

- [ ] **Step 6: Commit**

```bash
git add src/components/SettingsPanel.vue
git commit -m "feat: usb capture UI in diagnostic panel"
```

---

## PARTIE B — Service VPS (PureRGB-Telemetry)

### Task 8: Schéma DB — table `capture_uploads`

**Files:**
- Modify: `src/db/schema.sql`

- [ ] **Step 1: Ajouter la table**

Ajouter dans `src/db/schema.sql`, après la table `known_devices` :
```sql
CREATE TABLE IF NOT EXISTS capture_uploads (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  filename TEXT NOT NULL,
  vid TEXT NOT NULL,
  pid TEXT NOT NULL,
  device_name TEXT NOT NULL DEFAULT '',
  size_bytes INTEGER NOT NULL,
  uploaded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_capture_uploads_uploaded_at ON capture_uploads(uploaded_at);
```

- [ ] **Step 2: Vérifier**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npm test`
Expected: PASS, 15/15 (table nouvelle, `CREATE TABLE IF NOT EXISTS` s'applique dès le prochain `bootstrapDb`, aucune migration `ALTER` nécessaire puisque la table n'existe nulle part encore).

- [ ] **Step 3: Commit**

```bash
git add src/db/schema.sql
git commit -m "feat: capture_uploads table schema"
```

---

### Task 9: Stockage fichiers — `captureStorage.ts`

**Files:**
- Create: `src/captureStorage.ts`

- [ ] **Step 1: Implémenter**

```typescript
import { mkdirSync, writeFileSync, readFileSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))

export const CAPTURES_DIR = process.env.CAPTURES_DIR ?? join(__dirname, '../data/captures')

export function saveCaptureFile(filename: string, data: Buffer): void {
  mkdirSync(CAPTURES_DIR, { recursive: true })
  writeFileSync(join(CAPTURES_DIR, filename), data)
}

export function readCaptureFile(filename: string): Buffer {
  return readFileSync(join(CAPTURES_DIR, filename))
}
```

- [ ] **Step 2: Vérifier**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npx tsc --noEmit`
Expected: vert.

- [ ] **Step 3: Commit**

```bash
git add src/captureStorage.ts
git commit -m "feat: capture file storage helpers"
```

---

### Task 10: Comparaison de token à temps constant (TDD)

**Files:**
- Create: `src/utils/captureToken.ts`
- Create: `src/utils/captureToken.test.ts`

- [ ] **Step 1: Écrire le test (échoue — module inexistant)**

`src/utils/captureToken.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { timingSafeTokenMatch } from './captureToken.js'

test('accepte un token identique', () => {
  assert.equal(timingSafeTokenMatch('abc123', 'abc123'), true)
})

test('refuse un token différent', () => {
  assert.equal(timingSafeTokenMatch('abc123', 'xyz789'), false)
})

test('refuse si le token attendu est vide (non configuré)', () => {
  assert.equal(timingSafeTokenMatch('abc123', ''), false)
})

test('refuse une longueur différente sans planter', () => {
  assert.equal(timingSafeTokenMatch('short', 'a-much-longer-token-value'), false)
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npm test`
Expected: FAIL — `Cannot find module './captureToken.js'`

- [ ] **Step 3: Implémenter `src/utils/captureToken.ts`**

```typescript
import { timingSafeEqual } from 'crypto'

/** Compare deux tokens à temps constant — évite une fuite de timing sur
 * la longueur du préfixe correct. Refuse toujours si expected est vide
 * (protection non configurée = échec fermé, jamais un accès accordé). */
export function timingSafeTokenMatch(provided: string, expected: string): boolean {
  if (!expected) return false
  const a = Buffer.from(provided)
  const b = Buffer.from(expected)
  if (a.length !== b.length) return false
  return timingSafeEqual(a, b)
}
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS (4/4 nouveaux + 15 précédents = 19 tests).

- [ ] **Step 5: Commit**

```bash
git add src/utils/captureToken.ts src/utils/captureToken.test.ts
git commit -m "feat: constant-time token comparison for capture upload auth"
```

---

### Task 11: `POST /capture-upload` (TDD)

**Files:**
- Create: `src/routes/captureUpload.ts`
- Create: `src/routes/captureUpload.test.ts`

- [ ] **Step 1: Écrire les tests (échouent — route inexistante)**

`src/routes/captureUpload.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { Hono } from 'hono'
import { resetDbForTests, getDb } from '../db/index.js'
import { resetRateLimitForTests } from '../utils/rateLimit.js'
import { captureUploadRoutes } from './captureUpload.js'

process.env.IP_HASH_PEPPER = 'test-pepper-value'
process.env.CAPTURE_UPLOAD_TOKEN = 'test-capture-token'

function makeApp() {
  const app = new Hono()
  app.route('/capture-upload', captureUploadRoutes)
  return app
}

function freshDb() {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  resetRateLimitForTests()
}

function multipartBody(fields: Record<string, string>, fileContent: string): { body: FormData } {
  const form = new FormData()
  for (const [k, v] of Object.entries(fields)) form.append(k, v)
  form.append('file', new File([fileContent], 'capture.pcap', { type: 'application/octet-stream' }))
  return { body: form }
}

test('accepte un upload valide et le stocke', async () => {
  freshDb()
  const app = makeApp()
  const { body } = multipartBody({ vid: 'dead', pid: 'beef', device_name: 'Test Mouse' }, 'fake pcap bytes')
  const res = await app.request('/capture-upload', {
    method: 'POST',
    headers: { authorization: 'Bearer test-capture-token' },
    body,
  })
  assert.equal(res.status, 200)
  const rows = getDb().prepare('SELECT * FROM capture_uploads').all() as { vid: string; device_name: string }[]
  assert.equal(rows.length, 1)
  assert.equal(rows[0].vid, 'dead')
  assert.equal(rows[0].device_name, 'Test Mouse')
})

test('refuse un mauvais token', async () => {
  freshDb()
  const app = makeApp()
  const { body } = multipartBody({ vid: 'dead', pid: 'beef', device_name: 'Test' }, 'data')
  const res = await app.request('/capture-upload', {
    method: 'POST',
    headers: { authorization: 'Bearer wrong-token' },
    body,
  })
  assert.equal(res.status, 401)
})

test('refuse un vid/pid invalide', async () => {
  freshDb()
  const app = makeApp()
  const { body } = multipartBody({ vid: 'nope', pid: 'beef', device_name: 'Test' }, 'data')
  const res = await app.request('/capture-upload', {
    method: 'POST',
    headers: { authorization: 'Bearer test-capture-token' },
    body,
  })
  assert.equal(res.status, 400)
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npm test`
Expected: FAIL — `Cannot find module './captureUpload.js'`

- [ ] **Step 3: Implémenter `src/routes/captureUpload.ts`**

```typescript
import { Hono } from 'hono'
import { bodyLimit } from 'hono/body-limit'
import { randomUUID } from 'crypto'
import { getDb } from '../db/index.js'
import { checkRateLimit } from '../utils/rateLimit.js'
import { hashIp } from '../utils/ipHash.js'
import { timingSafeTokenMatch } from '../utils/captureToken.js'
import { saveCaptureFile } from '../captureStorage.js'

export const captureUploadRoutes = new Hono()

captureUploadRoutes.post(
  '/',
  bodyLimit({
    maxSize: 50 * 1024 * 1024,
    onError: (c) => c.json({ error: 'Capture trop volumineuse' }, 413),
  }),
  async (c) => {
    const ip = c.req.header('x-real-ip') ?? c.req.header('x-forwarded-for')?.split(',')[0].trim() ?? 'unknown'
    if (!checkRateLimit(hashIp(ip), 20, 60 * 60 * 1000)) {
      return c.json({ error: 'Trop de captures envoyées, réessayez plus tard.' }, 429)
    }

    const auth = c.req.header('authorization') ?? ''
    const token = auth.startsWith('Bearer ') ? auth.slice(7) : ''
    if (!timingSafeTokenMatch(token, process.env.CAPTURE_UPLOAD_TOKEN ?? '')) {
      return c.json({ error: 'Non autorisé' }, 401)
    }

    const form = await c.req.parseBody()
    const file = form.file
    if (!(file instanceof File)) {
      return c.json({ error: 'Fichier manquant' }, 400)
    }
    const vid = typeof form.vid === 'string' ? form.vid : ''
    const pid = typeof form.pid === 'string' ? form.pid : ''
    const deviceName = typeof form.device_name === 'string' ? form.device_name : ''
    if (!/^[0-9a-f]{4}$/.test(vid) || !/^[0-9a-f]{4}$/.test(pid)) {
      return c.json({ error: 'vid/pid invalides' }, 400)
    }

    const buffer = Buffer.from(await file.arrayBuffer())
    const filename = `${randomUUID()}.pcap`
    saveCaptureFile(filename, buffer)

    const db = getDb()
    db.prepare(
      'INSERT INTO capture_uploads (filename, vid, pid, device_name, size_bytes) VALUES (?, ?, ?, ?, ?)',
    ).run(filename, vid, pid, deviceName.slice(0, 128), buffer.length)

    return c.json({ ok: true })
  },
)
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS (3 nouveaux + 19 précédents = 22 tests).

- [ ] **Step 5: Commit**

```bash
git add src/routes/captureUpload.ts src/routes/captureUpload.test.ts
git commit -m "feat: POST /capture-upload endpoint with token auth and validation"
```

---

### Task 12: Page dashboard — liste + téléchargement des captures

**Files:**
- Modify: `src/views.ts`
- Modify: `src/routes/dashboard.ts`

- [ ] **Step 1: Ajouter `renderCaptures` dans `src/views.ts`**

Ajouter après `renderDashboard` :
```typescript
export interface CaptureUploadRow {
  id: number
  filename: string
  vid: string
  pid: string
  device_name: string
  size_bytes: number
  uploaded_at: string
}

export function renderCaptures(rows: CaptureUploadRow[]): string {
  const body = rows
    .map(
      (r) => `
    <tr>
      <td>${esc(r.vid)}:${esc(r.pid)}</td>
      <td>${esc(r.device_name) || '—'}</td>
      <td>${(r.size_bytes / 1024).toFixed(1)} Ko</td>
      <td>${esc(r.uploaded_at)}</td>
      <td><a href="/dashboard/captures/${r.id}/download">Télécharger</a></td>
    </tr>`,
    )
    .join('')
  return `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Captures USB — PureRGB Télémétrie</title>
<style>body{font-family:sans-serif;background:#111;color:#eee;padding:24px}table{border-collapse:collapse;width:100%}td,th{border:1px solid #333;padding:6px 10px;text-align:left}a{color:#8cf}</style>
</head><body>
<h1>Captures USB reçues (${rows.length})</h1>
<p><a href="/dashboard">← Retour au dashboard</a></p>
<table><tr><th>VID:PID</th><th>Appareil</th><th>Taille</th><th>Reçu le</th><th></th></tr>${body}</table>
</body></html>`
}
```

- [ ] **Step 2: Ajouter un lien depuis le dashboard principal**

Dans `renderDashboard` (même fichier), remplacer :
```typescript
<h1>Appareils non reconnus (${rows.length})</h1>
<form method="post" action="/logout"><button type="submit">Déconnexion</button></form>
```
par :
```typescript
<h1>Appareils non reconnus (${rows.length})</h1>
<p><a href="/dashboard/captures">Voir les captures USB reçues →</a></p>
<form method="post" action="/logout"><button type="submit">Déconnexion</button></form>
```

- [ ] **Step 3: Ajouter les routes dans `src/routes/dashboard.ts`**

Ajouter les imports en haut du fichier :
```typescript
import { renderCaptures, type CaptureUploadRow } from '../views.js'
import { readCaptureFile } from '../captureStorage.js'
```

Ajouter à la fin du fichier, après la route `GET /` existante :
```typescript
dashboardRoutes.get('/captures', (c) => {
  const db = getDb()
  const rows = db
    .prepare(
      'SELECT id, filename, vid, pid, device_name, size_bytes, uploaded_at FROM capture_uploads ORDER BY uploaded_at DESC',
    )
    .all() as CaptureUploadRow[]
  return c.html(renderCaptures(rows))
})

dashboardRoutes.get('/captures/:id/download', (c) => {
  const id = Number(c.req.param('id'))
  const db = getDb()
  const row = db.prepare('SELECT filename FROM capture_uploads WHERE id = ?').get(id) as
    | { filename: string }
    | undefined
  if (!row) return c.text('Introuvable', 404)
  const data = readCaptureFile(row.filename)
  return new Response(data, {
    headers: {
      'content-type': 'application/octet-stream',
      'content-disposition': `attachment; filename="${row.filename}"`,
    },
  })
})
```

- [ ] **Step 4: Vérifier**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npx tsc --noEmit && npm test`
Expected: les deux verts, 22/22 tests toujours (pas de nouveau test ajouté ici — pages HTML server-rendues déjà couvertes indirectement par la revue manuelle Task 15).

- [ ] **Step 5: Commit**

```bash
git add src/views.ts src/routes/dashboard.ts
git commit -m "feat: dashboard page listing and downloading received USB captures"
```

---

### Task 13: Montage de la route + configuration

**Files:**
- Modify: `src/index.ts`
- Modify: `.env.example`

- [ ] **Step 1: Monter la route dans `src/index.ts`**

Ajouter l'import :
```typescript
import { captureUploadRoutes } from './routes/captureUpload.js'
```

Ajouter le montage, avec les autres routes publiques :
```typescript
app.route('/capture-upload', captureUploadRoutes)
```

- [ ] **Step 2: Ajouter la variable au `.env.example`**

Ajouter dans `.env.example`, avec un placeholder — même convention que `JWT_SECRET`/`IP_HASH_PEPPER` déjà présents dans ce fichier (jamais une vraie valeur, même dans ce repo privé — `.env.example` est un gabarit, pas un secret) :
```
CAPTURE_UPLOAD_TOKEN=change_this_to_match_the_value_baked_into_the_app_build
```
La vraie valeur (posée en secret GitHub Actions du repo `PureRGB` à l'implémentation de Task 4, cf. Repères de contexte) doit être identique dans le `.env` réel du VPS — jamais écrite dans ce fichier `.env.example` ni dans aucun fichier suivi par git, y compris ce plan.

- [ ] **Step 3: Vérifier**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npx tsc --noEmit && npm run build && npm test`
Expected: tout vert, 22/22 tests.

- [ ] **Step 4: Commit**

```bash
git add src/index.ts .env.example
git commit -m "feat: mount capture-upload route, document CAPTURE_UPLOAD_TOKEN"
```

---

### Task 14: Déploiement VPS

**Files:** aucun (opérations infra)

- [ ] **Step 1: Push du repo**

```bash
cd "C:\Users\Momo\Desktop\PureRGB-Telemetry"
git push origin main
```

- [ ] **Step 2: Mettre à jour le `.env` réel sur le VPS**

La valeur réelle du token (identique à celle posée en secret GitHub Actions du repo `PureRGB` à l'implémentation de Task 4) doit être écrite directement dans le `.env` du VPS via SSH — **jamais écrite dans ce plan ni dans aucun fichier suivi par git** (le plan lui-même vit dans le repo `PureRGB`, qui est public). Récupérer la valeur depuis là où elle a été générée (contexte de la session d'implémentation, ou en régénérer une nouvelle avec `openssl rand -hex 32` si perdue — dans ce cas, mettre aussi à jour le secret GitHub Actions `CAPTURE_UPLOAD_TOKEN` du repo `PureRGB` avec la même nouvelle valeur, sinon l'app buildée par CI et le serveur auront des tokens différents et l'upload échouera systématiquement en 401) :
```bash
ssh root@212.227.140.45 "grep -q CAPTURE_UPLOAD_TOKEN /opt/purergb-telemetry/.env || echo 'CAPTURE_UPLOAD_TOKEN=<valeur réelle, jamais commitée>' >> /opt/purergb-telemetry/.env"
```
Expected : la ligne est ajoutée seulement si absente (idempotent — ne duplique pas si la tâche est relancée).

- [ ] **Step 3: Pull + rebuild + redémarrer**

```bash
ssh root@212.227.140.45 "cd /opt/purergb-telemetry && git pull origin main && docker compose up -d --build"
```

- [ ] **Step 4: Vérifier le conteneur et l'endpoint**

```bash
ssh root@212.227.140.45 "docker compose -f /opt/purergb-telemetry/docker-compose.yml ps"
```
Expected : `Up (healthy)`.

```bash
curl -s https://telemetry-purergb.heiphaistos.org/health
```
Expected : `{"ok":true,"app":"purergb-telemetry"}`

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X POST https://telemetry-purergb.heiphaistos.org/capture-upload
```
Expected : `401` (pas de token — confirme que la route existe et rejette bien sans auth, sans révéler d'erreur interne).

- [ ] **Step 5:** aucun commit (étape infra pure).

---

## PARTIE C — Vérification finale + version

### Task 15: Vérification complète + bump de version

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json`

- [ ] **Step 1: Build complet backend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check && cargo test`
Expected: vert, 33/33 tests (tous les tests précédents + les 3 nouveaux `usbcapture`).

- [ ] **Step 2: Build complet frontend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB" && npm run build`
Expected: vert.

- [ ] **Step 3: Vérification manuelle (nécessite une vraie fenêtre, à faire par Momo ou en session suivante)**

1. `npm run tauri dev`
2. Réglages → Diagnostic → repérer une ligne "non reconnu" (ex. le DualBlader, RF-903 ou RF-800 si branché)
3. Cliquer "Capturer le protocole USB" → lire l'avertissement → "Démarrer"
4. Confirmer l'installation USBPcap (si première fois) puis le chronomètre qui démarre
5. Ouvrir le logiciel officiel de l'appareil, changer une couleur
6. "Arrêter" → vérifier l'écran récapitulatif liste au moins un fichier non vide
7. "Envoyer pour analyse" → vérifier sur `https://telemetry-purergb.heiphaistos.org/dashboard/captures` que le fichier apparaît, le télécharger, confirmer qu'il s'ouvre dans Wireshark avec du trafic visible

- [ ] **Step 4: Bump de version mineure 0.14.0 → 0.15.0**

`src-tauri/Cargo.toml` : `version = "0.15.0"`
`src-tauri/tauri.conf.json` : `"version": "0.15.0"`
`package.json` : `"version": "0.15.0"`

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check` (resynchronise `Cargo.lock`)

- [ ] **Step 5: Commit version**

```bash
cd "C:\Users\Momo\Desktop\PureRGB"
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json package.json
git commit -m "chore: bump version to 0.15.0"
```

- [ ] **Step 6: Mettre à jour le statut de la spec**

Dans `docs/superpowers/specs/2026-07-21-usb-capture-design.md`, remplacer la section `## Statut` par :
```markdown
## Statut

Implémenté et déployé (v0.15.0). Endpoint `/capture-upload` + dashboard captures en production sur `telemetry-purergb.heiphaistos.org`. Build backend (cargo check/test) et frontend (npm build) verts des deux côtés. Vérification manuelle complète (capture réelle sur un appareil non reconnu, upload, téléchargement depuis le dashboard, ouverture Wireshark) à faire par Momo avec une fenêtre réelle devant lui.
```

```bash
git add docs/superpowers/specs/2026-07-21-usb-capture-design.md
git commit -m "docs: mark usb capture spec as implemented"
```

---

## Self-Review (fait par l'auteur du plan)

**Couverture spec** : installation USBPcap signée+SHA256 pinné (Task 2) ✓ · capture multi-hub sans corrélation appareil précise (Task 3) ✓ · upload via curl.exe existant (Task 4) ✓ · commandes Tauri + AppState (Task 5) ✓ · UI avec avertissement clavier explicite + consentement séparé à chaque capture + démarrer/arrêter manuel + arrêt auto 5 min (Task 7, simplifié côté frontend plutôt que thread Rust — documenté en Repères) ✓ · endpoint upload token statique partagé (Task 10, 11, 13) ✓ · dashboard liste+téléchargement (Task 12) ✓ · sécurité (rate-limit, taille plafonnée, token temps constant, jamais exposé au frontend) ✓.

**Décisions documentées vs spec** : (1) l'arrêt automatique à 5 minutes est géré en JS frontend plutôt que par un thread Rust séparé — équivalent fonctionnel, moins de complexité/état partagé, documenté en Repères de contexte. (2) le token upload est une constante compilée unique (pas un secret par installation) — corrigé dès l'auto-relecture de la spec elle-même, cohérent ici.

**Correction appliquée pendant la rédaction du plan** : le premier jet des Tasks 2/3 utilisait un `use std::os::windows::process::CommandExt;` global en tête de fichier pour `.creation_flags(...)`. Vérifié contre `netdev.rs:314-319` (`curl()`, déjà dans ce crate) : le pattern réel du projet gate cet appel `#[cfg(windows)] { use ...; cmd.creation_flags(...); }` localement à chaque spawn, pas en import global — corrigé dans les deux tasks pour matcher exactement l'existant.

**Dépendances** : zéro nouvelle dépendance Cargo (réutilise `curl()` existant, `std::process::Command`). Zéro nouvelle dépendance npm (`crypto.randomUUID`/`crypto.timingSafeEqual` natifs Node, `hono/body-limit` déjà utilisé pour `/report`).

**Vérification faite avant d'écrire ce plan** : URL et SHA-256 USBPcap réels (téléchargé et hashé en direct cette session, pas une valeur supposée) ; syntaxe CLI `USBPcapCMD.exe` confirmée contre la documentation officielle ; limite de filtrage par hub (pas par appareil) confirmée contre une issue GitHub réelle du projet upstream.
