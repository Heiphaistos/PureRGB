use crate::backends::Backend;
use crate::core::{Color, DeviceInfo};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Registre central : agrège les appareils de tous les backends
/// sous des ids globaux "<backend>:<id local>".
pub struct DeviceRegistry {
    backends: Vec<Box<dyn Backend>>,
    devices: HashMap<String, DeviceInfo>,
}

/// Fabricant normalisé pour comparer « Micro-Star International » (Windows) à « MSI » (OpenRGB).
fn maker(vendor: &str, name: &str) -> String {
    let text = format!("{vendor} {name}").to_lowercase();
    for (needle, key) in [("micro-star", "msi"), ("msi", "msi"), ("asus", "asus"), ("gigabyte", "gigabyte"), ("aorus", "gigabyte"), ("asrock", "asrock")] {
        if text.contains(needle) {
            return key.into();
        }
    }
    vendor.split_whitespace().next().unwrap_or("").to_lowercase()
}

pub type SharedRegistry = Arc<Mutex<DeviceRegistry>>;

impl DeviceRegistry {
    pub fn new(backends: Vec<Box<dyn Backend>>) -> Self {
        DeviceRegistry {
            backends,
            devices: HashMap::new(),
        }
    }

    pub fn shared(backends: Vec<Box<dyn Backend>>) -> SharedRegistry {
        Arc::new(Mutex::new(Self::new(backends)))
    }

    /// Re-scanne tous les backends. Les erreurs d'un backend n'empêchent pas les autres.
    pub fn scan_all(&mut self) -> Vec<DeviceInfo> {
        self.devices.clear();
        for backend in &mut self.backends {
            let bname = backend.name();
            match backend.scan() {
                Ok(devs) => {
                    log::info!("scan backend {bname}: {} appareil(s)", devs.len());
                    for mut d in devs {
                        d.backend = bname.to_string();
                        d.id = format!("{}:{}", bname, d.id);
                        self.devices.insert(d.id.clone(), d);
                    }
                }
                Err(e) => log::warn!("scan backend {bname} en échec: {e:#}"),
            }
        }
        self.drop_duplicates();
        self.device_list()
    }

    /// Un même appareil vu par deux backends n'est affiché qu'une fois : le driver natif activé l'emporte sur
    /// la copie OpenRGB, et une simple ligne « vu en USB » disparaît dès qu'un autre backend pilote l'appareil.
    fn drop_duplicates(&mut self) {
        let key = |d: &DeviceInfo| format!("{}|{:?}", maker(&d.vendor, &d.name), d.device_type);
        let mut natives: HashMap<String, usize> = HashMap::new();
        for d in self.devices.values().filter(|d| (d.backend == "hid" || d.backend == "smbus") && d.controllable && d.note.contains("natif expérimental")) {
            *natives.entry(key(d)).or_default() += 1;
        }
        let mut ids: Vec<String> = self.devices.values().filter(|d| d.backend == "openrgb").map(|d| d.id.clone()).collect();
        ids.sort();
        for id in ids {
            let k = key(&self.devices[&id]);
            if let Some(n) = natives.get_mut(&k).filter(|n| **n > 0) {
                *n -= 1;
                self.devices.remove(&id);
            }
        }
        let driven: Vec<String> = self.devices.values().filter(|d| d.controllable).map(key).collect();
        self.devices.retain(|_, d| d.backend != "hid" || d.controllable || !driven.contains(&key(d)));
    }

    pub fn device_list(&self) -> Vec<DeviceInfo> {
        let mut list: Vec<DeviceInfo> = self.devices.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub fn get(&self, id: &str) -> Option<&DeviceInfo> {
        self.devices.get(id)
    }

    fn split_id<'a>(&self, id: &'a str) -> Result<(&'a str, &'a str)> {
        id.split_once(':')
            .ok_or_else(|| anyhow!("id d'appareil invalide: {id}"))
    }

    pub fn set_colors(&mut self, id: &str, colors: &[Color]) -> Result<()> {
        let (bname, local) = self.split_id(id)?;
        let (bname, local) = (bname.to_string(), local.to_string());
        let backend = self
            .backends
            .iter_mut()
            .find(|b| b.name() == bname)
            .ok_or_else(|| anyhow!("backend inconnu: {bname}"))?;
        backend.set_colors(&local, colors)
    }

