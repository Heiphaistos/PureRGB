//! Famille « wooting » : claviers Wooting V2 (Two / HE) et V3 (80HE), couleurs RGB565 en un seul rapport.
//! Source : docs/rgb-protocols/wooting-keychron-qmk.md
//!
//! INCONNU (donc non revendiqué) : Wooting One / 60HE / UwU (largeur de matrice et version V2/V3 non précisées),
//! V1 legacy (tables PWM non transcrites), commandes 30-33, Keychron et QMK/VIA/Vial (hors lot).
//! INCONNU : nombre réel de colonnes du 80HE ; on expose la matrice brute 6 x 21 « même disposition que V2 ».

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x31E3;
/// Matrice 6 lignes x 21 colonnes.
const SLOTS: usize = 126;
const UP_V2: u16 = 0x1337;
const UP_V3: u16 = 0xFF55;
const V2_LEN: usize = 257;
const V3_LEN: usize = 2046;

/// (PID de base, nom, page d'usage). Le PID réel porte un suffixe de mode d'entrée 0, 1 ou 2 (poids faible).
const MODELS: &[(u16, &str, u16)] = &[
    (0x1200, "Wooting Two", UP_V2),
    (0x1210, "Wooting Two Lekker Edition", UP_V2),
    (0x1220, "Wooting Two HE", UP_V2),
    (0x1230, "Wooting Two HE (ARM)", UP_V2),
    (0x1400, "Wooting 80HE", UP_V3),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || pid & 0x000F > 2 {
        return None;
    }
    let (_, name, up) = MODELS.iter().find(|(base, _, _)| *base == pid & 0xFFF0)?;
    let mut m = Model::new(*name, DeviceType::Keyboard, vec![ZoneInfo::fixed("Matrice 6x21", SLOTS as u32)]);
    m.usage_page = Some(*up);
    Some(m)
}

struct Wooting {
    hid: Box<dyn Hid>,
    /// Longueur du rapport.
    len: usize,
    /// Octet 0 (report id) puis les trois octets magiques.
    head: [u8; 4],
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let (len, head) = match model.usage_page {
        Some(UP_V2) => (V2_LEN, [0x00, 0xD0, 0xDA, 0x0B]),
        Some(UP_V3) => (V3_LEN, [0x04, 0xD1, 0xDA, 0x0B]),
        other => bail!("Wooting : page d'usage {other:?} non prise en charge"),
    };
    Ok(Box::new(Wooting { hid, len, head }))
}

/// R5 G6 B5.
fn rgb565(c: Color) -> u16 {
    (u16::from(c.r >> 3) << 11) | (u16::from(c.g >> 2) << 5) | u16::from(c.b >> 3)
}

impl NativeRgb for Wooting {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > SLOTS {
            bail!("Wooting : {} couleurs pour {SLOTS} emplacements", colors.len());
        }
        let mut buf = vec![0u8; self.len];
        buf[..4].copy_from_slice(&self.head);
        for (i, c) in colors.iter().enumerate() {
            buf[4 + 2 * i..6 + 2 * i].copy_from_slice(&rgb565(*c).to_le_bytes());
        }
        self.hid.write(&buf)?;
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "wooting", doc: "docs/rgb-protocols/wooting-keychron-qmk.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn driver(pid: u16) -> (Box<dyn NativeRgb>, Sent) {
        let model = probe(VID, pid).expect("pid");
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent)
    }

    #[test]
    fn probe_claims_only_documented_pids_with_mode_suffix() {
        assert_eq!(probe(VID, 0x1200).map(|m| m.usage_page), Some(Some(0x1337)));
        assert_eq!(probe(VID, 0x1222).map(|m| m.usage_page), Some(Some(0x1337)));
        assert_eq!(probe(VID, 0x1401).map(|m| m.usage_page), Some(Some(0xFF55)));
        assert!(probe(VID, 0x1203).is_none()); // suffixe non documenté
        assert!(probe(VID, 0x1100).is_none()); // Wooting One : INCONNU
        assert!(probe(0x03EB, 0xFF02).is_none()); // V1 legacy : non revendiqué
    }

    #[test]
    fn v2_frame_red_green_blue_and_last_slot() {
        let (mut d, sent) = driver(0x1200);
        let mut c = vec![Color::BLACK; 126];
        c[0] = Color::new(255, 0, 0);
        c[1] = Color::new(0, 255, 0);
        c[2] = Color::new(0, 0, 255);
        c[125] = Color::new(255, 255, 255);
        d.set_colors(&c).expect("set");
        let f = sent.lock()[0].clone();
        assert_eq!(f.len(), 257);
        assert_eq!(&f[..4], &[0x00, 0xD0, 0xDA, 0x0B]);
        assert_eq!(&f[4..10], &[0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00]);
        assert_eq!(&f[254..256], &[0xFF, 0xFF]);
        assert_eq!(f[256], 0);
    }

    #[test]
    fn v3_frame_header_and_length() {
        let (mut d, sent) = driver(0x1400);
        d.set_colors(&[Color::new(255, 0, 0)]).expect("set");
        let f = sent.lock()[0].clone();
        assert_eq!(f.len(), 2046);
        assert_eq!(&f[..6], &[0x04, 0xD1, 0xDA, 0x0B, 0x00, 0xF8]);
        assert!(f[6..].iter().all(|b| *b == 0));
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_is_sent() {
        let (mut d, sent) = driver(0x1200);
        assert!(d.set_colors(&vec![Color::BLACK; 127]).is_err());
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn open_refuses_unknown_usage_page() {
        let mut m = probe(VID, 0x1200).expect("pid");
        m.usage_page = None;
        let (mock, _, _) = MockHid::new(vec![]);
        assert!(open(Box::new(mock), &m).is_err());
    }
}
