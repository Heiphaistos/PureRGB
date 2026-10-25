//! Famille « steelseries_arctis5 » : casque SteelSeries Arctis 5 (PID 1250 et 12AA, interface 5), VID 0x1038.
//!
//! SOURCE UNIQUE : docs/rgb-protocols/analyse-steelseries-gg.md §9.1, tirée d'une seule source tierce (contrôleur OpenRGB, testé sur matériel par
//! son auteur d'après ses en-têtes), NON confirmée dans SteelSeries GG. Aucun autre Arctis n'est revendiqué (Nova 3 12EC : trame d'application
//! `06 09`/`06 A3` peut-être une sauvegarde flash, REFUSÉ ; Arctis 7/9/Pro/Nova 5/7/Pro/Elite : INCONNU).
//!
//! Une couleur par zone (zone 0 = gauche, 1 = droite, une LED chacune) = 6 écritures de 37 octets, ID de rapport 06, aucune lecture :
//! 1-2 préparation (`81 43 01 22` puis `81 43 01 23`), 3 couleur (`8A 42 00 20 41 00 R G B FF 32 C8 C8`), 4 `8A 42 00 20 41 08 <zone> 01`,
//! 5 `8A 42 00 20 60 <zone>`, 6 application `8A 42 00 20 05`.
//! INCONNU : sens des octets 81/43, FF/32/C8/C8 ; cadence ; PERSISTANCE de l'application (6) : pourrait écrire en mémoire du casque.
//! Garde-fous en conséquence : une zone dont la couleur n'a pas changé n'est pas réécrite, et deux changements à moins de 1 s sont refusés
//! (pas d'animation) ; ce driver n'est donc pas fait pour des effets temps réel.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::{Duration, Instant};

const VID: u16 = 0x1038;
const PIDS: [u16; 2] = [0x1250, 0x12AA];
const INTERFACE: i32 = 5;
const REPORT_LEN: usize = 37;
const REPORT_ID: u8 = 0x06;
/// Écart minimal entre deux applications (persistance inconnue).
const MIN_APPLY_GAP: Duration = Duration::from_secs(1);

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || !PIDS.contains(&pid) {
        return None;
    }
    let mut m = Model::new("SteelSeries Arctis 5", DeviceType::Headset, vec![ZoneInfo::fixed("Gauche", 1), ZoneInfo::fixed("Droite", 1)]);
    m.interface = Some(INTERFACE);
    Some(m)
}

struct Arctis5 {
    hid: Box<dyn Hid>,
    last: [Option<Color>; 2],
    last_apply: Option<Instant>,
}

/// Rapport de 37 octets : `06` puis `body`, complété de zéros.
fn report(body: &[u8]) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = REPORT_ID;
    r[1..1 + body.len()].copy_from_slice(body);
    r
}

impl Arctis5 {
    fn set_zone(&self, zone: u8, c: Color) -> Result<()> {
        let frames = [
            report(&[0x81, 0x43, 0x01, 0x22]),
            report(&[0x81, 0x43, 0x01, 0x23]),
            report(&[0x8A, 0x42, 0x00, 0x20, 0x41, 0x00, c.r, c.g, c.b, 0xFF, 0x32, 0xC8, 0xC8]),
            report(&[0x8A, 0x42, 0x00, 0x20, 0x41, 0x08, zone, 0x01]),
            report(&[0x8A, 0x42, 0x00, 0x20, 0x60, zone]),
            report(&[0x8A, 0x42, 0x00, 0x20, 0x05]),
        ];
        for f in &frames {
            self.hid.write(f)?;
        }
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Arctis5 { hid, last: [None; 2], last_apply: None }))
}

