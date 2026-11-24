//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « Sapphire NITRO Glow » (V1 : RX 4xx/5xx/Vega, adresse 0x55 ; V3 : RX 5000+, adresse 0x28)
//! et faits de registres des contrôleurs de référence. Bus AMD (ADL) uniquement ; détection par liste blanche PCI + lecture d'un octet qui doit réussir.
//!
//! V1 : registre 0x00 = mode (0x04 = personnalisé), 0x01 = luminosité (0 = maximum, 2 = minimum), 0x03/0x04/0x05 = R/G/B.
//! V3 : registre 0x0F = contrôle externe (0 = désactivé), 0x10 = mode (0x06 = personnalisé), 0x1A/0x1B/0x1C = R/G/B.
//! Une couleur par carte. Aucune écriture d'enregistrement permanent n'existe dans cette famille.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

const V1_MODE: u8 = 0x00;
const V1_BRIGHTNESS: u8 = 0x01;
const V1_RED: u8 = 0x03;
const V1_MODE_CUSTOM: u8 = 0x04;
const V1_BRIGHTNESS_MAX: u8 = 0x00;

const V3_EXTERNAL_CONTROL: u8 = 0x0F;
const V3_MODE: u8 = 0x10;
const V3_RED: u8 = 0x1A;
const V3_MODE_CUSTOM: u8 = 0x06;

/// Présence : la lecture du registre de mode doit aboutir (équivalent de la lecture d'un octet de la source de référence).
pub fn verify(bus: &mut dyn SmbusBus, addr: u8, v3: bool) -> Result<()> {
    bus.read_byte_data(addr, if v3 { V3_MODE } else { V1_MODE }).map(|_| ())
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8, v3: bool) -> Result<()> {
    if v3 {
        bus.write_byte_data(addr, V3_EXTERNAL_CONTROL, 0)?;
        bus.write_byte_data(addr, V3_MODE, V3_MODE_CUSTOM)
    } else {
        bus.write_byte_data(addr, V1_MODE, V1_MODE_CUSTOM)?;
        bus.write_byte_data(addr, V1_BRIGHTNESS, V1_BRIGHTNESS_MAX)
    }
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, v3: bool, c: Color) -> Result<()> {
    let base = if v3 { V3_RED } else { V1_RED };
    bus.write_byte_data(addr, base, c.r)?;
    bus.write_byte_data(addr, base + 1, c.g)?;
    bus.write_byte_data(addr, base + 2, c.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Rec {
        writes: Vec<(u8, u8, u8)>,
        reads: Vec<u8>,
        fail_read: bool,
    }
    impl SmbusBus for Rec {
        fn read_byte_data(&mut self, _: u8, reg: u8) -> Result<u8> {
            self.reads.push(reg);
            if self.fail_read {
                anyhow::bail!("NACK");
            }
            Ok(0)
        }
        fn write_byte_data(&mut self, a: u8, reg: u8, v: u8) -> Result<()> {
            self.writes.push((a, reg, v));
            Ok(())
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!()
        }
    }

    #[test]
    fn verify_reads_the_mode_register_and_refuses_a_silent_bus() {
        let mut b = Rec::default();
        assert!(verify(&mut b, 0x28, true).is_ok());
        assert_eq!(b.reads, vec![0x10]);
        let mut b = Rec::default();
        verify(&mut b, 0x55, false).expect("ok");
        assert_eq!(b.reads, vec![0x00]);
        assert!(verify(&mut Rec { fail_read: true, ..Default::default() }, 0x28, true).is_err());
        assert!(b.writes.is_empty(), "la détection ne fait que lire");
    }

    #[test]
    fn v3_frame_bytes() {
        let mut b = Rec::default();
        init(&mut b, 0x28, true).expect("init");
        set_color(&mut b, 0x28, true, Color { r: 1, g: 2, b: 3 }).expect("couleur");
        assert_eq!(b.writes, vec![(0x28, 0x0F, 0), (0x28, 0x10, 6), (0x28, 0x1A, 1), (0x28, 0x1B, 2), (0x28, 0x1C, 3)]);
    }

    #[test]
    fn v1_frame_bytes() {
        let mut b = Rec::default();
        init(&mut b, 0x55, false).expect("init");
        set_color(&mut b, 0x55, false, Color { r: 4, g: 5, b: 6 }).expect("couleur");
        assert_eq!(b.writes, vec![(0x55, 0x00, 4), (0x55, 0x01, 0), (0x55, 0x03, 4), (0x55, 0x04, 5), (0x55, 0x05, 6)]);
    }
}
