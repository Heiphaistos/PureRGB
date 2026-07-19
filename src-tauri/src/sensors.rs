//! Capteurs matériels via le sidecar `sensord` (LibreHardwareMonitorLib, MPL 2.0).
//! sensord émet une ligne JSON toutes les 2 s sur stdout ; on garde le dernier
//! relevé en mémoire. Il se termine seul quand son stdin se ferme.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub id: String,
    pub hardware: String,
    pub name: String,
    /// Temperature, Fan, Load, Control, Power.
    #[serde(rename = "type")]
    pub kind: String,
    pub value: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Frame {
    sensors: Vec<Sensor>,
}

#[derive(Default)]
pub struct SensorHub {
    snapshot: Arc<Mutex<Vec<Sensor>>>,
    child: Mutex<Option<Child>>,
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

    /// Démarre sensord + thread lecteur. No-op si déjà lancé ou exe absent.
    pub fn start(self: &Arc<Self>) -> Result<bool> {
        if self.child.lock().is_some() {
            return Ok(false);
        }
        let exe = match self.locate() {
            Some(e) => e,
            None => return Ok(false), // sidecar absent : capteurs indisponibles
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

    /// Arrête sensord : fermer stdin le fait sortir proprement, kill en secours.
    pub fn stop(&self) {
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
