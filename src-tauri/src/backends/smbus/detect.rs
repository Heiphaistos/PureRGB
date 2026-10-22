//! Source : docs/rgb-protocols/smbus-i2c-acces-et-securite.md §4-§5 et docs/rgb-protocols/corsair-vengeance-dram.md « Détection sûre ».
//!
//! Détection en deux temps :
//! 1. sondage de signature : LECTURES SEULES, sur les 16 adresses candidates 0x58..0x5F puis 0x18..0x1F (jamais SPD/PMIC/hub) ;
//! 2. identification (uniquement pour une adresse qui a passé la signature) : lecture du bloc d'information, ce qui exige les
//!    deux seules écritures de sélection documentées (0x61 = 0, 0x21 = 0), voir `corsair_dram::read_info`.
//!
//! Écart avec la consigne : la note ne décrit PAS le motif 0xA0..0xAF = 0xBA pour la famille moderne (DDR5), seulement pour la
//! V1 (DDR4, 1 LED, contrôle JEDEC du SPD requis). La V1 n'est donc pas implémentée ; la famille moderne se détecte par 0x43/0x44.

use super::bus::SmbusBus;
use super::corsair_dram::{is_candidate_addr, probe_signature, read_info, Info, Model};

/// Barrette reconnue et identifiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub addr: u8,
    pub info: Info,
    pub model: Model,
}

/// Adresses candidates, dans l'ordre de la note.
pub fn candidates() -> impl Iterator<Item = u8> {
    (0x58..=0x5F).chain(0x18..=0x1F)
}

/// Balaye les candidates (pause de 10 ms entre chacune). Une adresse muette, à signature fausse, à CRC faux ou dont
/// le PID est hors tableau est ignorée (refus), jamais forcée.
pub fn scan(bus: &mut dyn SmbusBus) -> Vec<Found> {
    let mut found = Vec::new();
    for addr in candidates().filter(|a| is_candidate_addr(*a)) {
        match probe_signature(bus, addr) {
            Ok(true) => match read_info(bus, addr) {
                Ok((info, model)) => found.push(Found { addr, info, model }),
                Err(e) => log::warn!("smbus 0x{addr:02X} : signature Corsair mais identité refusée : {e:#}"),
            },
            Ok(false) => {}
            Err(e) => log::debug!("smbus 0x{addr:02X} : pas de réponse ({e:#})"),
        }
        bus.pause(10);
    }
    found
}

/// Sondage de signature SEUL (lectures uniquement, aucune écriture même documentée) : adresses où une barrette Corsair
/// de la famille moderne répond. Sert tant que l'écriture sur la mémoire n'est pas explicitement autorisée.
pub fn scan_signatures(bus: &mut dyn SmbusBus) -> Vec<u8> {
    let mut found = Vec::new();
    for addr in candidates().filter(|a| is_candidate_addr(*a)) {
        if matches!(probe_signature(bus, addr), Ok(true)) {
            found.push(addr);
        }
        bus.pause(10);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};

    #[test]
    fn accepts_a_vengeance_ddr5_and_reads_its_identity() {
        let mut bus = MockBus::default();
        bus.add_dimm(0x59, 0x0900, 4, 0x84);
        let f = scan(&mut bus);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].addr, 0x59);
        assert_eq!(f[0].model.leds, 10);
        assert!(!f[0].model.reversed);
        assert_eq!((f[0].info.pid, f[0].info.protocol), (0x0900, 4));
    }

    #[test]
    fn probing_never_writes_and_only_touches_candidate_addresses() {
        let mut bus = MockBus::default();
        scan(&mut bus); // aucune barrette : lectures seules
        assert!(bus.writes().is_empty());
        let expected: Vec<Txn> = (0x58..=0x5Fu8).chain(0x18..=0x1F).map(|a| Txn::Read(a, 0x43)).collect();
        assert_eq!(bus.log, expected);
    }

    #[test]
    fn identification_writes_only_the_two_documented_selectors() {
        let mut bus = MockBus::default();
        bus.add_dimm(0x58, 0x0900, 4, 0x84);
        scan(&mut bus);
        assert_eq!(bus.writes(), vec![&Txn::Write(0x58, 0x61, 0x00), &Txn::Write(0x58, 0x21, 0x00)]);
    }

    #[test]
    fn refuses_wrong_signature_values_without_any_write() {
        for (r43, r44) in [(0x19, 0x01), (0x1D, 0x01), (0x1A, 0x02), (0x1A, 0x05), (0x00, 0x00)] {
            let mut bus = MockBus::default();
            bus.add_dimm(0x5A, 0x0900, 4, 0x84);
            bus.mem.insert((0x5A, 0x43), r43);
            bus.mem.insert((0x5A, 0x44), r44);
            assert!(scan(&mut bus).is_empty(), "{r43:#x}/{r44:#x}");
            assert!(bus.writes().is_empty());
        }
    }

    #[test]
    fn accepts_every_documented_signature_value() {
        for (r43, r44) in [(0x1A, 0x01), (0x1B, 0x03), (0x1C, 0x04)] {
            let mut bus = MockBus::default();
            bus.add_dimm(0x5A, 0x0900, 4, 0x84);
            bus.mem.insert((0x5A, 0x43), r43);
            bus.mem.insert((0x5A, 0x44), r44);
            assert_eq!(scan(&mut bus).len(), 1, "{r43:#x}/{r44:#x}");
        }
    }

    #[test]
    fn signature_scan_is_read_only_even_when_a_dimm_answers() {
        let mut bus = MockBus::default();
        bus.add_dimm(0x58, 0x0900, 4, 0x84);
        assert_eq!(scan_signatures(&mut bus), vec![0x58]);
        assert!(bus.writes().is_empty());
    }

    #[test]
    fn refuses_bad_crc_and_unknown_pid() {
        let mut bus = MockBus::default();
        bus.add_dimm(0x58, 0x0900, 4, 0x85); // CRC attendu 0x84
        bus.add_dimm(0x59, 0x0FFF, 4, 0x00); // PID hors tableau
        assert!(scan(&mut bus).is_empty());
    }

    #[test]
    fn mute_address_is_ignored() {
        let mut bus = MockBus::default();
        bus.add_dimm(0x1C, 0x0900, 4, 0x84);
        bus.present.remove(&0x1C);
        assert!(scan(&mut bus).is_empty());
    }

    #[test]
    fn spd_pmic_and_hub_addresses_are_never_candidates() {
        assert!(candidates().all(|a| !matches!(a, 0x30..=0x37 | 0x48..=0x57)));
        assert_eq!(candidates().count(), 16);
    }
}
