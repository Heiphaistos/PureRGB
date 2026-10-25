//! Famille « dualshock4 » : Sony DualShock 4 (v1 et v2), barre lumineuse en couleur directe (non persistante).
//! Source : docs/rgb-protocols/manettes.md ; driver Linux `drivers/hid/hid-playstation.c` (master, consulté 2026-10-25),
//! `hid-ids.h` pour les PID. Réimplémenté d'après la description du format (aucun code copié, le driver est GPL).
//!
//! Bloc commun de 10 octets : [0] valid_flag0 (bit 0 moteurs, bit 1 barre, bit 2 clignotement), [1] valid_flag1, [2] réservé,
//! [3] moteur droit, [4] moteur gauche, [5] rouge, [6] vert, [7] bleu, [8] clignotement allumé, [9] clignotement éteint.
//! USB : 0x05 + bloc + 21 réservés = 32 octets, sans CRC. Bluetooth : 0x11, hw_control = 0x80 (HID) | 0x40 (CRC32) | 4 (intervalle
//! d'interrogation par défaut du driver), audio_control = 0, bloc, 61 réservés, CRC32 = 78 octets.
//! Seul valid_flag0 bit 1 est armé : moteurs et clignotement non touchés.
//! Non revendiqué : récepteur USB « dongle » 054C:0BA0 (comportement du rapport via le récepteur non établi), clones tiers.

use super::sony_crc::seal_output;
use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID_SONY: u16 = 0x054C;
const REPORT_USB: u8 = 0x05;
const REPORT_BT: u8 = 0x11;
const SIZE_USB: usize = 32;
const SIZE_BT: usize = 78;
const FLAG0_LED: u8 = 0x02;
const HW_CONTROL_BT: u8 = 0x80 | 0x40 | 4;
const CALIBRATION_BT_SIZE: usize = 41;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let name = match (vid, pid) {
        (VID_SONY, 0x05C4) => "Sony DualShock 4",
        (VID_SONY, 0x09CC) => "Sony DualShock 4 (v2)",
        _ => return None,
    };
    Some(Model::new(name, DeviceType::Gamepad, vec![ZoneInfo::fixed("Barre lumineuse", 1)]))
}

struct DualShock4 {
    hid: Box<dyn Hid>,
    bluetooth: bool,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.bluetooth {
        // Le driver Linux lit le rapport de calibration Bluetooth (0x05) à l'initialisation ; lecture seule.
        let mut buf = [0u8; CALIBRATION_BT_SIZE];
        buf[0] = 0x05;
        if let Err(e) = hid.get_feature_report(&mut buf) {
            log::debug!("DualShock 4 : lecture du rapport 0x05 refusée ({e:#})");
        }
    }
    Ok(Box::new(DualShock4 { hid, bluetooth: model.bluetooth }))
}

fn report(bluetooth: bool, c: Color) -> Vec<u8> {
    let (mut r, at) = if bluetooth {
        let mut r = vec![0u8; SIZE_BT];
        r[0] = REPORT_BT;
        r[1] = HW_CONTROL_BT;
        (r, 3)
    } else {
        let mut r = vec![0u8; SIZE_USB];
        r[0] = REPORT_USB;
        (r, 1)
    };
    r[at] = FLAG0_LED;
    r[at + 5..at + 8].copy_from_slice(&[c.r, c.g, c.b]);
    if bluetooth {
        seal_output(&mut r);
    }
    r
}

impl NativeRgb for DualShock4 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > 1 {
            bail!("DualShock 4 : {} couleurs pour 1 LED", colors.len());
        }
        let c = colors.first().copied().unwrap_or(Color::BLACK);
        self.hid.write(&report(self.bluetooth, c))?;
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "dualshock4", doc: "docs/rgb-protocols/manettes.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    fn driver(bt: bool) -> (Box<dyn NativeRgb>, super::super::Log) {
        let mut model = probe(VID_SONY, 0x09CC).expect("pid");
        model.bluetooth = bt;
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent)
    }

    #[test]
    fn probe_claims_the_two_pads_not_the_dongle_nor_dualsense() {
        assert!(probe(VID_SONY, 0x05C4).is_some());
        assert!(probe(VID_SONY, 0x09CC).is_some());
        assert!(probe(VID_SONY, 0x0BA0).is_none());
        assert!(probe(VID_SONY, 0x0CE6).is_none());
    }

    #[test]
    fn usb_report_is_32_bytes_with_color_at_6() {
        let (mut d, sent) = driver(false);
        d.set_colors(&[Color::new(0xAA, 0xBB, 0xCC)]).expect("set");
        let mut expect = vec![0u8; 32];
        expect[..2].copy_from_slice(&[0x05, 0x02]);
        expect[6..9].copy_from_slice(&[0xAA, 0xBB, 0xCC]);
        assert_eq!(sent.lock().as_slice(), [expect]);
    }

    #[test]
    fn bluetooth_report_is_78_bytes_with_header_and_crc() {
        let (mut d, sent) = driver(true);
        d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
        let s = sent.lock();
        let r = &s[0];
        assert_eq!((r.len(), r[0], r[1], r[2], r[3]), (78, 0x11, 0xC4, 0x00, 0x02));
        assert_eq!(&r[8..11], &[1, 2, 3]);
        // zlib.crc32(b"\xa2" + r[..74]) = 0xE5A79222, calculé indépendamment (Python), écrit en little-endian.
        assert_eq!(&r[74..], &[0x22, 0x92, 0xA7, 0xE5]);
    }
}
