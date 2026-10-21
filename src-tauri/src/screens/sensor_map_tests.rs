use super::*;

fn s(id: &str, hw: &str, name: &str, kind: &str, value: f64) -> Sensor {
    Sensor { id: id.into(), hardware: hw.into(), name: name.into(), kind: kind.into(), value, controllable: false }
}

fn map(v: &[Sensor]) -> HashMap<String, f64> {
    map_sensors(v).values
}

fn g(m: &HashMap<String, f64>, k: &str) -> Option<f64> {
    m.get(k).copied()
}

/// Vrai relevé sensord d'un portable Intel 11th Gen + RTX 3070 + iGPU + SSD SATA (sans carte mère lisible).
#[test]
fn real_machine_sample() {
    #[derive(serde::Deserialize)]
    struct F {
        sensors: Vec<Sensor>,
    }
    let f: F = serde_json::from_str(include_str!("testdata/sensord_i5_11400h_rtx3070.json")).unwrap();
    let r = map_sensors(&f.sensors);
    let m = &r.values;
    assert_eq!(g(m, "cpu.temp"), Some(81.0)); // CPU Package, pas Core Max (87)
    assert_eq!(g(m, "cpu.load"), Some(19.5));
    assert_eq!(g(m, "cpu.power"), Some(28.6));
    assert_eq!(g(m, "gpu.temp"), Some(61.0));
    assert_eq!(g(m, "gpu.hotspot"), Some(67.3));
    assert_eq!(g(m, "gpu.load"), Some(0.0)); // GPU Core NVIDIA, pas l'iGPU
    assert_eq!(g(m, "gpu.power"), Some(15.4));
    assert_eq!(g(m, "gpu.vram_used_pct"), Some(2.1));
    assert_eq!(g(m, "ssd.1.temp"), Some(30.0));
    assert_eq!(g(m, "ssd.1.used_pct"), Some(72.9));
    assert_eq!(g(m, "ssd.2.temp"), None);
    assert_eq!(g(m, "mobo.temp"), None);
    assert_eq!(r.used["cpu.temp"].name, "CPU Package");
    assert!(m.keys().any(|k| k.starts_with("sensor:/gpu-intel-integrated/0/")));
}

