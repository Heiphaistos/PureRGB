//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « Colorful iGame » (RTX 30/40/50, adresse 0x61).
//!
//! Écritures I2C BRUTES, avec somme de contrôle sur 16 bits (somme des octets, poids faible puis fort, aux deux derniers octets).
//! Poignée de main avant toute écriture : envoi de `AA EF 81 02 1C 02` puis lecture brute de 0x40 octets qui doivent commencer par `AA EF 81`.
//! Couleur : paquet de 11 octets `AA EF 12 03 01 FF R G B crcL crcH` ; variante « v2 » (RTX 4070 Vulcan OC-V et RTX 4090 Neptune OC-V, sous-devices
//! 0x1200 et 0x1301) : paquet de 122 octets `AA EF 01 04 88 26` + 38 fois `R G B` + somme de contrôle.
//! Une couleur par carte. Aucune écriture d'enregistrement permanent dans cette note.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

const V1_LEN: usize = 11;
const V2_LEN: usize = 122;
const HANDSHAKE: [u8; 6] = [0xAA, 0xEF, 0x81, 0x02, 0x1C, 0x02];

/// Sous-devices PCI Colorful qui utilisent le paquet de 122 octets.
pub fn is_v2(subdevice: u16) -> bool {
    matches!(subdevice, 0x1200 | 0x1301)
}

pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    bus.write_raw(addr, &HANDSHAKE)?;
    let r = bus.read_raw(addr, 0x40)?;
    ensure!(r.starts_with(&[0xAA, 0xEF, 0x81]), "poignée de main Colorful refusée à 0x{addr:02X} : {:02X?}", &r[..r.len().min(3)]);
    Ok(())
}

fn with_crc(mut p: Vec<u8>) -> Vec<u8> {
    let n = p.len() - 2;
    let crc: u32 = p[..n].iter().map(|b| u32::from(*b)).sum();
    p[n] = (crc & 0xFF) as u8;
    p[n + 1] = (crc >> 8) as u8;
    p
}

fn packet(v2: bool, c: Color) -> Vec<u8> {
    if v2 {
        let mut p = vec![0u8; V2_LEN];
        p[..6].copy_from_slice(&[0xAA, 0xEF, 0x01, 0x04, 0x88, 0x26]);
        for i in (6..V2_LEN - 2).step_by(3) {
            p[i..i + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        with_crc(p)
    } else {
        let mut p = vec![0xAA, 0xEF, 0x12, 0x03, 0x01, 0xFF, c.r, c.g, c.b, 0, 0];
        debug_assert_eq!(p.len(), V1_LEN);
        p.shrink_to_fit();
        with_crc(p)
    }
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, v2: bool, c: Color) -> Result<()> {
    bus.write_raw(addr, &packet(v2, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Raw {
        writes: Vec<Vec<u8>>,
        reply: Vec<u8>,
    }
    impl SmbusBus for Raw {
        fn read_byte_data(&mut self, _: u8, _: u8) -> Result<u8> {
            unreachable!()
        }
        fn write_byte_data(&mut self, _: u8, _: u8, _: u8) -> Result<()> {
            unreachable!()
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!()
        }
        fn write_raw(&mut self, _: u8, d: &[u8]) -> Result<()> {
            self.writes.push(d.to_vec());
            Ok(())
        }
        fn read_raw(&mut self, _: u8, len: usize) -> Result<Vec<u8>> {
            let mut r = self.reply.clone();
            r.resize(len, 0);
            Ok(r)
        }
    }

    #[test]
    fn handshake_must_echo_aa_ef_81() {
        let mut b = Raw { reply: vec![0xAA, 0xEF, 0x81, 1], ..Default::default() };
        assert!(verify(&mut b, 0x61).is_ok());
        assert_eq!(b.writes[0], HANDSHAKE.to_vec());
        b.reply = vec![0xAA, 0xEF, 0x00];
        assert!(verify(&mut b, 0x61).is_err());
        b.reply = vec![];
        assert!(verify(&mut b, 0x61).is_err());
    }

    #[test]
    fn v1_packet_has_the_documented_bytes_and_checksum() {
        let p = packet(false, Color { r: 1, g: 2, b: 3 });
        assert_eq!(p.len(), 11);
        assert_eq!(&p[..9], &[0xAA, 0xEF, 0x12, 0x03, 0x01, 0xFF, 1, 2, 3]);
        // somme à la main : 0xAA + 0xEF + 0x12 + 0x03 + 0x01 + 0xFF + 1 + 2 + 3 = 0x2B4 -> octets B4 02
        assert_eq!((p[9], p[10]), (0xB4, 0x02));
    }

    #[test]
    fn v2_packet_is_122_bytes_with_38_pixels_and_checksum() {
        let p = packet(true, Color { r: 9, g: 8, b: 7 });
        assert_eq!(p.len(), 122);
        assert_eq!(&p[..6], &[0xAA, 0xEF, 0x01, 0x04, 0x88, 0x26]);
        assert_eq!(&p[6..9], &[9, 8, 7]);
        assert_eq!(&p[117..120], &[9, 8, 7]);
        let sum: u32 = p[..120].iter().map(|b| u32::from(*b)).sum();
        assert_eq!((p[120], p[121]), ((sum & 0xFF) as u8, (sum >> 8) as u8));
        assert!(is_v2(0x1200) && is_v2(0x1301) && !is_v2(0x1201));
    }
}
