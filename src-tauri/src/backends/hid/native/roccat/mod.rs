//! Famille « roccat » (VID 0x1E7D) : souris Kone Pro filaire (0x2C88), Burst Pro (0x2DE1), Burst Pro Air (0x2CA6) ici ;
//! claviers Vulcan 100/120 Aimo et Magma/Magma Mini dans `keyboard.rs` (deux collections HID).
//! Sources : docs/rgb-protocols/roccat-turtlebeach.md (§ Kone Pro), docs/rgb-protocols/analyse-roccat-swarm.md (§ 4, source O
//! = OpenRGB, NON confirmée par les binaires officiels : tout est à valider sur matériel).
//!
//! - Kone Pro / Burst Pro : prise de contrôle feature `0E 06 01 <direct> 00 FF`, couleurs feature de 11 octets `0D 0B` + RVB à `2+3i` (2 LEDs).
//! - Burst Pro Air : pas de prise de contrôle séparée ; feature de 30 octets `06 01 4C 06 14` + 4 x (n° 1..4, 0xFF, R, G, B).
//!
//! Non revendiqués :
//! - Claviers non revendiqués : Vulcan Pro/TKL/II, Pyro, TB Vulcan II (voir `keyboard.rs`), Vulcan 121 et Kain 100/120/200 : PID INCONNU.
//! - Burst Core (0x2DE6) : interface HID non indiquée dans la note. Kone Pro Air (0x2C8E, 0x2C92) : champs `[4]`, valeurs de luminosité en
//!   direct et requêtes préalables non élucidés. Kone Aimo (0x2E27, 0x2E2C) : interface ambiguë (0 ou 1) et valeur de l'index LED non précisée.
//!   Kone XP (0x2C8B) : passe par un profil de 0xAE octets (écriture de config, exclue). Kova, Sense, Horde, Elo, KP7 : contrôleurs non lus.
//!
//! INCONNU : noms/emplacements des LEDs ; valeur exacte du champ `<direct>` (supposé 1 = direct, 0 = rendre la main, par analogie
//! avec le champ `<on>` des claviers de la note) ; effets matériels non utilisés ; Burst Pro Air : retour au mode matériel (la trame de
//! mode `06 01 4C 06 06 <mode> <vitesse> <luminosité> 0F` n'a pas de valeurs documentées pour « reprendre le mode précédent »), `release` ne fait rien.

mod keyboard;

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x1E7D;
const PID_KONE_PRO: u16 = 0x2C88;
const PID_BURST_PRO: u16 = 0x2DE1;
const PID_BURST_PRO_AIR: u16 = 0x2CA6;

/// Variantes de trame documentées.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Kone Pro, Burst Pro : 2 LEDs, prise de contrôle + feature de 11 octets.
    TwoLed,
    /// Burst Pro Air : 4 LEDs, feature de 30 octets, sans prise de contrôle.
    FourLed,
}

impl Kind {
    fn leds(self) -> usize {
        match self {
            Kind::TwoLed => 2,
            Kind::FourLed => 4,
        }
    }
}

/// (variante, nom, interface HID, page d'usage) d'après la note.
fn kind_of(pid: u16) -> Option<(Kind, &'static str, i32, u16)> {
    match pid {
        PID_KONE_PRO => Some((Kind::TwoLed, "Roccat Kone Pro", 3, 0xFF01)),
        PID_BURST_PRO => Some((Kind::TwoLed, "Roccat Burst Pro", 3, 0xFF01)),
        // Interface 0, page d'usage 0x01 (usage 2 : non filtrable par Model).
        PID_BURST_PRO_AIR => Some((Kind::FourLed, "Roccat Burst Pro Air", 0, 0x0001)),
        _ => None,
    }
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    if let Some(m) = keyboard::probe(pid) {
        return Some(m);
    }
    let (kind, name, interface, usage_page) = kind_of(pid)?;
    let zones = (1..=kind.leds())
        .map(|i| {
            let mut z = ZoneInfo::fixed(format!("LED {i}"), 1);
            z.zone_type = 0;
            z
        })
        .collect();
    let mut m = Model::new(name, DeviceType::Mouse, zones);
    m.interface = Some(interface);
    m.usage_page = Some(usage_page);
    Some(m)
}

struct Mouse {
    hid: Box<dyn Hid>,
    kind: Kind,
}