#[test]
fn intel_nvidia_asus() {
    let cpu = "Intel Core i7-12700K";
    let gpu = "NVIDIA GeForce RTX 4070";
    let mb = "ASUS PRIME Z690-P";
    let ec = "ASUS EC";
    let v = vec![
        s("/intelcpu/0/temperature/0", cpu, "Core Max", "Temperature", 61.0),
        s("/intelcpu/0/temperature/1", cpu, "CPU Package", "Temperature", 58.0),
        s("/intelcpu/0/load/0", cpu, "CPU Total", "Load", 23.5),
        s("/intelcpu/0/power/0", cpu, "CPU Package", "Power", 45.2),
        s("/intelcpu/0/clock/1", cpu, "CPU Core #1", "Clock", 4800.0),
        s("/intelcpu/0/clock/2", cpu, "CPU Core #2", "Clock", 5000.0),
        s("/intelcpu/0/clock/3", cpu, "Bus Speed", "Clock", 100.0),
        s("/intelcpu/0/voltage/0", cpu, "CPU Core", "Voltage", 1.21),
        s("/gpu-nvidia/0/temperature/0", gpu, "GPU Core", "Temperature", 55.0),
        s("/gpu-nvidia/0/temperature/1", gpu, "GPU Hot Spot", "Temperature", 63.0),
        s("/gpu-nvidia/0/load/0", gpu, "GPU Core", "Load", 77.0),
        s("/gpu-nvidia/0/load/4", gpu, "GPU Memory", "Load", 41.0),
        s("/gpu-nvidia/0/clock/0", gpu, "GPU Core", "Clock", 2475.0),
        s("/gpu-nvidia/0/clock/1", gpu, "GPU Memory", "Clock", 10500.0),
        s("/gpu-nvidia/0/smalldata/1", gpu, "GPU Memory Used", "SmallData", 4915.2),
        s("/gpu-nvidia/0/smalldata/2", gpu, "GPU Memory Total", "SmallData", 12288.0),
        s("/gpu-nvidia/0/power/0", gpu, "GPU Package", "Power", 120.0),
        s("/gpu-nvidia/0/fan/1", gpu, "GPU Fan", "Fan", 1450.0),
        s("/nvme/0/temperature/0", "Samsung SSD 980 PRO 1TB", "Temperature", "Temperature", 39.0),
        s("/nvme/0/temperature/1", "Samsung SSD 980 PRO 1TB", "Temperature 1", "Temperature", 45.0),
        s("/nvme/0/load/0", "Samsung SSD 980 PRO 1TB", "Used Space", "Load", 63.0),
        s("/nvme/1/temperature/0", "Samsung SSD 980 PRO 1TB", "Composite Temperature", "Temperature", 44.0),
        s("/lpc/nct6798d/0/temperature/0", mb, "CPU Core", "Temperature", 40.0),
        s("/lpc/nct6798d/0/temperature/1", mb, "System", "Temperature", 33.0),
        s("/lpc/nct6798d/0/fan/0", mb, "CPU Fan", "Fan", 1200.0),
        s("/lpc/nct6798d/0/fan/1", mb, "Chassis Fan #1", "Fan", 800.0),
        s("/lpc/nct6798d/0/fan/2", mb, "AIO Pump", "Fan", 2700.0),
        s("/lpc/asus/0/temperature/0", ec, "VRM", "Temperature", 47.0),
        s("/lpc/asus/0/temperature/1", ec, "Chipset", "Temperature", 52.0),
        s("/aio/0/temperature/0", "AIO", "Liquid Temperature", "Temperature", 31.0),
    ];
    let m = map(&v);
    assert_eq!(g(&m, "cpu.temp"), Some(58.0));
    assert_eq!(g(&m, "cpu.load"), Some(23.5));
    assert_eq!(g(&m, "cpu.clock"), Some(4900.0));
    assert_eq!(g(&m, "cpu.voltage"), Some(1.21));
    assert_eq!(g(&m, "cpu.fan"), Some(1200.0));
    assert_eq!(g(&m, "gpu.temp"), Some(55.0));
    assert_eq!(g(&m, "gpu.hotspot"), Some(63.0));
    assert_eq!(g(&m, "gpu.mem_clock"), Some(10500.0));
    assert_eq!(g(&m, "gpu.vram_used_pct"), Some(41.0));
    assert!((g(&m, "gpu.vram_used_gb").unwrap_or(0.0) - 4.8).abs() < 1e-6);
    assert_eq!(g(&m, "gpu.fan"), Some(1450.0));
    assert_eq!(g(&m, "ssd.1.temp"), Some(39.0));
    assert_eq!(g(&m, "ssd.1.used_pct"), Some(63.0));
    assert_eq!(g(&m, "ssd.2.temp"), Some(44.0)); // 2e disque de même modèle distinct
    assert_eq!(g(&m, "mobo.temp"), Some(33.0));
    assert_eq!(g(&m, "vrm.temp"), Some(47.0));
    assert_eq!(g(&m, "chipset.temp"), Some(52.0));
    assert_eq!(g(&m, "liquid.temp"), Some(31.0));
    assert_eq!(g(&m, "pump.rpm"), Some(2700.0));
    assert_eq!(g(&m, "mobo.fan.1"), Some(1200.0));
    assert_eq!(g(&m, "mobo.fan.2"), Some(800.0));
    assert_eq!(g(&m, "mobo.fan.3"), None); // la pompe n'est pas un ventilateur
    assert_eq!(g(&m, "fan.1.rpm"), Some(1450.0));
}

#[test]
fn amd_ryzen_radeon_gigabyte() {
    let cpu = "AMD Ryzen 7 5800X";
    let gpu = "AMD Radeon RX 6800 XT";
    let mb = "Gigabyte B550 AORUS ELITE";
    let v = vec![
        s("/amdcpu/0/temperature/2", cpu, "Core (Tctl/Tdie)", "Temperature", 66.0),
        s("/amdcpu/0/temperature/3", cpu, "CCD1 (Tdie)", "Temperature", 60.0),
        s("/amdcpu/0/load/0", cpu, "CPU Total", "Load", 12.0),
        s("/amdcpu/0/power/0", cpu, "Package", "Power", 65.0),
        s("/amdcpu/0/voltage/2", cpu, "Core (SVI2 TFN)", "Voltage", 1.1),
        s("/gpu-amd/0/temperature/0", gpu, "GPU Core", "Temperature", 52.0),
        s("/gpu-amd/0/temperature/1", gpu, "GPU Hot Spot", "Temperature", 70.0),
        s("/gpu-amd/0/temperature/2", gpu, "GPU Memory", "Temperature", 64.0),
        s("/gpu-amd/0/load/0", gpu, "GPU Core", "Load", 30.0),
        s("/gpu-amd/0/data/0", gpu, "GPU Memory Used", "Data", 3.2),
        s("/gpu-amd/0/data/1", gpu, "GPU Memory Total", "Data", 16.0),
        s("/gpu-amd/0/power/0", gpu, "GPU Package", "Power", 80.0),
        s("/lpc/it8688e/0/temperature/0", mb, "System #1", "Temperature", 35.0),
        s("/lpc/it8688e/0/temperature/1", mb, "PCH", "Temperature", 48.0),
        s("/lpc/it8688e/0/temperature/2", mb, "VRM MOS", "Temperature", 55.0),
        s("/lpc/it8688e/0/fan/0", mb, "Fan #1", "Fan", 900.0),
    ];
    let m = map(&v);
    assert_eq!(g(&m, "cpu.temp"), Some(66.0));
    assert_eq!(g(&m, "cpu.power"), Some(65.0));
    assert_eq!(g(&m, "cpu.voltage"), Some(1.1));
    assert_eq!(g(&m, "gpu.temp"), Some(52.0)); // pas la mémoire ni le hotspot
    assert_eq!(g(&m, "gpu.hotspot"), Some(70.0));
    assert_eq!(g(&m, "gpu.load"), Some(30.0));
    assert_eq!(g(&m, "gpu.vram_used_pct"), Some(20.0));
    assert_eq!(g(&m, "gpu.vram_used_gb"), Some(3.2));
    assert_eq!(g(&m, "mobo.temp"), Some(35.0));
    assert_eq!(g(&m, "chipset.temp"), Some(48.0));
    assert_eq!(g(&m, "vrm.temp"), Some(55.0));
    assert_eq!(g(&m, "cpu.fan"), None);
    assert_eq!(g(&m, "mobo.fan.1"), Some(900.0));
}

