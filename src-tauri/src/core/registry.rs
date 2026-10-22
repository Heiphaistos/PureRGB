use crate::backends::Backend;
use crate::core::{Color, DeviceInfo};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Échecs consécutifs d'un pilote intégré avant de passer automatiquement sur OpenRGB.
const FALLBACK_AFTER_FAILURES: u32 = 3;

/// Registre central : agrège les appareils de tous les backends
/// sous des ids globaux "<backend>:<id local>".
pub struct DeviceRegistry {
    backends: Vec<Box<dyn Backend>>,
    devices: HashMap<String, DeviceInfo>,
    /// Appareil piloté en natif -> id de sa copie OpenRGB (masquée), prête à prendre le relais.
    fallbacks: HashMap<String, String>,
    /// Appareils dont le pilote intégré a échoué : ils passent par OpenRGB jusqu'au prochain scan.
    degraded: HashSet<String>,
    failures: HashMap<String, u32>,
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
            fallbacks: HashMap::new(),
            degraded: HashSet::new(),
            failures: HashMap::new(),
        }
    }

    pub fn shared(backends: Vec<Box<dyn Backend>>) -> SharedRegistry {
        Arc::new(Mutex::new(Self::new(backends)))
    }

    /// Re-scanne tous les backends. Les erreurs d'un backend n'empêchent pas les autres.
    pub fn scan_all(&mut self) -> Vec<DeviceInfo> {
        self.devices.clear();
        self.fallbacks.clear();
        self.degraded.clear();
        self.failures.clear();
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

    /// Un même appareil vu par deux backends n'est affiché qu'une fois. Le pilote intégré activé est le pilote
    /// principal ; la copie OpenRGB est masquée mais gardée comme repli (voir `set_colors`). Une simple ligne
    /// « vu en USB » disparaît dès qu'un autre backend pilote l'appareil.
    fn drop_duplicates(&mut self) {
        let key = |d: &DeviceInfo| format!("{}|{:?}", maker(&d.vendor, &d.name), d.device_type);
        let mut natives: HashMap<String, Vec<String>> = HashMap::new();
        let mut native_ids: Vec<&DeviceInfo> = self
            .devices
            .values()
            .filter(|d| (d.backend == "hid" || d.backend == "smbus") && d.controllable && d.note.contains("natif expérimental"))
            .collect();
        native_ids.sort_by(|a, b| a.id.cmp(&b.id));
        for d in native_ids {
            natives.entry(key(d)).or_default().push(d.id.clone());
        }
        let mut ids: Vec<String> = self.devices.values().filter(|d| d.backend == "openrgb").map(|d| d.id.clone()).collect();
        ids.sort();
        for id in ids {
            let k = key(&self.devices[&id]);
            if let Some(primary) = natives.get_mut(&k).filter(|v| !v.is_empty()).map(|v| v.remove(0)) {
                // La copie OpenRGB reste interrogeable par son id (backend "openrgb"), mais n'est plus listée.
                self.devices.remove(&id);
                self.fallbacks.insert(primary, id);
            }
        }
        let driven: Vec<String> = self.devices.values().filter(|d| d.controllable).map(key).collect();
        self.devices.retain(|_, d| d.backend != "hid" || d.controllable || !driven.contains(&key(d)));
    }

    /// Bascule définitivement (jusqu'au prochain scan) un appareil sur sa copie OpenRGB et le dit dans sa note.
    fn degrade(&mut self, id: &str, why: &anyhow::Error) {
        self.degraded.insert(id.to_string());
        log::warn!("{id} : pilote intégré en échec ({why:#}), repli automatique sur OpenRGB");
        if let Some(d) = self.devices.get_mut(id) {
            d.note = format!("repli automatique sur OpenRGB (le pilote intégré a échoué : {why:#})");
        }
    }

    fn colors_via(&mut self, id: &str, colors: &[Color]) -> Result<()> {
        let (bname, local) = self.split_id(id)?;
        let (bname, local) = (bname.to_string(), local.to_string());
        let backend = self
            .backends
            .iter_mut()
            .find(|b| b.name() == bname)
            .ok_or_else(|| anyhow!("backend inconnu: {bname}"))?;
        backend.set_colors(&local, colors)
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

    /// Envoie les couleurs : au pilote intégré d'abord ; après `FALLBACK_AFTER_FAILURES` échecs de suite, à la
    /// copie OpenRGB du même appareil (un seul des deux écrit à la fois, jamais les deux).
    pub fn set_colors(&mut self, id: &str, colors: &[Color]) -> Result<()> {
        let fallback = self.fallbacks.get(id).cloned();
        if let Some(fb) = fallback.as_ref().filter(|_| self.degraded.contains(id)) {
            return self.colors_via(fb, colors);
        }
        match self.colors_via(id, colors) {
            Ok(()) => {
                self.failures.remove(id);
                Ok(())
            }
            Err(e) => {
                let Some(fb) = fallback else { return Err(e) };
                let n = self.failures.entry(id.to_string()).or_default();
                *n += 1;
                if *n < FALLBACK_AFTER_FAILURES {
                    return Err(e);
                }
                self.degrade(id, &e);
                self.colors_via(&fb, colors)
            }
        }
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

    use std::sync::atomic::{AtomicBool, Ordering};

    /// Faux backend : `fail` fait échouer set_colors ; `calls` journalise les appels reçus.
    struct Fake(&'static str, Vec<DeviceInfo>, Arc<AtomicBool>, Arc<Mutex<Vec<String>>>);

    fn fake(name: &'static str, devs: Vec<DeviceInfo>) -> Fake {
        Fake(name, devs, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Vec::new())))
    }

    impl Backend for Fake {
        fn name(&self) -> &'static str {
            self.0
        }
        fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
            Ok(self.1.clone())
        }
        fn set_colors(&mut self, local: &str, _: &[Color]) -> Result<()> {
            self.3.lock().push(format!("{}:{local}", self.0));
            if self.2.load(Ordering::SeqCst) {
                anyhow::bail!("panne simulée");
            }
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
        let mut r = DeviceRegistry::new(vec![Box::new(fake("openrgb", openrgb)), Box::new(fake("hid", hid))]);
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
        let mut r = DeviceRegistry::new(vec![Box::new(fake("openrgb", vec![orgb])), Box::new(fake("smbus", vec![nat]))]);
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

    /// Registre avec une carte mère vue par le pilote intégré (« hid ») ET par OpenRGB ; renvoie le registre,
    /// le drapeau de panne du pilote intégré et les journaux d'appels (hid, openrgb).
    fn merged() -> (DeviceRegistry, Arc<AtomicBool>, Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) {
        let orgb = dev("1", "MSI", "MSI MAG Z790", DeviceType::Motherboard, true, "");
        let nat = dev("1462:7d93:0", "Micro-Star", "MSI Mystic Light", DeviceType::Motherboard, true, "driver natif expérimental");
        let (o, h) = (fake("openrgb", vec![orgb]), fake("hid", vec![nat]));
        let (hid_fail, hid_calls, orgb_calls) = (h.2.clone(), h.3.clone(), o.3.clone());
        let mut r = DeviceRegistry::new(vec![Box::new(o), Box::new(h)]);
        assert_eq!(r.scan_all().len(), 1, "une seule carte affichée");
        (r, hid_fail, hid_calls, orgb_calls)
    }

    #[test]
    fn healthy_native_driver_is_used_and_openrgb_never_written() {
        let (mut r, _, hid, orgb) = merged();
        for _ in 0..5 {
            r.set_colors("hid:1462:7d93:0", &[Color::default()]).unwrap();
        }
        assert_eq!(hid.lock().len(), 5);
        assert!(orgb.lock().is_empty());
    }

    #[test]
    fn three_native_failures_switch_to_openrgb_for_good_and_stop_writing_natively() {
        let (mut r, fail, hid, orgb) = merged();
        let id = "hid:1462:7d93:0";
        fail.store(true, Ordering::SeqCst);
        assert!(r.set_colors(id, &[Color::default()]).is_err());
        assert!(r.set_colors(id, &[Color::default()]).is_err());
        assert!(r.set_colors(id, &[Color::default()]).is_ok(), "3e échec : repli immédiat sur OpenRGB");
        fail.store(false, Ordering::SeqCst); // même réparé, on n'écrit plus en natif : pas de conflit
        r.set_colors(id, &[Color::default()]).unwrap();
        assert_eq!(hid.lock().len(), 3);
        assert_eq!(orgb.lock().as_slice(), ["openrgb:1", "openrgb:1"]);
        assert!(r.get(id).unwrap().note.contains("repli automatique sur OpenRGB"));
        // Un nouveau scan redonne sa chance au pilote intégré.
        r.scan_all();
        r.set_colors(id, &[Color::default()]).unwrap();
        assert_eq!(hid.lock().len(), 4);
    }

    #[test]
    fn a_success_resets_the_failure_count() {
        let (mut r, fail, _, orgb) = merged();
        let id = "hid:1462:7d93:0";
        for _ in 0..2 {
            fail.store(true, Ordering::SeqCst);
            assert!(r.set_colors(id, &[Color::default()]).is_err());
            assert!(r.set_colors(id, &[Color::default()]).is_err());
            fail.store(false, Ordering::SeqCst);
            r.set_colors(id, &[Color::default()]).unwrap();
        }
        assert!(orgb.lock().is_empty(), "jamais 3 échecs de suite : pas de repli");
    }

    #[test]
    fn without_an_openrgb_copy_the_error_is_simply_returned() {
        let nat = dev("1462:7d93:0", "Micro-Star", "MSI Mystic Light", DeviceType::Motherboard, true, "driver natif expérimental");
        let h = fake("hid", vec![nat]);
        h.2.store(true, Ordering::SeqCst);
        let mut r = DeviceRegistry::new(vec![Box::new(h)]);
        r.scan_all();
        for _ in 0..5 {
            assert!(r.set_colors("hid:1462:7d93:0", &[Color::default()]).is_err());
        }
    }
}
