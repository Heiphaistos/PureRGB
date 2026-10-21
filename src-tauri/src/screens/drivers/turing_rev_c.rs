//! Révision C : Turing Smart Screen 2.1" (rond, 480x480), 5" (480x800) et 8.8" (480x1920) en série.
//!
//! Source : wiki turing-smart-screen-python (« Hardware revisions ») et description du
//! protocole de la révision C (documentation seule, aucun code copié).
//! Tout message est complété par des 0 jusqu'à un multiple de 250 octets (l'octet 0x2C pour
//! START_DISPLAY_BITMAP). Image plein écran : BGRA 4 octets/pixel, un 0x00 inséré tous les 249
//! octets. Séquence : HELLO (réponse « chs_… », 23 octets) -> STOP_VIDEO -> STOP_MEDIA (+lecture
//! 1024) -> OPTIONS ; puis par image PRE_UPDATE -> START_DISPLAY_BITMAP -> DISPLAY_BITMAP_xx ->
//! charge utile (+lecture 1024) -> QUERY_STATUS (+lecture 1024).
//! Non géré : mise à jour partielle (UPDATE_BITMAP) et ROM < 88 ; contenu des réponses d'état
//! INCONNU (lues et ignorées). Écran en veille : réveillé par le provider (ouverture du port).

use std::time::Duration;

use anyhow::{bail, Result};

use super::turing_serial::{check_frame, rotate_rgba, Orientation, SerialLink, Wire};
use crate::screens::driver::Frame;

const BLOCK: usize = 250;
const HELLO: [u8; 12] = [0x01, 0xef, 0x69, 0, 0, 0, 0x01, 0, 0, 0, 0xc5, 0xd3];
const OPTIONS: [u8; 11] = [0x7d, 0xef, 0x69, 0, 0, 0, 0x05, 0, 0, 0, 0x2d];
const SET_BRIGHTNESS: [u8; 10] = [0x7b, 0xef, 0x69, 0, 0, 0, 0x01, 0, 0, 0];
const STOP_VIDEO: [u8; 7] = [0x79, 0xef, 0x69, 0, 0, 0, 0x01];
const STOP_MEDIA: [u8; 7] = [0x96, 0xef, 0x69, 0, 0, 0, 0x01];
const QUERY_STATUS: [u8; 7] = [0xcf, 0xef, 0x69, 0, 0, 0, 0x01];
const PRE_UPDATE_BITMAP: [u8; 7] = [0x86, 0xef, 0x69, 0, 0, 0, 0x01];
const START_DISPLAY_BITMAP: u8 = 0x2c;
const READ_STATUS: usize = 1024;
const HELLO_ID_LEN: usize = 23;
const HELLO_TRIES: u32 = 3;
const READ_TIMEOUT: Duration = Duration::from_secs(1);
const HELLO_RETRY_PAUSE: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Inch21,
    Inch5,
    Inch88,
}

pub struct RevC {
    portrait: (u32, u32),
    orientation: Orientation,
}

impl RevC {
    pub fn new(portrait: (u32, u32), orientation: Orientation) -> Self {
        RevC { portrait, orientation }
    }

    fn panel(&self) -> Result<Panel> {
        match self.portrait {
            (480, 480) => Ok(Panel::Inch21),
            (480, 800) => Ok(Panel::Inch5),
            (480, 1920) => Ok(Panel::Inch88),
            (w, h) => bail!("résolution {w}x{h} non prise en charge par la révision C"),
        }
    }
}

/// Complète à un multiple de 250 octets avec `pad`.
pub(crate) fn padded(mut msg: Vec<u8>, pad: u8) -> Vec<u8> {
    let rem = msg.len() % BLOCK;
    if rem != 0 {
        msg.resize(msg.len() + BLOCK - rem, pad);
    }
    msg
}

/// Insère un 0x00 entre deux lots de 249 octets (pas après le dernier), puis complète à 250.
pub(crate) fn payload_stream(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 249 + BLOCK);
    for (i, c) in data.chunks(249).enumerate() {
        if i > 0 {
            out.push(0);
        }
        out.extend_from_slice(c);
    }
    padded(out, 0)
}

/// Rotation antihoraire logicielle selon l'orientation (le 8.8" a un panneau natif portrait).
fn rotation(panel: Panel, o: Orientation) -> u16 {
    use Orientation::{Landscape, Portrait, ReverseLandscape, ReversePortrait};
    match (panel == Panel::Inch88, o) {
        (true, Landscape) => 270,
        (true, ReverseLandscape) => 90,
        (true, Portrait) => 180,
        (true, ReversePortrait) => 0,
        (false, Portrait) => 90,
        (false, ReversePortrait) => 270,
        (false, ReverseLandscape) => 180,
        (false, Landscape) => 0,
    }
}

