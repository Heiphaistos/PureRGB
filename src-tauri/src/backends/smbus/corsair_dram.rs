//! Source : docs/rgb-protocols/corsair-vengeance-dram.md (famille 2 « moderne » : Vengeance RGB Pro, Dominator, Vengeance RGB DDR5...)
//! et docs/rgb-protocols/smbus-i2c-acces-et-securite.md §5.
//!
//! LISTE BLANCHE D'ÉCRITURES (tout le reste est refusé par le driver, avant même d'atteindre le bus) :
//! - octet : registre 0x61 (sélection du bloc info) et 0x21 (début de transfert), valeur 0x00 uniquement, adresses 0x58..0x5F / 0x18..0x1F ;
//! - bloc : registres 0x31 et 0x32 (tampon de couleur directe), 32 octets au plus, mêmes adresses.
//!
//! Jamais écrits : 0x23 (bootloader, risque de brique), 0x82 / 0x20 / 0x0B / 0x26 (effets, tampons, configuration), flash/EEPROM.
//!
//! INCONNU (refus ou non implémenté) :
//! - registre éventuel d'activation du « mode direct » : la note n'en donne aucun, seul le tampon 0x31/0x32 est écrit ;
//! - repli par écritures de mots si le bloc SMBus échoue (plan de découpage INCONNU) : l'erreur est remontée ;
//! - mode « tampon de couleurs » du protocole < 4 : refusé ; effets matériels : non implémentés ;
//! - retour au mode matériel (release) : la note ne dit pas comment restaurer l'effet précédent, donc pas de `release`.

use super::bus::SmbusBus;
use crate::core::Color;
use anyhow::{bail, ensure, Context, Result};
use std::time::{Duration, Instant};

const REG_INFO_SELECT: u8 = 0x61;
const REG_XFER_START: u8 = 0x21;
const REG_INFO_READ: u8 = 0x40;
const REG_INFO_CRC: u8 = 0x42;
const REG_SIG_A: u8 = 0x43;
const REG_SIG_B: u8 = 0x44;
const REG_DIRECT_1: u8 = 0x31;
const REG_DIRECT_2: u8 = 0x32;
const BLOCK_MAX: usize = 32;
const INFO_LEN: usize = 32;
/// Cadence : « 10 à 20 ms entre trames, ~30 Hz max » (recommandation de la note, non mesurée).
const MIN_FRAME_GAP: Duration = Duration::from_millis(20);

/// Adresses 7 bits candidates : 0x58..0x5F et 0x18..0x1F.
pub fn is_candidate_addr(addr: u8) -> bool {
    matches!(addr, 0x58..=0x5F | 0x18..=0x1F)
}

/// CRC8 : polynôme 0x07, init 0x00, xorout 0x00, sans réflexion.
pub fn crc8(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |mut crc, &b| {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
        }
        crc
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    pub name: &'static str,
    pub leds: u8,
    pub reversed: bool,
}

/// Tableau « PID interne -> modèle » de la note.
const MODELS: &[(&[u16], Model)] = &[
    (&[0x0100, 0x0101], Model { name: "Vengeance RGB Pro DDR4", leds: 10, reversed: false }),
    (&[0x0200, 0x0201], Model { name: "Dominator Platinum RGB DDR4", leds: 12, reversed: true }),
    (&[0x0300, 0x0301], Model { name: "Vengeance RGB Pro SL DDR4", leds: 10, reversed: false }),
    (&[0x0400, 0x0401], Model { name: "Vengeance RGB RS DDR4", leds: 6, reversed: false }),
    (&[0x0600, 0x0601], Model { name: "Dominator Platinum RGB DDR5", leds: 12, reversed: true }),
    (&[0x0800, 0x0801, 0x0810, 0x0811], Model { name: "Dominator Titanium RGB DDR5", leds: 12, reversed: true }),
    (&[0x0700, 0x0701, 0x0900, 0x0901, 0x0910, 0x0911], Model { name: "Vengeance RGB DDR5", leds: 10, reversed: false }),
    (&[0x0A00, 0x0A01, 0x0A10, 0x0A11], Model { name: "Vengeance Shugo Series DDR5", leds: 10, reversed: false }),
    (&[0x0B00, 0x0B01], Model { name: "Vengeance RGB RS DDR5", leds: 6, reversed: false }),
];

