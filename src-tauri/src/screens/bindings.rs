//! Sources de données des écrans : `ValueStore` associe des clés stables
//! (`cpu.temp`, `gpu.load`, `ram.used_pct`, …) aux dernières valeurs lues sur
//! `SensorHub` (sensord) et `sysinfo`, et garde un historique de ~10 min.
//!
//! Clés : voir `sensor_map.rs` (capteurs) et `read_system` (RAM, réseau, disques,
//! repli CPU). Index `ssd.N`, `fan.N.rpm`, `mobo.fan.N` à partir de **1**, dans
//! l'ordre du relevé sensord. `disk.<LETTRE>.*` vient de sysinfo (point de montage).
//! `net.*` et `disk.read/write` en Mo/s, `uptime` en secondes, `ram.*_gb` en Gio,
//! `*.clock` en MHz. `time` / `date` ne passent pas par ici (formatées au rendu).

use super::sensor_map::{map_sensors, SensorsReport, UsedSensor};
use crate::sensors::SensorHub;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{DiskRefreshKind, Disks, Networks, System};

const HISTORY_SECS: u64 = 600;
const HISTORY_MAX: usize = 1200;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Default)]
struct Inner {
    values: HashMap<String, f64>,
    used: HashMap<String, UsedSensor>,
    history: HashMap<String, VecDeque<(Instant, f32)>>,
}

pub struct ValueStore {
    hub: Option<Arc<SensorHub>>,
    inner: Mutex<Inner>,
    sys: Mutex<Sys>,
}

struct Sys {
    system: System,
    networks: Networks,
    disks: Disks,
    last: Instant,
    ticks: u32,
}

impl ValueStore {
    pub fn new(hub: Arc<SensorHub>) -> Arc<ValueStore> {
        Arc::new(Self::build(Some(hub)))
    }

    fn build(hub: Option<Arc<SensorHub>>) -> ValueStore {
        ValueStore {
            hub,
            inner: Mutex::new(Inner::default()),
            sys: Mutex::new(Sys {
                system: System::new(),
                networks: Networks::new_with_refreshed_list(),
                disks: Disks::new_with_refreshed_list(),
                last: Instant::now(),
                ticks: 0,
            }),
        }
    }

    /// Relit capteurs + système et alimente l'historique. À appeler ~1 fois/s.
    pub fn tick(&self) {
        let mut mapped = self.hub.as_ref().map(|h| map_sensors(&h.snapshot())).unwrap_or_default();
        self.read_system(&mut mapped.values);
        let (vals, used) = (mapped.values, mapped.used);
        let now = Instant::now();
        let mut inner = self.inner.lock();
        for (k, v) in &vals {
            push_history(inner.history.entry(k.clone()).or_default(), now, *v as f32);
        }
        inner.values = vals;
        inner.used = used;
    }

    fn read_system(&self, vals: &mut HashMap<String, f64>) {
        let mut s = self.sys.lock();
        s.system.refresh_memory();
        let total = s.system.total_memory() as f64;
        if total > 0.0 {
            let used = s.system.used_memory() as f64;
            vals.insert("ram.used_pct".into(), used / total * 100.0);
            vals.insert("ram.used_gb".into(), used / GIB);
            vals.insert("ram.free_gb".into(), s.system.available_memory() as f64 / GIB);
            vals.insert("ram.total_gb".into(), total / GIB);
        }
        vals.insert("uptime".into(), System::uptime() as f64);

        // Repli sans sensord (ou sans capteur CPU lisible) : usage et fréquence via sysinfo.
        s.system.refresh_cpu_usage();
        s.system.refresh_cpu_frequency();
        if s.ticks > 0 && !vals.contains_key("cpu.load") {
            vals.insert("cpu.load".into(), s.system.global_cpu_usage() as f64);
        }
        let freqs: Vec<f64> = s.system.cpus().iter().map(|c| c.frequency() as f64).filter(|f| *f > 0.0).collect();
        if !freqs.is_empty() && !vals.contains_key("cpu.clock") {
            vals.insert("cpu.clock".into(), freqs.iter().sum::<f64>() / freqs.len() as f64);
        }

        let dt = s.last.elapsed().as_secs_f64().max(0.001);
        s.last = Instant::now();
        s.networks.refresh(true);
        let (mut up, mut down) = (0u64, 0u64);
        for (_, n) in s.networks.iter() {
            up += n.transmitted();
            down += n.received();
        }
        vals.insert("net.up".into(), up as f64 / dt / 1e6);
        vals.insert("net.down".into(), down as f64 / dt / 1e6);

        // Espace disque : rafraîchi (liste comprise) toutes les 30 lectures ; débit à chaque lecture.
        if s.ticks % 30 == 0 {
            s.disks.refresh(true);
        } else {
            s.disks.refresh_specifics(false, DiskRefreshKind::nothing().with_io_usage());
        }
        s.ticks = s.ticks.wrapping_add(1);
        let (mut read, mut written) = (0u64, 0u64);
        for d in s.disks.list() {
            let usage = d.usage();
            read += usage.read_bytes;
            written += usage.written_bytes;
            let t = d.total_space() as f64;
            let Some(letter) = drive_letter(&d.mount_point().to_string_lossy()) else { continue };
            if t > 0.0 {
                let free = d.available_space() as f64;
                vals.insert(format!("disk.{letter}.used_pct"), (t - free) / t * 100.0);
                vals.insert(format!("disk.{letter}.free_gb"), free / GIB);
                vals.insert(format!("disk.{letter}.total_gb"), t / GIB);
            }
        }
        vals.insert("disk.read".into(), read as f64 / dt / 1e6);
        vals.insert("disk.write".into(), written as f64 / dt / 1e6);
    }

