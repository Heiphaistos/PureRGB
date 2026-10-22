//! Famille « hyperx » : schéma `0x81` en feature reports de 65 octets (Alloy Origins, Origins 60/65, tapis).
//! Source : docs/rgb-protocols/hyperx.md
//!
//! INCONNU (donc non revendiqué) : Alloy Elite 2 (octet 9 de l'init non précisé), Elite/FPS (rapport 0x07, direct 0x16),
//! Eve 1800, Origins Core, Origins 2 65, souris (Pulsefire), micros.
//! INCONNU : correspondance LED -> touche (tables `skip_idx`) ; les 144 emplacements sont exposés bruts, dans l'ordre des paquets.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID_KINGSTON: u16 = 0x0951;
const VID_HP: u16 = 0x03F0;
const PACKET: usize = 65;
const PER_PACKET: usize = 16;
const KB_PACKETS: usize = 9;
const MAT_PACKETS: usize = 2;
const UP_HP: u16 = 0xFF90;

/// (VID, PID, nom, clavier ?, interface, page d'usage). Interface 3 pour les claviers (l'entrée alternative « interface 0 » de la note n'est pas retenue).
const MODELS: &[(u16, u16, &str, bool, i32, Option<u16>)] = &[
    (VID_KINGSTON, 0x1734, "HyperX Alloy Origins 60", true, 3, None),
    (VID_HP, 0x0C8E, "HyperX Alloy Origins 60 (HP)", true, 3, Some(UP_HP)),
    (VID_HP, 0x038F, "HyperX Alloy Origins 65 (HP)", true, 3, Some(UP_HP)),
    (VID_KINGSTON, 0x16E5, "HyperX Alloy Origins", true, 3, None),
    (VID_HP, 0x0591, "HyperX Alloy Origins (HP)", true, 3, Some(UP_HP)),
    (VID_KINGSTON, 0x1705, "HyperX Fury Ultra", false, 0, None),
    (VID_HP, 0x0F8D, "HyperX Pulsefire Mat", false, 1, Some(UP_HP)),
    (VID_KINGSTON, 0x1741, "HyperX Pulsefire Mat RGB XL", false, 1, Some(UP_HP)),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let (_, _, name, keyboard, interface, up) = MODELS.iter().find(|m| m.0 == vid && m.1 == pid)?;
    let (ty, zone) = if *keyboard {
        (DeviceType::Keyboard, ZoneInfo::fixed("Emplacements", (KB_PACKETS * PER_PACKET) as u32))
    } else {
        (DeviceType::Mousemat, ZoneInfo::fixed("LEDs", (MAT_PACKETS * PER_PACKET) as u32))
    };
    let mut m = Model::new(*name, ty, vec![zone]);
    m.interface = Some(*interface);
    m.usage_page = *up;
    Some(m)
}

struct HyperX {
    hid: Box<dyn Hid>,
    packets: usize,
    /// Octet 9 du paquet d'init : 0x09 claviers Origins, 0x02 tapis.
    init9: u8,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let (packets, init9) = match model.device_type {
        DeviceType::Keyboard => (KB_PACKETS, 0x09),
        DeviceType::Mousemat => (MAT_PACKETS, 0x02),
        _ => bail!("HyperX : type d'appareil non pris en charge"),
    };
    Ok(Box::new(HyperX { hid, packets, init9 }))
}

impl NativeRgb for HyperX {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let slots = self.packets * PER_PACKET;
        if colors.len() > slots {
            bail!("HyperX : {} couleurs pour {slots} emplacements", colors.len());
        }
        // Chaque image commence par le paquet d'init du mode direct.
        let mut init = [0u8; PACKET];
        init[1] = 0x04;
        init[2] = 0xF2;
        init[9] = self.init9;
        self.hid.send_feature_report(&init)?;
        for p in 0..self.packets {
            let mut buf = [0u8; PACKET];
            for i in 0..PER_PACKET {
                let c = colors.get(p * PER_PACKET + i).copied().unwrap_or(Color::BLACK);
                buf[1 + 4 * i..5 + 4 * i].copy_from_slice(&[0x81, c.r, c.g, c.b]);
            }
            self.hid.send_feature_report(&buf)?;
        }
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "hyperx", doc: "docs/rgb-protocols/hyperx.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Feats = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn driver(vid: u16, pid: u16) -> (Box<dyn NativeRgb>, Feats) {
        let model = probe(vid, pid).expect("pid");
        let (mock, _, feats) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), feats)
    }

    #[test]
    fn probe_claims_documented_pids_only() {
        let m = probe(VID_HP, 0x038F).expect("origins 65");
        assert_eq!((m.led_count(), m.interface, m.usage_page), (144, Some(3), Some(0xFF90)));
        assert_eq!(probe(VID_HP, 0x0F8D).map(|m| m.led_count()), Some(32));
        assert!(probe(VID_KINGSTON, 0x1711).is_none()); // Elite 2 : INCONNU
        assert!(probe(VID_HP, 0x0CC2).is_none()); // Origins 2 65 : INCONNU
        assert!(probe(VID_KINGSTON, 0x16E6).is_none()); // Origins Core : INCONNU
    }

    #[test]
    fn keyboard_sends_init_then_nine_packets() {
        let (mut d, feats) = driver(VID_KINGSTON, 0x16E5);
        let mut c = vec![Color::BLACK; 144];
        c[0] = Color::new(1, 2, 3);
        c[16] = Color::new(4, 5, 6);
        c[143] = Color::new(7, 8, 9);
        d.set_colors(&c).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 10);
        let mut init = vec![0u8; 65];
        init[1] = 0x04;
        init[2] = 0xF2;
        init[9] = 0x09;
        assert_eq!(f[0], init);
        assert!(f.iter().all(|p| p.len() == 65 && p[0] == 0));
        assert_eq!(&f[1][1..9], &[0x81, 1, 2, 3, 0x81, 0, 0, 0]);
        assert_eq!(&f[2][1..5], &[0x81, 4, 5, 6]);
        assert_eq!(&f[9][61..65], &[0x81, 7, 8, 9]);
        // emplacement inutilisé = 81 00 00 00
        assert_eq!(&f[5][1..5], &[0x81, 0, 0, 0]);
    }

    #[test]
    fn mat_uses_init_9_equals_2_and_two_packets() {
        let (mut d, feats) = driver(VID_HP, 0x0F8D);
        d.set_colors(&[Color::new(255, 0, 0); 20]).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 3);
        assert_eq!((f[0][1], f[0][2], f[0][9]), (0x04, 0xF2, 0x02));
        assert_eq!(&f[2][1..5], &[0x81, 255, 0, 0]); // LED 16
        assert_eq!(&f[2][17..21], &[0x81, 0, 0, 0]); // LED 20 : au-delà des 20 fournies
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_is_sent() {
        let (mut d, feats) = driver(VID_HP, 0x0F8D);
        assert!(d.set_colors(&vec![Color::BLACK; 33]).is_err());
        assert!(feats.lock().is_empty());
    }
}
