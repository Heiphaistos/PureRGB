//! Cadre des drivers HID natifs : une famille de protocole = UN fichier de ce dossier.
//!
//! Chaque famille déclare quels VID/PID elle gère (`probe`, avec la disposition LED/ventilateurs du modèle) et comment
//! l'ouvrir (`open`). Les drivers ne parlent qu'au trait [`Hid`] (pas à `hidapi` directement) : les tests d'octets
//! utilisent [`MockHid`] et vérifient les trames construites à la main d'après la doc (`docs/rgb-protocols/`).
//! Tous sont expérimentaux (`verified: false`) tant qu'ils n'ont pas tourné sur le vrai matériel.

use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

pub mod asrock_polychrome;
pub mod asus_aura;
pub mod coolermaster;
pub mod corsair_commander;
pub mod corsair_core;
pub mod dualsense;
pub mod dualshock4;
pub mod gigabyte_fusion;
pub mod hyperx;
pub mod lianli_uni;
pub mod logitech;
pub mod msi_mystic;
pub mod nzxt_hue2;
pub mod razer;
pub mod roccat;
mod sony_crc;
pub mod steelseries;
pub mod thermaltake;
pub mod wooting;

/// Accès bas niveau à une interface HID (implémenté par `hidapi::HidDevice` et par `MockHid` en test).
pub trait Hid: Send {
    /// Écriture d'un rapport de sortie : `data[0]` = identifiant de rapport (0 si non numéroté).
    fn write(&self, data: &[u8]) -> Result<usize>;
    /// Rapport de fonctionnalité (« feature report ») : `data[0]` = identifiant de rapport.
    fn send_feature_report(&self, data: &[u8]) -> Result<()>;
    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize>;
    /// Lecture d'un rapport d'entrée, 0 si le délai expire.
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize>;
    /// Seconde collection HID du même appareil (voir `Model::aux`), pour les protocoles qui séparent contrôle et LEDs.
    fn aux(&self) -> Option<&dyn Hid> {
        None
    }
}

/// Une interface principale (`Hid`) accompagnée de sa seconde collection : les écritures du pilote vont à la première,
/// `aux()` donne la seconde.
pub struct PairedHid {
    pub main: Box<dyn Hid>,
    pub second: Box<dyn Hid>,
}

impl Hid for PairedHid {
    fn write(&self, data: &[u8]) -> Result<usize> {
        self.main.write(data)
    }
    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.main.send_feature_report(data)
    }
    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize> {
        self.main.get_feature_report(buf)
    }
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        self.main.read_timeout(buf, timeout_ms)
    }
    fn aux(&self) -> Option<&dyn Hid> {
        Some(self.second.as_ref())
    }
}

/// Sélection d'une collection HID (None = indifférent).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HidSelect {
    pub interface: Option<i32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
}

impl Hid for hidapi::HidDevice {
    fn write(&self, data: &[u8]) -> Result<usize> {
        Ok(hidapi::HidDevice::write(self, data)?)
    }
    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        Ok(hidapi::HidDevice::send_feature_report(self, data)?)
    }
    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize> {
        Ok(hidapi::HidDevice::get_feature_report(self, buf)?)
    }
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        Ok(hidapi::HidDevice::read_timeout(self, buf, timeout_ms)?)
    }
}

/// Un modèle reconnu : nom affiché, type, disposition (zones LED dans l'ordre où `set_colors` découpe le tampon).
#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub device_type: DeviceType,
    pub zones: Vec<ZoneInfo>,
    /// Nombre de canaux ventilateur pilotables (0 si aucun).
    pub fans: u8,
    /// Interface HID à ouvrir (`interface_number`), `None` = la première.
    pub interface: Option<i32>,
    /// Page d'usage HID à ouvrir (ex. 0xFF60), `None` = indifférent.
    pub usage_page: Option<u16>,
    /// Usage HID à ouvrir au sein de la page (ex. 0x0602), `None` = indifférent.
    pub usage: Option<u16>,
    /// Seconde collection HID à ouvrir en plus (protocoles à deux interfaces : contrôle + LEDs) ; le pilote
    /// la reçoit par `Hid::aux()`. `None` = une seule collection.
    pub aux: Option<HidSelect>,
    /// Appareil joint en Bluetooth (renseigné à l'ouverture par le backend) : certains protocoles changent de format de rapport.
    pub bluetooth: bool,
}

impl Model {
    pub fn new(name: impl Into<String>, device_type: DeviceType, zones: Vec<ZoneInfo>) -> Model {
        Model { name: name.into(), device_type, zones, fans: 0, interface: None, usage_page: None, usage: None, aux: None, bluetooth: false }
    }

    pub fn led_count(&self) -> u32 {
        self.zones.iter().map(|z| z.led_count).sum()
    }
}

/// Un appareil natif ouvert.
pub trait NativeRgb: Send {
    /// Couleurs de TOUTES les LEDs, zones à la suite dans l'ordre de `Model::zones`.
    fn set_colors(&mut self, colors: &[Color]) -> Result<()>;

    fn set_fan_duty(&mut self, _channel: u8, _percent: u8) -> Result<()> {
        bail!("contrôle ventilateur non supporté sur cet appareil")
    }

    /// Remise en état d'origine à la fermeture (mode matériel / BIOS), si le protocole le permet.
    fn release(&mut self) {}
}

/// Une famille de protocole.
pub struct Family {
    pub id: &'static str,
    /// Note de protocole dont le driver est issu (`docs/rgb-protocols/…`).
    pub doc: &'static str,
    pub probe: fn(vid: u16, pid: u16) -> Option<Model>,
    pub open: fn(Box<dyn Hid>, &Model) -> Result<Box<dyn NativeRgb>>,
}

