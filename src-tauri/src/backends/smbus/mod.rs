//! Source : docs/rgb-protocols/smbus-i2c-acces-et-securite.md et docs/rgb-protocols/corsair-vengeance-dram.md.
//!
//! Backend « smbus » : RAM RGB Corsair (famille moderne) en natif, sans OpenRGB. Actif uniquement si le réglage
//! `native_drivers_enabled` est vrai, sous Windows, avec PawnIO prêt. Sinon : scan vide, sans erreur.
//! La détection n'a lieu qu'une fois par session tant qu'elle trouve quelque chose (« mémoriser l'adresse, ne plus sonder »).

mod bus;
mod corsair_dram;
mod detect;
#[cfg(windows)]
mod pawnio;

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, ZoneInfo};
use anyhow::{bail, Context, Result};
use bus::SmbusBus;
use corsair_dram::CorsairDram;

const NOTE: &str = "driver natif expérimental (SMBus, mode direct) : ne pas lancer iCUE en même temps";

struct Dimm {
    id: String,
    info: DeviceInfo,
    driver: CorsairDram,
}

pub struct SmbusBackend {
    enabled: bool,
    /// Écriture sur la mémoire autorisée (réglage séparé, désactivé par défaut) : sinon détection en lecture seule.
    writes: bool,
    bus: Option<Box<dyn SmbusBus>>,
    dimms: Vec<Dimm>,
    /// Adresses vues en lecture seule (écriture non autorisée).
    seen: Vec<u8>,
}

impl SmbusBackend {
    pub fn new(enabled: bool, writes: bool) -> Self {
        SmbusBackend { enabled, writes, bus: None, dimms: Vec::new(), seen: Vec::new() }
    }

    pub fn set_writes_enabled(&mut self, on: bool) {
        if self.writes != on {
            self.writes = on;
            self.dimms.clear();
            self.seen.clear();
        }
    }

    pub fn set_native_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.dimms.clear();
            self.seen.clear();
            self.bus = None;
        }
    }

    #[cfg(windows)]
    fn open_bus() -> Option<Box<dyn SmbusBus>> {
        if !crate::backends::openrgb::manager::OpenRgbManager::pawnio_ready() {
            return None;
        }
        match pawnio::PawnioBus::open() {
            Ok(b) => Some(Box::new(b)),
            Err(e) => {
                log::warn!("smbus : PawnIO inutilisable : {e:#}");
                None
            }
        }
    }

    #[cfg(not(windows))]
    fn open_bus() -> Option<Box<dyn SmbusBus>> {
        None
    }

    /// Lecture seule : les barrettes sont listées comme détectées mais non pilotables.
    fn detect_read_only(&mut self) -> Result<()> {
        let bus = self.bus.as_mut().context("bus SMBus fermé")?;
        self.seen = detect::scan_signatures(bus.as_mut());
        Ok(())
    }

    fn read_only_info(addr: u8) -> DeviceInfo {
        DeviceInfo {
            id: format!("{addr:02x}"),
            name: "Mémoire Corsair RGB (détectée)".into(),
            vendor: "Corsair".into(),
            backend: String::new(),
            device_type: DeviceType::Dram,
            zones: Vec::new(),
            led_count: 0,
            fan_channels: Vec::new(),
            controllable: false,
            has_lcd: false,
            modes: Vec::new(),
            active_mode: -1,
            note: format!("détectée à l'adresse 0x{addr:02X} en lecture seule : pour la piloter, autorisez l'écriture sur la mémoire dans Réglages (expérimental)"),
        }
    }

    fn detect(&mut self) -> Result<()> {
        let bus = self.bus.as_mut().context("bus SMBus fermé")?;
        self.dimms = detect::scan(bus.as_mut())
            .into_iter()
            .filter_map(|f| {
                let driver = match CorsairDram::new(f.addr, &f.info, f.model) {
                    Ok(d) => d,
                    Err(e) => {
                        log::warn!("smbus 0x{:02X} : {e:#}", f.addr);
                        return None;
                    }
                };
                let leds = u32::from(f.model.leds);
                let id = format!("{:02x}", f.addr);
                let info = DeviceInfo {
                    id: id.clone(),
                    name: format!("Corsair {}", f.model.name),
                    vendor: "Corsair".into(),
                    backend: String::new(),
                    device_type: DeviceType::Dram,
                    zones: vec![ZoneInfo { name: "Barrette".into(), led_count: leds, zone_type: 1, leds_min: leds, leds_max: leds }],
                    led_count: leds,
                    fan_channels: Vec::new(),
                    controllable: true,
                    has_lcd: false,
                    modes: Vec::new(),
                    active_mode: -1,
                    note: format!("{NOTE} — adresse 0x{:02X}, firmware {}.{}", f.addr, f.info.fw_major, f.info.fw_minor),
                };
                Some(Dimm { id, info, driver })
            })
            .collect();
        Ok(())
    }
}

