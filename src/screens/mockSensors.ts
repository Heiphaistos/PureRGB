// Capteurs bruts factices (mode démo), cohérents avec `mockValues` : même valeur pour la clé et son capteur.
import { mockValues } from "./mockRender.ts";
import type { RawSensor, SensorsReport } from "./types.ts";

const CPU = "AMD Ryzen 7 5800X";
const GPU = "NVIDIA GeForce RTX 3070";
const BOARD = "ASUS ROG STRIX B550-F";
const SSD = "Samsung SSD 980 PRO 1TB";

// [clé, id, matériel, nom, type]
const MAP: [string, string, string, string, string][] = [
  ["cpu.temp", "/amdcpu/0/temperature/2", CPU, "Core (Tctl/Tdie)", "Temperature"],
  ["cpu.load", "/amdcpu/0/load/0", CPU, "CPU Total", "Load"],
  ["cpu.clock", "/amdcpu/0/clock/0", CPU, "Cores (Average)", "Clock"],
  ["cpu.power", "/amdcpu/0/power/0", CPU, "Package", "Power"],
  ["cpu.voltage", "/amdcpu/0/voltage/2", CPU, "Core (SVI2 TFN)", "Voltage"],
  ["cpu.fan", "/lpc/nct6798d/0/fan/1", BOARD, "CPU Fan", "Fan"],
  ["gpu.temp", "/gpu-nvidia/0/temperature/0", GPU, "GPU Core", "Temperature"],
  ["gpu.hotspot", "/gpu-nvidia/0/temperature/2", GPU, "GPU Hot Spot", "Temperature"],
  ["gpu.load", "/gpu-nvidia/0/load/0", GPU, "GPU Core", "Load"],
  ["gpu.clock", "/gpu-nvidia/0/clock/0", GPU, "GPU Core", "Clock"],
  ["gpu.mem_clock", "/gpu-nvidia/0/clock/4", GPU, "GPU Memory", "Clock"],
  ["gpu.power", "/gpu-nvidia/0/power/0", GPU, "GPU Package", "Power"],
  ["gpu.vram_used_pct", "/gpu-nvidia/0/load/3", GPU, "GPU Memory", "Load"],
  ["gpu.vram_used_gb", "/gpu-nvidia/0/smalldata/1", GPU, "GPU Memory Used", "Data"],
  ["gpu.fan", "/gpu-nvidia/0/fan/1", GPU, "GPU Fan", "Fan"],
  ["mobo.temp", "/lpc/nct6798d/0/temperature/1", BOARD, "System", "Temperature"],
  ["chipset.temp", "/lpc/asus/0/temperature/1", BOARD, "Chipset", "Temperature"],
  ["vrm.temp", "/lpc/asus/0/temperature/0", BOARD, "VRM", "Temperature"],
  ["mobo.fan.1", "/lpc/nct6798d/0/fan/1", BOARD, "CPU Fan", "Fan"],
  ["ssd.1.temp", "/nvme/0/temperature/0", SSD, "Temperature", "Temperature"],
  ["ssd.1.used_pct", "/nvme/0/load/0", SSD, "Used Space", "Load"],
  ["liquid.temp", "/aio/0/temperature/0", "AIO", "Liquid Temperature", "Temperature"],
  ["pump.rpm", "/lpc/nct6798d/0/fan/3", BOARD, "AIO Pump", "Fan"],
];

export function mockSensors(tMs: number): SensorsReport {
  const v = mockValues(tMs);
  const sensors: RawSensor[] = [];
  const used: SensorsReport["used"] = {};
  for (const [key, id, hardware, name, type] of MAP) {
    const value = v[key];
    if (value === undefined) continue;
    sensors.push({ id, hardware, name, type, value: Math.round(value * 10) / 10, controllable: false });
    used[key] = { id, hardware, name };
  }
  return { sensord_running: true, sensor_count: sensors.length, exe_path: "C:\Program Files\PureRGB\sensord\sensord.exe", sensors, used };
}
