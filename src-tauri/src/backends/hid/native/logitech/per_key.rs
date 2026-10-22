//! Feature 0x8080 (PER_KEY_LIGHTING) : G410 / G512 / G513 / G610 / G810 / G910, couleur par touche.
//! Source : docs/rgb-protocols/logitech.md (sections « Cadre HID++ » et « Feature 0x8080 »), analyse-logitech-ghub.md §3.3 / §6.
//!
//! Livré : IRoot (recherche de l'index de la feature 0x8080) puis SET_KEY_COLORS + FLUSH_LEDS (14 touches max par trame 0x12).
//! INCONNU : touches réellement présentes par modèle (les absentes sont envoyées telles quelles), mappage des touches spéciales (G-keys, logo,
//! multimédia = autres keyTypes, non pilotés), retour au mode matériel (aucune commande documentée : `release()` ne fait rien).
//! À VALIDER sur matériel : IRoot envoyé en rapport 0x12 (la note ne donne que le format générique), collection ouverte.

use super::{frame, lookup, Hid, NativeRgb, REPORT_VLONG, SW_ID};
use crate::core::Color;
use anyhow::{bail, Result};

const FEATURE_PER_KEY: u16 = 0x8080;
const FN_SET_KEY_COLORS: u8 = 0x30;
const FN_FLUSH_LEDS: u8 = 0x50;
/// keyType « clavier » (bit 0x01) : keyId = usage HID.
const KEY_TYPE_KEYBOARD: u16 = 0x0001;
const KEYS_PER_FRAME: usize = 14;

/// LED i -> usage HID clavier (standard USB HID : 0x04-0x65 alphanumérique/ponctuation/F1-F12/navigation, 0xE0-0xE7 modificateurs).
fn key_ids() -> impl Iterator<Item = u8> {
    (0x04u8..=0x65).chain(0xE0..=0xE7)
}
pub(super) const LEDS: usize = 0x62 + 8;

struct Logitech {
    hid: Box<dyn Hid>,
    /// Index de la feature 0x8080 dans la table de l'appareil.
    feature: u8,
}

pub(super) fn open(hid: Box<dyn Hid>) -> Result<Box<dyn NativeRgb>> {
    let Some(feature) = lookup(hid.as_ref(), REPORT_VLONG, FEATURE_PER_KEY)? else {
        bail!("Logitech : feature {FEATURE_PER_KEY:#06x} absente de cet appareil");
    };
    Ok(Box::new(Logitech { hid, feature }))
}

impl NativeRgb for Logitech {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > LEDS {
            bail!("Logitech : {} couleurs pour {LEDS} touches", colors.len());
        }
        let all: Vec<(u8, Color)> = key_ids().enumerate().map(|(i, k)| (k, colors.get(i).copied().unwrap_or(Color::BLACK))).collect();
        for chunk in all.chunks(KEYS_PER_FRAME) {
            let mut f = frame(REPORT_VLONG, self.feature, FN_SET_KEY_COLORS | SW_ID);
            f[4..6].copy_from_slice(&KEY_TYPE_KEYBOARD.to_be_bytes());
            f[6..8].copy_from_slice(&(chunk.len() as u16).to_be_bytes());
            for (i, (k, c)) in chunk.iter().enumerate() {
                f[8 + 4 * i..12 + 4 * i].copy_from_slice(&[*k, c.r, c.g, c.b]);
            }
            self.hid.write(&f)?;
        }
        self.hid.write(&frame(REPORT_VLONG, self.feature, FN_FLUSH_LEDS | SW_ID))?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::super::super::MockHid;
    use super::*;

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn iroot_reply(index: u8) -> Vec<u8> {
        let mut r = vec![0u8; 20];
        r[..5].copy_from_slice(&[0x11, 0xFF, 0x00, 0x07, index]);
        r
    }

    fn driver(replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, Sent) {
        let (mock, sent, _) = MockHid::new(replies);
        (open(Box::new(mock)), sent)
    }

    #[test]
    fn iroot_request_is_exact() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        assert!(d.is_ok());
        let f = sent.lock()[0].clone();
        let mut want = vec![0u8; 64];
        want[..6].copy_from_slice(&[0x12, 0xFF, 0x00, 0x07, 0x80, 0x80]);
        assert_eq!(f, want);
    }

    #[test]
    fn open_refuses_absent_feature_error_and_silence() {
        assert!(driver(vec![iroot_reply(0)]).0.is_err());
        let mut err = iroot_reply(0);
        err[2] = 0xFF;
        assert!(driver(vec![err]).0.is_err());
        assert!(driver(vec![]).0.is_err());
    }

    #[test]
    fn open_skips_unrelated_reports_before_the_answer() {
        let mut noise = vec![0u8; 20];
        noise[..4].copy_from_slice(&[0x11, 0xFF, 0x05, 0x10]);
        assert!(driver(vec![noise, iroot_reply(0x0B)]).0.is_ok());
    }

    #[test]
    fn set_colors_frames_and_flush_match_the_note() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        let mut d = d.expect("open");
        let mut c = vec![Color::BLACK; 106];
        c[0] = Color::new(1, 2, 3); // usage 0x04
        c[13] = Color::new(9, 8, 7); // usage 0x11, dernière touche de la trame 1
        c[14] = Color::new(4, 5, 6); // usage 0x12, première de la trame 2
        d.set_colors(&c).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 1 + 8 + 1); // IRoot + 8 trames (7 x 14 + 8) + FLUSH
        let f1 = &s[1];
        assert_eq!(f1.len(), 64);
        assert_eq!(&f1[..8], &[0x12, 0xFF, 0x0B, 0x37, 0x00, 0x01, 0x00, 0x0E]);
        assert_eq!(&f1[8..12], &[0x04, 1, 2, 3]);
        assert_eq!(&f1[8 + 52..8 + 56], &[0x11, 9, 8, 7]);
        assert_eq!(&s[2][6..8], &[0x00, 0x0E]);
        assert_eq!(&s[2][8..12], &[0x12, 4, 5, 6]);
        assert_eq!(&s[8][6..8], &[0x00, 0x08]); // dernière trame : 8 touches
        assert_eq!(&s[8][8 + 28..8 + 32], &[0xE7, 0, 0, 0]);
        assert!(s[8][8 + 32..].iter().all(|b| *b == 0));
        let mut flush = vec![0u8; 64];
        flush[..4].copy_from_slice(&[0x12, 0xFF, 0x0B, 0x57]);
        assert_eq!(s[9], flush);
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_more_is_sent() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        let mut d = d.expect("open");
        assert!(d.set_colors(&vec![Color::BLACK; 107]).is_err());
        assert_eq!(sent.lock().len(), 1); // seulement l'IRoot
    }
}