impl Backend for SmbusBackend {
    fn name(&self) -> &'static str {
        "smbus"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        if self.dimms.is_empty() && self.seen.is_empty() {
            if self.bus.is_none() {
                self.bus = Self::open_bus();
            }
            if self.bus.is_none() {
                return Ok(Vec::new());
            }
            if self.writes {
                self.detect()?;
            } else {
                self.detect_read_only()?;
            }
        }
        let mut out: Vec<DeviceInfo> = self.dimms.iter().map(|d| d.info.clone()).collect();
        out.extend(self.seen.iter().map(|a| Self::read_only_info(*a)));
        Ok(out)
    }

    fn set_colors(&mut self, local_id: &str, colors: &[Color]) -> Result<()> {
        if !self.enabled {
            bail!("drivers natifs désactivés — activer dans Réglages ou passer par OpenRGB");
        }
        if !self.writes {
            bail!("écriture sur la mémoire non autorisée (Réglages, drivers natifs) : détection en lecture seule");
        }
        let bus = self.bus.as_mut().context("bus SMBus fermé")?;
        let dimm = self.dimms.iter_mut().find(|d| d.id == local_id).with_context(|| format!("barrette inconnue : {local_id}"))?;
        dimm.driver.set_colors(bus.as_mut(), colors)
    }

    fn is_available(&self) -> bool {
        self.enabled && self.bus.is_some()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::{MockBus, Txn};
    use std::sync::{Arc, Mutex};

    /// Bus partagé pour inspecter le journal depuis le test.
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

    fn backend(enabled: bool) -> (SmbusBackend, Arc<Mutex<MockBus>>) {
        let mut m = MockBus::default();
        m.add_dimm(0x58, 0x0900, 4, 0x84);
        m.add_dimm(0x59, 0x0900, 4, 0x84);
        let shared = Arc::new(Mutex::new(m));
        let mut b = SmbusBackend::new(enabled, true);
        b.bus = Some(Box::new(Shared(shared.clone())));
        (b, shared)
    }

    #[test]
    fn disabled_setting_gives_empty_scan_and_no_bus_traffic() {
        let (mut b, bus) = backend(false);
        assert!(b.scan().expect("scan").is_empty());
        assert!(bus.lock().expect("verrou").log.is_empty());
        assert!(b.set_colors("58", &[Color::default(); 10]).is_err());
        assert!(!b.is_available());
    }

    #[test]
    fn enabled_scan_exposes_both_dimms_as_dram() {
        let (mut b, _) = backend(true);
        let list = b.scan().expect("scan");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "58");
        assert_eq!(list[0].name, "Corsair Vengeance RGB DDR5");
        assert_eq!((list[0].device_type, list[0].led_count, list[0].controllable), (DeviceType::Dram, 10, true));
        assert!(list[0].note.contains("natif expérimental"));
        assert!(b.is_available());
    }

    #[test]
    fn set_colors_writes_to_the_right_dimm_and_rejects_unknown_ids() {
        let (mut b, bus) = backend(true);
        b.scan().expect("scan");
        bus.lock().expect("verrou").log.clear();
        b.set_colors("59", &[Color { r: 255, g: 0, b: 0 }; 10]).expect("écriture");
        let log = bus.lock().expect("verrou").log.clone();
        assert!(matches!(log.as_slice(), [Txn::Block(0x59, 0x31, p)] if p.len() == 32 && p[0] == 10));
        assert!(b.set_colors("5a", &[Color::default(); 10]).is_err());
    }

    #[test]
    fn without_write_permission_the_memory_is_listed_but_never_written() {
        let (mut b, bus) = backend(true);
        b.set_writes_enabled(false);
        let list = b.scan().expect("scan");
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|d| !d.controllable && d.led_count == 0));
        assert!(bus.lock().expect("verrou").writes().is_empty(), "aucune écriture, pas même les sélecteurs d'identité");
        assert!(b.set_colors("58", &[Color::default(); 10]).is_err());
        assert!(bus.lock().expect("verrou").writes().is_empty());
    }

    #[test]
    fn disabling_the_setting_drops_devices() {
        let (mut b, _) = backend(true);
        b.scan().expect("scan");
        b.set_native_enabled(false);
        assert!(b.scan().expect("scan").is_empty());
    }
}
