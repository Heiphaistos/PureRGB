//! Test de compatibilité et de sécurité (Réglages) : vérifie, composant par composant et SANS rien écrire, que PureRGB
//! sait le piloter ; puis, à la demande et appareil par appareil, envoie une couleur très faible (2 écritures au total)
//! avant de remettre l'état d'origine. Une barrette de mémoire n'est jamais touchée sans l'autorisation d'écriture RAM.

use crate::core::{Color, DeviceInfo, DeviceType};
use crate::{conflicts, restore_saved_state, AppState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Au-delà, une écriture est signalée « lente » (bus saturé, source des délais dépassés).
const SLOW_WRITE: Duration = Duration::from_millis(500);
/// Durée pendant laquelle la couleur d'essai reste affichée.
const SHOW_FOR: Duration = Duration::from_millis(1500);

static TESTING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
pub struct Check {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

fn check(label: &str, ok: bool, detail: impl Into<String>) -> Check {
    Check { label: label.into(), ok, detail: detail.into() }
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub kind: String,
    /// « pilote intégré », « OpenRGB », « repli sur OpenRGB »…
    pub source: String,
    /// « standard » ou « élevé » (mémoire sur bus SMBus : une erreur d'écriture peut l'abîmer).
    pub risk: &'static str,
    pub can_test: bool,
    pub blocked: Option<String>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub ok: bool,
    pub steps: Vec<Check>,
    pub restored: String,
}

fn source_of(d: &DeviceInfo) -> String {
    if d.note.contains("repli automatique") {
        return "repli automatique sur OpenRGB".into();
    }
    match d.backend.as_str() {
        "hid" | "smbus" => "pilote intégré".into(),
        "openrgb" => "OpenRGB".into(),
        "liquidctl" => "liquidctl".into(),
        other => other.into(),
    }
}

fn risk_of(d: &DeviceInfo) -> &'static str {
    if d.device_type == DeviceType::Dram || d.backend == "smbus" {
        "élevé"
    } else {
        "standard"
    }
}

/// Pourquoi on refuse d'écrire sur cet appareil (None = autorisé).
fn blocked_reason(d: &DeviceInfo, ram_writes_allowed: bool) -> Option<String> {
    if !d.controllable {
        return Some("appareil détecté mais non pilotable (aucun pilote)".into());
    }
    if d.led_count == 0 {
        return Some("nombre de LEDs inconnu : rien à tester".into());
    }
    if risk_of(d) == "élevé" && !ram_writes_allowed {
        return Some("écriture sur la mémoire non autorisée : Réglages, Appareils, « Autoriser l'écriture sur la mémoire RAM »".into());
    }
    None
}

pub fn scan(state: &AppState) -> Vec<Item> {
    let ram_writes = state.settings.lock().smbus_writes_enabled;
    let pawnio = crate::backends::openrgb::manager::OpenRgbManager::pawnio_ready();
    let active: Vec<String> = conflicts::scan().0.into_iter().filter(|c| c.active).map(|c| c.name).collect();
    let devices = state.registry.lock().device_list();
    devices
        .iter()
        .map(|d| {
            let risk = risk_of(d);
            let blocked = blocked_reason(d, ram_writes);
            let mut checks = vec![
                check("Détecté", true, source_of(d)),
                check("Pilote prêt", d.controllable, if d.controllable { "oui" } else { "aucun pilote ne le contrôle" }),
                check("Nombre de LEDs connu", d.led_count > 0, format!("{} LED", d.led_count)),
                check(
                    "Aucun logiciel concurrent actif",
                    active.is_empty(),
                    if active.is_empty() { "ok".to_string() } else { active.join(", ") },
                ),
            ];
            if risk == "élevé" {
                checks.push(check("Pilote d'accès au bus (PawnIO) prêt", pawnio, if pawnio { "oui" } else { "non" }));
                checks.push(check("Écriture sur la mémoire autorisée", ram_writes, if ram_writes { "oui" } else { "non (réglage désactivé)" }));
            }
            Item {
                id: d.id.clone(),
                name: d.name.clone(),
                vendor: d.vendor.clone(),
                kind: format!("{:?}", d.device_type),
                source: source_of(d),
                risk,
                can_test: blocked.is_none(),
                blocked,
                checks,
            }
        })
        .collect()
}

/// Libère le verrou « un seul test à la fois » quoi qu'il arrive.
struct Release;
impl Drop for Release {
    fn drop(&mut self) {
        TESTING.store(false, Ordering::SeqCst);
    }
}

pub fn apply(state: &AppState, id: &str, confirmed: bool) -> Result<TestResult, String> {
    if !confirmed {
        return Err("confirmation requise".into());
    }
    if TESTING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("un test est déjà en cours".into());
    }
    let _release = Release;
    let (device, ram_writes, saved) = {
        let reg = state.registry.lock();
        let d = reg.get(id).cloned().ok_or_else(|| format!("appareil inconnu : {id}"))?;
        let s = state.settings.lock();
        (d, s.smbus_writes_enabled, s.clone())
    };
    if let Some(why) = blocked_reason(&device, ram_writes) {
        return Err(why);
    }

    // L'effet en cours est suspendu le temps du test (sinon il écraserait la couleur d'essai).
    state.engine.remove_effect(id);
    let n = device.led_count as usize;
    let mut steps = Vec::new();
    let mut ok = true;
    for (label, color) in [("Couleur d'essai (bleu très faible)", Color::new(0, 0, 40)), ("Extinction", Color::BLACK)] {
        let t = Instant::now();
        let r = state.registry.lock().set_colors(id, &vec![color; n]);
        let took = t.elapsed();
        match r {
            Ok(()) if took > SLOW_WRITE => steps.push(check(label, true, format!("accepté mais LENT ({} ms) : bus saturé ?", took.as_millis()))),
            Ok(()) => steps.push(check(label, true, format!("accepté en {} ms", took.as_millis()))),
            Err(e) => {
                steps.push(check(label, false, format!("{e:#}")));
                ok = false;
                break; // aucune autre écriture après une erreur
            }
        }
        if color != Color::BLACK {
            std::thread::sleep(SHOW_FOR);
        }
    }

    // Remise de l'état d'origine : effets enregistrés, sinon éteint.
    let had_effect = saved.effects.keys().any(|k| k == id || k.starts_with(&format!("{id}#z")));
    restore_saved_state(&state.registry, &state.engine, &saved);
    state.engine.invalidate();
    let restored = if had_effect {
        "effet enregistré remis".to_string()
    } else {
        "appareil laissé éteint (aucun effet enregistré)".to_string()
    };
    Ok(TestResult { ok, steps, restored })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(backend: &str, t: DeviceType, controllable: bool, leds: u32) -> DeviceInfo {
        DeviceInfo {
            id: format!("{backend}:1"),
            name: "x".into(),
            vendor: "v".into(),
            backend: backend.into(),
            device_type: t,
            zones: Vec::new(),
            led_count: leds,
            fan_channels: Vec::new(),
            settings: Vec::new(),
            controllable,
            has_lcd: false,
            modes: Vec::new(),
            active_mode: -1,
            note: String::new(),
        }
    }

    #[test]
    fn memory_is_high_risk_and_blocked_without_the_ram_write_setting() {
        let ram = dev("openrgb", DeviceType::Dram, true, 10);
        assert_eq!(risk_of(&ram), "élevé");
        assert!(blocked_reason(&ram, false).unwrap().contains("mémoire"));
        assert!(blocked_reason(&ram, true).is_none());
        let native = dev("smbus", DeviceType::Motherboard, true, 5);
        assert_eq!(risk_of(&native), "élevé", "tout ce qui passe par le bus SMBus est à risque");
    }

    #[test]
    fn other_devices_are_standard_risk_and_only_blocked_when_not_drivable() {
        let kb = dev("hid", DeviceType::Keyboard, true, 100);
        assert_eq!((risk_of(&kb), blocked_reason(&kb, false)), ("standard", None));
        assert!(blocked_reason(&dev("hid", DeviceType::Keyboard, false, 100), true).unwrap().contains("non pilotable"));
        assert!(blocked_reason(&dev("hid", DeviceType::Keyboard, true, 0), true).unwrap().contains("LEDs inconnu"));
    }

    #[test]
    fn source_names_are_clear_and_fallback_is_announced() {
        assert_eq!(source_of(&dev("hid", DeviceType::Mouse, true, 1)), "pilote intégré");
        assert_eq!(source_of(&dev("openrgb", DeviceType::Mouse, true, 1)), "OpenRGB");
        let mut d = dev("hid", DeviceType::Mouse, true, 1);
        d.note = "repli automatique sur OpenRGB (panne)".into();
        assert_eq!(source_of(&d), "repli automatique sur OpenRGB");
    }
}
