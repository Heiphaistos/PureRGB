//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « ASRock Phantom / Taichi / Steel Legend (AMD) », adresse 0x36, bus AMD (ADL).
//!
//! Trames de 12 octets en I2C brute : `[commande, 0x00, sous-commande, données…]`. Détection AVANT écriture : commande 0x14 sous-commande 0x01 puis,
//! après 20 ms, lecture brute : les octets 4-5 donnent la table des canaux actifs (bit n = canal n) ; commande 0x14 sous-commande 0x02 donne le nombre
//! de LEDs par canal (octet 4 + n). Aucun canal actif = refus. Chaque canal actif = une « LED » (une couleur par canal : ARGB 3, côté 6, ventilateur 7).
//! Couleur d'un canal : commande 0x10, sous-commande = numéro du canal, données `[0x01, R, G, B, 0x80, 0xFF, 0x00, 0x1A]` (octets de vitesse, luminosité,
//! direction et constante repris tels quels de la source de référence : sens INCONNU). Aucune écriture d'enregistrement permanent connue.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

const PACKET: usize = 12;
const READ: usize = 24; // limite de charge utile du bus ADL ; les octets utiles s'arrêtent à l'index 19

fn send(bus: &mut dyn SmbusBus, addr: u8, command: u8, sub: u8, data: &[u8]) -> Result<()> {
    let mut p = [0u8; PACKET];
    p[0] = command;
    p[2] = sub;
    let n = data.len().min(PACKET - 3);
    p[3..3 + n].copy_from_slice(&data[..n]);
    bus.write_raw(addr, &p)
}

/// Canaux actifs (numéros), lecture seule.
pub fn detect(bus: &mut dyn SmbusBus, addr: u8) -> Result<Vec<u8>> {
    send(bus, addr, 0x14, 0x01, &[])?;
    bus.pause(20);
    let r = bus.read_raw(addr, READ)?;
    ensure!(r.len() >= 6, "réponse ASRock trop courte à 0x{addr:02X}");
    let bits = u16::from(r[4]) | (u16::from(r[5]) << 8);
    let channels: Vec<u8> = (0..16u8).filter(|i| bits & (1 << i) != 0).collect();
    ensure!(!channels.is_empty(), "aucun canal actif sur la carte ASRock à 0x{addr:02X}");
    Ok(channels)
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, channels: &[u8], colors: &[Color]) -> Result<()> {
    for (i, ch) in channels.iter().enumerate() {
        let c = colors.get(i).or(colors.last()).copied().unwrap_or_default();
        send(bus, addr, 0x10, *ch, &[0x01, c.r, c.g, c.b, 0x80, 0xFF, 0x00, 0x1A])?;
    }
    Ok(())
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
        fn pause(&mut self, _: u64) {}
    }

    #[test]
    fn detect_reads_the_channel_bitmap_without_any_color_write() {
        // bits 3, 6 et 7 : ARGB, côté, ventilateur
        let mut b = Raw { reply: vec![0, 0, 0, 0, 0b1100_1000, 0], ..Default::default() };
        assert_eq!(detect(&mut b, 0x36).expect("ok"), vec![3, 6, 7]);
        assert_eq!(b.writes, vec![vec![0x14, 0, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0]]);
    }

    #[test]
    fn detect_refuses_an_empty_bitmap() {
        assert!(detect(&mut Raw::default(), 0x36).is_err());
    }

    #[test]
    fn channel_colors_are_12_byte_frames() {
        let mut b = Raw::default();
        set_colors(&mut b, 0x36, &[3, 6], &[Color { r: 1, g: 2, b: 3 }, Color { r: 4, g: 5, b: 6 }]).expect("ok");
        assert_eq!(b.writes[0], vec![0x10, 0, 3, 0x01, 1, 2, 3, 0x80, 0xFF, 0x00, 0x1A, 0]);
        assert_eq!(b.writes[1], vec![0x10, 0, 6, 0x01, 4, 5, 6, 0x80, 0xFF, 0x00, 0x1A, 0]);
        assert!(b.writes.iter().all(|w| w.len() == 12));
    }
}