pub const FAMILIES: &[&Family] = &[
    &asrock_polychrome::FAMILY,
    &asus_aura::FAMILY,
    &coolermaster::FAMILY,
    &corsair_commander::FAMILY,
    &corsair_core::FAMILY,
    &dualsense::FAMILY,
    &dualshock4::FAMILY,
    &gigabyte_fusion::FAMILY,
    &hyperx::FAMILY,
    &lianli_uni::FAMILY,
    &logitech::FAMILY,
    &msi_mystic::FAMILY,
    &nzxt_hue2::FAMILY,
    &razer::FAMILY,
    &roccat::FAMILY,
    &steelseries::FAMILY,
    &thermaltake::FAMILY,
    &wooting::FAMILY,
];

/// Famille + modèle pour ce VID/PID, si un driver natif existe.
pub fn probe(vid: u16, pid: u16) -> Option<(&'static Family, Model)> {
    FAMILIES.iter().find_map(|f| (f.probe)(vid, pid).map(|m| (*f, m)))
}

/// Découpe `colors` selon `zones` (LEDs manquantes complétées en noir) : utile aux drivers multi-canaux.
pub fn split_by_zone(colors: &[Color], zones: &[ZoneInfo]) -> Vec<Vec<Color>> {
    let mut at = 0usize;
    zones
        .iter()
        .map(|z| {
            let n = z.led_count as usize;
            let part = (0..n).map(|i| colors.get(at + i).copied().unwrap_or(Color::BLACK)).collect();
            at += n;
            part
        })
        .collect()
}

#[cfg(test)]
type Log = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

/// Faux périphérique HID : enregistre les trames envoyées et rejoue des réponses préparées.
#[cfg(test)]
pub struct MockHid {
    sent: Log,
    features: Log,
    replies: std::sync::Arc<parking_lot::Mutex<std::collections::VecDeque<Vec<u8>>>>,
    second: Option<Box<MockHid>>,
}

#[cfg(test)]
impl MockHid {
    /// Rend `(mock, rapports_écrits, features_envoyées)` ; `replies` est servi tour à tour par `read_timeout` et `get_feature_report`.
    pub fn new(replies: Vec<Vec<u8>>) -> (MockHid, Log, Log) {
        let sent: Log = Default::default();
        let features: Log = Default::default();
        let mock = MockHid { sent: sent.clone(), features: features.clone(), replies: std::sync::Arc::new(parking_lot::Mutex::new(replies.into())), second: None };
        (mock, sent, features)
    }

    /// Ajoute une seconde collection simulée (renvoyée par `aux()`).
    pub fn with_aux(mut self, second: MockHid) -> MockHid {
        self.second = Some(Box::new(second));
        self
    }

    fn next_reply(&self, buf: &mut [u8]) -> usize {
        match self.replies.lock().pop_front() {
            Some(r) => {
                let n = r.len().min(buf.len());
                buf[..n].copy_from_slice(&r[..n]);
                n
            }
            None => 0,
        }
    }
}

#[cfg(test)]
impl Hid for MockHid {
    fn write(&self, data: &[u8]) -> Result<usize> {
        self.sent.lock().push(data.to_vec());
        Ok(data.len())
    }
    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.features.lock().push(data.to_vec());
        Ok(())
    }
    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.next_reply(buf))
    }
    fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        Ok(self.next_reply(buf))
    }
    fn aux(&self) -> Option<&dyn Hid> {
        self.second.as_deref().map(|m| m as &dyn Hid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_second_collection_is_reachable_and_independent() {
        let (aux, aux_sent, _) = MockHid::new(vec![]);
        let (main, main_sent, _) = MockHid::new(vec![]);
        let hid = main.with_aux(aux);
        hid.write(&[1, 2]).unwrap();
        hid.aux().expect("seconde collection").write(&[9, 9, 9]).unwrap();
        assert_eq!(main_sent.lock().as_slice(), [vec![1, 2]]);
        assert_eq!(aux_sent.lock().as_slice(), [vec![9, 9, 9]]);
        let (plain, _, _) = MockHid::new(vec![]);
        assert!(plain.aux().is_none(), "sans seconde collection, aux() vaut None");
    }

    #[test]
    fn split_by_zone_pads_missing_leds_with_black() {
        let zones = vec![ZoneInfo::fixed("a", 2), ZoneInfo::fixed("b", 3)];
        let c = [Color::new(1, 1, 1), Color::new(2, 2, 2), Color::new(3, 3, 3)];
        let parts = split_by_zone(&c, &zones);
        assert_eq!(parts[0], vec![Color::new(1, 1, 1), Color::new(2, 2, 2)]);
        assert_eq!(parts[1], vec![Color::new(3, 3, 3), Color::BLACK, Color::BLACK]);
    }

    #[test]
    fn family_ids_are_unique_and_no_id_is_claimed_twice() {
        let mut ids: Vec<&str> = FAMILIES.iter().map(|f| f.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), FAMILIES.len());
        // Aucun couple VID/PID ne doit être revendiqué par deux familles.
        for vid in [0x0B05u16, 0x1462, 0x1532, 0x26CE, 0x048D, 0x1038, 0x0951, 0x03F0, 0x046D, 0x31E3, 0x264A, 0x1B1C, 0x0CF2, 0x1E71, 0x2516, 0x1E7D, 0x10F5, 0x054C] {
            for pid in 0..=u16::MAX {
                let n = FAMILIES.iter().filter(|f| (f.probe)(vid, pid).is_some()).count();
                assert!(n <= 1, "{vid:04x}:{pid:04x} revendiqué par {n} familles");
            }
        }
    }
}
