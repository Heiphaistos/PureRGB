//! Détection ET gestion des logiciels constructeur en conflit d'accès matériel
//! (deux logiciels qui écrivent sur le même contrôleur = comportement erratique,
//! handles HID verrouillés = appareils invisibles pour OpenRGB).
//!
//! Deux niveaux d'action, tous deux réversibles :
//! - stopper : kill des processus + arrêt des services (reviennent au reboot) ;
//! - désactiver : en plus, StartupType=Disabled (mode d'origine sauvegardé
//!   dans settings.json pour restauration à l'identique).
//!
//! Nécessite les droits administrateur (manifest requireAdministrator).

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub start_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictingSoftware {
    /// Clé stable de la famille (ex: "corsair"), utilisée par les commandes.
    pub family: String,
    pub name: String,
    /// Processus de la famille actuellement en cours d'exécution.
    pub processes: Vec<String>,
    /// Services Windows de la famille (même arrêtés/désactivés).
    pub services: Vec<ServiceInfo>,
    /// Marques matérielles concernées.
    pub affects: Vec<String>,
    /// true si au moins un processus tourne ou un service est démarré.
    pub active: bool,
}

/// (clé famille, nom affiché, marques, exe connus, mots-clés services)
/// Les mots-clés services sont comparés en minuscules au nom + display name.
/// Volontairement ciblés RGB/périphériques — jamais les services système du
/// constructeur (mises à jour, audio...).
struct Family {
    key: &'static str,
    name: &'static str,
    affects: &'static [&'static str],
    processes: &'static [&'static str],
    service_keywords: &'static [&'static str],
}

const FAMILIES: &[Family] = &[
    Family {
        key: "corsair",
        name: "Corsair iCUE",
        affects: &["Corsair"],
        processes: &["icue.exe", "icueuxprocess.exe", "corsair.service.exe", "icuedevicepluginhost.exe"],
        service_keywords: &["corsair"],
    },
    Family {
        key: "nzxt",
        name: "NZXT CAM",
        affects: &["NZXT"],
        processes: &["nzxt cam.exe", "cam.exe", "nzxt cam service.exe"],
        service_keywords: &["nzxt"],
    },
    Family {
        key: "asus",
        name: "ASUS Aura / Armoury Crate",
        affects: &["ASUS"],
        processes: &["lightingservice.exe", "armourycrate.service.exe", "armouryswagent.exe", "asusaurasyncservice.exe"],
        service_keywords: &["lightingservice", "armoury", "aura sync", "asusgamesdk"],
    },
    Family {
        key: "msi",
        name: "MSI Center / Mystic Light",
        affects: &["MSI"],
        processes: &["msi center.exe", "mysticlight.exe", "msi_led.exe"],
        service_keywords: &["mystic", "msi center", "msi central"],
    },
    Family {
        key: "gigabyte",
        name: "Gigabyte RGB Fusion / GCC",
        affects: &["Gigabyte"],
        processes: &["rgbfusion.exe", "gigabytecc.exe"],
        service_keywords: &["rgb fusion", "gigabyte control center", "gcc service"],
    },
    Family {
        key: "razer",
        name: "Razer Synapse / Chroma",
        affects: &["Razer"],
        processes: &["razer synapse 3.exe", "razer synapse service.exe", "rzsdkservice.exe"],
        service_keywords: &["razer", "rzsdk", "chroma sdk"],
    },
    Family {
        key: "logitech",
        name: "Logitech G HUB",
        affects: &["Logitech"],
        processes: &["lghub.exe", "lghub_agent.exe", "lghub_updater.exe"],
        service_keywords: &["lghub"],
    },
    Family {
        key: "steelseries",
        name: "SteelSeries GG",
        affects: &["SteelSeries"],
        processes: &["steelseriesgg.exe", "steelseriesengine.exe"],
        service_keywords: &["steelseries"],
    },
    Family {
        key: "lianli",
        name: "Lian Li L-Connect",
        affects: &["Lian Li"],
        processes: &["l-connect 3.exe"],
        service_keywords: &["l-connect", "lconnect"],
    },
    Family {
        key: "thermaltake",
        name: "TT RGB Plus",
        affects: &["Thermaltake"],
        processes: &["ttrgbplus.exe"],
        service_keywords: &["tt rgb", "thermaltake"],
    },
    Family {
        key: "coolermaster",
        name: "Cooler Master MasterPlus",
        affects: &["Cooler Master"],
        processes: &["masterplus.exe"],
        service_keywords: &["masterplus", "cooler master"],
    },
    Family {
        key: "signalrgb",
        name: "SignalRGB",
        affects: &["toutes"],
        processes: &["signalrgb.exe", "signalrgbservice.exe"],
        service_keywords: &["signalrgb"],
    },
];

fn find_family(key: &str) -> Result<&'static Family> {
    FAMILIES
        .iter()
        .find(|f| f.key == key)
        .with_context(|| format!("famille inconnue: {key}"))
}

/// OpenRGB n'est PAS un conflit : c'est notre pont. Listé à part.
pub fn openrgb_running(sys: &System) -> bool {
    sys.processes()
        .values()
        .any(|p| p.name().to_string_lossy().to_lowercase().starts_with("openrgb"))
}

#[derive(Debug, Deserialize)]
struct RawService {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "State")]
    state: Option<String>,
    #[serde(rename = "StartMode")]
    start_mode: Option<String>,
}