    /// Diagnostic pour l'interface : tous les capteurs bruts + clé -> capteur utilisé.
    pub fn sensors_report(&self) -> SensorsReport {
        let used = self.inner.lock().used.clone();
        match &self.hub {
            Some(h) => {
                let d = h.diag();
                SensorsReport { sensord_running: d.running, sensor_count: d.sensor_count, exe_path: d.exe_path, sensors: h.snapshot(), used }
            }
            None => SensorsReport { sensord_running: false, sensor_count: 0, exe_path: None, sensors: Vec::new(), used },
        }
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.inner.lock().values.get(key).copied()
    }

    /// Valeurs des `seconds` dernières secondes, de la plus ancienne à la plus récente.
    pub fn history(&self, key: &str, seconds: u32) -> Vec<f32> {
        let inner = self.inner.lock();
        let Some(h) = inner.history.get(key) else { return Vec::new() };
        let now = Instant::now();
        h.iter()
            .filter(|(t, _)| now.duration_since(*t).as_secs() <= seconds as u64)
            .map(|(_, v)| *v)
            .collect()
    }

    /// Toutes les valeurs courantes (inspecteur de l'éditeur).
    pub fn snapshot(&self) -> HashMap<String, f64> {
        self.inner.lock().values.clone()
    }

    /// Injecte une valeur (tests de rendu, aperçu) ; alimente aussi l'historique.
    pub fn set_for_test(&self, key: &str, val: f64) {
        let mut inner = self.inner.lock();
        inner.values.insert(key.to_string(), val);
        push_history(inner.history.entry(key.to_string()).or_default(), Instant::now(), val as f32);
    }

    /// Store sans capteurs (tests, aperçu hors-ligne).
    pub fn empty() -> ValueStore {
        Self::build(None)
    }
}

/// `C:\` -> `C` (point de montage Windows ; les autres formes sont ignorées).
fn drive_letter(mount: &str) -> Option<char> {
    let mut c = mount.chars();
    let l = c.next().filter(char::is_ascii_alphabetic)?;
    (c.next() == Some(':')).then(|| l.to_ascii_uppercase())
}

fn push_history(h: &mut VecDeque<(Instant, f32)>, now: Instant, v: f32) {
    h.push_back((now, v));
    while h.len() > HISTORY_MAX || h.front().is_some_and(|(t, _)| now.duration_since(*t).as_secs() > HISTORY_SECS) {
        h.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_accumulates_and_is_bounded() {
        let st = ValueStore::empty();
        for i in 0..5 {
            st.set_for_test("cpu.temp", i as f64);
        }
        assert_eq!(st.history("cpu.temp", 60), vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(st.get("cpu.temp"), Some(4.0));
        assert!(st.history("nope", 60).is_empty());
        for _ in 0..HISTORY_MAX + 50 {
            st.set_for_test("x", 1.0);
        }
        assert_eq!(st.history("x", 600).len(), HISTORY_MAX);
    }

    #[test]
    fn drive_letters() {
        assert_eq!(drive_letter("C:\\"), Some('C'));
        assert_eq!(drive_letter(r"d:\Mount"), Some('D'));
        assert_eq!(drive_letter("/mnt/data"), None);
        assert_eq!(drive_letter(""), None);
    }

    #[test]
    fn tick_fills_system_keys() {
        let st = ValueStore::empty();
        st.tick();
        assert!(st.get("ram.total_gb").is_some_and(|v| v > 0.0));
        assert!(st.get("uptime").is_some());
        assert!(st.get("ram.free_gb").is_some());
        assert!(st.get("disk.read").is_some());
        // Au moins un disque par lettre (Windows) avec les trois clés.
        let c = st.snapshot().keys().find_map(|k| k.strip_prefix("disk.")?.strip_suffix(".used_pct").map(String::from));
        if let Some(l) = c {
            assert!(st.get(&format!("disk.{l}.free_gb")).is_some() && st.get(&format!("disk.{l}.total_gb")).is_some());
        }
        assert!(st.sensors_report().sensors.is_empty());
    }
}