impl Mouse {
    /// Feature `0E 06 01 <direct> 00 FF` (Kone Pro, Burst Pro).
    fn control_mode(&self, direct: bool) -> Result<()> {
        self.hid.send_feature_report(&[0x0E, 0x06, 0x01, direct as u8, 0x00, 0xFF])
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.aux.is_some() {
        return keyboard::open(hid, model);
    }
    // Le modèle vient de `probe` : la variante se déduit du nombre de LEDs.
    let kind = if model.led_count() as usize == Kind::FourLed.leds() { Kind::FourLed } else { Kind::TwoLed };
    let dev = Mouse { hid, kind };
    if kind == Kind::TwoLed {
        dev.control_mode(true)?;
    }
    Ok(Box::new(dev))
}

impl NativeRgb for Mouse {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let leds = self.kind.leds();
        if colors.len() > leds {
            bail!("Roccat : {} couleurs pour {leds} LEDs", colors.len());
        }
        match self.kind {
            Kind::TwoLed => {
                // Feature de 11 octets : id 0x0D, 0x0B, puis RVB de la LED i aux octets 2+3i..4+3i ; le reste à 0.
                let mut f = [0u8; 11];
                f[0] = 0x0D;
                f[1] = 0x0B;
                for (i, c) in colors.iter().enumerate() {
                    f[2 + 3 * i..5 + 3 * i].copy_from_slice(&[c.r, c.g, c.b]);
                }
                self.hid.send_feature_report(&f)
            }
            Kind::FourLed => {
                // Feature de 30 octets : `06 01 4C 06 14`, puis par LED i (5 octets) : n° (1..4), 0xFF, R, G, B.
                let mut f = [0u8; 30];
                f[..5].copy_from_slice(&[0x06, 0x01, 0x4C, 0x06, 0x14]);
                for i in 0..leds {
                    let c = colors.get(i).copied().unwrap_or(Color::BLACK);
                    f[5 + 5 * i..10 + 5 * i].copy_from_slice(&[i as u8 + 1, 0xFF, c.r, c.g, c.b]);
                }
                self.hid.send_feature_report(&f)
            }
        }
    }

    fn release(&mut self) {
        // Rendre la main au mode matériel ; rien à rattraper si l'envoi échoue à la fermeture.
        // Burst Pro Air : aucune trame de sortie documentée (voir en-tête).
        if self.kind == Kind::TwoLed {
            let _ = self.control_mode(false);
        }
    }
}

pub const FAMILY: Family = Family { id: "roccat", doc: "docs/rgb-protocols/analyse-roccat-swarm.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::{probe as global_probe, MockHid};

    #[test]
    fn probe_claims_only_documented_mice() {
        for pid in [0x2C8E, 0x2C92, 0x2DE6, 0x2E27, 0x2E2C, 0x2C8B, 0x2F4E, 0x314C, 0x2CEE] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x10F5, 0x501B).is_none());
        assert!(probe(0x10F5, 0x2C88).is_none());
        let m = probe(VID, 0x2C88).expect("Kone Pro");
        assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFF01), 2));
        let m = probe(VID, 0x2DE1).expect("Burst Pro");
        assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFF01), 2));
        let m = probe(VID, 0x2CA6).expect("Burst Pro Air");
        assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(0), Some(0x0001), 4));
        for pid in [0x2C88, 0x2DE1, 0x2CA6] {
            assert_eq!(global_probe(VID, pid).map(|(f, _)| f.id), Some("roccat"));
        }
    }

    #[test]
    fn two_led_mice_send_documented_features() {
        for pid in [0x2C88, 0x2DE1] {
            let (mock, sent, feats) = MockHid::new(vec![]);
            let m = probe(VID, pid).expect("modèle");
            let mut dev = open(Box::new(mock), &m).expect("open");
            dev.set_colors(&[Color::new(1, 2, 3), Color::new(4, 5, 6)]).expect("set");
            dev.set_colors(&[Color::new(9, 8, 7)]).expect("set");
            dev.release();
            let f = feats.lock();
            assert_eq!(f.len(), 4);
            assert_eq!(f[0], vec![0x0E, 0x06, 0x01, 0x01, 0x00, 0xFF]);
            assert_eq!(f[1], vec![0x0D, 0x0B, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
            assert_eq!(f[2], vec![0x0D, 0x0B, 9, 8, 7, 0, 0, 0, 0, 0, 0]);
            assert_eq!(f[3], vec![0x0E, 0x06, 0x01, 0x00, 0x00, 0xFF]);
            assert!(sent.lock().is_empty());
        }
    }

    #[test]
    fn burst_pro_air_sends_30_byte_feature_without_control_frames() {
        let (mock, sent, feats) = MockHid::new(vec![]);
        let m = probe(VID, 0x2CA6).expect("Burst Pro Air");
        let mut dev = open(Box::new(mock), &m).expect("open");
        assert!(feats.lock().is_empty()); // pas de prise de contrôle séparée
        dev.set_colors(&[Color::new(1, 2, 3), Color::new(4, 5, 6), Color::new(7, 8, 9)]).expect("set");
        dev.release();
        let f = feats.lock();
        assert_eq!(f.len(), 1); // release n'envoie rien
        assert_eq!(
            f[0],
            vec![
                0x06, 0x01, 0x4C, 0x06, 0x14, //
                1, 0xFF, 1, 2, 3, //
                2, 0xFF, 4, 5, 6, //
                3, 0xFF, 7, 8, 9, //
                4, 0xFF, 0, 0, 0, //
                0, 0, 0, 0, 0 // 30 octets au total : 5 d'en-tête + 4 x 5, reste à 0
            ]
        );
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn too_many_colors_are_refused_without_sending() {
        for (pid, n) in [(0x2C88u16, 3usize), (0x2DE1, 3), (0x2CA6, 5)] {
            let (mock, _, feats) = MockHid::new(vec![]);
            let m = probe(VID, pid).expect("modèle");
            let mut dev = open(Box::new(mock), &m).expect("open");
            let before = feats.lock().len();
            assert!(dev.set_colors(&vec![Color::BLACK; n]).is_err());
            assert_eq!(feats.lock().len(), before);
        }
    }
}