fn send(link: &mut dyn SerialLink, msg: &[u8], read: bool) -> Result<()> {
    link.write_all(&padded(msg.to_vec(), 0))?;
    if read {
        // Contenu de l'état INCONNU : lu pour vider le tampon, jamais interprété.
        link.read(READ_STATUS, READ_TIMEOUT)?;
    }
    Ok(())
}

impl Wire for RevC {
    fn init(&mut self, link: &mut dyn SerialLink) -> Result<()> {
        self.panel()?;
        let mut ok = false;
        for attempt in 0..HELLO_TRIES {
            if attempt > 0 {
                link.pause(HELLO_RETRY_PAUSE);
            }
            link.clear_input()?;
            send(link, &HELLO, false)?;
            let id = link.read(HELLO_ID_LEN, READ_TIMEOUT)?;
            link.clear_input()?;
            let text: String = id.iter().filter(|b| (0x20..0x7f).contains(*b)).map(|&b| b as char).collect();
            if text.starts_with("chs_") {
                ok = true;
                break;
            }
        }
        if !ok {
            bail!("l'écran (révision C) n'a pas répondu au HELLO par un identifiant « chs_… »");
        }
        send(link, &STOP_VIDEO, false)?;
        send(link, &STOP_MEDIA, true)?;
        // OPTIONS : mode de démarrage par défaut, sans retournement matériel, sans veille.
        let mut opt = OPTIONS.to_vec();
        opt.extend_from_slice(&[0, 0, 0, 0]);
        send(link, &opt, false)
    }

    fn size(&self) -> (u32, u32) {
        self.orientation.oriented(self.portrait)
    }

    fn push(&mut self, link: &mut dyn SerialLink, frame: &Frame) -> Result<()> {
        let panel = self.panel()?;
        check_frame(frame, self.size())?;
        let (rgba, ..) = rotate_rgba(&frame.rgba, frame.width, frame.height, rotation(panel, self.orientation));
        let bgra: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();

        send(link, &PRE_UPDATE_BITMAP, false)?;
        link.write_all(&padded(vec![START_DISPLAY_BITMAP], START_DISPLAY_BITMAP))?;
        // Taille du panneau en unités de 64 pixels-octets, puis (quirk de la doc) 480*480/64 = 0x0E10.
        let size_code: [u8; 2] = match panel {
            Panel::Inch21 => [0x0E, 0x10],
            Panel::Inch5 => [0x17, 0x70],
            Panel::Inch88 => [0x38, 0x40],
        };
        let mut cmd = vec![0xc8, 0xef, 0x69, 0x00, size_code[0], size_code[1]];
        cmd.extend_from_slice(&((480u32 * 480 / 64) as u16).to_be_bytes());
        send(link, &cmd, false)?;
        link.write_all(&payload_stream(&bgra))?;
        link.read(READ_STATUS, READ_TIMEOUT)?;
        send(link, &QUERY_STATUS, true)
    }

