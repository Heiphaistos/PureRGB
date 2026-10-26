//! Famille « hyperx » : schéma `0x81` en feature reports de 65 octets (claviers Alloy Origins / Origins 60 / Origins 65 / Elite II,
//! tapis Pulsefire Mat et FURY Ultra, souris Pulsefire Haste filaire). Lumière directe seulement, rien n'est écrit en mémoire flash.
//!
//! Source : docs/rgb-protocols/hyperx-ngenuity.md (analyse de HyperX NGENUITY 5.38, 2026-10-26, marqueur [V]) et
//! docs/rgb-protocols/hyperx.md (OpenRGB, marqueur (O)). Aucun code ni asset HyperX copié : seuls des faits de format.
//!
//! Séquence d'une image [V] : (1) feature report d'init `00 04 F2 00.. [9]=<init9>` ; (2) lecture d'un feature report (id 0), l'appareil
//! acquitte par `[2]=F2` et `[4]=01` (sinon l'image est abandonnée, comme le fait NGENUITY) ; (3) `packets` feature reports dont le premier octet est
//! l'identifiant de rapport `led_id`, puis 16 LED par rapport à 4 octets `81 R V B` aux octets `1+4i`. Les emplacements non fournis restent à zéro
//! (NGENUITY n'écrit pas de marqueur `81` pour eux). Les souris Haste n'attendent pas d'acquittement.
//!
//! Identifiant de rapport des paquets de LED : 0 (Origins, Elite II, Haste), 7 (Origins 60/65), 4 (tapis) — tel que NGENUITY l'envoie ;
//! l'init part toujours avec l'identifiant 0. Non éprouvé sur matériel ici.
//!
//! INCONNU (donc non revendiqué) : Alloy Origins Core (rapports de sortie `A2`, protocole différent), Alloy Elite/FPS (rapport 0x07 de 264 octets),
//! souris Surge/Raid/Core/FPS Pro/Dart/Haste sans fil (le mode « application » demande d'abord un changement de profil actif, persistance non établie),
//! micros, casques. INCONNU : correspondance LED -> touche (les emplacements sont exposés bruts, dans l'ordre des paquets).

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID_KINGSTON: u16 = 0x0951;
const VID_HP: u16 = 0x03F0;
const PACKET: usize = 65;
const PER_PACKET: usize = 16;
const UP_HP: u16 = 0xFF90;
/// Identifiant de commande de l'init du mode direct (octet 2 du rapport d'init et de l'acquittement).
const CMD_DIRECT: u8 = 0xF2;

/// Paramètres de trame d'un modèle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spec {
    /// Octet 9 du rapport d'init.
    init9: u8,
    /// Nombre de LED exposées.
    leds: usize,
    /// Nombre de feature reports de LED envoyés par image (16 LED chacun, le surplus est à zéro).
    packets: usize,
    /// Identifiant de rapport (octet 0) des paquets de LED.
    led_id: u8,
    /// Lire l'acquittement de l'init avant d'envoyer les LED.
    ack: bool,
    device_type: DeviceType,
    zone: &'static str,
}

const ORIGINS: Spec = Spec { init9: 9, leds: 144, packets: 9, led_id: 0, ack: true, device_type: DeviceType::Keyboard, zone: "Emplacements" };
const ELITE2: Spec = ORIGINS;
const ORIGINS_60_65: Spec = Spec { init9: 5, leds: 80, packets: 5, led_id: 7, ack: true, device_type: DeviceType::Keyboard, zone: "Emplacements" };
const FURY_ULTRA: Spec = Spec { init9: 2, leds: 24, packets: 2, led_id: 4, ack: true, device_type: DeviceType::Mousemat, zone: "LEDs" };
const MAT_RGB: Spec = Spec { init9: 2, leds: 2, packets: 2, led_id: 4, ack: true, device_type: DeviceType::Mousemat, zone: "LEDs" };
const HASTE: Spec = Spec { init9: 2, leds: 1, packets: 2, led_id: 0, ack: false, device_type: DeviceType::Mouse, zone: "LED" };

