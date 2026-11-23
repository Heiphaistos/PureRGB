//! Source : docs/rgb-protocols/gpu-acces-i2c-nvapi-adl.md et gpu-controleurs-par-constructeur.md.
//!
//! Backend « gpu » : éclairage des cartes graphiques NVIDIA en natif, sans OpenRGB (bus I2C de la carte via NVAPI).
//! Actif seulement si les pilotes natifs sont activés ET si la carte figure dans la liste blanche PCI (`whitelist.rs`).
//! Écriture réservée au réglage « écriture sur le bus » (comme la mémoire) : sinon la carte est listée, non pilotable.
//! Familles : ASUS Aura GPU (hérité), ASUS ENE (RTX 30/40/50), MSI Mystic Light v1, Gigabyte Gen1 ; signature vérifiée avant toute écriture quand la note en donne une.
//! Une couleur par carte (toutes les LEDs identiques) ; jamais d'enregistrement permanent dans la mémoire de la carte.

mod aura;
mod colorful;
mod ene;
mod epicx;
mod evga;
mod evga3;
mod gigabyte;
mod gigabyte2;
mod gigabyte3;
mod msi;
mod palit;
mod pny;
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
    /// `true` = paquet « v2 » de 122 octets.
    Colorful(bool),
    EneGpu,
    EvgaTuring,
    EvgaAmpere,
    /// `true` = modèle « large » (RTX 5080 / 5090).
    PnyEpicX(bool),
    GigabyteGen1,
    GigabyteGen2,
    GigabyteGen3,
    MsiV1,
    MsiV2,
    PalitV2,
    PnyLovelace,
}

struct Card {
    id: String,
    info: DeviceInfo,
    family: Family,
    addr: u8,
    bus: Box<dyn SmbusBus>,
    initialized: bool,
    last: Option<(Vec<Color>, Instant)>,
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
                "colorful" => Family::Colorful(colorful::is_v2(entry.subdevice)),
                "msi_v1" => Family::MsiV1,
                "msi_v2" => Family::MsiV2,
                "palit_v2" => Family::PalitV2,
                "gigabyte_gen1" => Family::GigabyteGen1,
                "gigabyte_gen2" => Family::GigabyteGen2,
                "gigabyte_gen3" => Family::GigabyteGen3,
                "ene_gpu" => Family::EneGpu,
                "evga_turing" => Family::EvgaTuring,
                "evga_ampere" => Family::EvgaAmpere,
                "pny_epicx" => Family::PnyEpicX(entry.name.contains("5080") || entry.name.contains("5090")),
                "pny_lovelace" => Family::PnyLovelace,
                _ => continue,
            };
            let verified = match family {
                Family::Aura => aura::verify(bus.as_mut(), entry.addr).map(|()| 1),
                Family::Colorful(_) => colorful::verify(bus.as_mut(), entry.addr).map(|()| 1),
                Family::GigabyteGen1 => gigabyte::verify(bus.as_mut(), entry.addr, pci).map(|()| 1),
                Family::EneGpu => ene::verify(bus.as_mut(), entry.addr),
                Family::GigabyteGen3 => Ok(gigabyte3::leds()),
                Family::PalitV2 => palit::verify(bus.as_mut(), entry.addr).map(|()| 1),
                Family::GigabyteGen2 => gigabyte2::verify(bus.as_mut(), entry.addr).map(|()| gigabyte2::ZONES as u8),
                Family::MsiV1 | Family::MsiV2 | Family::EvgaTuring => Ok(1),
                Family::PnyEpicX(large) => Ok(epicx::leds(large)),
                Family::EvgaAmpere => evga3::verify(bus.as_mut(), entry.addr).map(|()| evga3::ZONES as u8),
                Family::PnyLovelace => Ok(pny::LEDS),
            };
            let leds = match verified {
                Ok(n) => u32::from(n),
                Err(e) => {
                    log::warn!("gpu {} : {e:#}", entry.name);
                    continue;
                }
            };
            let id = format!("{index}-{:02x}", entry.addr);
            let vendor = entry.name.split_whitespace().next().unwrap_or("GPU").to_string();
            let info = DeviceInfo {
                id: id.clone(),
                name: entry.name.clone(),
                vendor,
                backend: String::new(),
                device_type: DeviceType::Gpu,
                zones: vec![ZoneInfo { name: "Carte".into(), led_count: leds, zone_type: 0, leds_min: leds, leds_max: leds }],
                led_count: leds,
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
        let frame: Vec<Color> = colors.iter().copied().take(card.info.led_count as usize).collect();
        if let Some((last, at)) = &card.last {
            if (*last == frame && at.elapsed() < Duration::from_secs(5)) || at.elapsed() < MIN_INTERVAL {
                return Ok(());
            }
        }
        if !card.initialized {
            match card.family {
                Family::Aura => aura::init(card.bus.as_mut(), card.addr)?,
                Family::MsiV1 => msi::init(card.bus.as_mut(), card.addr)?,
                Family::GigabyteGen1 => gigabyte::init(card.bus.as_mut(), card.addr)?,
                Family::EneGpu => ene::init(card.bus.as_mut(), card.addr)?,
                Family::PnyLovelace | Family::GigabyteGen2 | Family::EvgaTuring | Family::MsiV2 | Family::PalitV2 | Family::Colorful(_) => {}
                Family::GigabyteGen3 => gigabyte3::init(card.bus.as_mut(), card.addr)?,
                Family::EvgaAmpere => evga3::init(card.bus.as_mut(), card.addr)?,
                Family::PnyEpicX(large) => epicx::init(card.bus.as_mut(), card.addr, large)?,
            }
            card.initialized = true;
        }
        match card.family {
            Family::Aura => aura::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::MsiV1 => msi::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::MsiV2 => msi::set_color_v2(card.bus.as_mut(), card.addr, color)?,
            Family::GigabyteGen3 => gigabyte3::set_colors(card.bus.as_mut(), card.addr, &frame)?,
            Family::PalitV2 => palit::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::Colorful(v2) => colorful::set_color(card.bus.as_mut(), card.addr, v2, color)?,
            Family::GigabyteGen1 => gigabyte::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::EneGpu => ene::set_colors(card.bus.as_mut(), card.addr, &frame)?,
            Family::PnyLovelace => pny::set_colors(card.bus.as_mut(), card.addr, &frame)?,
            Family::EvgaTuring => evga::set_color(card.bus.as_mut(), card.addr, color)?,
            Family::EvgaAmpere => evga3::set_colors(card.bus.as_mut(), card.addr, &frame)?,
            Family::PnyEpicX(large) => epicx::set_colors(card.bus.as_mut(), card.addr, large, &frame)?,
            Family::GigabyteGen2 => gigabyte2::set_colors(card.bus.as_mut(), card.addr, &frame)?,
        }
        card.last = Some((frame, Instant::now()));
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
