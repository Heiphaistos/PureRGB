//! Source : docs/rgb-protocols/gpu-acces-i2c-nvapi-adl.md et gpu-controleurs-par-constructeur.md.
//!
//! Backend « gpu » : éclairage des cartes graphiques NVIDIA en natif, sans OpenRGB (bus I2C de la carte via NVAPI).
//! Actif seulement si les pilotes natifs sont activés ET si la carte figure dans la liste blanche PCI (`whitelist.rs`).
//! Écriture réservée au réglage « écriture sur le bus » (comme la mémoire) : sinon la carte est listée, non pilotable.
//! Première livraison : ASUS Aura GPU (hérité, signature vérifiée avant toute écriture) et MSI Mystic Light v1.
//! Une couleur par carte (toutes les LEDs identiques) ; jamais d'enregistrement permanent dans la mémoire de la carte.

mod aura;
mod msi;
#[cfg(windows)]
mod nvapi;
mod whitelist;

use super::smbus::bus::SmbusBus;
use super::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, ZoneInfo};
use anyhow::{bail, Context, Result};
use std::time::{Duration, Instant};
use whitelist::Pci;

/// Pas d'écriture plus rapide : le bus I2C d'une carte est lent et partagé avec l'affichage.
const MIN_INTERVAL: Duration = Duration::from_millis(80);
const NOTE: &str = "driver natif expérimental (bus I2C de la carte) : fermer le logiciel RGB du constructeur";

pub type Source = Box<dyn Fn() -> Vec<(Pci, Box<dyn SmbusBus>)> + Send>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Family {
    Aura,
    MsiV1,
}

struct Card {
    id: String,
    info: DeviceInfo,
    family: Family,
    addr: u8,
    bus: Box<dyn SmbusBus>,
    initialized: bool,
    last: Option<(Color, Instant)>,
}

pub struct GpuBackend {
    enabled: bool,
    writes: bool,
    source: Source,
    cards: Vec<Card>,
    scanned: bool,
}

#[cfg(windows)]
fn default_source() -> Source {
    Box::new(nvapi::enumerate)
}

#[cfg(not(windows))]
fn default_source() -> Source {
    Box::new(Vec::new)
}

impl GpuBackend {
    pub fn new(enabled: bool, writes: bool) -> Self {
        Self::with_source(enabled, writes, default_source())
    }

    fn with_source(enabled: bool, writes: bool, source: Source) -> Self {
        GpuBackend { enabled, writes, source, cards: Vec::new(), scanned: false }
    }

    pub fn set_native_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.cards.clear();
            self.scanned = false;
        }
    }

    pub fn set_writes_enabled(&mut self, on: bool) {
        if self.writes != on {
            self.writes = on;
            self.cards.clear();
            self.scanned = false;
        }
    }

    fn detect(&mut self) {
        self.scanned = true;
        self.cards.clear();
        for (index, (pci, mut bus)) in (self.source)().into_iter().enumerate() {
            let Some(entry) = whitelist::find(pci) else { continue };
            let family = match entry.family.as_str() {
                "aura_gpu" => Family::Aura,
                "msi_v1" => Family::MsiV1,
                _ => continue,
            };
            if family == Family::Aura {
                if let Err(e) = aura::verify(bus.as_mut(), entry.addr) {
                    log::warn!("gpu {} : {e:#}", entry.name);
                    continue;
                }
            }
            let id = format!("{index}-{:02x}", entry.addr);
            let vendor = entry.name.split_whitespace().next().unwrap_or("GPU").to_string();
            let info = DeviceInfo {
                id: id.clone(),
                name: entry.name.clone(),
                vendor,
                backend: String::new(),
                device_type: DeviceType::Gpu,
                zones: vec![ZoneInfo { name: "Carte".into(), led_count: 1, zone_type: 0, leds_min: 1, leds_max: 1 }],
                led_count: 1,
                fan_channels: Vec::new(),
                settings: Vec::new(),
                controllable: self.writes,
                has_lcd: false,
                modes: Vec::new(),
                active_mode: -1,
                note: if self.writes {
                    format!("{NOTE} — adresse 0x{:02X}", entry.addr)
                } else {
                    format!("détectée (adresse 0x{:02X}) : pour la piloter, autorisez l'écriture sur le bus dans Réglages (expérimental)", entry.addr)
                },
            };
            self.cards.push(Card { id, info, family, addr: entry.addr, bus, initialized: false, last: None });
        }
    }
}

