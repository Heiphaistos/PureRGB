//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § EVGA GPUv3 (Ampere, RTX 30xx FTW3, adresse 0x2D) et signalrgb.md (EVGA Ampere).
//!
//! Blocs I2C à registre de 8 bits ; le premier octet de chaque bloc est sa longueur moins un (fait partie des données).
//! Détection : lecture du firmware (6 octets au registre 0xB1) qui doit réussir, sinon refus (avant toute écriture).
//! Activation (nécessaire avant que la carte réponde) : bloc `[04, C6, EB, EA, 15]` au registre 0xB2.
//! 4 zones (barre principale, logo latéral, logo arrière, ARGB) = 4 « LEDs » d'une couleur chacune :
//! mode des zones, registre 0xC0 : `[09, FF x8 avec le mode de la zone à l'index zone+1 (1 = statique), 00]` ;
//! couleur de la zone : registre 0xC1 + zone, `[04, luminosité 0xFF, R, G, B]`.
//! L'enregistrement permanent (registre 0x90, `[04 9E EB 00 90]`) n'est JAMAIS envoyé.
//! INCONNU : nombre de LEDs de la zone ARGB (lu à 0xC0 par la source de référence, non utilisé ici : une couleur par zone).

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

pub const ZONES: usize = 4;
const REG_FIRMWARE: u8 = 0xB1;
const REG_ENABLE: u8 = 0xB2;
const REG_MODE: u8 = 0xC0;
const REG_STATIC: u8 = 0xC1;
const MODE_STATIC: u8 = 0x01;
const PAUSE_MS: u64 = 10;

pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    let fw = bus.read_block_data(addr, REG_FIRMWARE, 6)?;
    ensure!(fw.len() == 6, "firmware EVGA illisible à 0x{addr:02X} ({} octets)", fw.len());
    Ok(())
}

fn block(bus: &mut dyn SmbusBus, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
    bus.write_block_data(addr, reg, data)?;
    bus.pause(PAUSE_MS);
    Ok(())
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    block(bus, addr, REG_ENABLE, &[0x04, 0xC6, 0xEB, 0xEA, 0x15])?;
    for zone in 0..ZONES {
        let mut pkt = [0xFF; 10];
        pkt[0] = 9;
        pkt[9] = 0;
        pkt[zone + 1] = MODE_STATIC;
        block(bus, addr, REG_MODE, &pkt)?;
    }
    Ok(())
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, colors: &[Color]) -> Result<()> {
    for zone in 0..ZONES {
        let c = colors.get(zone).or(colors.last()).copied().unwrap_or_default();
        block(bus, addr, REG_STATIC + zone as u8, &[0x04, 0xFF, c.r, c.g, c.b])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Rec {
        blocks: Vec<(u8, Vec<u8>)>,
        fw: Vec<u8>,
    }
    impl SmbusBus for Rec {
        fn read_byte_data(&mut self, _: u8, _: u8) -> Result<u8> {
            unreachable!()
        }
        fn write_byte_data(&mut self, _: u8, _: u8, _: u8) -> Result<()> {
            unreachable!()
        }
        fn write_block_data(&mut self, _: u8, reg: u8, d: &[u8]) -> Result<()> {
            self.blocks.push((reg, d.to_vec()));
            Ok(())
        }
        fn read_block_data(&mut self, _: u8, reg: u8, len: usize) -> Result<Vec<u8>> {
            assert_eq!((reg, len), (0xB1, 6));
            Ok(self.fw.clone())
        }
        fn pause(&mut self, _: u64) {}
    }

    #[test]
    fn verify_needs_a_six_byte_firmware_answer() {
        assert!(verify(&mut Rec { fw: vec![0, 0, 0, 1, 2, 3], ..Default::default() }, 0x2D).is_ok());
        assert!(verify(&mut Rec { fw: vec![], ..Default::default() }, 0x2D).is_err());
    }

    #[test]
    fn init_and_colors_match_the_note_and_never_save() {
        let mut b = Rec::default();
        init(&mut b, 0x2D).expect("init");
        assert_eq!(b.blocks[0], (0xB2, vec![4, 0xC6, 0xEB, 0xEA, 0x15]));
        assert_eq!(b.blocks[1], (0xC0, vec![9, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0]));
        assert_eq!(b.blocks[4], (0xC0, vec![9, 0xFF, 0xFF, 0xFF, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0]));
        set_colors(&mut b, 0x2D, &[Color { r: 1, g: 2, b: 3 }, Color { r: 4, g: 5, b: 6 }]).expect("couleurs");
        assert_eq!(b.blocks[5], (0xC1, vec![4, 0xFF, 1, 2, 3]));
        assert_eq!(b.blocks[6], (0xC2, vec![4, 0xFF, 4, 5, 6]));
        assert_eq!(b.blocks[8], (0xC4, vec![4, 0xFF, 4, 5, 6]), "dernière couleur répétée");
        assert!(b.blocks.iter().all(|(r, _)| *r != 0x90));
    }
}
