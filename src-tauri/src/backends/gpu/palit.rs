//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « Palit / Gainward v2 » (RTX 20/30/40, aussi PNY XLR8 Turing/Ampere), adresse 0x49.
//!
//! Détection AVANT écriture : registre 0x00 = 0 et registre 0xE0 < 5, sinon refus.
//! Couleur directe : 0xE0 <- 0 (mode statique), 0x60 <- 1 (contrôle logiciel), 0x6C/0x6D/0x6E = R/G/B, 0x6F = luminosité.
//! INCONNU : la source de référence passe la luminosité du mode (255 par défaut alors que la borne annoncée est 100) : on écrit 100 (0x64),
//! valeur maximale annoncée. Aucune écriture d'enregistrement permanent n'existe dans cette note pour cette famille.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

const REG_CONTROL: u8 = 0x60;
const REG_RED: u8 = 0x6C;
const REG_BRIGHTNESS: u8 = 0x6F;
const REG_MODE: u8 = 0xE0;
const BRIGHTNESS_MAX: u8 = 100;

pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    let id = bus.read_byte_data(addr, 0x00)?;
    let mode = bus.read_byte_data(addr, REG_MODE)?;
    ensure!(id == 0 && mode < 5, "signature Palit absente à 0x{addr:02X} (0x00 = {id:02X}, 0xE0 = {mode:02X})");
    Ok(())
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, c: Color) -> Result<()> {
    bus.write_byte_data(addr, REG_MODE, 0)?;
    bus.write_byte_data(addr, REG_CONTROL, 1)?;
    bus.write_byte_data(addr, REG_RED, c.r)?;
    bus.write_byte_data(addr, REG_RED + 1, c.g)?;
    bus.write_byte_data(addr, REG_RED + 2, c.b)?;
    bus.write_byte_data(addr, REG_BRIGHTNESS, BRIGHTNESS_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Rec {
        writes: Vec<(u8, u8)>,
        reg0: u8,
        mode: u8,
    }
    impl SmbusBus for Rec {
        fn read_byte_data(&mut self, _: u8, reg: u8) -> Result<u8> {
            Ok(if reg == 0 { self.reg0 } else { self.mode })
        }
        fn write_byte_data(&mut self, _: u8, reg: u8, v: u8) -> Result<()> {
            self.writes.push((reg, v));
            Ok(())
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!()
        }
    }

    #[test]
    fn verify_needs_reg0_zero_and_small_mode() {
        assert!(verify(&mut Rec::default(), 0x49).is_ok());
        assert!(verify(&mut Rec { reg0: 1, ..Default::default() }, 0x49).is_err());
        assert!(verify(&mut Rec { mode: 5, ..Default::default() }, 0x49).is_err());
    }

    #[test]
    fn direct_frame_bytes_and_no_other_register() {
        let mut m = Rec::default();
        set_color(&mut m, 0x49, Color { r: 1, g: 2, b: 3 }).expect("ok");
        assert_eq!(m.writes, vec![(0xE0, 0), (0x60, 1), (0x6C, 1), (0x6D, 2), (0x6E, 3), (0x6F, 100)]);
    }
}
