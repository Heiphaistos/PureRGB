//! Famille « litra » : Logitech Litra Beam LX (046D:C903), 7 zones couleur sur la face arrière (non persistant).
//! Source : docs/rgb-protocols/marques-groupe-c.md ; projet public litra-rs (MIT, `src/lib.rs`, consulté 2026-10-26), réimplémenté
//! d'après les faits de format (aucun code copié). Seule la face arrière est RGB : Litra Glow (C900) et Litra Beam (C901/B901)
//! n'ont qu'une lumière blanche (luminosité + température), non revendiqués ici.
//!
//! Rapport de sortie 20 octets : `11 FF 0C 1B <zone 1..7> R G B FF 00 00 00 FF 00 00 00 FF 00 00 00`, puis un rapport de
//! validation `11 FF 0C 7B 00 00 01 00 00` (9 octets chez litra-rs ; complété de zéros jusqu'à 20 par le pilote HID).
//! La source note que l'appareil « se comporte mal » si une composante vaut 0 : chaque composante est donc ramenée à 1 au minimum.
//! Collection HID : page d'usage 0xFF43. Allumage/luminosité de la face arrière : volontairement non touchés.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID_LOGITECH: u16 = 0x046D;
const PID_BEAM_LX: u16 = 0xC903;
const ZONES: usize = 7;
const REPORT_LEN: usize = 20;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if (vid, pid) != (VID_LOGITECH, PID_BEAM_LX) {
        return None;
    }
    let mut m = Model::new("Logitech Litra Beam LX", DeviceType::Light, vec![ZoneInfo::fixed("Face arrière", ZONES as u32)]);
    m.usage_page = Some(0xFF43);
    Some(m)
}

struct Litra {
    hid: Box<dyn Hid>,
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Litra { hid }))
}

fn zone_report(zone: u8, c: Color) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[..4].copy_from_slice(&[0x11, 0xFF, 0x0C, 0x1B]);
    r[4] = zone;
    r[5..8].copy_from_slice(&[c.r.max(1), c.g.max(1), c.b.max(1)]);
    r[8] = 0xFF;
    r[12] = 0xFF;
    r[16] = 0xFF;
    r
}

fn commit_report() -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[..9].copy_from_slice(&[0x11, 0xFF, 0x0C, 0x7B, 0, 0, 1, 0, 0]);
    r
}

impl NativeRgb for Litra {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > ZONES {
            bail!("Litra Beam LX : {} couleurs pour {ZONES} zones", colors.len());
        }
        for zone in 0..ZONES {
            let c = colors.get(zone).copied().unwrap_or(Color::BLACK);
            self.hid.write(&zone_report(zone as u8 + 1, c))?;
        }
        self.hid.write(&commit_report())?;
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "litra", doc: "docs/rgb-protocols/marques-groupe-c.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    #[test]
    fn probe_claims_only_the_beam_lx() {
        assert!(probe(VID_LOGITECH, 0xC903).is_some());
        for pid in [0xC900, 0xC901, 0xB901] {
            assert!(probe(VID_LOGITECH, pid).is_none(), "{pid:04x} est blanc seulement");
        }
        assert_eq!(probe(VID_LOGITECH, 0xC903).expect("lx").led_count(), 7);
    }

    #[test]
    fn seven_zone_reports_then_commit_with_floor_of_one() {
        let model = probe(VID_LOGITECH, 0xC903).expect("lx");
        let (mock, sent, _) = MockHid::new(vec![]);
        let mut d = open(Box::new(mock), &model).expect("open");
        d.set_colors(&[Color::new(0xAA, 0, 0x10)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 8, "7 zones + validation");
        let mut first = vec![0x11, 0xFF, 0x0C, 0x1B, 1, 0xAA, 1, 0x10, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0];
        assert_eq!(s[0], first);
        first[4] = 2;
        first[5..8].copy_from_slice(&[1, 1, 1]);
        assert_eq!(s[1], first, "LED manquante = noir, ramené à 1");
        assert_eq!(&s[7][..9], &[0x11, 0xFF, 0x0C, 0x7B, 0, 0, 1, 0, 0]);
        assert!(s.iter().all(|r| r.len() == 20));
    }

    #[test]
    fn too_many_colors_are_refused() {
        let model = probe(VID_LOGITECH, 0xC903).expect("lx");
        let (mock, sent, _) = MockHid::new(vec![]);
        let mut d = open(Box::new(mock), &model).expect("open");
        assert!(d.set_colors(&[Color::BLACK; 8]).is_err());
        assert!(sent.lock().is_empty());
    }
}
