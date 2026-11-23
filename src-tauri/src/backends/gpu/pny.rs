//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « PNY Lovelace » (RTX 40 XLR8 Verto, adresse 0x60).
//!
//! 3 LEDs (ventilateur, droite, gauche). Une trame par LED, bloc I2C de 7 octets au registre 0x02 :
//! `[0x06 (statique), 0xFF (luminosité), LED, 0x00, R, G, B]`. Détection par liste blanche PCI seulement (note : « aucune lecture
//! de contrôle »). Aucune écriture d'enregistrement permanent n'existe dans ce protocole.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

pub const LEDS: u8 = 3;
const REG_LIGHTING: u8 = 0x02;
const MODE_STATIC: u8 = 0x06;

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, colors: &[Color]) -> Result<()> {
    for (led, c) in colors.iter().take(usize::from(LEDS)).enumerate() {
        bus.write_block_data(addr, REG_LIGHTING, &[MODE_STATIC, 0xFF, led as u8, 0x00, c.r, c.g, c.b])?;
        bus.pause(10);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};

    #[test]
    fn one_seven_byte_block_per_led() {
        let mut m = MockBus::default();
        m.present.insert(0x60);
        set_colors(&mut m, 0x60, &[Color { r: 1, g: 2, b: 3 }, Color { r: 4, g: 5, b: 6 }, Color { r: 7, g: 8, b: 9 }, Color { r: 9, g: 9, b: 9 }]).expect("ok");
        assert_eq!(
            m.log,
            vec![
                Txn::Block(0x60, 0x02, vec![6, 0xFF, 0, 0, 1, 2, 3]),
                Txn::Block(0x60, 0x02, vec![6, 0xFF, 1, 0, 4, 5, 6]),
                Txn::Block(0x60, 0x02, vec![6, 0xFF, 2, 0, 7, 8, 9]),
            ],
            "3 LEDs seulement, la 4e couleur est ignorée"
        );
    }
}