pub fn model_for_pid(pid: u16) -> Option<Model> {
    MODELS.iter().find(|(pids, _)| pids.contains(&pid)).map(|(_, m)| *m)
}

/// Bloc d'information de 32 octets (VID/PID little-endian, firmware, version du protocole).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    pub vid: u16,
    pub pid: u16,
    pub fw_major: u8,
    pub fw_minor: u8,
    pub fw_build: u16,
    pub protocol: u8,
}

/// Signature de détection : LECTURES SEULES. 0x43 dans {0x1A,0x1B,0x1C} ET 0x44 dans {0x01,0x03,0x04}.
/// `Err` = l'adresse ne répond pas ; `Ok(false)` = répond mais n'est pas une barrette Corsair moderne.
pub fn probe_signature(bus: &mut dyn SmbusBus, addr: u8) -> Result<bool> {
    ensure!(is_candidate_addr(addr), "adresse 0x{addr:02X} hors des candidates Corsair");
    let a = bus.read_byte_data(addr, REG_SIG_A)?;
    if !matches!(a, 0x1A..=0x1C) {
        return Ok(false);
    }
    let b = bus.read_byte_data(addr, REG_SIG_B)?;
    Ok(matches!(b, 0x01 | 0x03 | 0x04))
}

fn write_sel(bus: &mut dyn SmbusBus, addr: u8, reg: u8) -> Result<()> {
    ensure!(is_candidate_addr(addr) && matches!(reg, REG_INFO_SELECT | REG_XFER_START), "écriture hors liste blanche 0x{addr:02X}/0x{reg:02X}");
    bus.write_byte_data(addr, reg, 0x00)
}

fn write_direct_block(bus: &mut dyn SmbusBus, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
    ensure!(is_candidate_addr(addr) && matches!(reg, REG_DIRECT_1 | REG_DIRECT_2), "écriture hors liste blanche 0x{addr:02X}/0x{reg:02X}");
    ensure!(!data.is_empty() && data.len() <= BLOCK_MAX, "bloc SMBus de {} octets invalide", data.len());
    bus.write_block_data(addr, reg, data)
}

/// Lit et valide l'identité (à n'appeler qu'après `probe_signature` = true) : écrit 0x61=0 puis 0x21=0, lit 32 octets
/// sur 0x40, compare le CRC8 à celui du registre 0x42, puis exige un PID connu.
pub fn read_info(bus: &mut dyn SmbusBus, addr: u8) -> Result<(Info, Model)> {
    write_sel(bus, addr, REG_INFO_SELECT)?;
    write_sel(bus, addr, REG_XFER_START)?;
    let mut raw = [0u8; INFO_LEN];
    for b in raw.iter_mut() {
        *b = bus.read_byte_data(addr, REG_INFO_READ)?;
    }
    let expected = bus.read_byte_data(addr, REG_INFO_CRC)?;
    ensure!(crc8(&raw) == expected, "CRC8 du bloc d'information invalide (calculé 0x{:02X}, attendu 0x{expected:02X})", crc8(&raw));
    let info = Info {
        vid: u16::from_le_bytes([raw[0], raw[1]]),
        pid: u16::from_le_bytes([raw[2], raw[3]]),
        fw_minor: raw[8],
        fw_major: raw[9],
        fw_build: u16::from_le_bytes([raw[10], raw[11]]),
        protocol: raw[28],
    };
    let model = model_for_pid(info.pid).with_context(|| format!("PID interne 0x{:04X} absent du tableau de la note", info.pid))?;
    Ok((info, model))
}

