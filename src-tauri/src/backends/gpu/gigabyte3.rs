//! Source : décompilation de GBT_VGA / GvLedLib.dll (Gigabyte Control Center 26.11.05.01, Ghidra : fonction `FUN_180067340`, paquet de
//! 0x40 octets écrit en I2C BRUTE par `CNvApiTemp::WriteI2C`) recoupée avec signalrgb.md (« Gigabyte Gen3 Xtreme », adresse 0x60).
//!
//! Paquet de zone (64 octets, complété de zéros) : `[0xE6, zone, mode 0x01 statique, luminosité 0x64, 0x02, 0x03, nombre de LEDs, R,G,B x n]`
//! (n <= 8 dans GCC). 5 zones d'identifiants 2..6 de 3, 3, 3, 3 et 6 LEDs = 18 LEDs. Init de la note SignalRGB : bloc brut de 32 octets `[0xD6, 0...]`.
//! La commande d'enregistrement (`0xAA`, 64 octets dans GCC) n'est JAMAIS envoyée. Détection par liste blanche PCI seulement (comme GCC,
//! qui route par identifiant de sous-système). INCONNU : signification exacte des octets 0x02 et 0x03 (repris de la note et de GCC).

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

/// (identifiant de zone, nombre de LEDs), dans l'ordre d'exposition.
const ZONES: [(u8, usize); 5] = [(2, 3), (3, 3), (4, 3), (5, 3), (6, 6)];
const PACKET: usize = 64;
const PAUSE_MS: u64 = 10;

pub fn leds() -> u8 {
    ZONES.iter().map(|z| z.1 as u8).sum()
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    let mut p = vec![0u8; 32];
    p[0] = 0xD6;
    bus.write_raw(addr, &p)?;
    bus.pause(PAUSE_MS);
    Ok(())
}

fn zone_packet(zone: u8, colors: &[Color]) -> Vec<u8> {
    let mut p = vec![0u8; PACKET];
    p[..7].copy_from_slice(&[0xE6, zone, 0x01, 0x64, 0x02, 0x03, colors.len() as u8]);
    for (i, c) in colors.iter().enumerate() {
        p[7 + i * 3..10 + i * 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, colors: &[Color]) -> Result<()> {
    let mut at = 0;
    for (zone, n) in ZONES {
        let part: Vec<Color> = (at..at + n).map(|i| colors.get(i).copied().unwrap_or_default()).collect();
        bus.write_raw(addr, &zone_packet(zone, &part))?;
        bus.pause(PAUSE_MS);
        at += n;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Raw(Vec<Vec<u8>>);
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
            self.0.push(d.to_vec());
            Ok(())
        }
        fn pause(&mut self, _: u64) {}
    }

    #[test]
    fn eighteen_leds_in_five_zones() {
        assert_eq!(leds(), 18);
    }

    #[test]
    fn init_is_a_32_byte_d6_block() {
        let mut b = Raw::default();
        init(&mut b, 0x60).expect("ok");
        assert_eq!(b.0[0].len(), 32);
        assert_eq!(b.0[0][0], 0xD6);
        assert!(b.0[0][1..].iter().all(|x| *x == 0));
    }

    #[test]
    fn zone_packets_are_64_bytes_with_header_count_and_rgb() {
        let mut b = Raw::default();
        let colors: Vec<Color> = (0..18u8).map(|i| Color { r: i, g: i + 100, b: i + 200 }).collect();
        set_colors(&mut b, 0x60, &colors).expect("ok");
        assert_eq!(b.0.len(), 5);
        assert!(b.0.iter().all(|p| p.len() == 64 && p[0] == 0xE6));
        assert_eq!(&b.0[0][..13], &[0xE6, 2, 1, 0x64, 2, 3, 3, 0, 100, 200, 1, 101, 201]);
        assert_eq!(&b.0[1][..10], &[0xE6, 3, 1, 0x64, 2, 3, 3, 3, 103, 203]);
        assert_eq!(&b.0[4][..8], &[0xE6, 6, 1, 0x64, 2, 3, 6, 12]);
        assert_eq!(b.0[4][7 + 5 * 3 + 2], 17 + 200, "6e LED de la dernière zone");
        assert!(b.0.iter().all(|p| p[0] != 0xAA));
    }
}
