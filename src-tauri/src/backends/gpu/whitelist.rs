//! Source : docs/rgb-protocols/gpu-controleurs-par-constructeur.md (§ liste blanche PCI) et signalrgb-devices.csv
//! (lignes `smbus-i2c-nvapi:*`), extraites dans `assets/gpu_whitelist.tsv`.
//!
//! Règle de sécurité (note gpu-acces-i2c-nvapi-adl.md §3) : on ne parle AU BUS DE LA CARTE qu'après correspondance exacte
//! (vendeur NVIDIA 10DE, device, sous-vendeur, sous-device). Une carte absente de la liste n'est jamais touchée.

use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub family: String,
    pub device: u16,
    pub subvendor: u16,
    pub subdevice: u16,
    /// Adresse I2C 7 bits du contrôleur RGB.
    pub addr: u8,
    pub name: String,
}

/// Identifiants PCI d'une carte, lus par le pilote graphique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pci {
    pub vendor: u16,
    pub device: u16,
    pub subvendor: u16,
    pub subdevice: u16,
}

pub const NVIDIA: u16 = 0x10DE;

fn parse(text: &str) -> Vec<Entry> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let hex = |s: &str| u16::from_str_radix(s, 16).ok();
            Some(Entry {
                family: f.first()?.to_string(),
                device: hex(f.get(1)?)?,
                subvendor: hex(f.get(2)?)?,
                subdevice: hex(f.get(3)?)?,
                addr: u8::from_str_radix(f.get(4)?, 16).ok()?,
                name: f.get(5)?.to_string(),
            })
        })
        .collect()
}

fn entries() -> &'static [Entry] {
    static E: OnceLock<Vec<Entry>> = OnceLock::new();
    E.get_or_init(|| parse(include_str!("../../../assets/gpu_whitelist.tsv")))
}

/// Entrée de la liste blanche pour cette carte, `None` = carte inconnue (aucun accès).
pub fn find(pci: Pci) -> Option<&'static Entry> {
    if pci.vendor != NVIDIA {
        return None;
    }
    entries().iter().find(|e| e.device == pci.device && e.subvendor == pci.subvendor && e.subdevice == pci.subdevice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_card_matches_and_unknown_does_not() {
        // ROG Strix GTX 1080 Ti Gaming (source : CSV, ligne 1) : adresse 0x29.
        let e = find(Pci { vendor: NVIDIA, device: 0x1B06, subvendor: 0x1043, subdevice: 0x85EA }).expect("carte listée");
        assert_eq!((e.family.as_str(), e.addr), ("aura_gpu", 0x29));
        assert!(find(Pci { vendor: NVIDIA, device: 0x1B06, subvendor: 0x1043, subdevice: 0x0000 }).is_none());
        assert!(find(Pci { vendor: 0x1002, device: 0x1B06, subvendor: 0x1043, subdevice: 0x85EA }).is_none());
    }

    #[test]
    fn every_entry_is_well_formed() {
        assert!(entries().len() >= 140);
        assert!(entries().iter().all(|e| matches!(e.family.as_str(), "aura_gpu" | "msi_v1" | "gigabyte_gen1" | "ene_gpu" | "pny_lovelace" | "gigabyte_gen2" | "evga_turing" | "evga_ampere") && e.addr < 0x80));
    }
}
