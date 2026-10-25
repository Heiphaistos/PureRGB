//! Famille « dualsense » : Sony DualSense / DualSense Edge, barre lumineuse + 5 LED de joueur (couleur directe, non persistante).
//! Source : docs/rgb-protocols/manettes.md ; driver Linux `drivers/hid/hid-playstation.c` (master, consulté 2026-10-25),
//! `hid-ids.h` pour les PID. Réimplémenté d'après la description du format (aucun code copié, le driver est GPL).
//!
//! Rapport de sortie « commun » de 47 octets : [0] valid_flag0, [1] valid_flag1, [2] moteur droit, [3] moteur gauche,
//! [4..=9] audio/micro/économie d'énergie, [10..=36] réservé, [37] audio_control2, [38] valid_flag2, [39..=40] réservé,
//! [41] lightbar_setup, [42] led_brightness, [43] player_leds, [44] rouge, [45] vert, [46] bleu.
//! USB : 0x02 + commun + 15 octets réservés = 63 octets. Bluetooth : 0x31, seq<<4, 0x10, commun, 24 réservés, CRC32 = 78 octets.
//! Seuls valid_flag1 bit 2 (barre) et bit 4 (LED de joueur) sont armés : moteurs, audio, gâchettes adaptatives non touchés.
//! Jamais envoyé : lightbar_setup/valid_flag2 (fondu), RELEASE_LEDS (sémantique non vérifiée), rapports de fonctionnalité d'écriture.
//! INCONNU : si Windows via hidapi accepte l'écriture 0x31 sur un DualSense encore en mode Bluetooth simple (non testé sans matériel).

use super::sony_crc::seal_output;
use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID_SONY: u16 = 0x054C;
const PID_DUALSENSE: u16 = 0x0CE6;
const PID_DUALSENSE_EDGE: u16 = 0x0DF2;
const REPORT_USB: u8 = 0x02;
const REPORT_BT: u8 = 0x31;
const SIZE_USB: usize = 63;
const SIZE_BT: usize = 78;
const TAG_BT: u8 = 0x10;
const FLAG1_LIGHTBAR: u8 = 1 << 2;
const FLAG1_PLAYER_LEDS: u8 = 1 << 4;
const COMMON: usize = 47;
const PLAYER_LEDS: usize = 5;
/// Seuil au-dessus duquel une « couleur » allume une LED de joueur (ces LED sont tout ou rien).
const PLAYER_ON_THRESHOLD: u8 = 128;
const CALIBRATION_FEATURE: usize = 41;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let name = match (vid, pid) {
        (VID_SONY, PID_DUALSENSE) => "Sony DualSense",
        (VID_SONY, PID_DUALSENSE_EDGE) => "Sony DualSense Edge",
        _ => return None,
    };
    let zones = vec![ZoneInfo::fixed("Barre lumineuse", 1), ZoneInfo::fixed("LED de joueur", PLAYER_LEDS as u32)];
    Some(Model::new(name, DeviceType::Gamepad, zones))
}

struct DualSense {
    hid: Box<dyn Hid>,
    bluetooth: bool,
    seq: u8,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let d = DualSense { hid, bluetooth: model.bluetooth, seq: 0 };
    if d.bluetooth {
        // Le driver Linux lit le rapport de calibration (0x05) à l'initialisation ; lecture seule, sans effet d'écriture.
        let mut buf = [0u8; CALIBRATION_FEATURE];
        buf[0] = 0x05;
        if let Err(e) = d.hid.get_feature_report(&mut buf) {
            log::debug!("DualSense : lecture du rapport 0x05 refusée ({e:#})");
        }
    }
    Ok(Box::new(d))
}

/// Rapport commun (47 octets) : barre + LED de joueur uniquement.
fn common(colors: &[Color]) -> [u8; COMMON] {
    let mut c = [0u8; COMMON];
    c[1] = FLAG1_LIGHTBAR | FLAG1_PLAYER_LEDS;
    let mut players = 0u8;
    for i in 0..PLAYER_LEDS {
        if colors.get(1 + i).is_some_and(|p| p.r.max(p.g).max(p.b) >= PLAYER_ON_THRESHOLD) {
            players |= 1 << i;
        }
    }
    c[43] = players;
    let bar = colors.first().copied().unwrap_or(Color::BLACK);
    c[44..47].copy_from_slice(&[bar.r, bar.g, bar.b]);
    c
}

