//! Mapping capteurs LibreHardwareMonitor (via sensord) -> clés stables des écrans.
//!
//! Chaque clé a une cascade de règles (noms exacts connus, puis repli sur le
//! maximum / la moyenne du matériel de la bonne famille). La famille vient
//! d'abord de l'id LHM (`/intelcpu/0/…`, `/gpu-nvidia/0/…`, `/nvme/0/…`,
//! `/lpc/…`), puis du nom du matériel. Le capteur retenu pour chaque clé est
//! gardé dans `used` pour le panneau de diagnostic « Capteurs détectés ».

use crate::sensors::Sensor;
use serde::Serialize;
use std::collections::HashMap;

#[cfg(test)]
#[path = "sensor_map_tests.rs"]
mod tests;

/// Capteur (ou calcul) à l'origine d'une clé ; `id` vide = valeur calculée.
#[derive(Debug, Clone, Serialize)]
pub struct UsedSensor {
    pub id: String,
    pub hardware: String,
    pub name: String,
}

/// Diagnostic complet pour l'interface (commande `screen_sensors`).
#[derive(Debug, Clone, Serialize)]
pub struct SensorsReport {
    pub sensord_running: bool,
    pub sensor_count: usize,
    pub exe_path: Option<String>,
    pub sensors: Vec<Sensor>,
    /// clé -> capteur utilisé.
    pub used: HashMap<String, UsedSensor>,
}

#[derive(Default)]
pub struct Mapped {
    pub values: HashMap<String, f64>,
    pub used: HashMap<String, UsedSensor>,
}

impl Mapped {
    fn put(&mut self, key: &str, s: Option<&Sensor>) {
        self.put_as(key, s, |s| s.value);
    }

    /// Comme `put` avec une conversion d'unité.
    fn put_as(&mut self, key: &str, s: Option<&Sensor>, f: fn(&Sensor) -> f64) {
        if let Some(s) = s.filter(|s| s.value.is_finite()) {
            self.values.insert(key.into(), f(s));
            self.used.insert(key.into(), UsedSensor { id: s.id.clone(), hardware: s.hardware.clone(), name: s.name.clone() });
        }
    }

    fn put_calc(&mut self, key: &str, v: f64, how: String) {
        if v.is_finite() {
            self.values.insert(key.into(), v);
            self.used.insert(key.into(), UsedSensor { id: String::new(), hardware: String::new(), name: how });
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Class {
    Cpu,
    Gpu,
    Storage,
    Board,
    Other,
}

fn classify(s: &Sensor) -> Class {
    let id = s.id.to_lowercase();
    let root = id.trim_start_matches('/').split('/').next().unwrap_or("");
    match root {
        "intelcpu" | "amdcpu" | "cpu" => return Class::Cpu,
        r if r.starts_with("gpu") => return Class::Gpu,
        "nvme" | "ssd" | "hdd" | "storage" => return Class::Storage,
        "lpc" | "mainboard" | "motherboard" | "superio" | "embeddedcontroller" | "ec" => return Class::Board,
        _ => {}
    }
    let hw = s.hardware.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| hw.contains(x));
    if has(&["nvidia", "geforce", "radeon", "intel arc", "graphics"]) {
        Class::Gpu
    } else if has(&["ryzen", "core i", "xeon", "threadripper", "epyc", "processor", "cpu"]) {
        Class::Cpu
    } else if has(&["ssd", "nvme", "hdd"]) {
        Class::Storage
    } else {
        Class::Other
    }
}

type List<'a> = Vec<&'a Sensor>;

fn low(s: &Sensor) -> String {
    s.name.to_lowercase()
}

fn kind<'a>(list: &[&'a Sensor], k: &str) -> List<'a> {
    list.iter().copied().filter(|s| s.kind.eq_ignore_ascii_case(k)).collect()
}

/// Premier capteur du type `k` dont le nom est exactement l'un de `names` (ordre = priorité).
fn named<'a>(list: &[&'a Sensor], k: &str, names: &[&str]) -> Option<&'a Sensor> {
    names.iter().find_map(|n| list.iter().copied().find(|s| s.kind.eq_ignore_ascii_case(k) && s.name.eq_ignore_ascii_case(n)))
}

fn hottest<'a>(it: impl Iterator<Item = &'a Sensor>) -> Option<&'a Sensor> {
    it.filter(|s| s.value.is_finite()).max_by(|a, b| a.value.total_cmp(&b.value))
}

fn avg(v: &[&Sensor]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().map(|s| s.value).sum::<f64>() / v.len() as f64)
}

/// Capacité en Gio d'un capteur Data (Go) ou SmallData (Mo).
fn gib(s: &Sensor) -> f64 {
    if s.kind.eq_ignore_ascii_case("SmallData") {
        s.value / 1024.0
    } else {
        s.value
    }
}

fn data<'a>(list: &[&'a Sensor], names: &[&str]) -> Option<&'a Sensor> {
    ["SmallData", "Data"].iter().find_map(|k| named(list, k, names))
}