/// Une barrette pilotable (mode direct, protocole >= 4).
pub struct CorsairDram {
    addr: u8,
    model: Model,
    last_frame: Option<Instant>,
}

impl CorsairDram {
    /// Refuse un protocole < 4 (mode tampon de couleurs non implémenté, INCONNU pour la sécurité d'écriture).
    pub fn new(addr: u8, info: &Info, model: Model) -> Result<Self> {
        ensure!(is_candidate_addr(addr), "adresse 0x{addr:02X} hors des candidates Corsair");
        if info.protocol < 4 {
            bail!("protocole Corsair {} < 4 : mode direct indisponible, écriture refusée", info.protocol);
        }
        Ok(CorsairDram { addr, model, last_frame: None })
    }

    pub fn led_count(&self) -> usize {
        self.model.leds as usize
    }

    /// Paquet direct : [nbLED, R,G,B par LED (ordre inversé pour Dominator), CRC8].
    fn packet(&self, colors: &[Color]) -> Vec<u8> {
        let n = self.led_count();
        let mut p = Vec::with_capacity(3 * n + 2);
        p.push(self.model.leds);
        for i in 0..n {
            let c = if self.model.reversed { colors[n - 1 - i] } else { colors[i] };
            p.extend_from_slice(&[c.r, c.g, c.b]);
        }
        p.push(crc8(&p));
        p
    }

