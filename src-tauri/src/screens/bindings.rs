//! Sources de données des écrans : `ValueStore` associe des clés stables
//! (`cpu.temp`, `gpu.load`, `ram.used_pct`, …) aux dernières valeurs lues sur
//! `SensorHub` (sensord) et `sysinfo`, et garde un historique de ~10 min.
//!
//! Index : `ssd.<n>` et `fan.<n>` sont numérotés à partir de **1**, dans l'ordre
//! du relevé sensord. `net.*` en Mo/s, `uptime` en secondes, `ram.*_gb` en Gio,
//! `*.clock` en MHz. Les clés `time` / `date` ne passent pas par ici (formatées
//! au rendu).

use crate::sensors::{Sensor, SensorHub};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

const HISTORY_SECS: u64 = 600;
const HISTORY_MAX: usize = 1200;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Default)]
struct Inner {
    values: HashMap<String, f64>,
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
        let mut vals = HashMap::new();
        if let Some(hub) = &self.hub {
            map_sensors(&hub.snapshot(), &mut vals);
        }
        self.read_system(&mut vals);
        let now = Instant::now();
        let mut inner = self.inner.lock();
        for (k, v) in &vals {
            push_history(inner.history.entry(k.clone()).or_default(), now, *v as f32);
        }
        inner.values = vals;
    }

    fn read_system(&self, vals: &mut HashMap<String, f64>) {
        let mut s = self.sys.lock();
        s.system.refresh_memory();
        let total = s.system.total_memory() as f64;
        if total > 0.0 {
            let used = s.system.used_memory() as f64;
            vals.insert("ram.used_pct".into(), used / total * 100.0);
            vals.insert("ram.used_gb".into(), used / GIB);
            vals.insert("ram.total_gb".into(), total / GIB);
        }
        vals.insert("uptime".into(), System::uptime() as f64);

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

        // Liste des disques : coûteuse, rafraîchie toutes les 30 lectures.
        s.ticks = s.ticks.wrapping_add(1);
        if s.ticks % 30 == 1 {
            s.disks.refresh(true);
        }
        for (i, d) in s.disks.list().iter().enumerate() {
            let t = d.total_space() as f64;
            if t > 0.0 {
                let pct = (t - d.available_space() as f64) / t * 100.0;
                vals.insert(format!("disk.{}.used_pct", i + 1), pct);
            }
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

fn push_history(h: &mut VecDeque<(Instant, f32)>, now: Instant, v: f32) {
    h.push_back((now, v));
    while h.len() > HISTORY_MAX || h.front().is_some_and(|(t, _)| now.duration_since(*t).as_secs() > HISTORY_SECS) {
        h.pop_front();
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Class {
    Cpu,
    Gpu,
    Storage,
    Other,
}

/// Classe un capteur par l'id LHM (`/intelcpu/0/…`, `/gpu-nvidia/0/…`, `/nvme/0/…`)
/// puis, à défaut, par le nom du matériel.
fn classify(s: &Sensor) -> Class {
    let id = s.id.to_lowercase();
    let hw = s.hardware.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| id.contains(w) || hw.contains(w));
    if has(&["/gpu", "nvidia", "geforce", "radeon", "intel arc"]) {
        Class::Gpu
    } else if has(&["/intelcpu", "/amdcpu", "ryzen", "intel core", "xeon", "threadripper", "cpu"]) {
        Class::Cpu
    } else if has(&["/nvme", "/hdd", "/ssd", "ssd", "nvme", "hdd"]) {
        Class::Storage
    } else {
        Class::Other
    }
}

fn first<'a>(list: &[&'a Sensor], kind: &str, names: &[&str]) -> Option<&'a Sensor> {
    names.iter().find_map(|n| {
        list.iter().copied().find(|s| s.kind.eq_ignore_ascii_case(kind) && s.name.eq_ignore_ascii_case(n))
    })
}

fn put(out: &mut HashMap<String, f64>, key: &str, s: Option<&Sensor>) {
    if let Some(s) = s.filter(|s| s.value.is_finite()) {
        out.insert(key.to_string(), s.value);
    }
}

fn of_kind<'a>(list: &[&'a Sensor], kind: &str) -> Vec<&'a Sensor> {
    list.iter().copied().filter(|s| s.kind.eq_ignore_ascii_case(kind)).collect()
}

