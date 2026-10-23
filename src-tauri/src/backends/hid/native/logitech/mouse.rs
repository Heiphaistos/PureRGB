//! Souris filaires Logitech à index de feature fixes (pas d'énumération IRoot : la note donne les index).
//! Source : docs/rgb-protocols/analyse-logitech-ghub.md §4.2 (index fixes, mode direct, G203 Lightsync), §7 (sécurité) ; logitech.md « Souris Lightsync / zones ».
//!
//! - `Proto::Legacy` : G203 / G303 / G403 (fonction 0x3A) et G Pro (0x3C), index 0x0E. Mode direct = rapport COURT `10 FF 0E 8A 01 01`
//!   (sans écriture flash), couleur = `11 FF 0E <fn> <zone> <mode> R G B 02`, libération `10 FF 0E 8A 00 00`.
//! - `Proto::Lightsync` : G203 Lightsync, SW_CONTROL `11 FF 0E 50 01 03 07`, LED `11 FF 12 10 <led> R G B FF`, commit `11 FF 12 70`, libération `[1,0,0]`.
//!
//! INCONNU / à valider sur matériel : numéro de zone et nombre de LEDs par modèle (une seule zone/LED, id 0, est pilotée), collection HID
//! (note : page 0xFF00, usage 2 long / usage 1 court), mode `11 FF 0E 10 00 <mode>` du G203 Lightsync, G403 Hero (C08F) et tout autre PID : non revendiqués.
//! Jamais envoyé : effets matériels, persist, NV_CONFIG, écriture flash.

use super::{Hid, NativeRgb, DEVICE, REPORT_LONG};
use crate::core::Color;
use anyhow::{bail, Result};

const REPORT_SHORT: u8 = 0x10;
const IDX_MODE: u8 = 0x0E;
const IDX_LIGHTSYNC_LED: u8 = 0x12;
const MODE_OFF: u8 = 0;
const MODE_STATIC: u8 = 1;
/// Une seule LED/zone (id 0) : voir INCONNU en tête de fichier.
pub(super) const LEDS: usize = 1;

#[derive(Clone, Copy)]
pub(super) enum Proto {
    /// Octet de fonction de la trame de couleur : 0x3A (G203/G303/G403) ou 0x3C (G Pro).
    Legacy(u8),
    Lightsync,
}

struct Mouse {
    hid: Box<dyn Hid>,
    proto: Proto,
}

fn long(idx: u8, function: u8) -> Vec<u8> {
    let mut f = vec![0u8; 20];
    f[..4].copy_from_slice(&[REPORT_LONG, DEVICE, idx, function]);
    f
}

pub(super) fn open(hid: Box<dyn Hid>, proto: Proto) -> Result<Box<dyn NativeRgb>> {
    let mouse = Mouse { hid, proto };
    mouse.take_control()?;
    Ok(Box::new(mouse))
}

impl Mouse {
    /// Le rapport court part sur la collection « court » (aux) si elle existe, sinon sur l'interface principale.
    fn write_short(&self, tail: [u8; 3]) -> Result<()> {
        let hid = self.hid.aux().unwrap_or(self.hid.as_ref());
        hid.write(&[REPORT_SHORT, DEVICE, IDX_MODE, tail[0], tail[1], tail[2], 0])?;
        Ok(())
    }

    fn take_control(&self) -> Result<()> {
        match self.proto {
            Proto::Legacy(_) => self.write_short([0x8A, 0x01, 0x01]),
            Proto::Lightsync => self.write_sw_control([0x01, 0x03, 0x07]),
        }
    }

    fn write_sw_control(&self, params: [u8; 3]) -> Result<()> {
        let mut f = long(IDX_MODE, 0x50);
        f[4..7].copy_from_slice(&params);
        self.hid.write(&f)?;
        Ok(())
    }
}

impl NativeRgb for Mouse {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > LEDS {
            bail!("Logitech souris : {} couleurs pour {LEDS} LED", colors.len());
        }
        let c = colors.first().copied().unwrap_or(Color::BLACK);
        match self.proto {
            Proto::Legacy(function) => {
                let off = (c.r, c.g, c.b) == (0, 0, 0);
                let mut f = long(IDX_MODE, function);
                // f[4] = zone 0
                f[5] = if off { MODE_OFF } else { MODE_STATIC };
                f[6..9].copy_from_slice(&[c.r, c.g, c.b]);
                f[9] = if off { 0x00 } else { 0x02 };
                self.hid.write(&f)?;
            }
            Proto::Lightsync => {
                let mut f = long(IDX_LIGHTSYNC_LED, 0x10);
                f[4..9].copy_from_slice(&[0x00, c.r, c.g, c.b, 0xFF]);
                self.hid.write(&f)?;
                self.hid.write(&long(IDX_LIGHTSYNC_LED, 0x70))?;
            }
        }
        Ok(())
    }

    fn release(&mut self) {
        let _ = match self.proto {
            Proto::Legacy(_) => self.write_short([0x8A, 0x00, 0x00]),
            Proto::Lightsync => self.write_sw_control([0x01, 0x00, 0x00]),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::MockHid;
    use super::*;

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn driver(proto: Proto) -> (Box<dyn NativeRgb>, Sent) {
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), proto).expect("open"), sent)
    }

    fn pad(head: &[u8]) -> Vec<u8> {
        let mut v = head.to_vec();
        v.resize(20, 0);
        v
    }

    #[test]
    fn legacy_init_color_and_release_match_the_note() {
        for function in [0x3Au8, 0x3C] {
            let (mut d, sent) = driver(Proto::Legacy(function));
            d.set_colors(&[Color::new(1, 2, 3)]).expect("set");
            d.set_colors(&[Color::BLACK]).expect("off");
            d.release();
            let s = sent.lock();
            assert_eq!(s[0], vec![0x10, 0xFF, 0x0E, 0x8A, 0x01, 0x01, 0x00]);
            assert_eq!(s[1], pad(&[0x11, 0xFF, 0x0E, function, 0x00, 0x01, 1, 2, 3, 0x02]));
            assert_eq!(s[2], pad(&[0x11, 0xFF, 0x0E, function, 0x00, 0x00, 0, 0, 0, 0x00]));
            assert_eq!(s[3], vec![0x10, 0xFF, 0x0E, 0x8A, 0x00, 0x00, 0x00]);
        }
    }

    #[test]
    fn lightsync_init_color_commit_and_release_match_the_note() {
        let (mut d, sent) = driver(Proto::Lightsync);
        d.set_colors(&[Color::new(9, 8, 7)]).expect("set");
        d.release();
        let s = sent.lock();
        assert_eq!(s.len(), 4);
        assert_eq!(s[0], pad(&[0x11, 0xFF, 0x0E, 0x50, 0x01, 0x03, 0x07]));
        assert_eq!(s[1], pad(&[0x11, 0xFF, 0x12, 0x10, 0x00, 9, 8, 7, 0xFF]));
        assert_eq!(s[2], pad(&[0x11, 0xFF, 0x12, 0x70]));
        assert_eq!(s[3], pad(&[0x11, 0xFF, 0x0E, 0x50, 0x01, 0x00, 0x00]));
    }

    #[test]
    fn too_many_colors_rejected_without_writing() {
        for proto in [Proto::Legacy(0x3A), Proto::Lightsync] {
            let (mut d, sent) = driver(proto);
            let before = sent.lock().len();
            assert!(d.set_colors(&[Color::BLACK; 2]).is_err());
            assert_eq!(sent.lock().len(), before);
        }
    }
}
