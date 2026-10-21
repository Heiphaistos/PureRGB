// Sources dynamiques (selon le matériel détecté) et unités des capteurs bruts.
import type { LiveValues } from "./types.ts";
import type { SourceDef } from "./widgets.ts";

const DISK_UNITS: Record<string, [string, string]> = {
  used_pct: ["utilisé", "%"],
  free_gb: ["libre", "Go"],
  total_gb: ["capacité", "Go"],
};

/** Clés à indice (disques par lettre, SSD n, ventilateurs n) présentes dans les valeurs courantes. */
export function dynamicSources(values: LiveValues): SourceDef[] {
  const out: SourceDef[] = [];
  for (const key of Object.keys(values).sort()) {
    let m = /^disk\.([A-Z])\.(used_pct|free_gb|total_gb)$/.exec(key);
    if (m) {
      const [what, unit] = DISK_UNITS[m[2]];
      out.push({ key, label: `Disque ${m[1]}: ${what}`, unit, group: "Disques" });
      continue;
    }
    m = /^ssd\.(\d+)\.(temp|used_pct)$/.exec(key);
    if (m) {
      out.push({ key, label: `Disque ${m[1]} ${m[2] === "temp" ? "température" : "utilisé"}`, unit: m[2] === "temp" ? "°C" : "%", group: "Disques" });
      continue;
    }
    m = /^mobo\.fan\.(\d+)$/.exec(key);
    if (m) out.push({ key, label: `Ventilateur carte mère ${m[1]}`, unit: "tr/min", group: "Carte mère" });
    m = /^fan\.(\d+)\.rpm$/.exec(key);
    if (m) out.push({ key, label: `Ventilateur ${m[1]}`, unit: "tr/min", group: "Refroidissement" });
  }
  return out;
}

const SENSOR_UNITS: Record<string, string> = {
  Temperature: "°C",
  Load: "%",
  Control: "%",
  Fan: "tr/min",
  Power: "W",
  Clock: "MHz",
  Voltage: "V",
  Data: "Go",
  SmallData: "Mo",
};

export const sensorUnit = (type: string): string => SENSOR_UNITS[type] ?? "";
