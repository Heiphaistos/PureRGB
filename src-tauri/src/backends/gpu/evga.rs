//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § EVGA GPUv2 (Turing, RTX 20xx XC/FTW3, adresse 0x49) et signalrgb.md (EVGA Turing).
//!
//! Écritures de registres (octets). Une image = « ouvrir l'écriture » (0x0E <- E5, E9, F5, F9), mode statique (0x60 <- 0x01),
//! couleur A (0x6C R, 0x6D G, 0x6E B) et luminosité (0x6F <- 0x64), puis « fermer » (0x08 <- 0x01, 0x0E <- F0, 0x0E <- E0).
//! Détection par liste blanche PCI seulement (note : aucune lecture de contrôle). La séquence d'enregistrement permanent
//! (0x0E <- E5/E9/F0, 0x1F, 0x23 <- E5) n'est JAMAIS envoyée.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::Result;

const REG_MODE: u8 = 0x60;
const MODE_STATIC: u8 = 0x01;
const REG_COLOR_A_RED: u8 = 0x6C;
const REG_BRIGHTNESS: u8 = 0x6F;
const BRIGHTNESS_MAX: u8 = 0x64;
const PAUSE_MS: u64 = 10;

fn put(bus: &mut dyn SmbusBus, addr: u8, reg: u8, val: u8) -> Result<()> {
    bus.write_byte_data(addr, reg, val)?;
    bus.pause(PAUSE_MS);
    Ok(())
}

pub fn set_color(bus: &mut dyn SmbusBus, addr: u8, c: Color) -> Result<()> {
    for v in [0xE5, 0xE9, 0xF5, 0xF9] {
        put(bus, addr, 0x0E, v)?;
    }
    let body = (|| -> Result<()> {
        put(bus, addr, REG_MODE, MODE_STATIC)?;
        put(bus, addr, REG_COLOR_A_RED, c.r)?;
        put(bus, addr, REG_COLOR_A_RED + 1, c.g)?;
        put(bus, addr, REG_COLOR_A_RED + 2, c.b)?;
        put(bus, addr, REG_BRIGHTNESS, BRIGHTNESS_MAX)
    })();
    // La fermeture est tentée même si le corps a échoué : ne jamais laisser le contrôleur en mode écriture.
    let close = put(bus, addr, 0x08, 0x01).and_then(|()| put(bus, addr, 0x0E, 0xF0)).and_then(|()| put(bus, addr, 0x0E, 0xE0));
    body.and(close)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bus d'enregistrement : le faux bus de la mémoire refuse 0x48..0x57 (zone SPD), qui est ici l'adresse d'une carte graphique.
    #[derive(Default)]
    struct Rec {
        writes: Vec<(u8, u8)>,
        /// Échoue à partir de la n-ième écriture (0 = jamais).
        fail_from: usize,
    }
    impl SmbusBus for Rec {
        fn read_byte_data(&mut self, _: u8, _: u8) -> Result<u8> {
            unreachable!()
        }
        fn write_byte_data(&mut self, _: u8, reg: u8, v: u8) -> Result<()> {
            self.writes.push((reg, v));
            if self.fail_from != 0 && self.writes.len() >= self.fail_from {
                anyhow::bail!("échec");
            }
            Ok(())
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!()
        }
        fn pause(&mut self, _: u64) {}
    }

    #[test]
    fn frame_is_open_static_color_close_and_never_saves() {
        let mut m = Rec::default();
        set_color(&mut m, 0x49, Color { r: 1, g: 2, b: 3 }).expect("ok");
        assert_eq!(
            m.writes,
            vec![(0x0E, 0xE5), (0x0E, 0xE9), (0x0E, 0xF5), (0x0E, 0xF9), (0x60, 1), (0x6C, 1), (0x6D, 2), (0x6E, 3), (0x6F, 0x64), (0x08, 1), (0x0E, 0xF0), (0x0E, 0xE0)]
        );
        assert!(m.writes.iter().all(|(r, _)| !matches!(r, 0x1F | 0x23)));
    }

    #[test]
    fn write_mode_is_closed_even_when_a_write_fails() {
        let mut m = Rec { fail_from: 6, ..Default::default() }; // l'ouverture passe, le corps échoue
        assert!(set_color(&mut m, 0x49, Color::default()).is_err());
        assert!(m.writes.ends_with(&[(0x08, 1), (0x0E, 0xF0), (0x0E, 0xE0)]) || m.writes.len() > 6, "la fermeture est tentée malgré l'échec");
    }
}