impl NativeRgb for Arctis5 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() != 2 {
            bail!("Arctis 5 : {} couleurs reçues, 2 attendues", colors.len());
        }
        let changed: Vec<u8> = (0..2u8).filter(|&z| self.last[z as usize] != Some(colors[z as usize])).collect();
        if changed.is_empty() {
            return Ok(());
        }
        if self.last_apply.is_some_and(|t| t.elapsed() < MIN_APPLY_GAP) {
            bail!("Arctis 5 : changements de couleur trop rapprochés refusés (l'application peut écrire en mémoire du casque, persistance INCONNUE)");
        }
        self.last_apply = Some(Instant::now());
        for z in changed {
            self.set_zone(z, colors[z as usize])?;
            self.last[z as usize] = Some(colors[z as usize]);
        }
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "steelseries_arctis5", doc: "docs/rgb-protocols/analyse-steelseries-gg.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::{probe as probe_all, MockHid};

    fn pad(head: &[u8]) -> Vec<u8> {
        let mut v = head.to_vec();
        v.resize(37, 0);
        v
    }
    fn dev() -> (Box<dyn NativeRgb>, std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>) {
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &probe(VID, 0x1250).unwrap()).unwrap(), sent)
    }
    /// Les 6 trames d'une zone, écrites à la main d'après la note §9.1.
    fn zone_frames(zone: u8, r: u8, g: u8, b: u8) -> Vec<Vec<u8>> {
        vec![
            pad(&[0x06, 0x81, 0x43, 0x01, 0x22]),
            pad(&[0x06, 0x81, 0x43, 0x01, 0x23]),
            pad(&[0x06, 0x8A, 0x42, 0x00, 0x20, 0x41, 0x00, r, g, b, 0xFF, 0x32, 0xC8, 0xC8]),
            pad(&[0x06, 0x8A, 0x42, 0x00, 0x20, 0x41, 0x08, zone, 0x01]),
            pad(&[0x06, 0x8A, 0x42, 0x00, 0x20, 0x60, zone]),
            pad(&[0x06, 0x8A, 0x42, 0x00, 0x20, 0x05]),
        ]
    }

    #[test]
    fn probe_claims_only_the_two_documented_arctis5_pids() {
        for pid in [0x1250, 0x12AA] {
            let (f, m) = probe_all(VID, pid).unwrap();
            assert_eq!((f.id, m.interface, m.led_count(), m.zones.len()), ("steelseries_arctis5", Some(5), 2, 2));
        }
        for pid in [0x12EC, 0x1229, 0x12AB, 0x1251, 0x12AD] {
            assert!(probe_all(VID, pid).is_none_or(|(f, _)| f.id != "steelseries_arctis5"), "{pid:04X}");
        }
        assert!(probe(0x1234, 0x1250).is_none());
    }

    #[test]
    fn one_zone_is_six_37_byte_writes() {
        let (mut d, sent) = dev();
        d.set_colors(&[Color::new(0x11, 0x22, 0x33), Color::new(0x11, 0x22, 0x33)]).unwrap();
        let mut want = zone_frames(0, 0x11, 0x22, 0x33);
        want.extend(zone_frames(1, 0x11, 0x22, 0x33));
        assert_eq!(*sent.lock(), want);
    }

    #[test]
    fn unchanged_zones_are_not_rewritten_and_fast_changes_are_refused() {
        let (mut d, sent) = dev();
        let a = Color::new(1, 2, 3);
        d.set_colors(&[a, a]).unwrap();
        assert_eq!(sent.lock().len(), 12);
        d.set_colors(&[a, a]).unwrap(); // identique : rien
        assert_eq!(sent.lock().len(), 12);
        assert!(d.set_colors(&[a, Color::new(9, 9, 9)]).is_err()); // < 1 s
        assert_eq!(sent.lock().len(), 12);
    }

    #[test]
    fn wrong_color_count_writes_nothing() {
        let (mut d, sent) = dev();
        assert!(d.set_colors(&[Color::BLACK]).is_err());
        assert!(d.set_colors(&[Color::BLACK; 3]).is_err());
        assert!(sent.lock().is_empty());
        assert!(d.set_fan_duty(0, 10).is_err());
    }
}
