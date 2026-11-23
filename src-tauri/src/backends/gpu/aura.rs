//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « ASUS Aura GPU (hérité) ».
//!
//! Registres (octets, `write_byte_data`) : 0x04 rouge, 0x05 vert, 0x06 bleu, 0x07 mode (1 = statique).
//! Détection : registres 0x20/0x21 = 0x15/0x89 (« magique » 0x1589), sinon refus.
//! Les écritures de couleur sont immédiates : le registre 0x0E (enregistrement permanent) n'est JAMAIS écrit.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

const REG_RED: u8 = 0x04;
const REG_MODE: u8 = 0x07;
const MODE_STATIC: u8 = 1;
const REG_MAGIC_HI: u8 = 0x20;
const REG_MAGIC_LO: u8 = 0x21;

pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    let hi = bus.read_byte_data(addr, REG_MAGIC_HI)?;
    let lo = bus.read_byte_data(addr, REG_MAGIC_LO)?;
    ensure!((hi, lo) == (0x15, 0x89), "signature Aura GPU absente à 0x{addr:02X} ({hi:02X}{lo:02X})");
    Ok(())
}

/// Mode statique (une fois), puis la couleur.
pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    bus.write_byte_data(addr, REG_MODE, MODE_STATIC)
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, c: Color) -> Result<()> {
    bus.write_byte_data(addr, REG_RED, c.r)?;
    bus.write_byte_data(addr, REG_RED + 1, c.g)?;
    bus.write_byte_data(addr, REG_RED + 2, c.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};

    #[test]
    fn verify_needs_the_magic_value() {
        let mut m = MockBus::default();
        m.present.insert(0x2A);
        m.mem.extend([((0x2A, 0x20), 0x15), ((0x2A, 0x21), 0x89)]);
        assert!(verify(&mut m, 0x2A).is_ok());
        m.mem.insert((0x2A, 0x21), 0x00);
        assert!(verify(&mut m, 0x2A).is_err());
    }

    #[test]
    fn color_frame_is_three_register_writes_and_never_saves() {
        let mut m = MockBus::default();
        m.present.insert(0x29);
        init(&mut m, 0x29).expect("mode");
        set_color(&mut m, 0x29, Color { r: 1, g: 2, b: 3 }).expect("couleur");
        assert_eq!(m.log, vec![Txn::Write(0x29, 0x07, 1), Txn::Write(0x29, 0x04, 1), Txn::Write(0x29, 0x05, 2), Txn::Write(0x29, 0x06, 3)]);
        assert!(m.log.iter().all(|t| !matches!(t, Txn::Write(_, 0x0E, _))));
    }
}
