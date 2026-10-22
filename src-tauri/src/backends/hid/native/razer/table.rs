//! Table des modèles Razer revendiqués.
//! Source : docs/rgb-protocols/razer.md (§ Interface HID à ouvrir) et docs/rgb-protocols/signalrgb.md (§ 6.1 Razer, 1.2 et 1.3),
//! signalrgb-devices.csv (familles `razer-modern-keyboard` / `razer-modern-mouse`) et docs/rgb-protocols/analyse-razer-synapse.md
//! (§ 4 claviers, § 5 souris, § 6 tapis/accessoires : transaction_id, matrice, interface/page d'usage).
//!
//! Seuls les PID dont les notes donnent interface + disposition sont listés. Tous les autres PID Razer sont omis :
//! matrices STA/LIN/CUSTOM (anciens modèles), dongles et appareils sans fil, Bluetooth, Kraken, Hanbo, ARGB 0F1F, ainsi que
//! tout PID dont une donnée est INCONNUE. Les entrées de la première livraison n'ont pas de transaction_id (`tid: None`) :
//! il est sondé à l'ouverture (signalrgb.md 1.1) ; les entrées de analyse-razer-synapse.md ont le leur, sans sondage.
//! Matrice = celle d'OpenRGB (ligne de conduite de la note : cellules en trop = inactives).
//! INCONNU : usage HID (le champ `usage` n'est pas filtrable par `Model`) ; ordre des zones sur le fil = ordre de la note.

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
    /// `None` = indifférent (BlackWidow V4 : page 0x01 ou 0x0C selon le firmware).
    pub usage_page: Option<u16>,
    /// Transaction id documenté ; `None` = sondé à l'ouverture.
    pub tid: Option<u8>,
    /// Découpage de la matrice en zones (nom, cellules), dans l'ordre du fil ; vide = une seule zone.
    pub zones: &'static [(&'static str, u16)],
    /// Mode appareil « logiciel » (0x03) à envoyer avant la détection (Charging Pad : note § 3).
    pub soft_mode: bool,
}

// Claviers pleine taille : matrice 6 x 22 (razer.md « 6 lignes pour 22 colonnes », signalrgb.md « 22 pour la plupart des claviers pleine taille »).
// Association touche -> cellule : INCONNU (vKeys du plugin non extraites) ; la zone exposée est donc la matrice brute, ligne par ligne.
// Interface 2 / usage page 0x01 : BlackWidow Chroma, V2, Elite (razer.md, OpenRGB) ; BlackWidow V3 : interface 3 / usage page 0x0C.
const fn kbd(pid: u16, name: &'static str, interface: i32, usage_page: u16) -> Entry {
    Entry { pid, name, device_type: DeviceType::Keyboard, rows: 6, cols: 22, interface, usage_page: Some(usage_page), tid: None, zones: &[], soft_mode: false }
}

// Souris modernes : interface 3 / usage page 0x0C (signalrgb.md 1.2 ; razer.md : « 0x03 pour Naga X / Basilisk V3 / V3 35K »).
// Une seule ligne de `leds` LEDs = nombre de LEDs du CSV (signalrgb.md 1.3 « ligne unique »).
const fn mouse(pid: u16, name: &'static str, leds: u8) -> Entry {
    Entry { pid, name, device_type: DeviceType::Mouse, rows: 1, cols: leds, interface: 3, usage_page: Some(0x000C), tid: None, zones: &[], soft_mode: false }
}

// Zones OpenRGB de la note (analyse-razer-synapse.md § 5), dans l'ordre de la note : logo 1, molette 1, bande 9.
const BASILISK_V3: &[(&str, u16)] = &[("Logo", 1), ("Molette", 1), ("Bande LED", 9)];

