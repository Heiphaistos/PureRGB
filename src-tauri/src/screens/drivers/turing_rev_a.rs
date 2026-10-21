//! Révision A : Turing Smart Screen 3.5" et UsbPCMonitor 3.5" / 5" / 7".
//!
//! Source : wiki turing-smart-screen-python (« Hardware revisions », identifiant série
//! USB35INCHIPS[V2]) et description du protocole de la révision A (documentation seule,
//! aucun code copié). Commandes de 6 octets : x, y, ex, ey sur 10 bits chacun
//! (x>>2 | x&3<<6+y>>4 | y&15<<4+ex>>6 | ex&63<<2+ey>>8 | ey&255) puis l'octet de commande.
//! Image : RGB565 petit-boutiste par lots de `largeur*8` octets, sans acquittement.

use std::time::Duration;

use anyhow::{bail, Result};

use super::turing_serial::{check_frame, rgb565, Orientation, SerialLink, Wire};
use crate::screens::driver::Frame;

const CMD_HELLO: u8 = 69;
const CMD_SET_BRIGHTNESS: u8 = 110;
const CMD_SET_ORIENTATION: u8 = 121;
const CMD_DISPLAY_BITMAP: u8 = 197;
/// Seul le HELLO attend une réponse (6 octets) ; le Turing 3.5" officiel ne répond pas.
const HELLO_TIMEOUT: Duration = Duration::from_millis(500);

pub struct RevA {
    portrait: (u32, u32),
    orientation: Orientation,
}

impl RevA {
    pub fn new(portrait: (u32, u32), orientation: Orientation) -> Self {
        RevA { portrait, orientation }
    }
}

/// Paquet de 6 octets ; les coordonnées tiennent sur 10 bits (0..=1023).
pub(crate) fn command(cmd: u8, x: u16, y: u16, ex: u16, ey: u16) -> Result<[u8; 6]> {
    if x.max(y).max(ex).max(ey) > 1023 {
        bail!("coordonnée hors 0-1023 pour la révision A");
    }
    Ok([
        (x >> 2) as u8,
        (((x & 3) << 6) + (y >> 4)) as u8,
        (((y & 15) << 4) + (ex >> 6)) as u8,
        (((ex & 63) << 2) + (ey >> 8)) as u8,
        (ey & 255) as u8,
        cmd,
    ])
}

/// Paquet de 16 octets : commande 121, orientation+100, puis largeur et hauteur (u16 BE).
pub(crate) fn orientation_packet(o: Orientation, size: (u32, u32)) -> Result<[u8; 16]> {
    let (w, h) = (u16::try_from(size.0)?, u16::try_from(size.1)?);
    let mut p = [0u8; 16];
    p[..6].copy_from_slice(&command(CMD_SET_ORIENTATION, 0, 0, 0, 0)?);
    p[6] = o as u8 + 100;
    p[7..9].copy_from_slice(&w.to_be_bytes());
    p[9..11].copy_from_slice(&h.to_be_bytes());
    Ok(p)
}

impl Wire for RevA {
    fn init(&mut self, link: &mut dyn SerialLink) -> Result<()> {
        link.clear_input()?;
        link.write_all(&[CMD_HELLO; 6])?;
        let answer = link.read(6, HELLO_TIMEOUT)?;
        link.clear_input()?;
        // Réponses documentées : 01x6 = UsbMonitor 3.5", 02x6 = 5", 03x6 = 7" ; silence = Turing 3.5".
        self.portrait = match answer.as_slice() {
            [] => self.portrait,
            [1, 1, 1, 1, 1, 1] => (320, 480),
            [2, 2, 2, 2, 2, 2] => (480, 800),
            [3, 3, 3, 3, 3, 3] => (600, 1024),
            other => bail!("réponse HELLO inconnue (INCONNU) : {other:02x?}"),
        };
        link.write_all(&orientation_packet(self.orientation, self.size())?)
    }

    fn size(&self) -> (u32, u32) {
        self.orientation.oriented(self.portrait)
    }

    fn push(&mut self, link: &mut dyn SerialLink, frame: &Frame) -> Result<()> {
        let (w, h) = self.size();
        check_frame(frame, (w, h))?;
        link.write_all(&command(CMD_DISPLAY_BITMAP, 0, 0, u16::try_from(w - 1)?, u16::try_from(h - 1)?)?)?;
        for chunk in rgb565(&frame.rgba, false).chunks(w as usize * 8) {
            link.write_all(chunk)?;
        }
        Ok(())
    }

