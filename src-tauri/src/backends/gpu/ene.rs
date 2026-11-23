//! Source : docs/rgb-protocols/gskill-trident-z-ene.md (protocole ENE) et gpu-controleurs-par-constructeur.md § « ASUS ENE GPU », adresse 0x67.
//!
//! Registres 16 bits : sélection = `write_word_data(addr, 0x00, ((reg << 8) & 0xFF00) | (reg >> 8))`, lecture = `read_byte_data(addr, 0x81)`,
//! écriture d'un octet = `write_byte_data(addr, 0x01, valeur)`. (L'écriture par bloc de la note n'est pas utilisée : un octet à la fois.)
//! Détection sûre, AVANT toute écriture : registres directs 0xA0..0xAF = 0x00..0x0F ; chaîne « Micron » à 0x1030 = refus (autre famille) ;
//! chaîne de version à 0x1000 dans la liste des GPU ASUS ; nombre de LEDs = table de configuration (0x1C00) octet 3. Sinon : refus.
//! Couleur directe (V2) : `0x8020 = 1`, `0x80A0 = 1`, puis couleurs à 0x8100 dans l'ordre R, B, G (voulu, ne pas « corriger »).
//! L'enregistrement permanent (`0x80A0 = 0xAA`) n'est JAMAIS écrit.

use crate::backends::smbus::bus::SmbusBus;
use crate::core::Color;
use anyhow::{bail, ensure, Result};

const REG_DEVICE_NAME: u16 = 0x1000;
const REG_MICRON_CHECK: u16 = 0x1030;
const REG_CONFIG_TABLE: u16 = 0x1C00;
const REG_DIRECT: u16 = 0x8020;
const REG_APPLY: u16 = 0x80A0;
const REG_COLORS_DIRECT_V2: u16 = 0x8100;
const CONFIG_LED_COUNT: usize = 0x03;
const MAX_LEDS: u8 = 10; // 30 octets de couleurs V2 = 10 LEDs
const GPU_VERSIONS: [&str; 7] =
    ["AUMA0-E6K5-0107", "AUMA0-E6K5-1107", "AUMA0-E6K5-1110", "AUMA0-E6K5-1111", "AUMA0-E6K5-0008", "AUMA0-E6K5-1113", "AUMA0-E6K5-1114"];

fn select(bus: &mut dyn SmbusBus, addr: u8, reg: u16) -> Result<()> {
    bus.write_word_data(addr, 0x00, ((reg << 8) & 0xFF00) | (reg >> 8))
}

fn read(bus: &mut dyn SmbusBus, addr: u8, reg: u16) -> Result<u8> {
    select(bus, addr, reg)?;
    bus.read_byte_data(addr, 0x81)
}

fn write(bus: &mut dyn SmbusBus, addr: u8, reg: u16, value: u8) -> Result<()> {
    select(bus, addr, reg)?;
    bus.write_byte_data(addr, 0x01, value)
}

fn read_string(bus: &mut dyn SmbusBus, addr: u8, reg: u16, len: u16) -> Result<String> {
    let mut v = Vec::new();
    for i in 0..len {
        match read(bus, addr, reg + i)? {
            0 => break,
            b => v.push(b),
        }
    }
    Ok(String::from_utf8_lossy(&v).into_owned())
}

/// Vérifie la signature ENE et renvoie le nombre de LEDs (lecture seule).
pub fn verify(bus: &mut dyn SmbusBus, addr: u8) -> Result<u8> {
    for i in 0..16u8 {
        let v = bus.read_byte_data(addr, 0xA0 + i)?;
        ensure!(v == i, "motif ENE absent à 0x{addr:02X} (registre 0x{:02X} = 0x{v:02X})", 0xA0 + i);
    }
    ensure!(read_string(bus, addr, REG_MICRON_CHECK, 6)? != "Micron", "contrôleur Micron/Crucial à 0x{addr:02X}, pas un ENE GPU");
    let version = read_string(bus, addr, REG_DEVICE_NAME, 16)?;
    if !GPU_VERSIONS.contains(&version.as_str()) {
        bail!("version ENE non reconnue pour un GPU ASUS : « {version} »");
    }
    let leds = read(bus, addr, REG_CONFIG_TABLE + CONFIG_LED_COUNT as u16)?;
    ensure!((1..=MAX_LEDS).contains(&leds), "nombre de LEDs ENE invalide : {leds}");
    Ok(leds)
}

pub fn init(bus: &mut dyn SmbusBus, addr: u8) -> Result<()> {
    write(bus, addr, REG_DIRECT, 1)?;
    write(bus, addr, REG_APPLY, 1)
}

