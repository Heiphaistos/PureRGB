//! Révision B / « flagship » : XuanFang 3.5".
//!
//! Source : wiki turing-smart-screen-python (« Hardware revisions », série USB `2017-2-25`) et
//! description du protocole de la révision B (documentation seule, aucun code copié).
//! Paquet de 10 octets : commande, 8 octets de données (complétés par 0), commande.
//! HELLO « HELLO » -> réponse de 10 octets (octet 6 = 0x0A, octet 7 = sous-révision).
//! Image : RGB565 gros-boutiste par lots de `largeur*8` octets, pause de 50 ms après le bitmap.
//! Portrait/paysage gérés par l'écran ; les orientations inversées par rotation 180° logicielle.
//! Non géré : couleur des LED du dos (commande 0xCD, modèle flagship).

use std::time::Duration;

use anyhow::{bail, Result};

use super::turing_serial::{check_frame, rgb565, rotate_rgba, Orientation, SerialLink, Wire};
use crate::screens::driver::Frame;

const CMD_HELLO: u8 = 0xCA;
const CMD_SET_ORIENTATION: u8 = 0xCB;
const CMD_DISPLAY_BITMAP: u8 = 0xCC;
const CMD_SET_BRIGHTNESS: u8 = 0xCE;
const HELLO_TIMEOUT: Duration = Duration::from_millis(1000);
/// Évite les « bitmaps corrompus » entre deux images (valeur de la doc).
const COOLDOWN: Duration = Duration::from_millis(50);

pub struct RevB {
    portrait: (u32, u32),
    orientation: Orientation,
    /// Sous-révision lue au HELLO : 0x01/0x02 = luminosité 0/1, 0x11/0x12 = 0-255.
    brightness_range: bool,
}

impl RevB {
    pub fn new(portrait: (u32, u32), orientation: Orientation) -> Self {
        RevB { portrait, orientation, brightness_range: false }
    }
}

pub(crate) fn packet(cmd: u8, payload: &[u8]) -> Result<[u8; 10]> {
    if payload.len() > 8 {
        bail!("charge utile > 8 octets");
    }
    let mut p = [0u8; 10];
    p[0] = cmd;
    p[1..1 + payload.len()].copy_from_slice(payload);
    p[9] = cmd;
    Ok(p)
}

impl Wire for RevB {
    fn init(&mut self, link: &mut dyn SerialLink) -> Result<()> {
        link.clear_input()?;
        link.write_all(&packet(CMD_HELLO, b"HELLO")?)?;
        let r = link.read(10, HELLO_TIMEOUT)?;
        link.clear_input()?;
        if r.len() != 10 || r[0] != CMD_HELLO || r[9] != CMD_HELLO || &r[1..6] != b"HELLO" {
            bail!("réponse HELLO invalide (révision B) : {r:02x?}");
        }
        if r[6] != 0x0A {
            bail!("famille de sous-révision inconnue (INCONNU) : {:#04x}", r[6]);
        }
        self.brightness_range = match r[7] {
            0x01 | 0x02 => false,
            0x11 | 0x12 => true,
            v => bail!("sous-révision inconnue (INCONNU) : {v:#04x}"),
        };
        let landscape = u8::from(self.orientation.is_landscape());
        link.write_all(&packet(CMD_SET_ORIENTATION, &[landscape])?)
    }

    fn size(&self) -> (u32, u32) {
        self.orientation.oriented(self.portrait)
    }

    fn push(&mut self, link: &mut dyn SerialLink, frame: &Frame) -> Result<()> {
        let (w, h) = self.size();
        check_frame(frame, (w, h))?;
        // Plein écran : (0,0)-(w-1,h-1) ; en orientation inversée l'image est tournée de 180°
        // et le rectangle plein écran reste le même.
        let (x1, y1) = (u16::try_from(w - 1)?.to_be_bytes(), u16::try_from(h - 1)?.to_be_bytes());
        link.write_all(&packet(CMD_DISPLAY_BITMAP, &[0, 0, 0, 0, x1[0], x1[1], y1[0], y1[1]])?)?;
        let data = if self.orientation.is_reverse() {
            rgb565(&rotate_rgba(&frame.rgba, w, h, 180).0, true)
        } else {
            rgb565(&frame.rgba, true)
        };
        for chunk in data.chunks(w as usize * 8) {
            link.write_all(chunk)?;
        }
        link.pause(COOLDOWN);
        Ok(())
    }