/// (VID, PID, nom, paramètres, interface, page d'usage). Interface 3 pour les claviers (l'entrée alternative « interface 0 » de la note n'est pas retenue).
const MODELS: &[(u16, u16, &str, Spec, i32, Option<u16>)] = &[
    (VID_KINGSTON, 0x1734, "HyperX Alloy Origins 60", ORIGINS_60_65, 3, None),
    (VID_HP, 0x0C8E, "HyperX Alloy Origins 60 (HP)", ORIGINS_60_65, 3, Some(UP_HP)),
    (VID_HP, 0x038F, "HyperX Alloy Origins 65 (HP)", ORIGINS_60_65, 3, Some(UP_HP)),
    (VID_KINGSTON, 0x16E5, "HyperX Alloy Origins", ORIGINS, 3, None),
    (VID_HP, 0x0591, "HyperX Alloy Origins (HP)", ORIGINS, 3, Some(UP_HP)),
    (VID_KINGSTON, 0x1711, "HyperX Alloy Elite 2", ELITE2, 3, None),
    (VID_HP, 0x058F, "HyperX Alloy Elite 2 (HP)", ELITE2, 3, Some(UP_HP)),
    (VID_KINGSTON, 0x1705, "HyperX FURY Ultra", FURY_ULTRA, 0, None),
    (VID_HP, 0x0493, "HyperX FURY Ultra (HP)", FURY_ULTRA, 0, None),
    (VID_HP, 0x0F8D, "HyperX Pulsefire Mat", MAT_RGB, 1, Some(UP_HP)),
    (VID_KINGSTON, 0x1741, "HyperX Pulsefire Mat RGB XL", MAT_RGB, 1, Some(UP_HP)),
    (VID_KINGSTON, 0x1727, "HyperX Pulsefire Haste", HASTE, 3, None),
    (VID_HP, 0x0F8F, "HyperX Pulsefire Haste (HP)", HASTE, 3, Some(UP_HP)),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let (_, _, name, spec, interface, up) = MODELS.iter().find(|m| m.0 == vid && m.1 == pid)?;
    let mut m = Model::new(*name, spec.device_type, vec![ZoneInfo::fixed(spec.zone, spec.leds as u32)]);
    m.interface = Some(*interface);
    m.usage_page = *up;
    Some(m)
}

struct HyperX {
    hid: Box<dyn Hid>,
    spec: Spec,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let Some((_, _, _, spec, _, _)) = MODELS.iter().find(|m| m.2 == model.name) else {
        bail!("HyperX : modèle {} non pris en charge", model.name);
    };
    Ok(Box::new(HyperX { hid, spec: *spec }))
}

impl HyperX {
    /// Lit l'acquittement de l'init : `[2]` = commande, `[4]` = 1.
    fn acked(&self) -> Result<bool> {
        std::thread::sleep(std::time::Duration::from_millis(1));
        let mut buf = [0u8; PACKET];
        let n = self.hid.get_feature_report(&mut buf)?;
        Ok(n > 4 && buf[2] == CMD_DIRECT && buf[4] == 1)
    }
}

impl NativeRgb for HyperX {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let s = self.spec;
        if colors.len() > s.leds {
            bail!("HyperX : {} couleurs pour {} emplacements", colors.len(), s.leds);
        }
        let mut init = [0u8; PACKET];
        init[1] = 0x04;
        init[2] = CMD_DIRECT;
        init[9] = s.init9;
        self.hid.send_feature_report(&init)?;
        if s.ack && !self.acked()? {
            bail!("HyperX : init du mode direct non acquittée");
        }
        for p in 0..s.packets {
            let mut buf = [0u8; PACKET];
            buf[0] = s.led_id;
            for i in 0..PER_PACKET {
                if let Some(c) = colors.get(p * PER_PACKET + i) {
                    buf[1 + 4 * i..5 + 4 * i].copy_from_slice(&[0x81, c.r, c.g, c.b]);
                }
            }
            self.hid.send_feature_report(&buf)?;
        }
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "hyperx", doc: "docs/rgb-protocols/hyperx-ngenuity.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Feats = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    /// Réponse d'acquittement : id 0, `[2]=F2`, `[4]=1`.
    fn ack() -> Vec<u8> {
        let mut r = vec![0u8; PACKET];
        r[2] = CMD_DIRECT;
        r[4] = 1;
        r
    }

    fn driver_with(vid: u16, pid: u16, replies: Vec<Vec<u8>>) -> (Box<dyn NativeRgb>, Feats) {
        let model = probe(vid, pid).expect("pid");
        let (mock, _, feats) = MockHid::new(replies);
        (open(Box::new(mock), &model).expect("open"), feats)
    }