fn usb_report(colors: &[Color]) -> Vec<u8> {
    let mut r = vec![0u8; SIZE_USB];
    r[0] = REPORT_USB;
    r[1..1 + COMMON].copy_from_slice(&common(colors));
    r
}

fn bt_report(colors: &[Color], seq: u8) -> Vec<u8> {
    let mut r = vec![0u8; SIZE_BT];
    r[0] = REPORT_BT;
    r[1] = (seq & 0x0F) << 4;
    r[2] = TAG_BT;
    r[3..3 + COMMON].copy_from_slice(&common(colors));
    seal_output(&mut r);
    r
}

impl NativeRgb for DualSense {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > 1 + PLAYER_LEDS {
            bail!("DualSense : {} couleurs pour {} LED", colors.len(), 1 + PLAYER_LEDS);
        }
        let report = if self.bluetooth {
            let r = bt_report(colors, self.seq);
            self.seq = (self.seq + 1) & 0x0F;
            r
        } else {
            usb_report(colors)
        };
        self.hid.write(&report)?;
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "dualsense", doc: "docs/rgb-protocols/manettes.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    fn driver(bt: bool) -> (Box<dyn NativeRgb>, super::super::Log) {
        let mut model = probe(VID_SONY, PID_DUALSENSE).expect("pid");
        model.bluetooth = bt;
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent)
    }

    #[test]
    fn probe_claims_dualsense_and_edge_only() {
        assert_eq!(probe(VID_SONY, 0x0CE6).map(|m| m.led_count()), Some(6));
        assert!(probe(VID_SONY, 0x0DF2).is_some());
        assert!(probe(VID_SONY, 0x05C4).is_none()); // DualShock 4 : autre famille
        assert!(probe(0x045E, 0x0CE6).is_none());
    }

    #[test]
    fn usb_report_is_63_bytes_with_lightbar_at_44() {
        let (mut d, sent) = driver(false);
        d.set_colors(&[Color::new(0x11, 0x22, 0x33)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 1);
        let r = &s[0];
        assert_eq!(r.len(), 63);
        assert_eq!(r[0], 0x02);
        assert_eq!(r[1], 0x00); // valid_flag0 : aucun moteur/audio
        assert_eq!(r[2], 0x14); // valid_flag1 : barre (bit 2) + LED de joueur (bit 4)
        assert_eq!(r[1 + 38], 0x00); // valid_flag2 : pas de réglage de fondu
        assert_eq!(r[1 + 43], 0x00); // aucune LED de joueur
        assert_eq!(&r[45..48], &[0x11, 0x22, 0x33]);
        assert!(r[48..].iter().all(|&b| b == 0));
    }

    #[test]
    fn player_leds_are_bits_0_to_4() {
        let (mut d, sent) = driver(false);
        let on = Color::new(255, 255, 255);
        let off = Color::BLACK;
        d.set_colors(&[off, on, off, on, off, on]).expect("set");
        assert_eq!(sent.lock()[0][1 + 43], 0b10101);
    }

    #[test]
    fn bluetooth_report_is_78_bytes_with_header_and_valid_crc() {
        let (mut d, sent) = driver(true);
        d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
        d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
        let s = sent.lock();
        let r = &s[0];
        assert_eq!((r.len(), r[0], r[1], r[2]), (78, 0x31, 0x00, 0x10));
        assert_eq!(r[3 + 1], 0x14);
        assert_eq!(&r[3 + 44..3 + 47], &[1, 2, 3]);
        // zlib.crc32(b"\xa2" + r[..74]) = 0x45E39552, calculé indépendamment (Python), écrit en little-endian.
        assert_eq!(&r[74..], &[0x52, 0x95, 0xE3, 0x45]);
        assert_eq!(s[1][1], 0x10); // numéro de séquence suivant dans le quartet haut
    }

    #[test]
    fn too_many_colors_sends_nothing() {
        let (mut d, sent) = driver(false);
        assert!(d.set_colors(&[Color::BLACK; 7]).is_err());
        assert!(sent.lock().is_empty());
    }
}