    fn brightness(&mut self, link: &mut dyn SerialLink, percent: u8) -> Result<bool> {
        // 0 = le plus lumineux, 255 = le plus sombre : 255 - floor(pct*255/100).
        let level = (25_500 - u16::from(percent) * 255) / 100;
        link.write_all(&command(CMD_SET_BRIGHTNESS, level, 0, 0, 0)?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::turing_serial::mock::Mock;
    use super::*;

    #[test]
    fn command_bytes_by_hand() {
        // x=0,y=0,ex=319,ey=479 : ex>>6=4, (319&63)<<2=252, ey>>8=1 -> 253, 479&255=223.
        assert_eq!(command(197, 0, 0, 319, 479).ok(), Some([0x00, 0x00, 0x04, 0xFD, 0xDF, 0xC5]));
        // x=1023 : x>>2=255, (x&3)<<6=192.
        assert_eq!(command(110, 1023, 0, 0, 0).ok(), Some([0xFF, 0xC0, 0x00, 0x00, 0x00, 0x6E]));
        assert!(command(1, 1024, 0, 0, 0).is_err());
    }

    #[test]
    fn orientation_landscape_packet() {
        // 102 = 2+100 ; 480 = 0x01E0 ; 320 = 0x0140.
        let p = orientation_packet(Orientation::Landscape, (480, 320)).ok();
        assert_eq!(p, Some([0, 0, 0, 0, 0, 121, 102, 0x01, 0xE0, 0x01, 0x40, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn turing_35_silent_hello_then_orientation() {
        let mock = Mock::default();
        let mut a = RevA::new((320, 480), Orientation::Portrait);
        assert!(a.init(&mut mock.clone()).is_ok());
        let w = mock.writes();
        assert_eq!(w[0], [69; 6]);
        assert_eq!(w[1], [0, 0, 0, 0, 0, 121, 100, 0x01, 0x40, 0x01, 0xE0, 0, 0, 0, 0, 0]);
        assert_eq!(a.size(), (320, 480));
    }

    #[test]
    fn usbmonitor_5_hello_changes_size_and_garbage_is_rejected() {
        let mut a = RevA::new((320, 480), Orientation::Portrait);
        assert!(a.init(&mut Mock::with_rx(&[2; 6])).is_ok());
        assert_eq!(a.size(), (480, 800));
        let mut b = RevA::new((320, 480), Orientation::Portrait);
        assert!(b.init(&mut Mock::with_rx(&[9; 6])).is_err());
    }

    #[test]
    fn full_frame_bytes() {
        let mock = Mock::default();
        let mut a = RevA::new((320, 480), Orientation::Portrait);
        let mut link = mock.clone();
        assert!(a.init(&mut link).is_ok());
        let mut f = Frame::new(320, 480);
        f.rgba[..4].copy_from_slice(&[255, 0, 0, 255]); // pixel 0 rouge -> F800 LE = 00 F8
        assert!(a.push(&mut link, &f).is_ok());
        let w = mock.writes();
        assert_eq!(w[2], [0x00, 0x00, 0x04, 0xFD, 0xDF, 0xC5]);
        assert_eq!(&w[3][..2], [0x00, 0xF8]);
        // 320*480*2 octets en lots de 320*8 = 2560 -> 120 lots.
        assert_eq!(w[3..].len(), 120);
        assert!(w[3..].iter().all(|c| c.len() == 2560));
    }

    #[test]
    fn brightness_levels() {
        let mock = Mock::default();
        let mut a = RevA::new((320, 480), Orientation::Portrait);
        for p in [25, 0, 100] {
            assert_eq!(a.brightness(&mut mock.clone(), p).ok(), Some(true));
        }
        let w = mock.writes();
        // 25 % -> (25500-6375)/100 = 191 = 0xBF : 191>>2 = 47, (191&3)<<6 = 0xC0.
        assert_eq!(w[0], [47, 0xC0, 0, 0, 0, 110]);
        assert_eq!(w[1], [63, 0xC0, 0, 0, 0, 110]); // 0 % -> 255
        assert_eq!(w[2], [0, 0, 0, 0, 0, 110]); // 100 % -> 0
    }
}
