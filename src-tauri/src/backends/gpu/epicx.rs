//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « PNY EpicX / XLR8 » (RTX 50 ARGB Epic-X, adresse 0x60).
//!
//! Chaque LED = un bloc I2C de 7 octets au registre de sa zone : `[0x09 (direct), 0xFF, index, 0x00, R, G, B]`.
//! Zones (ordre d'exposition) : logo latéral 4 LEDs (registre 0x04, index 20..23), face avant 20 LEDs (registre 0x04, index 0..19),
//! flèche 17 LEDs (modèle « small ») ou 19 (« large » : RTX 5080 et 5090) au registre 0x02, et pour le modèle large seulement un logo arrière
//! d'une LED (registre 0x0F, index 0). Avant les couleurs, chaque zone passe en mode direct :
//! `[0x09, luminosité 0xFF, vitesse 0x00, 0x00, 0, 0, 0]` au registre de la zone.
//! Détection par liste blanche PCI seulement (la source lit un registre 0x81 de détection sans en exploiter la valeur : INCONNU).
//! Aucune écriture d'enregistrement permanent dans ce protocole.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

const MODE_DIRECT: u8 = 0x09;
const REG_ARROW: u8 = 0x02;
const REG_FRONT: u8 = 0x04;
const REG_LOGO: u8 = 0x0F;

/// (registre, index de LED sur le fil), dans l'ordre d'exposition.
fn layout(large: bool) -> Vec<(u8, u8)> {
    let mut v: Vec<(u8, u8)> = (20..24).map(|i| (REG_FRONT, i)).collect();
    v.extend((0..20).map(|i| (REG_FRONT, i)));
    v.extend((0..if large { 19 } else { 17 }).map(|i| (REG_ARROW, i)));
    if large {
        v.push((REG_LOGO, 0));
    }
    v
}

pub fn leds(large: bool) -> u8 {
    layout(large).len() as u8
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8, large: bool) -> Result<()> {
    let mut regs = vec![REG_FRONT, REG_ARROW];
    if large {
        regs.push(REG_LOGO);
    }
    for reg in regs {
        bus.write_block_data(addr, reg, &[MODE_DIRECT, 0xFF, 0x00, 0x00, 0, 0, 0])?;
    }
    Ok(())
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, large: bool, colors: &[Color]) -> Result<()> {
    for ((reg, idx), c) in layout(large).into_iter().zip(colors.iter().chain(std::iter::repeat(&Color::default()))) {
        bus.write_block_data(addr, reg, &[MODE_DIRECT, 0xFF, idx, 0x00, c.r, c.g, c.b])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};

    #[test]
    fn led_counts_per_variant() {
        assert_eq!((leds(false), leds(true)), (41, 44));
    }

    #[test]
    fn init_sets_direct_mode_on_each_zone_register() {
        let mut m = MockBus::default();
        m.present.insert(0x60);
        init(&mut m, 0x60, false).expect("ok");
        assert_eq!(m.log, vec![Txn::Block(0x60, 4, vec![9, 0xFF, 0, 0, 0, 0, 0]), Txn::Block(0x60, 2, vec![9, 0xFF, 0, 0, 0, 0, 0])]);
        let mut m = MockBus::default();
        m.present.insert(0x60);
        init(&mut m, 0x60, true).expect("ok");
        assert_eq!(m.log.len(), 3);
    }

    #[test]
    fn each_led_is_one_block_on_its_zone_register() {
        let mut m = MockBus::default();
        m.present.insert(0x60);
        let colors: Vec<Color> = (0..44u8).map(|i| Color { r: i, g: 1, b: 2 }).collect();
        set_colors(&mut m, 0x60, true, &colors).expect("ok");
        assert_eq!(m.log.len(), 44);
        assert_eq!(m.log[0], Txn::Block(0x60, 4, vec![9, 0xFF, 20, 0, 0, 1, 2]), "logo latéral : index 20");
        assert_eq!(m.log[4], Txn::Block(0x60, 4, vec![9, 0xFF, 0, 0, 4, 1, 2]), "face avant : index 0");
        assert_eq!(m.log[24], Txn::Block(0x60, 2, vec![9, 0xFF, 0, 0, 24, 1, 2]), "flèche");
        assert_eq!(m.log[43], Txn::Block(0x60, 0x0F, vec![9, 0xFF, 0, 0, 43, 1, 2]), "logo arrière");
        let mut m = MockBus::default();
        m.present.insert(0x60);
        set_colors(&mut m, 0x60, false, &colors).expect("ok");
        assert_eq!(m.log.len(), 41);
    }
}
