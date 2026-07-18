//! Détection des logiciels constructeur en cours d'exécution.
//! But : prévenir l'utilisateur AVANT tout conflit d'accès matériel
//! (deux logiciels qui écrivent sur le même contrôleur = comportement erratique).

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

#[derive(Debug, Clone, Serialize)]
pub struct ConflictingSoftware {
    pub name: String,
    pub process: String,
    /// Marques matérielles concernées.
    pub affects: Vec<String>,
}

/// (nom process en minuscules, nom logiciel, marques concernées)
const KNOWN_SOFTWARE: &[(&str, &str, &[&str])] = &[
    ("icue.exe", "Corsair iCUE", &["Corsair"]),
    ("icueuxprocess.exe", "Corsair iCUE", &["Corsair"]),
    ("corsair.service.exe", "Corsair Service", &["Corsair"]),
    ("nzxt cam.exe", "NZXT CAM", &["NZXT"]),
    ("cam.exe", "NZXT CAM", &["NZXT"]),
    ("lightingservice.exe", "ASUS Aura LightingService", &["ASUS"]),
    ("armourycrate.service.exe", "ASUS Armoury Crate", &["ASUS"]),
    ("armouryswagent.exe", "ASUS Armoury Crate", &["ASUS"]),
    ("asusaurasyncservice.exe", "ASUS Aura Sync", &["ASUS"]),
    ("msi center.exe", "MSI Center", &["MSI"]),
    ("mysticlight.exe", "MSI Mystic Light", &["MSI"]),
    ("msi_led.exe", "MSI Mystic Light", &["MSI"]),
    ("rgbfusion.exe", "Gigabyte RGB Fusion", &["Gigabyte"]),
    ("gigabytecc.exe", "Gigabyte Control Center", &["Gigabyte"]),
    ("razer synapse 3.exe", "Razer Synapse", &["Razer"]),
    ("razer synapse service.exe", "Razer Synapse", &["Razer"]),
    ("rzsdkservice.exe", "Razer Chroma SDK", &["Razer"]),
    ("lghub.exe", "Logitech G HUB", &["Logitech"]),
    ("lghub_agent.exe", "Logitech G HUB", &["Logitech"]),
    ("steelseriesgg.exe", "SteelSeries GG", &["SteelSeries"]),
    ("l-connect 3.exe", "Lian Li L-Connect", &["Lian Li"]),
    ("ttrgbplus.exe", "TT RGB Plus", &["Thermaltake"]),
    ("masterplus.exe", "Cooler Master MasterPlus", &["Cooler Master"]),
    ("signalrgb.exe", "SignalRGB", &["toutes"]),
    ("wallpaper_engine.exe", "Wallpaper Engine (plugin RGB possible)", &[]),
];

/// OpenRGB n'est PAS un conflit : c'est notre pont. Listé à part.
pub fn openrgb_running(sys: &System) -> bool {
    sys.processes()
        .values()
        .any(|p| p.name().to_string_lossy().to_lowercase().starts_with("openrgb"))
}

pub fn scan() -> (Vec<ConflictingSoftware>, bool) {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Never)),
    );
    let mut found: Vec<ConflictingSoftware> = Vec::new();
    for process in sys.processes().values() {
        let pname = process.name().to_string_lossy().to_lowercase();
        if let Some((_, soft, brands)) = KNOWN_SOFTWARE.iter().find(|(p, _, _)| *p == pname) {
            if !found.iter().any(|c| c.name == *soft) {
                found.push(ConflictingSoftware {
                    name: soft.to_string(),
                    process: pname.clone(),
                    affects: brands.iter().map(|s| s.to_string()).collect(),
                });
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    (found, openrgb_running(&sys))
}