    /// Une couleur par LED, exactement `led_count()`. Premier bloc (32 octets max) sur 0x31, reste sur 0x32.
    pub fn set_colors(&mut self, bus: &mut dyn SmbusBus, colors: &[Color]) -> Result<()> {
        ensure!(colors.len() == self.led_count(), "{} couleurs reçues, {} attendues pour {}", colors.len(), self.led_count(), self.model.name);
        if let Some(t) = self.last_frame {
            if let Some(wait) = MIN_FRAME_GAP.checked_sub(t.elapsed()) {
                bus.pause(wait.as_millis() as u64 + 1);
            }
        }
        let p = self.packet(colors);
        let (first, rest) = p.split_at(p.len().min(BLOCK_MAX));
        write_direct_block(bus, self.addr, REG_DIRECT_1, first)?;
        if !rest.is_empty() {
            write_direct_block(bus, self.addr, REG_DIRECT_2, rest)?;
        }
        self.last_frame = Some(Instant::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};
    use crate::backends::smbus::detect::scan;

    fn open(bus: &mut MockBus, pid: u16, crc: u8) -> CorsairDram {
        bus.add_dimm(0x58, pid, 4, crc);
        let f = scan(bus).remove(0);
        bus.log.clear();
        CorsairDram::new(f.addr, &f.info, f.model).expect("pilote")
    }

    #[test]
    fn crc8_check_vector() {
        assert_eq!(crc8(b"123456789"), 0xF4);
    }

    #[test]
    fn leds_and_order_per_pid() {
        for (pid, leds, rev) in [
            (0x0100, 10, false), (0x0201, 12, true), (0x0300, 10, false), (0x0401, 6, false), (0x0600, 12, true),
            (0x0811, 12, true), (0x0901, 10, false), (0x0A10, 10, false), (0x0B01, 6, false),
        ] {
            let m = model_for_pid(pid).expect("pid connu");
            assert_eq!((m.leds, m.reversed), (leds, rev), "{pid:#06x}");
        }
        assert!(model_for_pid(0x0000).is_none());
        assert!(model_for_pid(0x0500).is_none());
    }

    #[test]
    fn ten_red_leds_are_one_32_byte_block_on_0x31() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0900, 0x84);
        d.set_colors(&mut bus, &[Color { r: 255, g: 0, b: 0 }; 10]).expect("écriture");
        let mut want = vec![10u8];
        want.extend([255, 0, 0].repeat(10));
        want.push(0x6C);
        assert_eq!(bus.log, vec![Txn::Block(0x58, 0x31, want)]);
    }

    #[test]
    fn twelve_leds_reversed_split_32_plus_6() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0201, 0x47);
        let colors: Vec<Color> = (0..12u8).map(|i| Color { r: 10 * i, g: 10 * i + 1, b: 10 * i + 2 }).collect();
        d.set_colors(&mut bus, &colors).expect("écriture");
        assert_eq!(
            bus.log,
            vec![
                Txn::Block(0x58, 0x31, vec![
                    12, 110, 111, 112, 100, 101, 102, 90, 91, 92, 80, 81, 82, 70, 71, 72,
                    60, 61, 62, 50, 51, 52, 40, 41, 42, 30, 31, 32, 20, 21, 22, 10,
                ]),
                Txn::Block(0x58, 0x32, vec![11, 12, 0, 1, 2, 204]),
            ]
        );
    }

    #[test]
    fn six_leds_packet_length_and_crc_are_consistent() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0400, 0x68);
        d.set_colors(&mut bus, &[Color { r: 1, g: 2, b: 3 }; 6]).expect("écriture");
        match &bus.log[0] {
            Txn::Block(0x58, 0x31, p) => {
                assert_eq!(p.len(), 3 * 6 + 2);
                assert_eq!(p[0], 6);
                assert_eq!(crc8(p), 0); // propriété du CRC : données + CRC => 0
            }
            t => panic!("transaction inattendue {t:?}"),
        }
    }

    #[test]
    fn only_whitelisted_registers_are_ever_written() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0900, 0x84);
        d.set_colors(&mut bus, &[Color::default(); 10]).expect("écriture");
        for t in bus.writes() {
            assert!(matches!(t, Txn::Block(0x58, 0x31 | 0x32, _)), "{t:?}");
        }
    }

    #[test]
    fn rejects_wrong_color_count_without_touching_the_bus() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0900, 0x84);
        assert!(d.set_colors(&mut bus, &[]).is_err());
        assert!(d.set_colors(&mut bus, &[Color::default(); 9]).is_err());
        assert!(d.set_colors(&mut bus, &[Color::default(); 11]).is_err());
        assert!(bus.log.is_empty());
    }

    #[test]
    fn old_protocol_and_foreign_address_are_refused() {
        let info = Info { vid: 0x1B1C, pid: 0x0900, fw_major: 1, fw_minor: 0, fw_build: 0, protocol: 3 };
        let m = model_for_pid(0x0900).expect("modèle");
        assert!(CorsairDram::new(0x58, &info, m).is_err());
        let ok = Info { protocol: 4, ..info };
        for addr in [0x48, 0x4F, 0x50, 0x57, 0x36, 0x37, 0x20, 0x60, 0x00] {
            assert!(CorsairDram::new(addr, &ok, m).is_err(), "{addr:#x}");
        }
    }

    #[test]
    fn bus_write_errors_are_reported() {
        let mut bus = MockBus::default();
        let mut d = open(&mut bus, 0x0900, 0x84);
        bus.fail_writes = true;
        assert!(d.set_colors(&mut bus, &[Color::default(); 10]).is_err());
    }

    #[test]
    fn bus_guard_refuses_spd_pmic_and_hub_even_if_called_directly() {
        let mut bus = MockBus::default();
        for addr in [0x30u8, 0x36, 0x37, 0x48, 0x4F, 0x50, 0x57] {
            assert!(bus.write_byte_data(addr, 0x31, 0).is_err());
            assert!(bus.write_block_data(addr, 0x31, &[1]).is_err());
        }
        assert!(bus.log.is_empty());
    }

    #[test]
    fn read_info_errors_on_bus_failure() {
        let mut bus = MockBus::default(); // adresse non présente
        assert!(read_info(&mut bus, 0x58).is_err());
        assert!(probe_signature(&mut bus, 0x50).is_err()); // hors candidates
        assert!(bus.writes().iter().all(|t| matches!(t, Txn::Write(0x58, 0x61 | 0x21, 0))));
    }
}