pub fn set_colors(bus: &mut dyn SmbusBus, addr: u8, colors: &[Color]) -> Result<()> {
    for (i, c) in colors.iter().take(MAX_LEDS as usize).enumerate() {
        let base = REG_COLORS_DIRECT_V2 + (i as u16) * 3;
        // ordre du fil : rouge, bleu, vert
        for (k, v) in [c.r, c.b, c.g].into_iter().enumerate() {
            write(bus, addr, base + k as u16, v)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Ene {
        ptr: u16,
        mem: HashMap<u16, u8>,
        writes: Vec<(u16, u8)>,
        words: Vec<u16>,
        bad_pattern: bool,
    }

    impl Ene {
        fn with(version: &str, micron: bool, leds: u8) -> Ene {
            let mut e = Ene::default();
            for (i, b) in version.bytes().enumerate() {
                e.mem.insert(REG_DEVICE_NAME + i as u16, b);
            }
            if micron {
                for (i, b) in b"Micron".iter().enumerate() {
                    e.mem.insert(REG_MICRON_CHECK + i as u16, *b);
                }
            }
            e.mem.insert(REG_CONFIG_TABLE + 3, leds);
            e
        }
    }

    impl SmbusBus for Ene {
        fn read_byte_data(&mut self, _: u8, reg: u8) -> Result<u8> {
            if (0xA0..0xB0).contains(&reg) {
                return Ok(if self.bad_pattern { 0xFF } else { reg - 0xA0 });
            }
            assert_eq!(reg, 0x81, "lecture ENE = registre 0x81");
            Ok(self.mem.get(&self.ptr).copied().unwrap_or(0))
        }
        fn write_word_data(&mut self, _: u8, reg: u8, word: u16) -> Result<()> {
            assert_eq!(reg, 0x00);
            self.words.push(word);
            self.ptr = (word << 8) | (word >> 8);
            Ok(())
        }
        fn write_byte_data(&mut self, _: u8, reg: u8, v: u8) -> Result<()> {
            assert_eq!(reg, 0x01, "écriture ENE = registre 0x01");
            self.writes.push((self.ptr, v));
            Ok(())
        }
        fn write_block_data(&mut self, _: u8, _: u8, _: &[u8]) -> Result<()> {
            unreachable!("pas d'écriture par bloc")
        }
    }

    #[test]
    fn register_select_word_matches_the_note() {
        let mut e = Ene::default();
        select(&mut e, 0x67, 0x8021).expect("sel");
        select(&mut e, 0x67, 0x80F9).expect("sel");
        select(&mut e, 0x67, 0x1000).expect("sel");
        assert_eq!(e.words, vec![0x2180, 0xF980, 0x0010]);
    }

    #[test]
    fn verify_accepts_a_gpu_and_reads_led_count_without_writing() {
        let mut e = Ene::with("AUMA0-E6K5-1110", false, 6);
        assert_eq!(verify(&mut e, 0x67).expect("ok"), 6);
        assert!(e.writes.is_empty());
    }

    #[test]
    fn verify_refuses_bad_pattern_micron_unknown_version_and_bad_count() {
        let mut e = Ene::with("AUMA0-E6K5-0107", false, 4);
        e.bad_pattern = true;
        assert!(verify(&mut e, 0x67).is_err());
        assert!(verify(&mut Ene::with("AUMA0-E6K5-0107", true, 4), 0x67).is_err());
        assert!(verify(&mut Ene::with("DIMM_LED-0102", false, 4), 0x67).is_err(), "version de RAM, pas de GPU");
        assert!(verify(&mut Ene::with("AUMA0-E6K5-0107", false, 0), 0x67).is_err());
        assert!(verify(&mut Ene::with("AUMA0-E6K5-0107", false, 11), 0x67).is_err());
    }

    #[test]
    fn direct_mode_and_colors_are_r_b_g_and_never_save() {
        let mut e = Ene::default();
        init(&mut e, 0x67).expect("init");
        set_colors(&mut e, 0x67, &[Color { r: 0xFF, g: 0x01, b: 0x02 }, Color { r: 3, g: 4, b: 5 }]).expect("couleurs");
        assert_eq!(e.writes[..2], [(0x8020, 1), (0x80A0, 1)]);
        assert_eq!(e.writes[2..5], [(0x8100, 0xFF), (0x8101, 0x02), (0x8102, 0x01)]);
        assert_eq!(e.writes[5..8], [(0x8103, 3), (0x8104, 5), (0x8105, 4)]);
        assert!(e.writes.iter().all(|w| w.1 != 0xAA));
    }
}