    fn brightness(&mut self, link: &mut dyn SerialLink, percent: u8) -> Result<bool> {
        let level = if self.brightness_range {
            (u16::from(percent) * 255 / 100) as u8
        } else {
            // 1 = éteint, 0 = pleine luminosité.
            u8::from(percent == 0)
        };
        link.write_all(&packet(CMD_SET_BRIGHTNESS, &[level])?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::turing_serial::mock::Mock;
    use super::*;

    fn hello_reply(sub: u8) -> Vec<u8> {
        vec![0xCA, b'H', b'E', b'L', b'L', b'O', 0x0A, sub, 0x00, 0xCA]
    }

    #[test]
    fn hello_and_orientation_bytes() {
        let mock = Mock::with_rx(&hello_reply(0x11));
        let mut b = RevB::new((320, 480), Orientation::Landscape);
        assert!(b.init(&mut mock.clone()).is_ok());
        let w = mock.writes();
        assert_eq!(w[0], [0xCA, 0x48, 0x45, 0x4C, 0x4C, 0x4F, 0, 0, 0, 0xCA]);
        assert_eq!(w[1], [0xCB, 1, 0, 0, 0, 0, 0, 0, 0, 0xCB]);
        assert_eq!(b.size(), (480, 320));
    }

    #[test]
    fn bad_hello_is_rejected() {
        let mut b = RevB::new((320, 480), Orientation::Portrait);
        assert!(b.init(&mut Mock::default()).is_err()); // silence
        assert!(b.init(&mut Mock::with_rx(&hello_reply(0x33))).is_err());
        let mut bad = hello_reply(0x01);
        bad[9] = 0;
        assert!(b.init(&mut Mock::with_rx(&bad)).is_err());
    }

    #[test]
    fn bitmap_packet_and_chunks() {
        let mock = Mock::with_rx(&hello_reply(0x01));
        let mut b = RevB::new((320, 480), Orientation::Portrait);
        let mut link = mock.clone();
        assert!(b.init(&mut link).is_ok());
        let mut f = Frame::new(320, 480);
        f.rgba[..4].copy_from_slice(&[0, 0, 255, 255]); // bleu -> 001F BE
        assert!(b.push(&mut link, &f).is_ok());
        let w = mock.writes();
        // 319 = 0x013F ; 479 = 0x01DF.
        assert_eq!(w[2], [0xCC, 0, 0, 0, 0, 0x01, 0x3F, 0x01, 0xDF, 0xCC]);
        assert_eq!(&w[3][..2], [0x00, 0x1F]);
        assert_eq!(w[3..].len(), 120);
    }

    #[test]
    fn reverse_orientation_rotates_180() {
        let mock = Mock::with_rx(&hello_reply(0x01));
        let mut b = RevB::new((320, 480), Orientation::ReversePortrait);
        let mut link = mock.clone();
        assert!(b.init(&mut link).is_ok());
        assert_eq!(mock.writes()[1][1], 0); // portrait côté écran
        let mut f = Frame::new(320, 480);
        let last = f.rgba.len() - 4;
        f.rgba[last..].copy_from_slice(&[255, 0, 0, 255]); // dernier pixel rouge -> premier après 180°
        assert!(b.push(&mut link, &f).is_ok());
        assert_eq!(&mock.writes()[3][..2], [0xF8, 0x00]);
    }

    #[test]
    fn brightness_binary_and_range() {
        let mock = Mock::with_rx(&hello_reply(0x01));
        let mut b = RevB::new((320, 480), Orientation::Portrait);
        let mut link = mock.clone();
        assert!(b.init(&mut link).is_ok());
        assert!(b.brightness(&mut link, 0).is_ok());
        assert!(b.brightness(&mut link, 60).is_ok());
        let w = mock.writes();
        assert_eq!(w[2], [0xCE, 1, 0, 0, 0, 0, 0, 0, 0, 0xCE]);
        assert_eq!(w[3], [0xCE, 0, 0, 0, 0, 0, 0, 0, 0, 0xCE]);

        let mock = Mock::with_rx(&hello_reply(0x12));
        let mut b = RevB::new((320, 480), Orientation::Portrait);
        let mut link = mock.clone();
        assert!(b.init(&mut link).is_ok());
        assert!(b.brightness(&mut link, 50).is_ok()); // 50*255/100 = 127
        assert_eq!(mock.writes()[2], [0xCE, 127, 0, 0, 0, 0, 0, 0, 0, 0xCE]);
    }
}
