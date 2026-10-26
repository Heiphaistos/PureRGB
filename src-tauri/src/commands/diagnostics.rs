//! Diagnostic matériel, capture USB, inventaire, test de sécurité et télémétrie.

use crate::backends::hid::HidBackend;
use crate::backends::liquidctl::LiquidctlBackend;
use crate::backends::openrgb::manager::{OpenRgbManager, OpenRgbStatus};
use crate::core::registry::SharedRegistry;
use crate::sensors::SensorHub;
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use crate::commands::devices::*;
use crate::{diag_extra, diag_report as diag_extra_report, inventory, screens, selftest, telemetry, usbcapture};
use crate::AppState;

#[derive(Serialize)]
pub(crate) struct HardwareDiagnostics {
    pub(crate) liquidctl: crate::backends::liquidctl::LiquidctlDiag,
    pub(crate) sensord: crate::sensors::SensorDiag,
    pub(crate) openrgb: OpenRgbStatus,
    pub(crate) hid_raw: Vec<crate::backends::hid::RawHidDevice>,
    /// Diagnostics étendus (télémétrie et diagnostic complet) ; absent des appels légers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) extra: Option<diag_extra::ExtraDiag>,
}

/// Diagnostic complet : capture les erreurs réelles de chaque sous-système
/// (liquidctl, sensord, OpenRGB, énumération HID brute) au lieu de les
/// avaler dans les logs serveur — pour comprendre pourquoi un binaire "ne
/// se charge pas" sur une machine donnée sans accès à la console.
/// Extrait pour être réutilisable par le thread `hw-init` (envoi auto au
/// démarrage) ET par les commandes Tauri (diagnostic manuel, envoi manuel).
pub(crate) fn compute_hardware_diagnostics(
    registry: &SharedRegistry,
    sensors: &SensorHub,
    openrgb_mgr: &OpenRgbManager,
    openrgb_host: &str,
    openrgb_port: u16,
) -> HardwareDiagnostics {
    let liquidctl = {
        let mut reg = registry.lock();
        reg.backends_mut()
            .iter_mut()
            .find(|b| b.name() == "liquidctl")
            .and_then(|b| b.as_any_mut().downcast_mut::<LiquidctlBackend>())
            .map(|lc| lc.diagnose())
            .unwrap_or(crate::backends::liquidctl::LiquidctlDiag {
                exe_path: None,
                version: Err("backend liquidctl absent du registre".into()),
                list: Err("—".into()),
                initialize: Err("—".into()),
                status: Err("—".into()),
            })
    };
    let hid_raw = {
        let mut reg = registry.lock();
        reg.backends_mut()
            .iter_mut()
            .find(|b| b.name() == "hid")
            .and_then(|b| b.as_any_mut().downcast_mut::<HidBackend>())
            .and_then(|hid| hid.list_raw().ok())
            .unwrap_or_default()
    };
    HardwareDiagnostics {
        liquidctl,
        sensord: sensors.diag(),
        openrgb: openrgb_mgr.status(openrgb_host, openrgb_port),
        hid_raw,
        extra: None,
    }
}

#[derive(Serialize)]
pub(crate) struct CaptureFileInfo {
    path: String,
    hub: u32,
    size_bytes: u64,
}

#[tauri::command(async)]
pub(crate) fn usb_capture_ready() -> bool {
    usbcapture::usbpcap_ready()
}

#[tauri::command(async)]
pub(crate) fn usb_capture_install() -> Result<(), String> {
    usbcapture::usbpcap_install().map_err(|e| format!("{e:#}"))
}

