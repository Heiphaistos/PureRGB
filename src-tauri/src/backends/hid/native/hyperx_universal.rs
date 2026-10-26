//! Famille « hyperx_universal » : protocole « Universal » des souris Pulsefire Haste 2 (rapports de sortie de 64 octets, `[0]` = identifiant de
//! rapport, `[1]` = sous-rapport). Lumière directe (1 LED) et deux réglages en RAM. Rien n'est écrit dans un profil de l'appareil.
//!
//! Source : docs/rgb-protocols/hyperx-ngenuity.md (analyse de HyperX NGENUITY 5.38, 2026-10-26, marqueur [V]). Aucun code copié.
//!
//! - Lumière directe, identifiant `44` : `44 01 <n>` (n = nombre de rapports de LED, ici 1) puis `44 02 <i> 00 R V B ...` (20 LED par rapport à
//!   partir de l'octet 4, i = numéro du rapport). Envoyé à chaque image par NGENUITY.
//! - Luminosité, identifiant `40` : `40 01 <masque profils u16 petit-boutiste> <niveau>`. Masque 0 = « préréglage applicatif » : NGENUITY l'emploie
//!   pour chaque mouvement du curseur et réserve les vrais numéros de profil à l'envoi vers la mémoire de l'appareil. Niveau = 255 x pourcentage / 100.
//! - Seuil de batterie faible, identifiant `50` : `50 01 <seuil %> <enregistrer>`, avec enregistrer = 0 (NGENUITY le met à 1 seulement lors de la
//!   synchronisation vers l'appareil). Plage 5..25 %, 10 par défaut côté NGENUITY. Modèles sans fil seulement.
//!
//! INCONNU (non implémenté) : DPI, fréquence d'interrogation, boutons, macros (ce sont des tables complètes dont l'état courant n'est pas relu),
//! lecture de la batterie (le cadre des réglages n'a pas de valeur en lecture seule), Haste 2 sans fil première génération (trame de LED différente),
//! claviers Alloy Rise (nombre de rapports dépendant de la disposition lue sur l'appareil), enfants des récepteurs 3-en-1.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceSetting, DeviceType, SettingKind, ZoneInfo};
use anyhow::{bail, Result};

const VID_HP: u16 = 0x03F0;
const REPORT_LEN: usize = 64;
const PER_REPORT: usize = 20;
const ID_LIGHTING_DIRECT: u8 = 0x44;
const ID_BRIGHTNESS: u8 = 0x40;
const ID_BATTERY: u8 = 0x50;

/// (PID, nom, sans fil ?, interface HID). Une seule LED sur tous ces modèles.
const MODELS: &[(u16, &str, bool, i32)] = &[
    (0x0B97, "HyperX Pulsefire Haste 2", false, 2),
    (0x0BA0, "HyperX Pulsefire Haste 2 Mini Wireless", true, 2),
    (0x0DA0, "HyperX Pulsefire Haste 2 Mini Wireless (récepteur)", true, 2),
    (0x0AB8, "HyperX Pulsefire Haste 2 S", true, 2),
    (0x0BB8, "HyperX Pulsefire Haste 2 S (récepteur)", true, 2),
    (0x0ABD, "HyperX Pulsefire Haste 2 Pro", true, 2),
    (0x0CBD, "HyperX Pulsefire Haste 2 Pro (récepteur)", true, 3),
    (0x0AB5, "HyperX Pulsefire Haste 2 Core Wireless (récepteur)", true, 2),
];

fn entry(name: &str) -> Option<&'static (u16, &'static str, bool, i32)> {
    MODELS.iter().find(|m| m.1 == name)
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID_HP {
        return None;
    }
    let (_, name, _, interface) = MODELS.iter().find(|m| m.0 == pid)?;
    let mut m = Model::new(*name, DeviceType::Mouse, vec![ZoneInfo::fixed("LED", 1)]);
    m.interface = Some(*interface);
    Some(m)
}

struct Universal {
    hid: Box<dyn Hid>,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if entry(&model.name).is_none() {
        bail!("HyperX Universal : modèle {} non pris en charge", model.name);
    }
    Ok(Box::new(Universal { hid }))
}

/// Réglages proposés : luminosité de la LED, et seuil de batterie faible sur les modèles sans fil.
pub fn settings(model: &Model) -> Vec<DeviceSetting> {
    let slider = |min, max, step| SettingKind::Slider { min, max, step, unit: "%".into() };
    let mut v = vec![DeviceSetting { id: "brightness".into(), label: "Luminosité de la LED".into(), kind: slider(0, 100, 5), default: 100 }];
    if entry(&model.name).is_some_and(|m| m.2) {
        v.push(DeviceSetting { id: "low_battery".into(), label: "Seuil d'alerte batterie faible".into(), kind: slider(5, 25, 5), default: 10 });
    }
    v
}

