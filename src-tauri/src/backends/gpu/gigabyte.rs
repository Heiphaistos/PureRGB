//! Source : docs/rgb-protocols/signalrgb.md § GPU (« Gigabyte Gen1 Vision ») et gpu-controleurs-par-constructeur.md § Gigabyte RGB Fusion v1.
//!
//! Protocole par écritures I2C BRUTES (aucun registre) : détection = envoi de `[0xAB]` complété de zéros (8 octets pour
//! 0x32/0x50/0x62/0x71, sinon 4) puis lecture de 4 octets : `o[0] == 0xAB` et `o[1]` parmi 0x10/0x11/0x12/0x14.
//! Exception de la note : sous-device 0x405E ou device 0x2486 acceptés sans réponse valide (cartes de la liste blanche).
//! Mode statique : `[0x88, 0x01, 0x05, 0x63]` ; couleur : `[0x40, R, G, B]` ; trames de 8 octets (zéros) pour 0x32/0x62, sinon 4.
//! Pause 10 ms par trame. L'enregistrement permanent (`0xAA`) n'est JAMAIS envoyé.
//! Attention (note) : une trame brute envoyée au mauvais appareil est lue comme des données : liste blanche stricte.

use super::whitelist::Pci;
use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

const PAUSE_MS: u64 = 10;
const MODE_STATIC: [u8; 4] = [0x88, 0x01, 0x05, 0x63];

fn detect_len(addr: u8) -> usize {
    if matches!(addr, 0x32 | 0x50 | 0x62 | 0x71) {
        8
    } else {
        4
    }
}

fn frame_len(addr: u8) -> usize {
    if matches!(addr, 0x32 | 0x62) {
        8
    } else {
        4
    }
}

fn padded(bytes: &[u8], len: usize) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.resize(len.max(bytes.len()), 0);
    v
}

pub fn verify(bus: &mut dyn SmbusBus, addr: u8, pci: Pci) -> Result<()> {
    bus.write_raw(addr, &padded(&[0xAB], detect_len(addr)))?;
    bus.pause(PAUSE_MS);
    let r = bus.read_raw(addr, 4)?;
    let exempt = pci.subdevice == 0x405E || pci.device == 0x2486;
    ensure!(
        exempt || (r.first() == Some(&0xAB) && matches!(r.get(1), Some(0x10 | 0x11 | 0x12 | 0x14))),
        "réponse Gigabyte inattendue à 0x{addr:02X} : {r:02X?}"
    );
    Ok(())
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    bus.write_raw(addr, &padded(&MODE_STATIC, frame_len(addr)))?;
    bus.pause(PAUSE_MS);
    Ok(())
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, c: Color) -> Result<()> {
    bus.write_raw(addr, &padded(&[0x40, c.r, c.g, c.b], frame_len(addr)))?;
    bus.pause(PAUSE_MS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Raw {
        writes: Vec<(u8, Vec<u8>)>,
        reply: Vec<u8>,
    }
    impl SmbusBus for Raw {
        fn read_byte_data(&mut self, _: u8, _: u8) -> Result<u8> {
            unreachable!("protocole brut uniquement")
        }
        fn write_byte_data(&mut self, _: u8, _: u8, _: u8) -> Result<()> {
            unreachable!("protocole brut uniquement")
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!("protocole brut uniquement")
        }
        fn write_raw(&mut self, a: u8, d: &[u8]) -> Result<()> {
            self.writes.push((a, d.to_vec()));
            Ok(())
        }
        fn read_raw(&mut self, _: u8, len: usize) -> Result<Vec<u8>> {
            Ok(self.reply[..len].to_vec())
        }
        fn pause(&mut self, _: u64) {}
    }

    const PCI: Pci = Pci { vendor: 0x10DE, device: 0x1B80, subvendor: 0x1458, subdevice: 0x3702 };

    #[test]
    fn verify_accepts_documented_replies_only() {
        let mut b = Raw { reply: vec![0xAB, 0x11, 0x52, 0x03], ..Default::default() };
        assert!(verify(&mut b, 0x47, PCI).is_ok());
        assert_eq!(b.writes[0], (0x47, vec![0xAB, 0, 0, 0]), "adresse 0x47 : trame de 4 octets");
        b.reply = vec![0xAB, 0xAB, 0xAB, 0xAB]; // écho du bus d'écran
        assert!(verify(&mut b, 0x47, PCI).is_err());
        b.reply = vec![0, 0, 0, 0];
        assert!(verify(&mut b, 0x47, PCI).is_err());
        assert!(verify(&mut b, 0x47, Pci { subdevice: 0x405E, ..PCI }).is_ok(), "exception documentée");
    }

    #[test]
    fn frames_are_padded_per_address_and_never_save() {
        let mut b = Raw { reply: vec![0xAB, 0x10, 0, 0], ..Default::default() };
        verify(&mut b, 0x62, PCI).expect("ok");
        assert_eq!(b.writes[0].1.len(), 8);
        init(&mut b, 0x62).expect("init");
        set_color(&mut b, 0x62, Color { r: 1, g: 2, b: 3 }).expect("couleur");
        assert_eq!(b.writes[1].1, vec![0x88, 0x01, 0x05, 0x63, 0, 0, 0, 0]);
        assert_eq!(b.writes[2].1, vec![0x40, 1, 2, 3, 0, 0, 0, 0]);
        init(&mut b, 0x47).expect("init");
        set_color(&mut b, 0x47, Color { r: 4, g: 5, b: 6 }).expect("couleur");
        assert_eq!(b.writes[3].1, vec![0x88, 0x01, 0x05, 0x63]);
        assert_eq!(b.writes[4].1, vec![0x40, 4, 5, 6]);
        assert!(b.writes.iter().all(|(_, d)| d[0] != 0xAA));
    }
}
