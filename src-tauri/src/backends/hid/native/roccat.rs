//! Famille « roccat » (VID 0x1E7D) : souris Kone Pro filaire (PID 0x2C88), 2 LEDs en mode direct.
//! Source : docs/rgb-protocols/roccat-turtlebeach.md (§ Kone Pro), signalrgb-devices.csv (2 LEDs).
//!
//! Non revendiqués (faits insuffisants dans la note) :
//! - Claviers Vulcan / Pyro / Magma (Roccat et Turtle Beach 0x10F5) : en-tête des paquets suivants, ordre des octets de la
//!   longueur (dépend de la version du firmware, lecture non documentée), correspondance LED -> touche (INCONNU).
//! - Kone Pro Air (0x2C8E, 0x2C92) : la note ne dit pas qu'ils partagent le contrôleur du Kone Pro.
//! - Burst, Kone Aimo / XP, Kova, Sense, Elo, Horde Aimo, Turtle Beach KP7 : contrôleurs non lus.
//!
//! INCONNU : noms/emplacements des 2 LEDs ; valeur exacte du champ `<direct>` (supposé 1 = direct, 0 = rendre la main, par analogie
//! avec le champ `<on>` des claviers de la note) ; effets matériels (rapport 0x06) non utilisés.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x1E7D;
const PID_KONE_PRO: u16 = 0x2C88;
const LEDS: usize = 2;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if (vid, pid) != (VID, PID_KONE_PRO) {
        return None;
    }
    let zones = (1..=LEDS)
        .map(|i| {
            let mut z = ZoneInfo::fixed(format!("LED {i}"), 1);
            z.zone_type = 0;
            z
        })
        .collect();
    let mut m = Model::new("Roccat Kone Pro", DeviceType::Mouse, zones);
    m.interface = Some(3);
    m.usage_page = Some(0xFF01);
    Some(m)
}

struct KonePro {
    hid: Box<dyn Hid>,
}

impl KonePro {
    /// Feature `0E 06 01 <direct> 00 FF`.
    fn control_mode(&self, direct: bool) -> Result<()> {
        self.hid.send_feature_report(&[0x0E, 0x06, 0x01, direct as u8, 0x00, 0xFF])
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    let dev = KonePro { hid };
    dev.control_mode(true)?;
    Ok(Box::new(dev))
}

impl NativeRgb for KonePro {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > LEDS {
            bail!("Roccat Kone Pro : {} couleurs pour {LEDS} LEDs", colors.len());
        }
        // Feature de 11 octets : id 0x0D, 0x0B, puis RVB de la LED i aux octets 2+3i..4+3i ; le reste à 0.
        let mut f = [0u8; 11];
        f[0] = 0x0D;
        f[1] = 0x0B;
        for (i, c) in colors.iter().enumerate() {
            f[2 + 3 * i..5 + 3 * i].copy_from_slice(&[c.r, c.g, c.b]);
        }
        self.hid.send_feature_report(&f)
    }

    fn release(&mut self) {
        // Rendre la main au mode matériel ; rien à rattraper si l'envoi échoue à la fermeture.
        let _ = self.control_mode(false);
    }
}

pub const FAMILY: Family = Family { id: "roccat", doc: "docs/rgb-protocols/roccat-turtlebeach.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::{probe as global_probe, MockHid};

    #[test]
    fn probe_claims_only_kone_pro() {
        assert!(probe(VID, 0x2C8E).is_none()); // Kone Pro Air : non documenté comme identique
        assert!(probe(VID, 0x2F4E).is_none()); // Vulcan II : INCONNU
        assert!(probe(0x10F5, 0x501B).is_none());
        let m = probe(VID, 0x2C88).expect("Kone Pro");
        assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFF01), 2));
        assert_eq!(global_probe(VID, 0x2C88).map(|(f, _)| f.id), Some("roccat"));
    }

    #[test]
    fn open_set_and_release_send_documented_features() {
        let (mock, sent, feats) = MockHid::new(vec![]);
        let m = probe(VID, 0x2C88).expect("Kone Pro");
        let mut dev = open(Box::new(mock), &m).expect("open");
        dev.set_colors(&[Color::new(1, 2, 3), Color::new(4, 5, 6)]).expect("set");
        dev.set_colors(&[Color::new(9, 8, 7)]).expect("set");
        dev.release();
        let f = feats.lock();
        assert_eq!(f[0], vec![0x0E, 0x06, 0x01, 0x01, 0x00, 0xFF]);
        assert_eq!(f[1], vec![0x0D, 0x0B, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
        assert_eq!(f[2], vec![0x0D, 0x0B, 9, 8, 7, 0, 0, 0, 0, 0, 0]);
        assert_eq!(f[3], vec![0x0E, 0x06, 0x01, 0x00, 0x00, 0xFF]);
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn too_many_colors_are_refused() {
        let (mock, _, _) = MockHid::new(vec![]);
        let m = probe(VID, 0x2C88).expect("Kone Pro");
        let mut dev = open(Box::new(mock), &m).expect("open");
        assert!(dev.set_colors(&[Color::BLACK; 3]).is_err());
    }
}