fn report(id: u8, sub: u8) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = id;
    r[1] = sub;
    r
}

impl NativeRgb for Universal {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > PER_REPORT {
            bail!("HyperX Universal : {} couleurs pour 1 LED", colors.len());
        }
        let c = colors.first().copied().unwrap_or(Color::BLACK);
        let mut start = report(ID_LIGHTING_DIRECT, 1);
        start[2] = 1;
        self.hid.write(&start)?;
        let mut data = report(ID_LIGHTING_DIRECT, 2);
        data[4..7].copy_from_slice(&[c.r, c.g, c.b]);
        self.hid.write(&data)?;
        Ok(())
    }

    fn set_setting(&mut self, id: &str, value: i32) -> Result<()> {
        match id {
            "brightness" => {
                if !(0..=100).contains(&value) {
                    bail!("HyperX Universal : luminosité {value} hors de 0..100");
                }
                let mut r = report(ID_BRIGHTNESS, 1); // octets 2-3 : masque de profils = 0 (préréglage applicatif)
                r[4] = (255 * value / 100) as u8;
                self.hid.write(&r)?;
            }
            "low_battery" => {
                if !(5..=25).contains(&value) {
                    bail!("HyperX Universal : seuil {value} hors de 5..25");
                }
                let mut r = report(ID_BATTERY, 1);
                r[2] = value as u8; // octet 3 = 0 : ne pas enregistrer dans l'appareil
                self.hid.write(&r)?;
            }
            _ => bail!("HyperX Universal : réglage {id} inconnu"),
        }
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "hyperx_universal", doc: "docs/rgb-protocols/hyperx-ngenuity.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Log = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn driver(pid: u16) -> (Box<dyn NativeRgb>, Log, Model) {
        let model = probe(VID_HP, pid).expect("pid");
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent, model)
    }

    #[test]
    fn probe_claims_only_listed_pids_and_never_bootloader_ones() {
        assert_eq!(probe(VID_HP, 0x0B97).map(|m| (m.led_count(), m.interface)), Some((1, Some(2))));
        assert_eq!(probe(VID_HP, 0x0CBD).and_then(|m| m.interface), Some(3));
        assert!(probe(0x0951, 0x0B97).is_none());
        assert!(probe(VID_HP, 0x0C97).is_none()); // amorçage (DFU) de la Haste 2
        assert!(probe(VID_HP, 0x0F98).is_none()); // Haste 2 sans fil : trame de LED de première génération différente
    }

    #[test]
    fn lighting_sends_start_then_one_data_report() {
        let (mut d, sent, _) = driver(0x0B97);
        d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|r| r.len() == 64));
        assert_eq!(&s[0][..4], &[0x44, 0x01, 0x01, 0x00]);
        assert_eq!(&s[1][..8], &[0x44, 0x02, 0x00, 0x00, 1, 2, 3, 0]);
        assert!(s[1][8..].iter().all(|b| *b == 0));
    }

    #[test]
    fn brightness_uses_the_zero_profile_mask() {
        let (mut d, sent, _) = driver(0x0B97);
        d.set_setting("brightness", 50).expect("set");
        d.set_setting("brightness", 100).expect("set");
        let s = sent.lock();
        assert_eq!(&s[0][..5], &[0x40, 0x01, 0x00, 0x00, 127]);
        assert_eq!(s[1][4], 255);
        assert_eq!(s[0].len(), 64);
    }

    #[test]
    fn low_battery_never_asks_the_device_to_save() {
        let (mut d, sent, _) = driver(0x0BA0);
        d.set_setting("low_battery", 15).expect("set");
        assert_eq!(&sent.lock()[0][..4], &[0x50, 0x01, 15, 0x00]);
    }

    #[test]
    fn out_of_range_and_unknown_settings_send_nothing() {
        let (mut d, sent, _) = driver(0x0BA0);
        assert!(d.set_setting("brightness", 101).is_err());
        assert!(d.set_setting("brightness", -1).is_err());
        assert!(d.set_setting("low_battery", 4).is_err());
        assert!(d.set_setting("dpi", 800).is_err());
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn schema_offers_the_battery_threshold_on_wireless_models_only() {
        let wired = settings(&probe(VID_HP, 0x0B97).unwrap());
        let wireless = settings(&probe(VID_HP, 0x0AB8).unwrap());
        assert_eq!(wired.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["brightness"]);
        assert_eq!(wireless.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["brightness", "low_battery"]);
    }
}
