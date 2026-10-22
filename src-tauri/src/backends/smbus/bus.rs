//! Source : docs/rgb-protocols/smbus-i2c-acces-et-securite.md (§3 anatomie d'une barrette, §5 ce qu'il ne faut JAMAIS écrire).
//!
//! Abstraction du bus SMBus : le driver ne parle qu'à ce trait (réel = `pawnio::PawnioBus`, tests = `MockBus`).
//! Garde-fou commun : toute écriture sur une adresse de SPD / hub / PMIC / capteur est refusée ICI, quel que soit l'appelant.
//! `write_quick` n'existe volontairement pas : la note le déconseille comme sonde (§5.3).

use anyhow::{bail, Result};

/// Adresses 7 bits où une écriture est interdite quoi qu'il arrive (note §3) :
/// 0x30..0x37 (capteur DDR5, sélection de page SPD), 0x48..0x4F (PMIC DDR5), 0x50..0x57 (SPD / hub SPD5118).
pub fn ensure_writable(addr: u8) -> Result<()> {
    if matches!(addr, 0x30..=0x37 | 0x48..=0x57) {
        bail!("écriture SMBus refusée à 0x{addr:02X} : zone SPD / PMIC / capteur d'une barrette");
    }
    Ok(())
}

pub trait SmbusBus: Send {
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8>;
    fn write_byte_data(&mut self, addr: u8, reg: u8, value: u8) -> Result<()>;
    /// Bloc SMBus de 1 à 32 octets sur `reg`.
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()>;
    /// Pause entre deux transactions (les contrôleurs RGB sont lents, note §4.6).
    fn pause(&mut self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

#[cfg(test)]
pub use mock::{MockBus, Txn};

#[cfg(test)]
mod mock {
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Txn {
        Read(u8, u8),
        Write(u8, u8, u8),
        Block(u8, u8, Vec<u8>),
    }

    /// Bus simulé : mémoire par (adresse, registre), adresses « présentes », flux de lecture du registre 0x40.
    #[derive(Default)]
    pub struct MockBus {
        pub mem: HashMap<(u8, u8), u8>,
        pub present: HashSet<u8>,
        /// Octets renvoyés par lectures successives de 0x40 après une écriture sur 0x21.
        pub info: HashMap<u8, Vec<u8>>,
        stream: HashMap<u8, VecDeque<u8>>,
        pub log: Vec<Txn>,
        pub fail_writes: bool,
    }

    impl MockBus {
        pub fn writes(&self) -> Vec<&Txn> {
            self.log.iter().filter(|t| !matches!(t, Txn::Read(..))).collect()
        }

        /// Barrette Corsair moderne simulée : signature 0x1A/0x01, bloc info (VID 0x1B1C, firmware 1.2 build 5,
        /// octet 28 = `protocol`), CRC8 attendu `crc` dans le registre 0x42.
        pub fn add_dimm(&mut self, addr: u8, pid: u16, protocol: u8, crc: u8) {
            let mut b = vec![0u8; 32];
            b[..4].copy_from_slice(&[0x1C, 0x1B, (pid & 0xFF) as u8, (pid >> 8) as u8]);
            b[8..11].copy_from_slice(&[2, 1, 5]);
            b[28] = protocol;
            self.present.insert(addr);
            self.mem.insert((addr, 0x43), 0x1A);
            self.mem.insert((addr, 0x44), 0x01);
            self.mem.insert((addr, 0x42), crc);
            self.info.insert(addr, b);
        }
    }

    impl SmbusBus for MockBus {
        fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
            self.log.push(Txn::Read(addr, reg));
            if !self.present.contains(&addr) {
                bail!("NACK à 0x{addr:02X}");
            }
            if reg == 0x40 {
                return Ok(self.stream.get_mut(&addr).and_then(|q| q.pop_front()).unwrap_or(0));
            }
            Ok(self.mem.get(&(addr, reg)).copied().unwrap_or(0))
        }
        fn write_byte_data(&mut self, addr: u8, reg: u8, value: u8) -> Result<()> {
            ensure_writable(addr)?;
            self.log.push(Txn::Write(addr, reg, value));
            if self.fail_writes || !self.present.contains(&addr) {
                bail!("échec d'écriture à 0x{addr:02X}");
            }
            if reg == 0x21 {
                let data = self.info.get(&addr).cloned().unwrap_or_default();
                self.stream.insert(addr, data.into());
            }
            self.mem.insert((addr, reg), value);
            Ok(())
        }
        fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
            ensure_writable(addr)?;
            self.log.push(Txn::Block(addr, reg, data.to_vec()));
            if self.fail_writes || !self.present.contains(&addr) {
                bail!("échec d'écriture à 0x{addr:02X}");
            }
            Ok(())
        }
        fn pause(&mut self, _ms: u64) {}
    }
}