fn is_core(s: &Sensor) -> bool {
    let n = low(s);
    (n.starts_with("cpu core #") || n.starts_with("core #")) && !n.contains("distance") && !n.contains("tjmax")
}

fn map_cpu(m: &mut Mapped, cpu: &[&Sensor], board: &[&Sensor]) {
    let temps = kind(cpu, "Temperature");
    let t = named(
        cpu,
        "Temperature",
        &["CPU Package", "Core (Tctl/Tdie)", "CPU (Tctl/Tdie)", "Tctl/Tdie", "Tdie", "Tctl", "Package", "Core Max"],
    )
    .or_else(|| hottest(temps.iter().copied().filter(|s| is_core(s))))
    .or_else(|| named(cpu, "Temperature", &["Core Average"]))
    .or_else(|| hottest(temps.iter().copied().filter(|s| low(s).starts_with("ccd"))))
    .or_else(|| hottest(temps.iter().copied().filter(|s| !low(s).contains("distance") && !low(s).contains("tjmax"))))
    .or_else(|| named(board, "Temperature", &["CPU", "CPU Temperature", "CPUTIN", "CPU Core"]));
    m.put("cpu.temp", t);

    match named(cpu, "Load", &["CPU Total"]) {
        Some(s) => m.put("cpu.load", Some(s)),
        None => {
            let l = kind(cpu, "Load");
            let cores: List = l.iter().copied().filter(|s| low(s).starts_with("cpu core #") && !low(s).contains("thread")).collect();
            let threads: List = l.iter().copied().filter(|s| low(s).starts_with("cpu core #")).collect();
            let pick = if cores.is_empty() { threads } else { cores };
            if let Some(v) = avg(&pick) {
                m.put_calc("cpu.load", v, format!("moyenne de {} cœurs", pick.len()));
            }
        }
    }

    m.put("cpu.power", named(cpu, "Power", &["CPU Package", "Package", "CPU Cores", "Core (SVI2 TFN)"]));

    match named(cpu, "Clock", &["Cores (Average)"]) {
        Some(s) => m.put("cpu.clock", Some(s)),
        None => {
            let c: List = kind(cpu, "Clock").into_iter().filter(|s| is_core(s)).collect();
            if let Some(v) = avg(&c) {
                m.put_calc("cpu.clock", v, format!("moyenne de {} cœurs", c.len()));
            }
        }
    }

    let vnames = ["CPU Core", "Core (SVI2 TFN)", "CPU VCore", "VCore", "Vcore"];
    m.put("cpu.voltage", named(cpu, "Voltage", &vnames).or_else(|| named(board, "Voltage", &vnames)));
}

/// Carte graphique principale : une dédiée si présente, sinon l'iGPU.
fn primary_gpu<'a>(gpu: &[&'a Sensor]) -> List<'a> {
    let rank = |s: &Sensor| s.id.to_lowercase().contains("integrated") as u8;
    let Some(min) = gpu.iter().map(|s| rank(s)).min() else {
        return Vec::new();
    };
    // Premier matériel du meilleur rang dans l'ordre du relevé.
    let hw = gpu.iter().find(|s| rank(s) == min).map(|s| s.hardware.clone()).unwrap_or_default();
    gpu.iter().copied().filter(|s| s.hardware == hw).collect()
}

fn map_gpu(m: &mut Mapped, all_gpu: &[&Sensor]) {
    let gpu = primary_gpu(all_gpu);
    let temps = kind(&gpu, "Temperature");
    let hot = ["GPU Hot Spot", "GPU Hotspot", "Hot Spot", "GPU Junction", "GPU Junction Temperature"];
    let is_aux = |s: &Sensor| {
        let n = low(s);
        n.contains("hot") || n.contains("junction") || n.contains("memory") || n.contains("vram")
    };
    m.put("gpu.hotspot", named(&gpu, "Temperature", &hot));
    m.put(
        "gpu.temp",
        named(&gpu, "Temperature", &["GPU Core", "GPU Temperature", "GPU"]).or_else(|| temps.iter().copied().find(|s| !is_aux(s))).or_else(|| temps.first().copied()),
    );
    let loads = kind(&gpu, "Load");
    m.put(
        "gpu.load",
        named(&gpu, "Load", &["GPU Core", "D3D 3D", "GPU Total"]).or_else(|| hottest(loads.iter().copied().filter(|s| low(s).starts_with("d3d 3d")))),
    );
    m.put("gpu.clock", named(&gpu, "Clock", &["GPU Core", "GPU Clock"]));
    m.put("gpu.mem_clock", named(&gpu, "Clock", &["GPU Memory", "GPU Memory Clock"]));
    let powers = kind(&gpu, "Power");
    m.put(
        "gpu.power",
        named(&gpu, "Power", &["GPU Package", "GPU Power", "GPU Total Board Power", "GPU Core"]).or_else(|| powers.first().copied()),
    );
    m.put("gpu.fan", kind(&gpu, "Fan").first().copied());

    let used = data(&gpu, &["GPU Memory Used", "D3D Dedicated Memory Used"]);
    let total = data(&gpu, &["GPU Memory Total", "D3D Dedicated Memory Total"]);
    let pct_sensor = named(&gpu, "Load", &["GPU Memory"]);
    match pct_sensor {
        Some(s) => m.put("gpu.vram_used_pct", Some(s)),
        None => {
            if let (Some(u), Some(t)) = (used, total) {
                if gib(t) > 0.0 {
                    m.put_calc("gpu.vram_used_pct", gib(u) / gib(t) * 100.0, "utilisé / total".into());
                }
            }
        }
    }
    match (used, total, pct_sensor) {
        (Some(u), _, _) => m.put_as("gpu.vram_used_gb", Some(u), gib),
        (None, Some(t), Some(p)) => m.put_calc("gpu.vram_used_gb", gib(t) * p.value / 100.0, "total x pourcentage".into()),
        _ => {}
    }
}

