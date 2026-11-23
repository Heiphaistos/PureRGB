//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « MSI Mystic Light GPU » (v1, adresse 0x68).
//!
//! Registres (octets) : 0x22 mode (0x13 = statique), 0x36 luminosité (0..5, multiple de 20), 0x26 « inconnu » (écrit à 0
//! avant chaque image, comme la source de référence), 0x30/0x31/0x32 = R1/G1/B1. Le registre 0x3F (enregistrement
//! permanent) n'est JAMAIS écrit. Détection par liste blanche PCI seulement (aucune lecture de contrôle documentée).
//! 20 ms entre deux écritures (prudence de la note : la source n'impose aucun délai en v1).

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

const REG_MODE: u8 = 0x22;
const MODE_STATIC: u8 = 0x13;
const REG_BRIGHTNESS: u8 = 0x36;
const BRIGHTNESS_MAX: u8 = 5 * 20;
const REG_UNKNOWN: u8 = 0x26;
const REG_R1: u8 = 0x30;
const PAUSE_MS: u64 = 20;

fn write(bus: &mut dyn SmbusBus, addr: u8, reg: u8, val: u8) -> Result<()> {
    bus.write_byte_data(addr, reg, val)?;
    bus.pause(PAUSE_MS);
    Ok(())
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    write(bus, addr, REG_BRIGHTNESS, BRIGHTNESS_MAX)?;
    write(bus, addr, REG_MODE, MODE_STATIC)
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, c: Color) -> Result<()> {
    write(bus, addr, REG_UNKNOWN, 0)?;
    write(bus, addr, REG_R1, c.r)?;
    write(bus, addr, REG_R1 + 1, c.g)?;
    write(bus, addr, REG_R1 + 2, c.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};

    #[test]
    fn init_and_frame_bytes_and_no_save() {
        let mut m = MockBus::default();
        m.present.insert(0x68);
        init(&mut m, 0x68).expect("init");
        set_color(&mut m, 0x68, Color { r: 9, g: 8, b: 7 }).expect("couleur");
        assert_eq!(
            m.log,
            vec![
                Txn::Write(0x68, 0x36, 100),
                Txn::Write(0x68, 0x22, 0x13),
                Txn::Write(0x68, 0x26, 0),
                Txn::Write(0x68, 0x30, 9),
                Txn::Write(0x68, 0x31, 8),
                Txn::Write(0x68, 0x32, 7),
            ]
        );
        assert!(m.log.iter().all(|t| !matches!(t, Txn::Write(_, 0x3F, _))));
    }
}