    pub fn set_fan_duty(&mut self, id: &str, channel: u8, percent: u8) -> Result<()> {
        let (bname, local) = self.split_id(id)?;
        let (bname, local) = (bname.to_string(), local.to_string());
        let backend = self
            .backends
            .iter_mut()
            .find(|b| b.name() == bname)
            .ok_or_else(|| anyhow!("backend inconnu: {bname}"))?;
        backend.set_fan_duty(&local, channel, percent.min(100))
    }

    pub fn backend_status(&self) -> Vec<(String, bool)> {
        self.backends
            .iter()
            .map(|b| (b.name().to_string(), b.is_available()))
            .collect()
    }

    pub fn backends_mut(&mut self) -> &mut Vec<Box<dyn Backend>> {
        &mut self.backends
    }
}

#[cfg(test)]
mod dedup_tests {
    use super::*;
    use crate::core::DeviceType;

    struct Fake(&'static str, Vec<DeviceInfo>);

    impl Backend for Fake {
        fn name(&self) -> &'static str {
            self.0
        }
        fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
            Ok(self.1.clone())
        }
        fn set_colors(&mut self, _: &str, _: &[Color]) -> Result<()> {
            Ok(())
        }
        fn is_available(&self) -> bool {
            true
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    fn dev(id: &str, vendor: &str, name: &str, t: DeviceType, controllable: bool, note: &str) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: name.into(),
            vendor: vendor.into(),
            backend: String::new(),
            device_type: t,
            zones: Vec::new(),
            led_count: 0,
            fan_channels: Vec::new(),
            controllable,
            has_lcd: false,
            modes: Vec::new(),
            active_mode: -1,
            note: note.into(),
        }
    }

    fn scan(openrgb: Vec<DeviceInfo>, hid: Vec<DeviceInfo>) -> Vec<String> {
        let mut r = DeviceRegistry::new(vec![Box::new(Fake("openrgb", openrgb)), Box::new(Fake("hid", hid))]);
        r.scan_all().into_iter().map(|d| d.id).collect()
    }

    #[test]
    fn enabled_native_driver_replaces_the_openrgb_copy_of_the_same_board() {
        let orgb = dev("1", "MSI", "MSI MAG Z790", DeviceType::Motherboard, true, "");
        let nat = dev("1462:7d93:0", "Micro-Star", "MSI Mystic Light", DeviceType::Motherboard, true, "driver natif expérimental");
        let other = dev("2", "Corsair", "Vengeance", DeviceType::Dram, true, "");
        assert_eq!(scan(vec![orgb, other], vec![nat]), vec!["hid:1462:7d93:0", "openrgb:2"]);
    }

    #[test]
    fn native_smbus_dram_replaces_the_openrgb_copy() {
        let orgb = dev("3", "Corsair", "Corsair Vengeance RGB", DeviceType::Dram, true, "");
        let nat = dev("58", "Corsair", "Corsair Vengeance RGB DDR5", DeviceType::Dram, true, "driver natif expérimental");
        let mut r = DeviceRegistry::new(vec![Box::new(Fake("openrgb", vec![orgb])), Box::new(Fake("smbus", vec![nat]))]);
        let ids: Vec<String> = r.scan_all().into_iter().map(|d| d.id).collect();
        assert_eq!(ids, vec!["smbus:58"]);
    }

    #[test]
    fn usb_info_line_disappears_when_openrgb_drives_the_same_device() {
        let orgb = dev("1", "MSI", "MSI MAG Z790", DeviceType::Motherboard, true, "");
        let info = dev("1462:7d93:0", "MSI", "MSI Mystic Light", DeviceType::Motherboard, false, "vu en USB (info)");
        let lone = dev("046d:c539:0", "Logitech", "Appareil Logitech", DeviceType::Mouse, false, "vu en USB (info)");
        assert_eq!(scan(vec![orgb], vec![info, lone]), vec!["hid:046d:c539:0", "openrgb:1"]);
    }

    #[test]
    fn two_native_boards_hide_at_most_two_openrgb_copies() {
        let o = |i: &str| dev(i, "ASUS", "ASUS board", DeviceType::Motherboard, true, "");
        let n = |i: &str| dev(i, "ASUSTeK", "ASUS Aura", DeviceType::Motherboard, true, "driver natif expérimental");
        assert_eq!(scan(vec![o("1"), o("2"), o("3")], vec![n("a")]).len(), 3);
    }
}
