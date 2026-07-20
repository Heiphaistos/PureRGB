//! Capteurs matériels via le sidecar `sensord` (LibreHardwareMonitorLib, MPL 2.0).
//! sensord émet une ligne JSON toutes les 2 s sur stdout ; on garde le dernier
//! relevé en mémoire. Il se termine seul quand son stdin se ferme.

use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Même limitation que liquidctl (voir backends/liquidctl/mod.rs) : exe
/// portable sans resources/. sensord est notre propre build .NET self-contained.
const SENSORD_URL: &str =
    "https://github.com/Heiphaistos/PureRGB/releases/download/sidecars-v1/sensord.exe";
const SENSORD_SHA256: &str = "c60ecbea5f4d4608467bcea106ad6b8f53ac22eb27977840065192f17475f2a5";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub id: String,
    pub hardware: String,
    pub name: String,
    /// Temperature, Fan, Load, Control, Power.
    #[serde(rename = "type")]
    pub kind: String,
    pub value: f64,
    /// true = canal pilotable logiciellement (ventilateur carte mère via LHM).
    #[serde(default)]
    pub controllable: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Frame {
    sensors: Vec<Sensor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorDiag {
    pub exe_path: Option<String>,
    pub running: bool,
    pub sensor_count: usize,
}

#[derive(Default)]
pub struct SensorHub {
    snapshot: Arc<Mutex<Vec<Sensor>>>,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    resource_dir: Mutex<Option<PathBuf>>,
}

impl SensorHub {
    pub fn new() -> Arc<Self> {
        Arc::new(SensorHub::default())
    }

    pub fn set_resource_dir(&self, dir: PathBuf) {
        *self.resource_dir.lock() = Some(dir);
    }

    pub fn snapshot(&self) -> Vec<Sensor> {
        self.snapshot.lock().clone()
    }

    /// Dernière valeur d'un capteur par id.
    pub fn value(&self, sensor_id: &str) -> Option<f64> {
        self.snapshot
            .lock()
            .iter()
            .find(|s| s.id == sensor_id)
            .map(|s| s.value)
    }

    fn locate(&self) -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(res) = self.resource_dir.lock().clone() {
            // resource_dir() = dossier d'install (fichiers sous resources/) ou resources/.
            candidates.push(res.join("resources").join("sensord").join("sensord.exe"));
            candidates.push(res.join("sensord").join("sensord.exe"));
        }
        if let Some(app) = std::env::var_os("APPDATA") {
            candidates.push(
                PathBuf::from(app)
                    .join("PureRGB")
                    .join("sensord")
                    .join("sensord.exe"),
            );
        }
        candidates.into_iter().find(|p| p.is_file())
    }

    fn appdata_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB").join("sensord"))
    }

    /// Télécharge sensord.exe (SHA-256 pinné) vers %APPDATA%\PureRGB\sensord\.
    /// Publish self-contained .NET 8 : runtime déjà inclus, pas de DLL à part.
    /// Téléchargement vers un fichier temporaire puis renommage seulement
    /// après vérification du hash (Move-Item = rename NTFS quasi-atomique) —
    /// un download interrompu ne doit jamais laisser un binaire corrompu au
    /// chemin que `locate()` fait confiance ensuite sans re-vérifier.
    fn install() -> Result<PathBuf> {
        let dir = Self::appdata_dir().context("APPDATA introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let exe = dir.join("sensord.exe");
        let tmp = dir.join("sensord.exe.download");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{tmp}' -UseBasicParsing; \
             $h = (Get-FileHash '{tmp}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{tmp}' -Force; throw \"hash mismatch: $h\" }}; \
             Move-Item '{tmp}' '{exe}' -Force",
            url = SENSORD_URL,
            tmp = tmp.display(),
            exe = exe.display(),
            sha = SENSORD_SHA256,
        );
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().context("téléchargement sensord")?;
        if !output.status.success() {
            bail!(
                "téléchargement sensord échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if !exe.is_file() {
            bail!("sensord.exe absent après téléchargement");
        }
        Ok(exe)
    }

    /// Démarre sensord + thread lecteur. No-op si déjà lancé ou exe absent.
    pub fn start(self: &Arc<Self>) -> Result<bool> {
        if self.child.lock().is_some() {
            return Ok(false);
        }
        let exe = match self.locate() {
            Some(e) => e,
            None => match Self::install() {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("installation sensord: {e:#}");
                    return Ok(false);
                }
            },
        };
        let mut cmd = Command::new(&exe);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().context("lancement sensord")?;
        let stdout = child.stdout.take().context("stdout sensord")?;
        *self.stdin.lock() = child.stdin.take();
        *self.child.lock() = Some(child);

        let snapshot = self.snapshot.clone();
        std::thread::Builder::new()
            .name("sensord-reader".into())
            .spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    match serde_json::from_str::<Frame>(&line) {
                        Ok(f) => *snapshot.lock() = f.sensors,
                        Err(e) => log::warn!("trame sensord invalide: {e}"),
                    }
                }
                log::info!("sensord terminé");
                snapshot.lock().clear();
            })
            .context("spawn sensord-reader")?;
        Ok(true)
    }

    /// true si sensord tourne (stdin ouvert).
    pub fn running(&self) -> bool {
        self.child.lock().is_some()
    }

    /// Diagnostic : chemin localisé (ou None si absent partout), état de
    /// marche, nombre de capteurs dans le dernier relevé.
    pub fn diag(&self) -> SensorDiag {
        SensorDiag {
            exe_path: self.locate().map(|p| p.display().to_string()),
            running: self.running(),
            sensor_count: self.snapshot.lock().len(),
        }
    }

    /// Envoie une commande JSON à sensord (une ligne).
    fn send(&self, payload: &serde_json::Value) -> Result<()> {
        use std::io::Write;
        let mut guard = self.stdin.lock();
        let stdin = guard.as_mut().context("sensord non démarré")?;
        writeln!(stdin, "{payload}").context("écriture stdin sensord")?;
        stdin.flush().context("flush stdin sensord")
    }

    /// Pilote un canal Control LHM (ventilateur carte mère) en %.
    pub fn set_control(&self, sensor_id: &str, percent: u8) -> Result<()> {
        self.send(&serde_json::json!({
            "cmd": "set",
            "id": sensor_id,
            "value": percent.min(100),
        }))
    }

    /// Rend la main au BIOS sur un canal.
    pub fn reset_control(&self, sensor_id: &str) -> Result<()> {
        self.send(&serde_json::json!({ "cmd": "reset", "id": sensor_id }))
    }

    /// Arrête sensord : fermer stdin le fait sortir proprement, kill en secours.
    pub fn stop(&self) {
        drop(self.stdin.lock().take());
        if let Some(mut child) = self.child.lock().take() {
            drop(child.stdin.take());
            std::thread::sleep(std::time::Duration::from_millis(400));
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for SensorHub {
    fn drop(&mut self) {
        self.stop();
    }
}
