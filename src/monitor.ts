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

export type CategoryId = "cpu" | "gpu" | "memory" | "board" | "disk" | "fans" | "voltage" | "power" | "other";

export const CATEGORIES: { id: CategoryId; label: string }[] = [
  { id: "cpu", label: "Processeur" },
  { id: "gpu", label: "Carte graphique" },
  { id: "memory", label: "Mémoire" },
  { id: "board", label: "Carte mère" },
  { id: "disk", label: "Disques" },
  { id: "fans", label: "Ventilateurs" },
  { id: "voltage", label: "Tensions" },
  { id: "power", label: "Puissance" },
  { id: "other", label: "Autres capteurs" },
];

export interface CategoryGroup {
  id: CategoryId;
  label: string;
  /** Sous-blocs : par type pour les composants, par composant pour ventilateurs/tensions/puissance. */
  sections: { title: string; items: Sensor[] }[];
  count: number;
}

/** Catégorie d'un capteur : le type prime pour ventilateurs/tensions/puissance, sinon le nom du composant. */
export function categoryOf(s: Pick<Sensor, "hardware" | "type">): CategoryId {
  if (s.type === "Fan" || s.type === "Control") return "fans";
  if (s.type === "Voltage") return "voltage";
  if (s.type === "Power") return "power";
  const h = norm(s.hardware);
  if (/nvidia|geforce|radeon|\barc\b|gpu|graphi/.test(h)) return "gpu";
  if (/memory|memoire|\bram\b|dimm|ddr/.test(h)) return "memory";
  if (/ssd|nvme|hdd|disk|disque|samsung|crucial|kingston|seagate|toshiba|sandisk|western|\bwd\b|storage/.test(h)) return "disk";
  if (/nuvoton|\bite\b|fintek|asus|msi|gigabyte|asrock|mainboard|motherboard|carte mere|super ?io|embedded|chipset/.test(h)) return "board";
  if (/intel|amd|ryzen|\bcore\b|cpu|xeon|threadripper|processor|processeur/.test(h)) return "cpu";
  return "other";
}

/** Capteurs filtrés puis rangés par catégorie claire (ordre de CATEGORIES), catégories vides omises. */
export function groupByCategory(sensors: Sensor[], query = ""): CategoryGroup[] {
  const q = norm(query.trim());
  const byCat = new Map<CategoryId, Sensor[]>();
  for (const s of sensors) {
    if (q && !norm(`${s.hardware} ${s.name} ${TYPE_LABEL[s.type] ?? s.type}`).includes(q)) continue;
    const c = categoryOf(s);
    byCat.set(c, [...(byCat.get(c) ?? []), s]);
  }
  const rank = (t: string) => {
    const i = TYPE_ORDER.indexOf(t);
    return i < 0 ? TYPE_ORDER.length : i;
  };
  return CATEGORIES.filter((c) => byCat.has(c.id)).map((c) => {
    const items = byCat.get(c.id)!;
    const perHardware = ["fans", "voltage", "power"].includes(c.id);
    const m = new Map<string, Sensor[]>();
    for (const s of items) {
      const k = perHardware ? s.hardware : s.type;
      m.set(k, [...(m.get(k) ?? []), s]);
    }
    const sections = [...m.entries()]
      .sort((a, b) => (perHardware ? a[0].localeCompare(b[0]) : rank(a[0]) - rank(b[0])))
      .map(([k, list]) => ({ title: perHardware ? k : (TYPE_LABEL[k] ?? k), items: list }));
    return { id: c.id, label: c.label, sections, count: items.length };
  });
}

/** Température représentative d'une catégorie (package/cœur/point chaud si présent, sinon la plus haute). */
export function headlineTemp(sensors: Sensor[], cat: CategoryId): Sensor | null {
  const temps = sensors.filter((s) => s.type === "Temperature" && categoryOf(s) === cat);
  if (!temps.length) return null;
  const pref = temps.find((s) => /package|gpu core|tctl|composite|temperature$/i.test(s.name));
  return pref ?? temps.reduce((m, s) => (s.value > m.value ? s : m));
}

/** Remplissage de jauge 0..100 pour les mesures en pourcentage ou en température (sinon null). */
export function gaugePercent(s: Pick<Sensor, "type" | "value">): number | null {
  const v = s.type === "Temperature" ? (s.value / 110) * 100 : s.type === "Load" || s.type === "Control" ? s.value : null;
  return v == null ? null : Math.max(0, Math.min(100, v));
}
