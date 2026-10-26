// Regroupement et mise en forme des capteurs matériels (lecture seule) pour le panneau Surveillance.
import type { Sensor } from "./types";

export const TYPE_ORDER = ["Temperature", "Load", "Clock", "Power", "Voltage", "Fan", "Control", "Data", "SmallData"];

export const TYPE_LABEL: Record<string, string> = {
  Temperature: "Températures",
  Load: "Charge",
  Clock: "Fréquences",
  Power: "Puissance",
  Voltage: "Tensions",
  Fan: "Ventilateurs (tr/min)",
  Control: "Commandes ventilateurs",
  Data: "Données",
  SmallData: "Données (Mo)",
};

const UNIT: Record<string, string> = {
  Temperature: "°C",
  Load: "%",
  Clock: "MHz",
  Power: "W",
  Voltage: "V",
  Fan: "tr/min",
  Control: "%",
  Data: "Go",
  SmallData: "Mo",
};

export type Level = "ok" | "warn" | "hot";

export interface TypeGroup {
  type: string;
  label: string;
  items: Sensor[];
}
export interface HardwareGroup {
  hardware: string;
  types: TypeGroup[];
}

export function formatValue(s: Pick<Sensor, "type" | "value">): string {
  const v = s.type === "Voltage" ? s.value.toFixed(3) : Math.abs(s.value) >= 100 ? Math.round(s.value).toString() : s.value.toFixed(1);
  return `${v} ${UNIT[s.type] ?? ""}`.trim();
}

/** Seuils prudents (les limites réelles varient selon la puce) : seules les températures sont colorées. */
export function levelOf(s: Pick<Sensor, "type" | "value">): Level {
  if (s.type !== "Temperature") return "ok";
  if (s.value >= 90) return "hot";
  if (s.value >= 80) return "warn";
  return "ok";
}

function norm(s: string): string {
  return s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Groupe par composant puis par type ; `query` filtre sur composant, nom ou type. */
export function groupSensors(sensors: Sensor[], query = ""): HardwareGroup[] {
  const q = norm(query.trim());
  const byHw = new Map<string, Map<string, Sensor[]>>();
  for (const s of sensors) {
    if (q && !norm(`${s.hardware} ${s.name} ${TYPE_LABEL[s.type] ?? s.type}`).includes(q)) continue;
    const types = byHw.get(s.hardware) ?? new Map<string, Sensor[]>();
    const list = types.get(s.type) ?? [];
    list.push(s);
    types.set(s.type, list);
    byHw.set(s.hardware, types);
  }
  const rank = (t: string) => {
    const i = TYPE_ORDER.indexOf(t);
    return i < 0 ? TYPE_ORDER.length : i;
  };
  return [...byHw.entries()]
    .map(([hardware, types]) => ({
      hardware,
      types: [...types.entries()]
        .sort((a, b) => rank(a[0]) - rank(b[0]))
        .map(([type, items]) => ({ type, label: TYPE_LABEL[type] ?? type, items })),
    }))
    .sort((a, b) => a.hardware.localeCompare(b.hardware));
}

/** Historique glissant par capteur (valeurs récentes, pour mini-courbe et min/max). */
export function pushHistory(hist: Record<string, number[]>, sensors: Sensor[], max = 90): void {
  for (const s of sensors) {
    const h = hist[s.id] ?? (hist[s.id] = []);
    h.push(s.value);
    if (h.length > max) h.shift();
  }
}

/** Points SVG « x,y » d'une mini-courbe (viewBox w×h). */
export function sparkPoints(values: number[], w = 80, h = 20): string {
  if (values.length < 2) return "";
  const lo = Math.min(...values);
  const span = Math.max(...values) - lo || 1;
  return values.map((v, i) => `${((i / (values.length - 1)) * w).toFixed(1)},${(h - ((v - lo) / span) * h).toFixed(1)}`).join(" ");
}
