//! Diagnostics étendus de la télémétrie opt-in : de quoi comprendre à distance pourquoi un matériel
//! n'est pas détecté (carte mère, mémoire, USB RGB sans pilote, constats de l'inventaire, écrans,
//! logiciels en conflit, erreurs récentes du journal). Aucun numéro de série, chemin ni nom de compte.

use crate::screens::manager::ScreenManager;
use crate::{conflicts, inventory, AppState};
use serde::Serialize;

#[derive(Serialize)]
pub struct UsbSeen {
    pub vid: String,
    pub pid: String,
    pub windows_name: String,
    /// Modèle du catalogue RGB (None = fabricant connu mais PID absent du catalogue).
    pub catalog_model: Option<String>,
}

#[derive(Serialize)]
pub struct ZoneLite {
    pub name: String,
    pub leds: u32,
    pub min: u32,
    pub max: u32,
}

#[derive(Serialize)]
pub struct DeviceLite {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub backend: String,
    pub kind: String,
    pub controllable: bool,
    pub zones: Vec<ZoneLite>,
}

#[derive(Serialize)]
pub struct ScreensDiag {
    pub detected: Vec<String>,
    pub scan_errors: Vec<String>,
    /// VID:PID de mini-écrans du catalogue branchés mais sans pilote actif.
    pub plugged_but_idle: Vec<String>,
}

#[derive(Serialize)]
pub struct ExtraDiag {
    pub board: String,
    pub ram: Vec<String>,
    pub gpus: Vec<String>,
    pub usb_rgb: Vec<UsbSeen>,
    pub findings: Vec<inventory::Finding>,
    pub devices: Vec<DeviceLite>,
    pub conflicts_active: Vec<String>,
    /// Processus de logiciels d'écran / RGB constructeur en cours d'exécution (nom + PID) : suspects d'un « accès refusé ».
    pub suspect_processes: Vec<String>,
    pub screens: ScreensDiag,
    pub native_drivers_enabled: bool,
    pub inventory_error: Option<String>,
    /// Dernières lignes WARN/ERROR du journal, débarrassées du nom de compte.
    pub log_tail: Vec<String>,
}

/// Contexte de l'inventaire : état d'OpenRGB, PawnIO, capteurs, logiciels actifs.
pub fn inventory_context(state: &AppState) -> inventory::Context {
    context_with(state, &conflicts::scan().0)
}

pub(crate) fn context_with(state: &AppState, found: &[conflicts::ConflictingSoftware]) -> inventory::Context {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    let st = state.openrgb_mgr.status(&host, port);
    let sensors = state.sensors.snapshot();
    inventory::Context {
        openrgb_up: st.server_reachable,
        pawnio_ready: st.pawnio_ready,
        sensord_up: state.sensors.running(),
        fans_measured: sensors.iter().filter(|s| s.kind == "Fan" && s.id.starts_with("/lpc/")).count(),
        fans_controllable: sensors.iter().filter(|s| s.kind == "Control" && s.controllable && s.id.starts_with("/lpc/")).count(),
        active_conflicts: found.iter().filter(|c| c.active).map(|c| c.name.clone()).collect(),
        openrgb_error: st.last_error,
    }
}

/// Retire le nom de compte et de machine d'une ligne de journal.
pub fn scrub(line: &str) -> String {
    let mut out = line.to_string();
    for var in ["USERNAME", "COMPUTERNAME"] {
        if let Some(v) = std::env::var_os(var).map(|v| v.to_string_lossy().into_owned()).filter(|v| v.len() > 2) {
            out = out.replace(&v, "<anonyme>");
        }
    }
    out
}