#[test]
fn cpu_fallbacks_hottest_core_and_load_average() {
    let hw = "AMD Ryzen 5 3600";
    let v = vec![
        s("/amdcpu/0/temperature/2", hw, "Core #1", "Temperature", 50.0),
        s("/amdcpu/0/temperature/3", hw, "Core #2", "Temperature", 64.0),
        s("/amdcpu/0/temperature/4", hw, "Core #1 Distance to TjMax", "Temperature", 99.0),
        s("/amdcpu/0/load/1", hw, "CPU Core #1", "Load", 10.0),
        s("/amdcpu/0/load/2", hw, "CPU Core #2", "Load", 30.0),
    ];
    let r = map_sensors(&v);
    assert_eq!(g(&r.values, "cpu.temp"), Some(64.0));
    assert_eq!(g(&r.values, "cpu.load"), Some(20.0));
    assert!(r.used["cpu.load"].id.is_empty());
    assert!(!r.values.contains_key("gpu.temp"));
}

#[test]
fn cpu_temp_from_board_when_cpu_unreadable() {
    let v = vec![
        s("/intelcpu/0/load/0", "Intel Core i3-10100", "CPU Total", "Load", 5.0),
        s("/lpc/nct6791d/0/temperature/0", "MSI B460", "CPU", "Temperature", 41.0),
        s("/lpc/nct6791d/0/temperature/1", "MSI B460", "Temperature #3", "Temperature", 30.0),
        s("/lpc/nct6791d/0/temperature/2", "MSI B460", "Temperature #2", "Temperature", 29.0),
    ];
    let m = map(&v);
    assert_eq!(g(&m, "cpu.temp"), Some(41.0));
    assert_eq!(g(&m, "mobo.temp"), Some(29.0)); // « Temperature #n » le plus bas
}

#[test]
fn intel_igpu_only_and_discrete_preferred() {
    let igpu = "Intel(R) UHD Graphics";
    let v = vec![
        s("/gpu-intel-integrated/0/load/8", igpu, "D3D 3D", "Load", 9.0),
        s("/gpu-intel-integrated/0/power/0", igpu, "GPU Power", "Power", 2.0),
    ];
    let m = map(&v);
    assert_eq!(g(&m, "gpu.load"), Some(9.0));
    assert_eq!(g(&m, "gpu.power"), Some(2.0));
    assert_eq!(g(&m, "gpu.temp"), None);

    let mut both = v.clone();
    both.push(s("/gpu-nvidia/0/load/0", "NVIDIA GeForce GTX 1660", "GPU Core", "Load", 50.0));
    assert_eq!(g(&map(&both), "gpu.load"), Some(50.0));
}

#[test]
fn sata_and_nvme_numbering() {
    let v = vec![
        s("/ssd/0/temperature/0", "Crucial MX500", "Temperature", "Temperature", 33.0),
        s("/ssd/0/load/30", "Crucial MX500", "Used Space", "Load", 80.0),
        s("/hdd/0/temperature/0", "WDC WD10EZEX", "Temperature", "Temperature", 36.0),
        s("/nvme/0/temperature/0", "WD_BLACK SN850X", "Composite Temperature", "Temperature", 41.0),
    ];
    let m = map(&v);
    assert_eq!(g(&m, "ssd.1.temp"), Some(33.0));
    assert_eq!(g(&m, "ssd.1.used_pct"), Some(80.0));
    assert_eq!(g(&m, "ssd.2.temp"), Some(36.0));
    assert_eq!(g(&m, "ssd.3.temp"), Some(41.0));
    assert_eq!(g(&m, "sensor:/hdd/0/temperature/0"), Some(36.0));
}

#[test]
fn empty_and_nan_are_safe() {
    assert!(map(&[]).is_empty());
    let v = vec![s("/intelcpu/0/temperature/0", "Intel Core i5", "CPU Package", "Temperature", f64::NAN)];
    assert!(!map(&v).contains_key("cpu.temp"));
}