    fn brightness(&mut self, link: &mut dyn SerialLink, percent: u8) -> Result<bool> {
        let mut msg = SET_BRIGHTNESS.to_vec();
        msg.push((u16::from(percent) * 255 / 100) as u8);
        send(link, &msg, false)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::turing_serial::mock::Mock;
    use super::*;

    fn hello_rx() -> Vec<u8> {
        let mut v = b"chs_5inch.1.87".to_vec();
        v.resize(HELLO_ID_LEN, 0);
        v
    }

    fn block(prefix: &[u8], pad: u8) -> Vec<u8> {
        padded(prefix.to_vec(), pad)
    }

    #[test]
    fn padding_rules() {
        assert_eq!(padded(vec![1; 250], 0).len(), 250);
        assert_eq!(padded(vec![1; 251], 0).len(), 500);
        assert_eq!(padded(vec![1], 9)[249], 9);
    }

    #[test]
    fn payload_stream_inserts_separator_every_249() {
        let s = payload_stream(&vec![7u8; 250]);
        assert_eq!(s.len(), 500);
        assert_eq!(s[248], 7);
        assert_eq!(s[249], 0); // séparateur
        assert_eq!(s[250], 7); // 2e lot : 1 octet
        assert_eq!(s[251], 0); // remplissage
        assert_eq!(payload_stream(&[1, 2, 3]).len(), 250);
    }

    #[test]
    fn init_sequence_bytes() {
        let mock = Mock::with_rx(&hello_rx());
        let mut c = RevC::new((480, 480), Orientation::Portrait);
        assert!(c.init(&mut mock.clone()).is_ok());
        let w = mock.writes();
        assert_eq!(w.len(), 4);
        assert_eq!(w[0], block(&[0x01, 0xef, 0x69, 0, 0, 0, 0x01, 0, 0, 0, 0xc5, 0xd3], 0));
        assert_eq!(w[1], block(&[0x79, 0xef, 0x69, 0, 0, 0, 0x01], 0));
        assert_eq!(w[2], block(&[0x96, 0xef, 0x69, 0, 0, 0, 0x01], 0));
        assert_eq!(w[3], block(&[0x7d, 0xef, 0x69, 0, 0, 0, 0x05, 0, 0, 0, 0x2d, 0, 0, 0, 0], 0));
    }

    #[test]
    fn hello_failure_is_bounded_error() {
        let mock = Mock::default(); // aucune réponse
        let mut c = RevC::new((480, 800), Orientation::Portrait);
        assert!(c.init(&mut mock.clone()).is_err());
        assert_eq!(mock.writes().len(), HELLO_TRIES as usize); // 3 essais puis abandon
        assert!(RevC::new((100, 100), Orientation::Portrait).init(&mut Mock::default()).is_err());
    }

    #[test]
    fn brightness_bytes() {
        let mock = Mock::default();
        let mut c = RevC::new((480, 800), Orientation::Portrait);
        assert!(c.brightness(&mut mock.clone(), 100).is_ok());
        assert!(c.brightness(&mut mock.clone(), 50).is_ok()); // 127
        let w = mock.writes();
        assert_eq!(w[0], block(&[0x7b, 0xef, 0x69, 0, 0, 0, 0x01, 0, 0, 0, 255], 0));
        assert_eq!(w[1][10], 127);
    }

    #[test]
    fn full_frame_2_1_inch() {
        let mock = Mock::default();
        let mut c = RevC::new((480, 480), Orientation::Portrait);
        let mut f = Frame::new(480, 480);
        f.rgba[..4].copy_from_slice(&[255, 128, 0, 255]); // pixel (0,0)
        assert!(c.push(&mut mock.clone(), &f).is_ok());
        let w = mock.writes();
        assert_eq!(w.len(), 5);
        assert_eq!(w[0], block(&[0x86, 0xef, 0x69, 0, 0, 0, 0x01], 0));
        assert_eq!(w[1], vec![0x2c; 250]);
        // 2.1" : c8 ef 69 00 0E 10 puis 0E 10 (480*480/64 = 3600).
        assert_eq!(w[2], block(&[0xc8, 0xef, 0x69, 0x00, 0x0E, 0x10, 0x0E, 0x10], 0));
        assert_eq!(w[4], block(&[0xcf, 0xef, 0x69, 0, 0, 0, 0x01], 0));
        // 480*480*4 = 921600 octets -> 3702 lots de 249 -> 3701 séparateurs = 925301 -> 925500.
        let p = &w[3];
        assert_eq!(p.len(), 925_500);
        // Portrait = rotation antihoraire 90 : pixel (0,0) -> (x=0, y=479) ; BGRA = 00 80 FF FF.
        // octet de données 479*480*4 = 919680 ; + 919680/249 = 3693 séparateurs.
        assert_eq!(&p[923_373..923_377], [0x00, 0x80, 0xFF, 0xFF]);
        assert_eq!(p[249], 0);
    }

    #[test]
    fn size_codes_5_and_8_inch() {
        for (portrait, code) in [((480, 800), [0x17, 0x70]), ((480, 1920), [0x38, 0x40])] {
            let mock = Mock::default();
            let mut c = RevC::new(portrait, Orientation::Portrait);
            let size = c.size();
            assert!(c.push(&mut mock.clone(), &Frame::new(size.0, size.1)).is_ok());
            assert_eq!(&mock.writes()[2][..8], [0xc8, 0xef, 0x69, 0, code[0], code[1], 0x0E, 0x10]);
        }
    }

    #[test]
    fn rotation_table() {
        use Orientation::*;
        assert_eq!(rotation(Panel::Inch5, Portrait), 90);
        assert_eq!(rotation(Panel::Inch5, Landscape), 0);
        assert_eq!(rotation(Panel::Inch88, Portrait), 180);
        assert_eq!(rotation(Panel::Inch88, Landscape), 270);
    }
}
