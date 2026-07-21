//! Table de reconnaissance distante (VID/PID ajoutés depuis le dashboard
//! télémétrie), fusionnée avec la table compilée `known::KNOWN_DEVICES`
//! au moment de l'affichage diagnostic UNIQUEMENT (`list_raw()`).
//!
//! Ne touche jamais `scan()` : un ajout distant ne doit jamais faire
//! apparaître une entrée fantôme non pilotable dans la grille Éclairage —
//! le vrai pilotage reste 100% OpenRGB, cette table n'améliore que
//! l'étiquetage "reconnu / non reconnu" du panneau diagnostic.

use parking_lot::RwLock;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteDevice {
    pub vid: String,
    pub pid: String,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub device_type: String,
    #[allow(dead_code)]
    pub vendor: String,
}

static REGISTRY: OnceLock<RwLock<HashMap<(String, String), RemoteDevice>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<(String, String), RemoteDevice>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Remplace le registre distant en mémoire (appelé après un fetch réussi
/// ou une relecture du cache local).
pub fn set_remote(devices: Vec<RemoteDevice>) {
    let map = devices
        .into_iter()
        .map(|d| ((d.vid.to_lowercase(), d.pid.to_lowercase()), d))
        .collect();
    *registry().write() = map;
}

/// Vrai si ce VID/PID a été ajouté depuis le dashboard — utilisé
/// uniquement pour le champ `recognized` du diagnostic.
pub fn is_known_remote(vid: u16, pid: u16) -> bool {
    let key = (format!("{vid:04x}"), format!("{pid:04x}"));
    registry().read().contains_key(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_un_appareil_ajoute_a_distance() {
        set_remote(vec![RemoteDevice {
            vid: "dead".into(),
            pid: "beef".into(),
            name: "Test".into(),
            device_type: "hub".into(),
            vendor: "Test".into(),
        }]);
        assert!(is_known_remote(0xDEAD, 0xBEEF));
        assert!(!is_known_remote(0x1234, 0x5678));
    }

    #[test]
    fn set_remote_remplace_completement_le_registre_precedent() {
        set_remote(vec![RemoteDevice {
            vid: "0001".into(),
            pid: "0001".into(),
            name: "A".into(),
            device_type: "hub".into(),
            vendor: "".into(),
        }]);
        assert!(is_known_remote(0x0001, 0x0001));
        set_remote(vec![]);
        assert!(!is_known_remote(0x0001, 0x0001));
    }

    #[test]
    fn set_remote_normalise_cas_vid_pid() {
        set_remote(vec![RemoteDevice {
            vid: "DEAD".into(),
            pid: "BEEF".into(),
            name: "UpperCase".into(),
            device_type: "hub".into(),
            vendor: "Test".into(),
        }]);
        // Uppercase input should match lowercase lookup
        assert!(is_known_remote(0xDEAD, 0xBEEF));

        set_remote(vec![RemoteDevice {
            vid: "DeAd".into(),
            pid: "BeEf".into(),
            name: "MixedCase".into(),
            device_type: "hub".into(),
            vendor: "Test".into(),
        }]);
        // Mixed case input should also match
        assert!(is_known_remote(0xDEAD, 0xBEEF));
    }
}
