//! Famille « lenovo_4zone » : clavier 4 zones des portables Lenovo Legion 5 / IdeaPad (contrôleur ITE VID 048D, collection 0xFF89/usage 0xCC).
//! Pilote expérimental, désactivé par défaut (`verified: false`). Source : docs/rgb-protocols/oem-portables.md (§ Lenovo) ; faits de format
//! relevés dans les sources publiques OpenRGB (GPL-2, consultées 2026-10-26, [S] : aucun code copié) ; non confirmé sur le logiciel Lenovo (Vantage /
//! Legion Zone absents de ce poste, non installables).
//!
//! Trame : feature report de 33 octets, identifiant de rapport 0xCC : `CC 16 <effet> <vitesse> <luminosité> R0 G0 B0 … R3 G3 B3 00 <vague g->d> <vague d->g>`
//! complétée de zéros. Seul l'effet 1 (statique) est utilisé, vitesse 1 ; luminosité 1 (basse) ou 2 (haute). Zones : gauche, centre gauche,
//! centre droit, droite. INCONNU : persistance après redémarrage (la source ne le dit pas) ; effets 3/4/6 non utilisés.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceSetting, DeviceType, SettingKind, ZoneInfo};
use anyhow::{bail, Result};

const VID_ITE: u16 = 0x048D;
/// Legion 5 / IdeaPad 2020 à 2023 (0xC963 sert à la fois l'IdeaPad 3-15ACH6 et la variante IdeaPad 2021).
const PIDS: [(u16, &str); 7] = [
    (0xC955, "Lenovo Legion 5 2020 (clavier 4 zones)"),
    (0xC965, "Lenovo Legion 5 2021 (clavier 4 zones)"),
    (0xC963, "Lenovo IdeaPad/Legion 2021 (clavier 4 zones)"),
    (0xC975, "Lenovo Legion 5 2022 (clavier 4 zones)"),
    (0xC973, "Lenovo IdeaPad 2022 (clavier 4 zones)"),
    (0xC985, "Lenovo Legion 5 2023 (clavier 4 zones)"),
    (0xC984, "Lenovo IdeaPad 2023 (clavier 4 zones)"),
];
const LEN: usize = 33;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID_ITE {
        return None;
    }
    let (_, name) = PIDS.iter().find(|(p, _)| *p == pid)?;
    let zones = ["Gauche", "Centre gauche", "Centre droit", "Droite"].map(|n| ZoneInfo::fixed(n, 1)).to_vec();
    let mut m = Model::new(*name, DeviceType::Keyboard, zones);
    m.usage_page = Some(0xFF89);
    m.usage = Some(0xCC);
    Some(m)
}

struct Lenovo4 {
    hid: Box<dyn Hid>,
    bright: bool,
    last: [Color; 4],
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Lenovo4 { hid, bright: true, last: [Color::BLACK; 4] }))
}

fn frame(colors: &[Color; 4], bright: bool) -> [u8; LEN] {
    let mut r = [0u8; LEN];
    r[..5].copy_from_slice(&[0xCC, 0x16, 0x01, 0x01, if bright { 2 } else { 1 }]);
    for (i, c) in colors.iter().enumerate() {
        r[5 + 3 * i..8 + 3 * i].copy_from_slice(&[c.r, c.g, c.b]);
    }
    r
}

/// Réglage non lumineux : niveau de luminosité du rétroéclairage (2 niveaux matériels), RAM de l'appareil.
pub fn settings() -> Vec<DeviceSetting> {
    vec![DeviceSetting { id: "bright".into(), label: "Luminosité haute".into(), kind: SettingKind::Toggle, default: 1 }]
}

impl Lenovo4 {
    fn send(&self) -> Result<()> {
        self.hid.send_feature_report(&frame(&self.last, self.bright))
    }
}

impl NativeRgb for Lenovo4 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > 4 {
            bail!("Lenovo 4 zones : {} couleurs pour 4 zones", colors.len());
        }
        let mut next = [Color::BLACK; 4];
        next[..colors.len()].copy_from_slice(colors);
        if next != self.last {
            self.last = next;
            self.send()?;
        }
        Ok(())
    }

    fn set_setting(&mut self, id: &str, value: i32) -> Result<()> {
        if id != "bright" {
            bail!("réglage inconnu : {id}");
        }
        self.bright = value != 0;
        self.send()
    }
}

pub const FAMILY: Family = Family { id: "lenovo_4zone", doc: "docs/rgb-protocols/oem-portables.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    #[test]
    fn probe_claims_only_the_listed_ite_pids() {
        for (pid, _) in PIDS {
            let m = probe(VID_ITE, pid).expect("pid");
            assert_eq!((m.led_count(), m.usage_page, m.usage), (4, Some(0xFF89), Some(0xCC)));
        }
        assert!(probe(VID_ITE, 0xC956).is_none(), "Legion 7 (par touche) hors périmètre");
        assert!(probe(0x1234, 0xC965).is_none());
    }

    #[test]
    fn frame_is_cc16_static_with_four_rgb_triplets() {
        let model = probe(VID_ITE, 0xC965).expect("pid");
        let (mock, _, feat) = MockHid::new(vec![]);
        let mut d = open(Box::new(mock), &model).expect("open");
        let c = [Color::new(1, 2, 3), Color::new(4, 5, 6), Color::new(7, 8, 9), Color::new(10, 11, 12)];
        d.set_colors(&c).expect("set");
        d.set_colors(&c).expect("identique : rien");
        d.set_setting("bright", 0).expect("lum");
        assert!(d.set_setting("autre", 1).is_err());
        assert!(d.set_colors(&[Color::BLACK; 5]).is_err());
        let f = feat.lock();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].len(), 33);
        assert_eq!(&f[0][..17], &[0xCC, 0x16, 1, 1, 2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert!(f[0][17..].iter().all(|b| *b == 0));
        assert_eq!(f[1][4], 1, "luminosité basse");
    }
}