fn avg_clock(list: &[&Sensor]) -> Option<f64> {
    let v: Vec<f64> = of_kind(list, "Clock").iter().filter(|s| s.name.contains('#')).map(|s| s.value).collect();
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// Mapping capteurs sensord → clés stables.
fn map_sensors(all: &[Sensor], out: &mut HashMap<String, f64>) {
    for s in all.iter().filter(|s| s.value.is_finite()) {
        out.insert(format!("sensor:{}", s.id), s.value);
    }
    let by = |c: Class| -> Vec<&Sensor> { all.iter().filter(|s| classify(s) == c).collect() };
    let (cpu, gpu, sto) = (by(Class::Cpu), by(Class::Gpu), by(Class::Storage));

    let cpu_temp = first(
        &cpu,
        "Temperature",
        &["CPU Package", "Core Max", "CPU (Tctl/Tdie)", "Core (Tctl/Tdie)", "Tctl/Tdie", "Tdie", "Tctl"],
    )
    .map(|s| s.value)
    .or_else(|| of_kind(&cpu, "Temperature").iter().filter(|s| s.name.starts_with("Core")).map(|s| s.value).reduce(f64::max));
    if let Some(v) = cpu_temp {
        out.insert("cpu.temp".into(), v);
    }
    put(out, "cpu.load", first(&cpu, "Load", &["CPU Total"]));
    put(out, "cpu.power", first(&cpu, "Power", &["CPU Package", "Package"]));
    if let Some(v) = first(&cpu, "Clock", &["Cores (Average)"]).map(|s| s.value).or_else(|| avg_clock(&cpu)) {
        out.insert("cpu.clock".into(), v);
    }

    put(out, "gpu.temp", first(&gpu, "Temperature", &["GPU Core"]).or_else(|| of_kind(&gpu, "Temperature").first().copied()));
    put(out, "gpu.load", first(&gpu, "Load", &["GPU Core", "D3D 3D"]));
    put(out, "gpu.clock", first(&gpu, "Clock", &["GPU Core"]));
    put(
        out,
        "gpu.power",
        first(&gpu, "Power", &["GPU Package", "GPU Power", "GPU Total Board Power"]).or_else(|| of_kind(&gpu, "Power").first().copied()),
    );
    put(out, "gpu.fan", of_kind(&gpu, "Fan").first().copied());
    let vram = first(&gpu, "Load", &["GPU Memory"]).map(|s| s.value).or_else(|| {
        let find = |names: &[&str]| ["SmallData", "Data"].iter().find_map(|k| first(&gpu, k, names));
        let used = find(&["GPU Memory Used", "D3D Dedicated Memory Used"])?;
        let total = find(&["GPU Memory Total", "D3D Dedicated Memory Total"])?;
        (total.value > 0.0).then(|| used.value / total.value * 100.0)
    });
    if let Some(v) = vram {
        out.insert("gpu.vram_used_pct".into(), v);
    }

    // Disques : un indice par matériel distinct, dans l'ordre du relevé.
    let mut order: Vec<&str> = Vec::new();
    for s in &sto {
        if !order.contains(&s.hardware.as_str()) {
            order.push(&s.hardware);
        }
    }
    for (i, hw) in order.iter().enumerate() {
        let mine: Vec<&Sensor> = sto.iter().copied().filter(|s| s.hardware == *hw).collect();
        let n = i + 1;
        let temp = first(&mine, "Temperature", &["Temperature", "Composite Temperature"])
            .or_else(|| of_kind(&mine, "Temperature").first().copied());
        put(out, &format!("ssd.{n}.temp"), temp);
        put(out, &format!("ssd.{n}.used_pct"), first(&mine, "Load", &["Used Space"]));
    }

    let low = |s: &Sensor| s.name.to_lowercase();
    let liquid = all
        .iter()
        .find(|s| s.kind.eq_ignore_ascii_case("Temperature") && ["liquid", "coolant", "water"].iter().any(|w| low(s).contains(w)));
    put(out, "liquid.temp", liquid);
    let fans: Vec<&Sensor> = all.iter().filter(|s| s.kind.eq_ignore_ascii_case("Fan")).collect();
    put(out, "pump.rpm", fans.iter().copied().find(|s| low(s).contains("pump")));
    put(out, "cpu.fan", fans.iter().copied().find(|s| classify(s) == Class::Cpu || low(s).contains("cpu")));
    for (i, f) in fans.iter().filter(|s| !low(s).contains("pump")).enumerate() {
        put(out, &format!("fan.{}.rpm", i + 1), Some(f));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str, hardware: &str, name: &str, kind: &str, value: f64) -> Sensor {
        Sensor { id: id.into(), hardware: hardware.into(), name: name.into(), kind: kind.into(), value, controllable: false }
    }

    fn lhm() -> Vec<Sensor> {
        let cpu = "Intel Core i7-12700K";
        let gpu = "NVIDIA GeForce RTX 4070";
        vec![
            s("/intelcpu/0/temperature/0", cpu, "Core Max", "Temperature", 61.0),
            s("/intelcpu/0/temperature/1", cpu, "CPU Package", "Temperature", 58.0),
            s("/intelcpu/0/load/0", cpu, "CPU Total", "Load", 23.5),
            s("/intelcpu/0/power/0", cpu, "CPU Package", "Power", 45.2),
            s("/intelcpu/0/clock/1", cpu, "CPU Core #1", "Clock", 4800.0),
            s("/intelcpu/0/clock/2", cpu, "CPU Core #2", "Clock", 5000.0),
            s("/gpu-nvidia/0/temperature/0", gpu, "GPU Core", "Temperature", 55.0),
            s("/gpu-nvidia/0/load/0", gpu, "GPU Core", "Load", 77.0),
            s("/gpu-nvidia/0/load/3", gpu, "D3D 3D", "Load", 12.0),
            s("/gpu-nvidia/0/load/4", gpu, "GPU Memory", "Load", 41.0),
            s("/gpu-nvidia/0/clock/0", gpu, "GPU Core", "Clock", 2475.0),
            s("/gpu-nvidia/0/power/0", gpu, "GPU Package", "Power", 120.0),
            s("/gpu-nvidia/0/fan/1", gpu, "GPU Fan", "Fan", 1450.0),
            s("/nvme/0/temperature/0", "Samsung SSD 980 PRO", "Composite Temperature", "Temperature", 39.0),
            s("/nvme/0/load/0", "Samsung SSD 980 PRO", "Used Space", "Load", 63.0),
            s("/nvme/1/temperature/0", "WD_BLACK SN850X", "Temperature", "Temperature", 44.0),
            s("/lpc/it8688e/0/fan/0", "ASUS PRIME Z690", "CPU Fan", "Fan", 1200.0),
            s("/lpc/it8688e/0/fan/1", "ASUS PRIME Z690", "Chassis Fan #1", "Fan", 800.0),
            s("/lpc/it8688e/0/fan/2", "ASUS PRIME Z690", "AIO Pump", "Fan", 2700.0),
            s("/aio/0/temperature/0", "AIO", "Liquid Temperature", "Temperature", 31.0),
        ]
    }

    #[test]
    fn maps_lhm_sensors() {
        let mut m = HashMap::new();
        map_sensors(&lhm(), &mut m);
        let g = |k: &str| m.get(k).copied();
        assert_eq!(g("cpu.temp"), Some(58.0)); // CPU Package prioritaire sur Core Max
        assert_eq!(g("cpu.load"), Some(23.5));
        assert_eq!(g("cpu.power"), Some(45.2));
        assert_eq!(g("cpu.clock"), Some(4900.0));
        assert_eq!(g("cpu.fan"), Some(1200.0));
        assert_eq!(g("gpu.temp"), Some(55.0));
        assert_eq!(g("gpu.load"), Some(77.0)); // GPU Core, pas D3D 3D
        assert_eq!(g("gpu.clock"), Some(2475.0));
        assert_eq!(g("gpu.power"), Some(120.0));
        assert_eq!(g("gpu.vram_used_pct"), Some(41.0));
        assert_eq!(g("gpu.fan"), Some(1450.0));
        assert_eq!(g("ssd.1.temp"), Some(39.0));
        assert_eq!(g("ssd.1.used_pct"), Some(63.0));
        assert_eq!(g("ssd.2.temp"), Some(44.0));
        assert_eq!(g("liquid.temp"), Some(31.0));
        assert_eq!(g("pump.rpm"), Some(2700.0));
        assert_eq!(g("fan.1.rpm"), Some(1450.0));
        assert_eq!(g("sensor:/gpu-nvidia/0/fan/1"), Some(1450.0));
        assert_eq!(g("fan.5.rpm"), None);
    }

    #[test]
    fn cpu_temp_falls_back_to_hottest_core() {
        let hw = "AMD Ryzen 7 5800X";
        let v = vec![
            s("/amdcpu/0/temperature/2", hw, "Core #1", "Temperature", 50.0),
            s("/amdcpu/0/temperature/3", hw, "Core #2", "Temperature", 64.0),
        ];
        let mut m = HashMap::new();
        map_sensors(&v, &mut m);
        assert_eq!(m.get("cpu.temp"), Some(&64.0));
        assert!(!m.contains_key("gpu.temp"));
    }

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
    fn tick_fills_system_keys() {
        let st = ValueStore::empty();
        st.tick();
        assert!(st.get("ram.total_gb").is_some_and(|v| v > 0.0));
        assert!(st.get("uptime").is_some());
    }
}