fn map_board(m: &mut Mapped, board: &[&Sensor]) {
    let temps = kind(board, "Temperature");
    let find = |f: &dyn Fn(&str) -> bool| temps.iter().copied().find(|s| f(&low(s)));
    let generic = || {
        temps
            .iter()
            .copied()
            .filter_map(|s| Some((low(s).strip_prefix("temperature #")?.trim().parse::<u32>().ok()?, s)))
            .min_by_key(|(n, _)| *n)
            .map(|(_, s)| s)
    };
    m.put(
        "mobo.temp",
        named(board, "Temperature", &["Motherboard", "System", "Mainboard", "System Temperature", "Motherboard Temperature", "MB", "Board"])
            .or_else(|| find(&|n| n.starts_with("system")))
            .or_else(generic),
    );
    m.put("chipset.temp", find(&|n| n.contains("chipset") || n.contains("pch")));
    m.put("vrm.temp", find(&|n| n.contains("vrm") || n.contains("mosfet") || n.ends_with(" mos") || n.contains("mos ")));
}

fn map_storage(m: &mut Mapped, sto: &[&Sensor]) {
    // Un indice par périphérique (préfixe d'id `/ssd/2`), dans l'ordre du relevé : deux
    // disques de même modèle restent distincts.
    let dev = |s: &Sensor| s.id.rsplitn(3, '/').nth(2).unwrap_or(&s.id).to_string();
    let mut order: Vec<String> = Vec::new();
    for s in sto {
        let d = dev(s);
        if !order.contains(&d) {
            order.push(d);
        }
    }
    for (i, d) in order.iter().enumerate() {
        let mine: List = sto.iter().copied().filter(|s| dev(s) == *d).collect();
        let n = i + 1;
        let temps = kind(&mine, "Temperature");
        m.put(&format!("ssd.{n}.temp"), named(&mine, "Temperature", &["Temperature", "Composite Temperature"]).or_else(|| temps.first().copied()));
        m.put(&format!("ssd.{n}.used_pct"), named(&mine, "Load", &["Used Space"]));
    }
}

fn map_cooling(m: &mut Mapped, all: &[&Sensor], board: &[&Sensor]) {
    let temps = kind(all, "Temperature");
    let liquid = ["liquid", "coolant", "water in", "water"].iter().find_map(|w| temps.iter().copied().find(|s| low(s).contains(w)));
    m.put("liquid.temp", liquid);

    let fans = kind(all, "Fan");
    let (pumps, plain): (List, List) = fans.iter().copied().partition(|s| low(s).contains("pump"));
    m.put("pump.rpm", pumps.first().copied());
    let cpu_fan = plain.iter().copied().find(|s| low(s).contains("cpu") && !low(s).contains("opt")).or_else(|| plain.iter().copied().find(|s| low(s).contains("cpu")));
    m.put("cpu.fan", cpu_fan);
    for (i, f) in plain.iter().enumerate() {
        m.put(&format!("fan.{}.rpm", i + 1), Some(f));
    }
    let board_fans = kind(board, "Fan");
    for (i, f) in board_fans.iter().filter(|s| !low(s).contains("pump")).enumerate() {
        m.put(&format!("mobo.fan.{}", i + 1), Some(f));
    }
}

/// Mapping complet. `sensor:<id>` est ajouté pour tout capteur fini (en cas de doublon d'id, le dernier gagne).
pub fn map_sensors(all: &[Sensor]) -> Mapped {
    let mut m = Mapped::default();
    for s in all.iter().filter(|s| s.value.is_finite()) {
        m.put(&format!("sensor:{}", s.id), Some(s));
    }
    // Les `sensor:` ne figurent pas dans le diagnostic « clé -> capteur » (ils sont triviaux).
    m.used.clear();

    let by = |c: Class| -> List { all.iter().filter(|s| classify(s) == c).collect() };
    let (cpu, gpu, sto, board) = (by(Class::Cpu), by(Class::Gpu), by(Class::Storage), by(Class::Board));
    let every: List = all.iter().collect();

    map_cpu(&mut m, &cpu, &board);
    map_gpu(&mut m, &gpu);
    map_board(&mut m, &board);
    map_storage(&mut m, &sto);
    map_cooling(&mut m, &every, &board);
    m
}
