//! Source : docs/rgb-protocols/smbus-temperature-ram.md (JEDEC JESD300-5B / SPD5118, JEDEC 21-C TSE2004, JC-42.4).
//!
//! Températures des barrettes, LECTURE SEULE. Aucune écriture : les seules « sélections » sont l'octet de commande
//! (pointeur de registre) inclus dans une transaction SMBus de lecture, jamais de page SPD, de configuration ni d'EEPROM.
//! - DDR4 : capteur TSE2004 à 0x18+n (registre 0x05, mot 16 bits poids fort d'abord, 13 bits signés, 0,0625 °C/LSB),
//!   accepté seulement si le SPD voisin 0x50+n annonce DDR4 (octet 2 = 0x0C).
//! - DDR5 : hub SPD5118 à 0x50+n (MR0:MR1 = 0x51 0x18, MR5 bit 1 = capteur présent, MR26 bit 0 = capteur coupé,
//!   MR49:MR50 = température, poids faible d'abord, bits 12:2 signés, 0,25 °C/LSB).

use super::bus::SmbusBus;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ddr4,
    Ddr5,
}

/// Emplacement SMBus n (0..7) et type de mémoire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub n: u8,
    pub kind: Kind,
}

const SPD_BASE: u8 = 0x50;
const TS_BASE: u8 = 0x18;
const DDR4_TYPE: u8 = 0x0C;
const TS_REG: u8 = 0x05;
const MR_CAP: u8 = 0x05;
const MR_TS_CONFIG: u8 = 0x1A;
const MR_TEMP: u8 = 0x31;
/// Plage plausible : en dehors, la lecture est rejetée (bus parasité, capteur absent).
const PLAUSIBLE: std::ops::RangeInclusive<f64> = -40.0..=150.0;

/// TSE2004 : mot SMBus reçu (octet de poids fort du registre en premier, donc à l'envers) -> °C.
pub fn ddr4_celsius(word: u16) -> f64 {
    let raw = (word.swap_bytes() & 0x1FFF) as i32;
    let signed = if raw & 0x1000 != 0 { raw - 0x2000 } else { raw };
    f64::from(signed) * 0.0625
}

/// SPD5118 : MR49 (bas) et MR50 (haut) -> °C ; température sur les bits 12:2, 11 bits signés.
pub fn ddr5_celsius(lo: u8, hi: u8) -> f64 {
    let v = i32::from((u16::from(hi) << 8 | u16::from(lo)) >> 2) & 0x7FF;
    let signed = if v & 0x400 != 0 { v - 0x800 } else { v };
    f64::from(signed) * 0.25
}

fn probe(bus: &mut dyn SmbusBus, n: u8) -> Result<Option<Kind>> {
    let spd = SPD_BASE + n;
    let (r0, r1) = (bus.read_byte_data(spd, 0x00)?, bus.read_byte_data(spd, 0x01)?);
    if r0 == 0x51 && (r1 & 0xEF) == 0x08 {
        let cap = bus.read_byte_data(spd, MR_CAP)?;
        let cfg = bus.read_byte_data(spd, MR_TS_CONFIG)?;
        return Ok((cap & 0x02 != 0 && cfg & 0x01 == 0).then_some(Kind::Ddr5));
    }
    if bus.read_byte_data(spd, 0x02)? == DDR4_TYPE && bus.read_word_data(TS_BASE + n, TS_REG).is_ok() {
        return Ok(Some(Kind::Ddr4));
    }
    Ok(None)
}

/// Emplacements occupés avec capteur lisible (à mémoriser : ne plus sonder ensuite).
pub fn detect(bus: &mut dyn SmbusBus) -> Vec<Slot> {
    let mut out = Vec::new();
    for n in 0..8 {
        if let Ok(Some(kind)) = probe(bus, n) {
            out.push(Slot { n, kind });
        }
        bus.pause(1);
    }
    out
}