const fn mouse_z(pid: u16, name: &'static str, leds: u8, zones: &'static [(&'static str, u16)]) -> Entry {
    Entry { zones, ..mouse(pid, name, leds) }
}

/// Entrée de analyse-razer-synapse.md : matrice étendue (classe 0x0F), TID connu.
const fn ext(pid: u16, name: &'static str, device_type: DeviceType, tid: u8, rows: u8, cols: u8, interface: i32, usage_page: Option<u16>) -> Entry {
    Entry { pid, name, device_type, rows, cols, interface, usage_page, tid: Some(tid), zones: &[], soft_mode: false }
}

/// Clavier de analyse-razer-synapse.md § 4 (page d'usage 0x01 si interface 2, 0x0C si interface 3).
const fn kb(pid: u16, name: &'static str, tid: u8, rows: u8, cols: u8, interface: i32) -> Entry {
    let page = if interface == 2 { 0x0001 } else { 0x000C };
    ext(pid, name, DeviceType::Keyboard, tid, rows, cols, interface, Some(page))
}

/// Accessoire 1 x N sur l'interface 0 / page 0x01 (analyse-razer-synapse.md § 6, colonne IPU `00 01/02`).
const fn acc(pid: u16, name: &'static str, device_type: DeviceType, tid: u8, rows: u8, cols: u8) -> Entry {
    ext(pid, name, device_type, tid, rows, cols, 0, Some(0x0001))
}

// Huntsman Elite / V2 Analog : 9 x 22 = clavier 6 x 22 (132) + « underglow » 3 x 22 (66), ligne par ligne.
const HUNTSMAN_9X22: &[(&str, u16)] = &[("Clavier", 132), ("Sous-éclairage", 66)];

pub const MODELS: &[Entry] = &[
    kbd(0x0203, "Razer BlackWidow Chroma", 2, 0x0001),
    kbd(0x0221, "Razer BlackWidow Chroma V2", 2, 0x0001),
    kbd(0x0228, "Razer BlackWidow Elite", 2, 0x0001),
    kbd(0x024E, "Razer BlackWidow V3", 3, 0x000C),
    mouse(0x0096, "Razer Naga X", 2),
    mouse_z(0x0099, "Razer Basilisk V3", 11, BASILISK_V3),
    mouse_z(0x00CB, "Razer Basilisk V3 35K", 11, BASILISK_V3),
    // Souris « razer-modern-mouse » à nombre de LEDs explicite (signalrgb-devices.csv). Dongles sans fil, formes legacy
    // (0x0053, 0x0046, 0x0044), Hyperflux (0x0068) et LEDs INCONNUES omis ; une souris sans matrice moderne est refusée à l'ouverture.
    mouse(0x004D, "Razer DeathAdder Essential", 2),
    mouse(0x005C, "Razer DeathAdder Elite", 3),
    mouse(0x0059, "Razer Lancehead Tournament Edition", 16),
    mouse(0x0060, "Razer Lancehead Tournament Edition (0060)", 16),
    mouse(0x0064, "Razer Basilisk", 2),
    mouse(0x0065, "Razer Basilisk Essential", 1),
    mouse(0x0067, "Razer Naga Trinity", 3),
    mouse(0x0069, "Razer Mamba", 2),
    mouse(0x006B, "Razer Abyssus Essential", 3),
    mouse_z(0x006C, "Razer Mamba Elite", 20, &[("Molette", 1), ("Logo", 1), ("Gauche", 9), ("Droite", 9)]),
    mouse(0x006F, "Razer Lancehead", 3),
    mouse(0x0070, "Razer Lancehead (0070)", 3),
    mouse(0x0072, "Razer Mamba (0072)", 2),
    mouse(0x0073, "Razer Mamba (0073)", 2),
    mouse(0x0078, "Razer Viper", 1),
    mouse(0x0084, "Razer DeathAdder V2", 2),
    mouse(0x0085, "Razer Basilisk V2", 2),
    mouse(0x008A, "Razer Viper Mini", 1),
    mouse(0x008C, "Razer DeathAdder V2 Mini", 1),
    mouse(0x008D, "Razer Naga Left-Handed", 3),
    mouse(0x0091, "Razer Viper 8KHz", 1),
    mouse(0x0093, "Razer Naga Classic", 3),
    mouse(0x00A3, "Razer Cobra", 10),
    mouse(0x00B9, "Razer Basilisk V3 X HyperSpeed", 1),
    // --- analyse-razer-synapse.md § 4 : claviers filaires, matrice étendue, TID connu (pas de sondage). ---
    // Omis : STA (BlackWidow Chroma TE/Overwatch/X, Core), LIN (DeathStalker Chroma), Cynosa Lite (1x1 vs 2x2 Synapse), Tartarus
    // (interface INCONNUE), sans fil / dongle / Bluetooth (0271, 0296, 0290, 02B4, 02D5, 02D2, 025C, 025B).
    kb(0x0241, "Razer BlackWidow 2019", 0x1F, 6, 22, 2),
    kb(0x025A, "Razer BlackWidow V3 Pro (filaire)", 0x3F, 6, 22, 2),
    kb(0x0A24, "Razer BlackWidow V3 TKL", 0x3F, 6, 18, 2),
    kb(0x0258, "Razer BlackWidow V3 Mini (filaire)", 0x1F, 5, 16, 3),
    // BlackWidow V4 / V4 Pro / V4 75 % filaire : page 0x01 (usage 0) si firmware < 1.5, 0x0C sinon -> page indifférente.
    ext(0x0287, "Razer BlackWidow V4", DeviceType::Keyboard, 0x1F, 8, 23, 3, None),
    ext(0x028D, "Razer BlackWidow V4 Pro", DeviceType::Keyboard, 0x1F, 8, 23, 3, None),
    ext(0x02A5, "Razer BlackWidow V4 75% (filaire)", DeviceType::Keyboard, 0x1F, 6, 18, 3, None),
    kb(0x02B3, "Razer BlackWidow V4 Pro 75% (filaire)", 0x1F, 6, 18, 3),
    kb(0x0293, "Razer BlackWidow V4 X", 0x1F, 6, 22, 2),
    kb(0x02D7, "Razer BlackWidow V4 TKL (filaire)", 0x1F, 6, 18, 3),
    kb(0x02D4, "Razer BlackWidow V4 Low Profile TKL (filaire)", 0x1F, 6, 18, 3),
    kb(0x022A, "Razer Cynosa Chroma", 0x3F, 6, 22, 2),
    kb(0x025E, "Razer Cynosa V2", 0x1F, 6, 22, 2),
    kb(0x021E, "Razer Ornata Chroma", 0x3F, 6, 22, 2),
    kb(0x025D, "Razer Ornata Chroma V2", 0x1F, 6, 22, 2),
    kb(0x028F, "Razer Ornata V3", 0x1F, 1, 10, 2),
    kb(0x02A1, "Razer Ornata V3 rev2", 0x1F, 1, 10, 2),
    kb(0x02A3, "Razer Ornata V3 TKL", 0x1F, 1, 8, 2),
    kb(0x0294, "Razer Ornata V3 X", 0x1F, 1, 1, 2),
    kb(0x02A2, "Razer Ornata V3 X Rev2", 0x1F, 1, 1, 2),
    kb(0x0295, "Razer DeathStalker V2", 0x1F, 6, 22, 3),
    kb(0x0298, "Razer DeathStalker V2 Pro TKL (filaire)", 0x1F, 6, 17, 3),
    kb(0x0292, "Razer DeathStalker V2 Pro (filaire)", 0x9F, 6, 22, 3),
    kb(0x0227, "Razer Huntsman", 0x3F, 6, 22, 2),
    Entry { zones: HUNTSMAN_9X22, ..kb(0x0226, "Razer Huntsman Elite", 0x3F, 9, 22, 2) },
    Entry { zones: HUNTSMAN_9X22, ..kb(0x0266, "Razer Huntsman V2 Analog", 0x3F, 9, 22, 3) },
    kb(0x0257, "Razer Huntsman Mini", 0x3F, 5, 15, 2),
    kb(0x0282, "Razer Huntsman Mini Analog", 0x1F, 5, 15, 3),
    kb(0x0243, "Razer Huntsman Tournament Edition", 0x3F, 6, 18, 2),
    kb(0x026B, "Razer Huntsman V2 TKL", 0x3F, 6, 17, 3),
    kb(0x026C, "Razer Huntsman V2", 0x3F, 6, 22, 3),
    kb(0x02A6, "Razer Huntsman V3 Pro", 0x3F, 6, 22, 3),
    kb(0x02A7, "Razer Huntsman V3 Pro TKL White", 0x3F, 6, 19, 3),
    kb(0x02E4, "Razer Huntsman V3 HE Magnetic Mini", 0x3F, 6, 22, 3),
    // --- analyse-razer-synapse.md § 6 : tapis et accessoires, matrice étendue. Omis : LIN/STA (Firefly 0C00, Core 0215, Mug 0F07),
    // Hyperflux 0068, Mouse Dock Pro 00A4 (aussi récepteur sans fil), O11 0F13 (4x16 vs 24x3), Seiren 0F1B, docks Thunderbolt,
    // Leviathan (report-id 7), Nommo, Tiamat, Kraken, Hanbo, ARGB 0F1F/0F2D/0F2C, moniteurs. ---
    acc(0x0C04, "Razer Firefly V2", DeviceType::Mousemat, 0x3F, 1, 19),
    acc(0x0C08, "Razer Firefly V2 Pro", DeviceType::Mousemat, 0x1F, 1, 17),
    acc(0x0C01, "Razer Goliathus Chroma", DeviceType::Mousemat, 0x3F, 1, 1),
    acc(0x0C02, "Razer Goliathus Chroma Extended", DeviceType::Mousemat, 0x3F, 1, 1),
    acc(0x0C06, "Razer Goliathus Chroma 3XL", DeviceType::Mousemat, 0x1F, 1, 1),
    acc(0x0C05, "Razer Strider Chroma", DeviceType::Mousemat, 0x3F, 1, 19),
    acc(0x0F08, "Razer Base Station Chroma", DeviceType::HeadsetStand, 0x3F, 1, 15),
    acc(0x0F20, "Razer Base Station V2 Chroma", DeviceType::HeadsetStand, 0x1F, 1, 8),
    ext(0x0F1A, "Razer Core X Chroma", DeviceType::Accessory, 0x3F, 1, 16, 2, Some(0x0001)),
    ext(0x0F09, "Razer Chroma HDK", DeviceType::Accessory, 0x3F, 4, 16, 2, Some(0x0001)),
    ext(0x0F0E, "Razer PC Case Lighting Kit", DeviceType::Case, 0x3F, 2, 16, 2, Some(0x0001)),
    acc(0x0F0D, "Razer Laptop Stand Chroma", DeviceType::Accessory, 0x1F, 1, 15),
    acc(0x0F2B, "Razer Laptop Stand Chroma V2", DeviceType::Accessory, 0x1F, 1, 15),
    acc(0x007E, "Razer Mouse Dock Chroma", DeviceType::Accessory, 0x3F, 1, 1),
    acc(0x0F1D, "Razer Mouse Bungee V3 Chroma", DeviceType::Accessory, 0x1F, 1, 8),
    Entry { soft_mode: true, ..ext(0x0F26, "Razer Charging Pad Chroma", DeviceType::Accessory, 0x1F, 1, 10, 0, Some(0x000C)) },
    ext(0x0F19, "Razer Kitty Edition", DeviceType::Headset, 0x1F, 1, 4, 1, Some(0x0001)),
];

pub fn by_pid(pid: u16) -> Option<&'static Entry> {
    MODELS.iter().find(|e| e.pid == pid)
}

pub fn by_name(name: &str) -> Option<&'static Entry> {
    MODELS.iter().find(|e| e.name == name)
}
