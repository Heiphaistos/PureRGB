//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md § « Gigabyte RGB Fusion 2 GPU » et signalrgb.md (Gen2 « Master »).
//!
//! Trames de 8 octets par écriture I2C brute (sans registre). Détection : envoi de `AB 00 00 00 00 00 00 00` puis lecture de 4 octets :
//! le premier doit valoir 0xAB et la réponse ne doit PAS être `AB AB AB AB` (écho du bus d'écran DDC, par exemple le second bus 0x71
//! d'une AORUS qui gère l'écran LCD : ne jamais l'écrire). Adresse 0x50 : exemptée du test (note SignalRGB).
//! Mode statique par zone (4 zones = 4 « LED » : zones 0..3), vitesse « normale » 0x02, luminosité max 0x63 :
//! `[0x88, 0x01, vitesse, luminosité, 0x00, zone+1, 0, 0]` puis la couleur : zones 0 et 1 `[0xB0, 0x01, R0 G0 B0, R1 G1 B1]`
//! (gauche + milieu), zone 2 `[0xB1, 0x01, R G B]`, zone 3 `[0x40, R, G, B, zone+1]`.
//! L'enregistrement permanent (`0xAA`) n'est JAMAIS envoyé.
//! INCONNU : nombre réel de zones de chaque carte (la source de référence en suppose 4) ; un modèle à moins de zones ignore les autres.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{ensure, Result};

pub const ZONES: usize = 4;
const PAUSE_MS: u64 = 10;
const SPEED_NORMAL: u8 = 0x02;
const BRIGHTNESS_MAX: u8 = 0x63;
const MODE_STATIC: u8 = 0x01;

fn send(bus: &mut dyn SmbusBus, addr: u8, pkt: [u8; 8]) -> Result<()> {
    bus.write_raw(addr, &pkt)?;
    bus.pause(PAUSE_MS);
    Ok(())
}

pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    if addr == 0x50 {
        return Ok(());
    }
    send(bus, addr, [0xAB, 0, 0, 0, 0, 0, 0, 0])?;
    let r = bus.read_raw(addr, 4)?;
    ensure!(r.first() == Some(&0xAB) && r.iter().any(|b| *b != 0xAB), "réponse Gigabyte Fusion2 inattendue à 0x{addr:02X} : {r:02X?}");
    Ok(())
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, colors: &[Color]) -> Result<()> {
    let c = |i: usize| colors.get(i).or(colors.last()).copied().unwrap_or_default();
    for zone in 0..ZONES {
        send(bus, addr, [0x88, MODE_STATIC, SPEED_NORMAL, BRIGHTNESS_MAX, 0x00, zone as u8 + 1, 0, 0])?;
        let pkt = match zone {
            0 | 1 => [0xB0, MODE_STATIC, c(0).r, c(0).g, c(0).b, c(1).r, c(1).g, c(1).b],
            2 => [0xB1, MODE_STATIC, c(2).r, c(2).g, c(2).b, 0, 0, 0],
            _ => [0x40, c(3).r, c(3).g, c(3).b, zone as u8 + 1, 0, 0, 0],
        };
        send(bus, addr, pkt)?;
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
            Ok(self.reply[..len].to_vec())
        }
        fn pause(&mut self, _: u64) {}
    }

    #[test]
    fn verify_rejects_the_ddc_echo_and_wrong_replies_but_skips_0x50() {
        let mut b = Raw { reply: vec![0xAB, 0x11, 0x52, 0x03], ..Default::default() };
        assert!(verify(&mut b, 0x71).is_ok());
        assert_eq!(b.writes[0], vec![0xAB, 0, 0, 0, 0, 0, 0, 0]);
        b.reply = vec![0xAB; 4];
        assert!(verify(&mut b, 0x71).is_err());
        b.reply = vec![0, 0, 0, 0];
        assert!(verify(&mut b, 0x71).is_err());
        let n = b.writes.len();
        assert!(verify(&mut b, 0x50).is_ok());
        assert_eq!(b.writes.len(), n, "0x50 : aucun envoi");
    }

    #[test]
    fn four_zone_static_frames_match_the_note_and_never_save() {
        let mut b = Raw::default();
        let col = |r, g, bl| Color { r, g, b: bl };
        set_colors(&mut b, 0x62, &[col(1, 2, 3), col(4, 5, 6), col(7, 8, 9), col(10, 11, 12)]).expect("ok");
        assert_eq!(b.writes.len(), 8);
        assert_eq!(b.writes[0], vec![0x88, 1, 2, 0x63, 0, 1, 0, 0]);
        assert_eq!(b.writes[1], vec![0xB0, 1, 1, 2, 3, 4, 5, 6]);
        assert_eq!(b.writes[3], vec![0xB0, 1, 1, 2, 3, 4, 5, 6]);
        assert_eq!(b.writes[5], vec![0xB1, 1, 7, 8, 9, 0, 0, 0]);
        assert_eq!(b.writes[7], vec![0x40, 10, 11, 12, 4, 0, 0, 0]);
        assert!(b.writes.iter().all(|w| w[0] != 0xAA));
        // moins de couleurs que de zones : la dernière est répétée
        let mut b = Raw::default();
        set_colors(&mut b, 0x62, &[col(9, 9, 9)]).expect("ok");
        assert_eq!(b.writes[7], vec![0x40, 9, 9, 9, 4, 0, 0, 0]);
    }
}