/// Température courante en °C, ou erreur si la lecture échoue ou sort de la plage plausible.
pub fn read(bus: &mut dyn SmbusBus, slot: Slot) -> Result<f64> {
    let c = match slot.kind {
        Kind::Ddr4 => ddr4_celsius(bus.read_word_data(TS_BASE + slot.n, TS_REG)?),
        Kind::Ddr5 => {
            let spd = SPD_BASE + slot.n;
            ddr5_celsius(bus.read_byte_data(spd, MR_TEMP)?, bus.read_byte_data(spd, MR_TEMP + 1)?)
        }
    };
    anyhow::ensure!(PLAUSIBLE.contains(&c), "température implausible ({c} °C)");
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::super::bus::MockBus;
    use super::*;

    fn ddr5_dimm(m: &mut MockBus, n: u8, lo: u8, hi: u8) {
        let a = SPD_BASE + n;
        m.present.insert(a);
        m.mem.insert((a, 0x00), 0x51);
        m.mem.insert((a, 0x01), 0x18);
        m.mem.insert((a, MR_CAP), 0x02);
        m.mem.insert((a, MR_TEMP), lo);
        m.mem.insert((a, MR_TEMP + 1), hi);
    }

    fn ddr4_dimm(m: &mut MockBus, n: u8, wire_word: u16) {
        m.present.insert(SPD_BASE + n);
        m.mem.insert((SPD_BASE + n, 0x00), 0x23);
        m.mem.insert((SPD_BASE + n, 0x02), DDR4_TYPE);
        m.present.insert(TS_BASE + n);
        m.words.insert((TS_BASE + n, TS_REG), wire_word);
    }

    #[test]
    fn ddr4_decoding() {
        // registre 0x0190 = 25,0 °C ; envoyé poids fort d'abord -> mot reçu 0x9001
        assert_eq!(ddr4_celsius(0x9001), 25.0);
        // 0x1FF0 = -1,0 °C ; drapeaux de poids fort (bits 15:13) ignorés
        assert_eq!(ddr4_celsius(0xF0FF), -1.0);
        // 0x0194 = 25,25 °C
        assert_eq!(ddr4_celsius(0x9401), 25.25);
    }

    #[test]
    fn ddr5_decoding() {
        // 40,25 °C = 161 pas de 0,25 -> bits 12:2 : 161 << 2 = 0x284
        assert_eq!(ddr5_celsius(0x84, 0x02), 40.25);
        // -5,0 °C = -20 pas -> 11 bits 0x7EC -> << 2 = 0x1FB0
        assert_eq!(ddr5_celsius(0xB0, 0x1F), -5.0);
        // bits 1:0 et 15:13 ignorés
        assert_eq!(ddr5_celsius(0x87, 0xE2), 40.25);
    }

    #[test]
    fn detects_both_generations_and_reads_without_any_write() {
        let mut m = MockBus::default();
        ddr5_dimm(&mut m, 1, 0x84, 0x02);
        ddr4_dimm(&mut m, 3, 0x9001);
        let slots = detect(&mut m);
        assert_eq!(slots, vec![Slot { n: 1, kind: Kind::Ddr5 }, Slot { n: 3, kind: Kind::Ddr4 }]);
        assert_eq!(read(&mut m, slots[0]).expect("ddr5"), 40.25);
        assert_eq!(read(&mut m, slots[1]).expect("ddr4"), 25.0);
        assert!(m.writes().is_empty());
    }

    #[test]
    fn refuses_disabled_or_absent_sensors_and_implausible_values() {
        let mut m = MockBus::default();
        ddr5_dimm(&mut m, 0, 0, 0);
        m.mem.insert((SPD_BASE, MR_TS_CONFIG), 0x01); // capteur coupé : on ne le réactive jamais
        ddr5_dimm(&mut m, 1, 0, 0);
        m.mem.insert((SPD_BASE + 1, MR_CAP), 0x00); // pas de capteur
        ddr4_dimm(&mut m, 2, 0x9001);
        m.present.remove(&(TS_BASE + 2)); // SPD DDR4 sans capteur répondant
        assert!(detect(&mut m).is_empty());
        assert!(m.writes().is_empty());
        ddr5_dimm(&mut m, 4, 0xFC, 0x0F); // 1023 pas = 255,75 °C
        assert!(read(&mut m, Slot { n: 4, kind: Kind::Ddr5 }).is_err());
    }
}