#[tauri::command(async)]
pub(crate) fn usb_capture_start(state: State<AppState>) -> Result<(), String> {
    let mut active = state.active_capture.lock();
    if active.is_some() {
        return Err("Une capture est déjà en cours".into());
    }
    let session = usbcapture::start_capture().map_err(|e| format!("{e:#}"))?;
    *active = Some(session);
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn usb_capture_stop(state: State<AppState>) -> Result<Vec<CaptureFileInfo>, String> {
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
pub(crate) fn usb_capture_upload(vid: String, pid: String, device_name: String, path: String) -> Result<(), String> {
    usbcapture::upload_capture(std::path::Path::new(&path), &vid, &pid, &device_name)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command(async)]
pub(crate) fn hardware_diagnostics(state: State<AppState>) -> HardwareDiagnostics {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    compute_hardware_diagnostics(&state.registry, &state.sensors, &state.openrgb_mgr, &host, port)
}

/// Rapport de diagnostic local : cartes par sous-système + texte à copier (lecture seule, rien n'est envoyé).
#[tauri::command(async)]
pub(crate) fn diagnostic_report(app: tauri::AppHandle) -> diag_extra_report::DiagReport {
    let state = app.state::<AppState>();
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    let hw = compute_hardware_diagnostics(&state.registry, &state.sensors, &state.openrgb_mgr, &host, port);
    diag_extra_report::local_report(&state, &app.state::<std::sync::Arc<screens::manager::ScreenManager>>(), hw)
}

/// Enregistre le texte du rapport à l'emplacement choisi par l'utilisateur (boîte de dialogue côté interface).
#[tauri::command(async)]
pub(crate) fn diagnostic_report_save(path: String, text: String) -> Result<(), String> {
    std::fs::write(&path, text).map_err(|e| format!("écriture impossible : {e}"))
}

/// Un périphérique RGB vient d'être branché : on laisse USB finir son énumération, on relance l'OpenRGB
/// géré par PureRGB (il ne détecte qu'au démarrage) puis on re-scanne et on prévient l'interface.
pub(crate) fn rescan_after_plug(app: &tauri::AppHandle, plugged: &[crate::backends::hid::RawHidDevice]) {
    std::thread::sleep(std::time::Duration::from_secs(3));
    let state = app.state::<AppState>();
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    if state.openrgb_mgr.status(&host, port).managed {
        state.openrgb_mgr.stop();
        if let Err(e) = state.openrgb_mgr.ensure_running(&host, port) {
            log::warn!("redémarrage OpenRGB après branchement : {e:#}");
        }
    }
    rescan_and_notify(app, &format!("branchement RGB ({} nouveau(x))", plugged.len()));
}

/// Re-scan complet puis prévient l'interface (événement `devices-changed`).
pub(crate) fn rescan_and_notify(app: &tauri::AppHandle, why: &str) {
    let state = app.state::<AppState>();
    let sizes = state.settings.lock().zone_sizes.clone();
    let devices = scan_with_zone_sizes(&mut state.registry.lock(), &sizes);
    state.engine.invalidate();
    log::info!("{why} : {} appareil(s) après re-détection", devices.len());
    if let Err(e) = app.emit("devices-changed", devices.len()) {
        log::debug!("émission devices-changed : {e:#}");
    }
}

/// Test de compatibilité : vérifications en lecture seule de chaque composant détecté.
#[tauri::command(async)]
pub(crate) fn selftest_scan(state: State<AppState>) -> Vec<selftest::Item> {
    selftest::scan(&state)
}

/// Essai d'écriture minimal sur UN appareil (couleur très faible puis extinction), après confirmation explicite.
#[tauri::command(async)]
pub(crate) fn selftest_apply(state: State<AppState>, id: String, confirmed: bool) -> Result<selftest::TestResult, String> {
    selftest::apply(&state, &id, confirmed)
}

/// Inventaire du PC (WMI) comparé au matériel pilotable : explique les écarts (driver SMBus, logiciel en conflit…).
#[tauri::command(async)]
pub(crate) fn rgb_inventory(state: State<AppState>) -> Result<Vec<inventory::Finding>, String> {
    let inv = inventory::collect()?;
    let ctx = diag_extra::inventory_context(&state);
    let devices = state.registry.lock().device_list();
    Ok(inventory::analyze(&inv, &devices, &ctx))
}

/// Commande manuelle du bouton "Envoyer maintenant" — ignore le hash de
/// déduplication, envoie toujours si l'opt-in est actif. Point d'application
/// réel de l'opt-in pour cette commande : `telemetry::send_report_now` ne
/// vérifie rien lui-même (par design, c'est l'appel qui vaut consentement),
/// donc c'est ici que le refus doit avoir lieu si l'utilisateur a désactivé
/// la télémétrie entre-temps (l'UI masque le bouton, mais `invoke()` reste
/// appelable directement, p. ex. depuis les devtools).
#[tauri::command(async)]
pub(crate) fn send_telemetry_report(app: tauri::AppHandle) -> Result<(), String> {
    if !app.state::<AppState>().settings.lock().telemetry_opt_in {
        return Err("Télémétrie désactivée dans les réglages".into());
    }
    let json = diagnostics_json(&app).map_err(|e| e.to_string())?;
    telemetry::send_report_now(&json, env!("CARGO_PKG_VERSION")).map_err(|e| format!("{e:#}"))
}

/// JSON du diagnostic complet, réduit (journal, appareils) sous la limite de 64 Ko du serveur de télémétrie.
pub(crate) fn diagnostics_json(app: &tauri::AppHandle) -> serde_json::Result<String> {
    const LIMIT: usize = 60_000;
    let mut diag = full_diagnostics(app);
    let mut json = serde_json::to_string(&diag)?;
    for step in 0..3 {
        if json.len() <= LIMIT {
            break;
        }
        if let Some(x) = diag.extra.as_mut() {
            match step {
                0 => x.log_tail.truncate(10),
                1 => x.devices.truncate(30),
                _ => x.findings.clear(),
            }
        }
        json = serde_json::to_string(&diag)?;
    }
    Ok(json)
}

/// Diagnostic complet prêt à envoyer : base + extras, chemins réduits au nom de fichier.
pub(crate) fn full_diagnostics(app: &tauri::AppHandle) -> HardwareDiagnostics {
    let state = app.state::<AppState>();
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    let mut diag = compute_hardware_diagnostics(&state.registry, &state.sensors, &state.openrgb_mgr, &host, port);
    diag.extra = Some(diag_extra::collect(&state, &app.state::<std::sync::Arc<screens::manager::ScreenManager>>()));
    telemetry::redact_paths_for_telemetry(&mut diag);
    diag
}
