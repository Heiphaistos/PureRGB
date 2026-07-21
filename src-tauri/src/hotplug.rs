//! Détection des nouveaux appareils USB non reconnus, par sondage périodique
//! de `HidBackend::list_raw()` (comparaison d'ensemble entre deux sondages).
//! Le pilotage réel reste inchangé — ce module ne fait que repérer les
//! nouveautés pour proposer un diagnostic, jamais pour piloter quoi que ce soit.

use crate::backends::hid::RawHidDevice;
use std::collections::HashSet;

/// Renvoie les appareils de `current` absents de `previous` (par VID/PID) et
/// non reconnus (`recognized == false`) — ceux qui méritent une notification.
pub fn diff_new_unrecognized(
    previous: &HashSet<(String, String)>,
    current: &[RawHidDevice],
) -> Vec<RawHidDevice> {
    current
        .iter()
        .filter(|d| !d.recognized && !previous.contains(&(d.vid.clone(), d.pid.clone())))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(vid: &str, pid: &str, recognized: bool) -> RawHidDevice {
        RawHidDevice {
            vid: vid.into(),
            pid: pid.into(),
            manufacturer: "Test".into(),
            product: "Device".into(),
            recognized,
            has_native_driver: false,
        }
    }

    #[test]
    fn detecte_un_nouvel_appareil_non_reconnu() {
        let previous = HashSet::new();
        let current = vec![device("dead", "beef", false)];
        let result = diff_new_unrecognized(&previous, &current);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ignore_un_appareil_deja_vu() {
        let mut previous = HashSet::new();
        previous.insert(("dead".to_string(), "beef".to_string()));
        let current = vec![device("dead", "beef", false)];
        let result = diff_new_unrecognized(&previous, &current);
        assert!(result.is_empty());
    }

    #[test]
    fn ignore_un_nouvel_appareil_reconnu() {
        let previous = HashSet::new();
        let current = vec![device("1b1c", "0c0b", true)];
        let result = diff_new_unrecognized(&previous, &current);
        assert!(result.is_empty());
    }

    #[test]
    fn regroupe_plusieurs_nouveaux_appareils() {
        let previous = HashSet::new();
        let current = vec![
            device("dead", "beef", false),
            device("1b1c", "0c0b", true),
            device("cafe", "babe", false),
        ];
        let result = diff_new_unrecognized(&previous, &current);
        assert_eq!(result.len(), 2);
    }
}