fn run_powershell(script: &str) -> Result<std::process::Output> {
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output().context("lancement PowerShell")
}

/// Tous les services Windows (nom, display, état, mode de démarrage).
fn list_services() -> Result<Vec<RawService>> {
    let output = run_powershell(
        "Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,StartMode | ConvertTo-Json -Compress",
    )?;
    if !output.status.success() {
        bail!(
            "énumération services: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    // ConvertTo-Json renvoie un objet seul (pas un tableau) s'il n'y a qu'un élément.
    if text.starts_with('[') {
        serde_json::from_str(text).context("parse JSON services")
    } else {
        Ok(vec![serde_json::from_str(text).context("parse JSON service")?])
    }
}

fn family_services(fam: &Family, all: &[RawService]) -> Vec<ServiceInfo> {
    all.iter()
        .filter_map(|s| {
            let name = s.name.clone()?;
            let display = s.display_name.clone().unwrap_or_default();
            let hay = format!("{} {}", name.to_lowercase(), display.to_lowercase());
            if fam.service_keywords.iter().any(|k| hay.contains(k)) {
                Some(ServiceInfo {
                    name,
                    display_name: display,
                    state: s.state.clone().unwrap_or_default(),
                    start_mode: s.start_mode.clone().unwrap_or_default(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Scan complet : processus en cours + services installés, par famille.
/// Une famille apparaît si au moins un processus tourne OU un service existe.
pub fn scan() -> (Vec<ConflictingSoftware>, bool) {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Never)),
    );
    let running: Vec<String> = sys
        .processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_lowercase())
        .collect();
    let all_services = list_services().unwrap_or_else(|e| {
        log::warn!("énumération services: {e:#}");
        Vec::new()
    });

    let mut found: Vec<ConflictingSoftware> = Vec::new();
    for fam in FAMILIES {
        let processes: Vec<String> = fam
            .processes
            .iter()
            .filter(|p| running.iter().any(|r| r == *p))
            .map(|p| p.to_string())
            .collect();
        let services = family_services(fam, &all_services);
        if processes.is_empty() && services.is_empty() {
            continue;
        }
        let active = !processes.is_empty()
            || services.iter().any(|s| s.state.eq_ignore_ascii_case("Running"));
        found.push(ConflictingSoftware {
            family: fam.key.to_string(),
            name: fam.name.to_string(),
            processes,
            services,
            affects: fam.affects.iter().map(|s| s.to_string()).collect(),
            active,
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    (found, openrgb_running(&sys))
}

/// Stoppe une famille : services arrêtés (+ désactivés si `disable`) puis
/// processus tués. Renvoie la map service → mode d'origine (à persister)
/// quand `disable` est vrai.
pub fn stop_family(key: &str, disable: bool) -> Result<HashMap<String, String>> {
    let fam = find_family(key)?;
    let all = list_services()?;
    let services = family_services(fam, &all);
    let mut original_modes = HashMap::new();

    for svc in &services {
        if disable && !svc.start_mode.eq_ignore_ascii_case("Disabled") {
            original_modes.insert(svc.name.clone(), svc.start_mode.clone());
        }
        let mut script = format!(
            "Stop-Service -Name '{}' -Force -ErrorAction SilentlyContinue",
            svc.name.replace('\'', "''")
        );
        if disable {
            script.push_str(&format!(
                "; Set-Service -Name '{}' -StartupType Disabled",
                svc.name.replace('\'', "''")
            ));
        }
        let out = run_powershell(&script)?;
        if !out.status.success() {
            log::warn!(
                "service {}: {}",
                svc.name,
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    for proc_name in fam.processes {
        let mut cmd = Command::new("taskkill.exe");
        cmd.args(["/F", "/IM", proc_name]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.output(); // absent = déjà mort, pas une erreur
    }
    Ok(original_modes)
}

/// Réactive une famille : StartupType restauré (modes sauvegardés, sinon le
/// mode actuel est conservé) puis services redémarrés best-effort.
pub fn restore_family(key: &str, saved_modes: &HashMap<String, String>) -> Result<Vec<String>> {
    let fam = find_family(key)?;
    let all = list_services()?;
    let services = family_services(fam, &all);
    let mut restored = Vec::new();

    for svc in &services {
        let mode = saved_modes
            .get(&svc.name)
            .map(String::as_str)
            .unwrap_or(if svc.start_mode.eq_ignore_ascii_case("Disabled") {
                "Automatic"
            } else {
                svc.start_mode.as_str()
            });
        // Win32_Service dit "Auto", Set-Service attend "Automatic".
        let mode = match mode.to_lowercase().as_str() {
            "auto" | "automatic" => "Automatic",
            "manual" => "Manual",
            "disabled" => "Automatic",
            other => {
                log::warn!("mode inattendu {other}, Automatic par défaut");
                "Automatic"
            }
        };
        let name = svc.name.replace('\'', "''");
        let script = format!(
            "Set-Service -Name '{name}' -StartupType {mode}; Start-Service -Name '{name}' -ErrorAction SilentlyContinue"
        );
        let out = run_powershell(&script)?;
        if out.status.success() {
            restored.push(svc.name.clone());
        } else {
            log::warn!(
                "restauration {}: {}",
                svc.name,
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    Ok(restored)
}
