//! Révision D : Kipye Qiye Smart Display 3.5" (USB 454d:4e41).
//!
//! Source : wiki turing-smart-screen-python (« Hardware revisions ») et description du
//! protocole de la révision D (documentation seule, aucun code copié).
//! Commandes : SETORG 43 48 00 00 / SET180 43 47 00 00 (portrait / inversé), SETBL 43 43 +
//! u16 BE (0-500), BLOCKWRITE 43 41 + x0,x1,y0,y1 (u16 BE), INTOPICMODE 44 00 00 00,
//! données = lots de 63 octets RGB565 gros-boutiste précédés de 0x50, OUTPICMODE 41 00 00 00.
//! L'écran envoie des acquittements qui ne sont pas exploités : l'entrée est purgée.
//! Paysage = rotation logicielle de 90° horaire (270° antihoraire).

use anyhow::Result;

use super::turing_serial::{check_frame, rgb565, rotate_rgba, Orientation, SerialLink, Wire};
use crate::screens::driver::Frame;

const SETORG: [u8; 4] = [67, 72, 0, 0];
const SET180: [u8; 4] = [67, 71, 0, 0];
const SETBL: [u8; 2] = [67, 67];
const BLOCKWRITE: [u8; 2] = [67, 65];
const INTOPICMODE: [u8; 4] = [68, 0, 0, 0];
const OUTPICMODE: [u8; 4] = [65, 0, 0, 0];
const DATA_PREFIX: u8 = 0x50;
const DATA_CHUNK: usize = 63;

pub struct RevD {
    portrait: (u32, u32),
    orientation: Orientation,
}

impl RevD {
    pub fn new(portrait: (u32, u32), orientation: Orientation) -> Self {
        RevD { portrait, orientation }
    }
}

fn send(link: &mut dyn SerialLink, data: &[u8]) -> Result<()> {
    link.write_all(data)?;
    link.clear_input()
}

pub(crate) fn blockwrite(x0: u16, x1: u16, y0: u16, y1: u16) -> [u8; 10] {
    let mut p = [0u8; 10];
    p[..2].copy_from_slice(&BLOCKWRITE);
    for (i, v) in [x0, x1, y0, y1].iter().enumerate() {
        p[2 + i * 2..4 + i * 2].copy_from_slice(&v.to_be_bytes());
    }
    p
}

impl Wire for RevD {
    fn init(&mut self, link: &mut dyn SerialLink) -> Result<()> {
        send(link, if self.orientation.is_reverse() { &SET180 } else { &SETORG })
    }

    fn size(&self) -> (u32, u32) {
        self.orientation.oriented(self.portrait)
    }

    fn push(&mut self, link: &mut dyn SerialLink, frame: &Frame) -> Result<()> {
        check_frame(frame, self.size())?;
        // Le panneau est toujours adressé en portrait ; le paysage est tourné côté logiciel.
        let (rgba, w, h) = if self.orientation.is_landscape() {
            rotate_rgba(&frame.rgba, frame.width, frame.height, 270)
        } else {
            (frame.rgba.clone(), frame.width, frame.height)
        };
        send(link, &blockwrite(0, u16::try_from(w - 1)?, 0, u16::try_from(h - 1)?))?;
        send(link, &INTOPICMODE)?;
        let mut line = [0u8; DATA_CHUNK + 1];
        line[0] = DATA_PREFIX;
        for chunk in rgb565(&rgba, true).chunks(DATA_CHUNK) {
            line[1..1 + chunk.len()].copy_from_slice(chunk);
            send(link, &line[..1 + chunk.len()])?;
        }
        send(link, &OUTPICMODE)
    }

    fn brightness(&mut self, link: &mut dyn SerialLink, percent: u8) -> Result<bool> {
        let mut p = [0u8; 4];
        p[..2].copy_from_slice(&SETBL);
        p[2..].copy_from_slice(&(u16::from(percent) * 5).to_be_bytes());
        // La doc envoie la commande deux fois : elle n'est parfois pas appliquée.
        send(link, &p)?;
        send(link, &p)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::turing_serial::mock::Mock;
    use super::*;

    #[test]
    fn orientation_commands() {
        let m = Mock::default();
        assert!(RevD::new((320, 480), Orientation::Portrait).init(&mut m.clone()).is_ok());
        assert!(RevD::new((320, 480), Orientation::ReverseLandscape).init(&mut m.clone()).is_ok());
        assert_eq!(m.writes(), [vec![0x43, 0x48, 0, 0], vec![0x43, 0x47, 0, 0]]);
    }

    #[test]
    fn brightness_sent_twice() {
        let m = Mock::default();
        // 25 % -> 125 = 0x007D.
        let mut d = RevD::new((320, 480), Orientation::Portrait);
        assert_eq!(d.brightness(&mut m.clone(), 25).ok(), Some(true));
        assert_eq!(m.writes(), [vec![0x43, 0x43, 0x00, 0x7D], vec![0x43, 0x43, 0x00, 0x7D]]);
    }

    #[test]
    fn blockwrite_order_is_x0_x1_y0_y1() {
        // x0=0, x1=319=0x013F, y0=0, y1=479=0x01DF.
        assert_eq!(blockwrite(0, 319, 0, 479), [0x43, 0x41, 0, 0, 0x01, 0x3F, 0, 0, 0x01, 0xDF]);
    }

    #[test]
    fn portrait_frame_sequence() {
        let m = Mock::default();
        let mut d = RevD::new((320, 480), Orientation::Portrait);
        let mut f = Frame::new(320, 480);
        f.rgba[..4].copy_from_slice(&[0, 255, 0, 255]); // vert -> 07E0
        assert!(d.push(&mut m.clone(), &f).is_ok());
        let w = m.writes();
        assert_eq!(w[0], [0x43, 0x41, 0, 0, 0x01, 0x3F, 0, 0, 0x01, 0xDF]);
        assert_eq!(w[1], [68, 0, 0, 0]);
        assert_eq!(&w[2][..3], [0x50, 0x07, 0xE0]);
        assert_eq!(w[2].len(), 64);
        // 307200 octets / 63 = 4876 lots pleins + 1 lot de 12 octets (+ préfixe).
        let data = &w[2..w.len() - 1];
        assert_eq!(data.len(), 4877);
        assert_eq!(data[4876].len(), 13);
        assert_eq!(w[w.len() - 1], [65, 0, 0, 0]);
    }

    #[test]
    fn landscape_rotates_panel_data_to_portrait() {
        // Paysage 480x320 -> envoi 320x480 : x0 = 320-0-320 = 0, x1 = 319, y0 = 0, y1 = 479.
        let m = Mock::default();
        let mut d = RevD::new((320, 480), Orientation::Landscape);
        assert_eq!(d.size(), (480, 320));
        let mut f = Frame::new(480, 320);
        f.rgba[..4].copy_from_slice(&[255, 0, 0, 255]); // coin haut-gauche
        assert!(d.push(&mut m.clone(), &f).is_ok());
        let w = m.writes();
        assert_eq!(w[0], [0x43, 0x41, 0, 0, 0x01, 0x3F, 0, 0, 0x01, 0xDF]);
        // Rotation antihoraire de 270 : haut-gauche -> haut-droite de l'image 320x480,
        // soit le 320e pixel (indice 319) de la 1re ligne, octets 638..640 : lot 10 (630..693).
        let data: Vec<u8> = w[2..w.len() - 1].iter().flat_map(|l| l[1..].to_vec()).collect();
        assert_eq!(&data[638..640], [0xF8, 0x00]);
        assert_eq!(&data[..2], [0x00, 0x00]);
    }
}