/// Secondes UNIX d'un horodatage ISO 8601 UTC en début de ligne (`[2026-10-26T10:00:00Z WARN …`).
fn line_epoch(line: &str) -> Option<i64> {
    let t = line.trim_start().trim_start_matches('[');
    let n = |r: std::ops::Range<usize>| t.get(r)?.parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    // Jours depuis 1970 (algorithme civil de H. Hinnant).
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// Garde les lignes datées des dernières 24 h avant `now` (secondes UNIX).
fn keep_recent(lines: Vec<String>, now: i64) -> Vec<String> {
    lines.into_iter().filter(|l| line_epoch(l).is_some_and(|t| now - t <= 86_400)).collect()
}

fn log_tail(max: usize) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Some(path) = crate::settings::dirs_dir().map(|d| d.join("purergb.log")) else { return Vec::new() };
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut buf = Vec::new();
    if f.seek(SeekFrom::Start(len.saturating_sub(96 * 1024))).is_err() || f.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&buf);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let mut lines: Vec<String> = keep_recent(
        text.lines().filter(|l| l.contains(" WARN ") || l.contains(" ERROR ")).map(|l| scrub(l.trim()).chars().take(240).collect()).collect(),
        now,
    );
    let skip = lines.len().saturating_sub(max);
    lines.drain(..skip);
    lines
}

pub fn collect(state: &AppState, screens: &ScreenManager) -> ExtraDiag {
    collect_from(state, screens, inventory::collect(), inventory_context(state))
}

/// Même collecte à partir d'un inventaire et d'un contexte déjà relevés (le rapport local les réutilise).
pub(crate) fn collect_from(state: &AppState, screens: &ScreenManager, inv: Result<inventory::Inventory, String>, ctx: inventory::Context) -> ExtraDiag {
    let devices = state.registry.lock().device_list();
    let (inv, inventory_error) = match inv {
        Ok(i) => (i, None),
        Err(e) => (inventory::Inventory::default(), Some(e)),
    };
    let cat = crate::rgb_catalog::catalog();
    let usb_rgb = inv
        .usb
        .iter()
        .filter(|u| cat.vendor(u.vid).is_some())
        .map(|u| UsbSeen {
            vid: format!("{:04x}", u.vid),
            pid: format!("{:04x}", u.pid),
            windows_name: u.name.clone(),
            catalog_model: cat.device(u.vid, u.pid).map(str::to_string),
        })
        .collect();
    ExtraDiag {
        board: format!("{} {}", inv.board.manufacturer, inv.board.product).trim().to_string(),
        ram: inv.ram.iter().map(|r| format!("{} {}", r.manufacturer, r.part.trim()).trim().to_string()).collect(),
        gpus: inv.gpus.clone(),
        usb_rgb,
        findings: inventory::analyze(&inv, &devices, &ctx),
        devices: devices
            .iter()
            .map(|d| DeviceLite {
                id: d.id.clone(),
                name: d.name.clone(),
                vendor: d.vendor.clone(),
                backend: d.backend.clone(),
                kind: format!("{:?}", d.device_type),
                controllable: d.controllable,
                zones: d.zones.iter().map(|z| ZoneLite { name: z.name.clone(), leds: z.led_count, min: z.leds_min, max: z.leds_max }).collect(),
            })
            .collect(),
        conflicts_active: ctx.active_conflicts,
        suspect_processes: conflicts::running_screen_software(),
        screens: ScreensDiag {
            detected: screens.list().iter().map(|s| format!("{} {} ({})", s.vendor, s.model, s.id)).collect(),
            scan_errors: screens.scan_errors().iter().map(|e| scrub(e)).collect(),
            plugged_but_idle: screens.plugged_but_idle(),
        },
        native_drivers_enabled: state.settings.lock().native_drivers_enabled,
        inventory_error,
        log_tail: log_tail(40),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_last_24h_of_log_lines_are_kept() {
        let now = line_epoch("2026-10-26T12:00:00Z").unwrap();
        let lines = vec![
            "[2026-10-23T09:00:00Z WARN  a] vieux".to_string(),
            "[2026-10-25T13:00:00Z ERROR a] récent".to_string(),
            "pas de date WARN".to_string(),
        ];
        assert_eq!(keep_recent(lines, now), vec!["[2026-10-25T13:00:00Z ERROR a] récent".to_string()]);
        assert_eq!(line_epoch("1970-01-02T00:00:00Z"), Some(86_400));
    }

    #[test]
    fn scrub_hides_the_account_name() {
        std::env::set_var("USERNAME", "JeanDupont");
        assert_eq!(scrub(r"échec C:\Users\JeanDupont\x"), r"échec C:\Users\<anonyme>\x");
    }
}