    fn driver(vid: u16, pid: u16) -> (Box<dyn NativeRgb>, Feats) {
        driver_with(vid, pid, vec![ack()])
    }

    #[test]
    fn probe_claims_documented_pids_only() {
        let m = probe(VID_HP, 0x038F).expect("origins 65");
        assert_eq!((m.led_count(), m.interface, m.usage_page), (80, Some(3), Some(0xFF90)));
        assert_eq!(probe(VID_KINGSTON, 0x16E5).map(|m| m.led_count()), Some(144));
        assert_eq!(probe(VID_KINGSTON, 0x1711).map(|m| m.led_count()), Some(144)); // Elite 2 : [V] depuis NGENUITY
        assert_eq!(probe(VID_HP, 0x0F8D).map(|m| (m.led_count(), m.device_type)), Some((2, DeviceType::Mousemat)));
        assert_eq!(probe(VID_HP, 0x0F8F).map(|m| (m.led_count(), m.device_type)), Some((1, DeviceType::Mouse)));
        assert!(probe(VID_HP, 0x0CC2).is_none()); // Origins 2 65 : INCONNU
        assert!(probe(VID_KINGSTON, 0x16E6).is_none()); // Origins Core : protocole différent, non revendiqué
        assert!(probe(VID_HP, 0x0D8E).is_none()); // PID de chargeur d'amorçage (DFU) : jamais revendiqué
        assert!(probe(VID_HP, 0x0691).is_none());
    }

    #[test]
    fn origins_sends_init_ack_then_nine_packets() {
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
    }

    #[test]
    fn missing_leds_stay_zero_without_marker() {
        let (mut d, feats) = driver(VID_KINGSTON, 0x1711);
        d.set_colors(&[Color::new(9, 9, 9)]).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 10); // Elite II : init + 9 paquets
        assert_eq!(f[0][9], 9);
        assert_eq!(&f[1][1..9], &[0x81, 9, 9, 9, 0, 0, 0, 0]);
        assert!(f[2..].iter().all(|p| p.iter().all(|b| *b == 0)));
    }

    #[test]
    fn origins_60_65_use_five_packets_init_9_is_5_and_report_id_7() {
        let (mut d, feats) = driver(VID_KINGSTON, 0x1734);
        d.set_colors(&vec![Color::new(10, 20, 30); 80]).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 6);
        assert_eq!((f[0][0], f[0][1], f[0][2], f[0][9]), (0, 0x04, 0xF2, 5));
        assert!(f[1..].iter().all(|p| p[0] == 7 && p.len() == 65));
        assert_eq!(&f[5][61..65], &[0x81, 10, 20, 30]); // LED 79
    }

    #[test]
    fn mat_sends_two_leds_then_a_zero_packet_with_report_id_4() {
        let (mut d, feats) = driver(VID_HP, 0x0F8D);
        d.set_colors(&[Color::new(255, 0, 0), Color::new(0, 0, 255)]).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 3);
        assert_eq!((f[0][0], f[0][1], f[0][2], f[0][9]), (0, 0x04, 0xF2, 0x02));
        assert_eq!(f[1][0], 4);
        assert_eq!(&f[1][1..9], &[0x81, 255, 0, 0, 0x81, 0, 0, 255]);
        assert_eq!(f[2][0], 4);
        assert!(f[2][1..].iter().all(|b| *b == 0));
    }

    #[test]
    fn haste_does_not_wait_for_an_ack_and_sends_one_led() {
        let (mut d, feats) = driver_with(VID_KINGSTON, 0x1727, vec![]);
        d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
        let f = feats.lock();
        assert_eq!(f.len(), 3);
        assert_eq!((f[0][1], f[0][2], f[0][9]), (0x04, 0xF2, 0x02));
        assert_eq!(&f[1][..5], &[0, 0x81, 1, 2, 3]);
        assert!(f[2].iter().all(|b| *b == 0));
    }

    #[test]
    fn a_missing_ack_aborts_the_frame_after_the_init() {
        let (mut d, feats) = driver_with(VID_KINGSTON, 0x16E5, vec![vec![0u8; PACKET]]);
        assert!(d.set_colors(&[Color::BLACK]).is_err());
        assert_eq!(feats.lock().len(), 1); // seule l'init est partie
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_is_sent() {
        let (mut d, feats) = driver(VID_HP, 0x0F8D);
        assert!(d.set_colors(&vec![Color::BLACK; 3]).is_err());
        assert!(feats.lock().is_empty());
    }
}
