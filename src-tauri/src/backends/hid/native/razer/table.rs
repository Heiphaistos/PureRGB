//! Table des modèles Razer revendiqués.
//! Source : docs/rgb-protocols/razer.md (§ Interface HID à ouvrir) et docs/rgb-protocols/signalrgb.md (§ 6.1 Razer, 1.2 et 1.3),
//! signalrgb-devices.csv (familles `razer-modern-keyboard` / `razer-modern-mouse`).
//!
//! Seuls les PID dont les notes donnent interface + disposition sont listés. Tous les autres PID Razer sont omis :
//! transaction_id (R), interface par modèle ou dimensions de matrice INCONNUS. Le transaction_id n'est PAS dans la table :
//! il est sondé à l'ouverture (signalrgb.md 1.1).

use crate::core::DeviceType;

pub struct Entry {
    pub pid: u16,
    pub name: &'static str,
    pub device_type: DeviceType,
    /// Lignes de la matrice (une commande par ligne).
    pub rows: u8,
    /// Cellules par ligne (= LEDs par paquet).
    pub cols: u8,
    pub interface: i32,
    pub usage_page: u16,
}

// Claviers pleine taille : matrice 6 x 22 (razer.md « 6 lignes pour 22 colonnes », signalrgb.md « 22 pour la plupart des claviers pleine taille »).
// Association touche -> cellule : INCONNU (vKeys du plugin non extraites) ; la zone exposée est donc la matrice brute, ligne par ligne.
// Interface 2 / usage page 0x01 : BlackWidow Chroma, V2, Elite (razer.md, OpenRGB) ; BlackWidow V3 : interface 3 / usage page 0x0C.
const fn kbd(pid: u16, name: &'static str, interface: i32, usage_page: u16) -> Entry {
    Entry { pid, name, device_type: DeviceType::Keyboard, rows: 6, cols: 22, interface, usage_page }
}

// Souris modernes : interface 3 / usage page 0x0C (signalrgb.md 1.2 ; razer.md : « 0x03 pour Naga X / Basilisk V3 / V3 35K »).
// Une seule ligne de `leds` LEDs = nombre de LEDs du CSV (signalrgb.md 1.3 « ligne unique »).
const fn mouse(pid: u16, name: &'static str, leds: u8) -> Entry {
    Entry { pid, name, device_type: DeviceType::Mouse, rows: 1, cols: leds, interface: 3, usage_page: 0x000C }
}

pub const MODELS: &[Entry] = &[
    kbd(0x0203, "Razer BlackWidow Chroma", 2, 0x0001),
    kbd(0x0221, "Razer BlackWidow Chroma V2", 2, 0x0001),
    kbd(0x0228, "Razer BlackWidow Elite", 2, 0x0001),
    kbd(0x024E, "Razer BlackWidow V3", 3, 0x000C),
    mouse(0x0096, "Razer Naga X", 2),
    mouse(0x0099, "Razer Basilisk V3", 11),
    mouse(0x00CB, "Razer Basilisk V3 35K", 11),
];

pub fn by_pid(pid: u16) -> Option<&'static Entry> {
    MODELS.iter().find(|e| e.pid == pid)
}

pub fn by_name(name: &str) -> Option<&'static Entry> {
    MODELS.iter().find(|e| e.name == name)
}