impl Backend for GpuBackend {
    fn name(&self) -> &'static str {
        "gpu"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        if !self.scanned {
            self.detect();
        }
        Ok(self.cards.iter().map(|c| c.info.clone()).collect())
    }

    fn set_colors(&mut self, local_id: &str, colors: &[Color]) -> Result<()> {
        if !self.enabled {
            bail!("drivers natifs désactivés — activer dans Réglages ou passer par OpenRGB");
        }
        if !self.writes {
            bail!("écriture sur le bus de la carte non autorisée (Réglages, drivers natifs)");
        }
        let card = self.cards.iter_mut().find(|c| c.id == local_id).with_context(|| format!("carte inconnue : {local_id}"))?;
        let color = colors.first().copied().unwrap_or_default();
        if let Some((last, at)) = card.last {
            let same = last.r == color.r && last.g == color.g && last.b == color.b;
            if (same && at.elapsed() < Duration::from_secs(5)) || at.elapsed() < MIN_INTERVAL {
                return Ok(());
            }
        }
        if !card.initialized {
            match card.family {
                Family::Aura => aura::init(card.bus.as_mut(), card.addr)?,
                Family::MsiV1 => msi::init(card.bus.as_mut(), card.addr)?,
            }
            card.initialized = true;
        }
        match card.family {
            Family::Aura => aura::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::MsiV1 => msi::set_color(card.bus.as_mut(), card.addr, color)?,
        }
        card.last = Some((color, Instant::now()));
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.enabled && !self.cards.is_empty()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::smbus::bus::{MockBus, Txn};
    use std::sync::{Arc, Mutex};

    struct Shared(Arc<Mutex<MockBus>>);
    impl SmbusBus for Shared {
        fn read_byte_data(&mut self, a: u8, r: u8) -> Result<u8> {
            self.0.lock().expect("verrou").read_byte_data(a, r)
        }
        fn write_byte_data(&mut self, a: u8, r: u8, v: u8) -> Result<()> {
            self.0.lock().expect("verrou").write_byte_data(a, r, v)
        }
        fn write_block_data(&mut self, a: u8, r: u8, d: &[u8]) -> Result<()> {
            self.0.lock().expect("verrou").write_block_data(a, r, d)
        }
        fn pause(&mut self, _: u64) {}
    }

    const ASUS_1080TI: Pci = Pci { vendor: 0x10DE, device: 0x1B06, subvendor: 0x1043, subdevice: 0x85EA };
    const UNKNOWN: Pci = Pci { vendor: 0x10DE, device: 0x2204, subvendor: 0x1234, subdevice: 0x5678 };

    fn backend(enabled: bool, writes: bool, magic_ok: bool) -> (GpuBackend, Arc<Mutex<MockBus>>) {
        let mut m = MockBus::default();
        m.present.insert(0x29);
        m.mem.extend([((0x29, 0x20), 0x15), ((0x29, 0x21), if magic_ok { 0x89 } else { 0 })]);
        let bus = Arc::new(Mutex::new(m));
        let (a, b) = (bus.clone(), bus.clone());
        let src: Source = Box::new(move || vec![(ASUS_1080TI, Box::new(Shared(a.clone())) as Box<dyn SmbusBus>), (UNKNOWN, Box::new(Shared(b.clone())))]);
        (GpuBackend::with_source(enabled, writes, src), bus)
    }

    #[test]
    fn disabled_gives_empty_scan_and_no_traffic() {
        let (mut b, bus) = backend(false, true, true);
        assert!(b.scan().expect("scan").is_empty());
        assert!(bus.lock().expect("verrou").log.is_empty());
        assert!(b.set_colors("0-29", &[Color::default()]).is_err());
    }

    #[test]
    fn only_whitelisted_card_is_exposed_and_unknown_card_is_never_touched() {
        let (mut b, bus) = backend(true, true, true);
        let list = b.scan().expect("scan");
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].device_type, list[0].controllable, list[0].led_count), (DeviceType::Gpu, true, 1));
        // la détection ne fait que lire (signature 0x20/0x21)
        assert!(bus.lock().expect("verrou").writes().is_empty());
    }

    #[test]
    fn wrong_signature_hides_the_card() {
        let (mut b, _) = backend(true, true, false);
        assert!(b.scan().expect("scan").is_empty());
    }

    #[test]
    fn without_write_permission_card_is_listed_but_never_written() {
        let (mut b, bus) = backend(true, false, true);
        let list = b.scan().expect("scan");
        assert!(!list[0].controllable);
        assert!(b.set_colors("0-29", &[Color { r: 1, g: 1, b: 1 }]).is_err());
        assert!(bus.lock().expect("verrou").writes().is_empty());
    }

    #[test]
    fn writes_mode_once_then_color_and_skips_unchanged_frames() {
        let (mut b, bus) = backend(true, true, true);
        b.scan().expect("scan");
        bus.lock().expect("verrou").log.clear();
        b.set_colors("0-29", &[Color { r: 5, g: 6, b: 7 }]).expect("écriture");
        let n = bus.lock().expect("verrou").log.len();
        assert_eq!(bus.lock().expect("verrou").log[0], Txn::Write(0x29, 0x07, 1));
        assert_eq!(n, 4);
        b.set_colors("0-29", &[Color { r: 5, g: 6, b: 7 }]).expect("identique");
        assert_eq!(bus.lock().expect("verrou").log.len(), n, "image identique : aucune écriture");
        assert!(b.set_colors("9-99", &[Color::default()]).is_err());
    }
}
