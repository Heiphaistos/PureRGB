// Catalogue des widgets : libellés, icônes, valeurs par défaut, sources de données.
import type { ColorRamp, Layout, TextStyle, Widget, WidgetType } from "./types.ts";
import { LAYOUT_VERSION } from "./types.ts";

export const WIDGET_TYPES: WidgetType[] = [
  "text",
  "value",
  "gauge_arc",
  "bar",
  "graph",
  "rect",
  "image",
  "analog_clock",
];

export const WIDGET_LABELS: Record<WidgetType, string> = {
  text: "Texte",
  value: "Valeur live",
  gauge_arc: "Jauge circulaire",
  bar: "Barre",
  graph: "Courbe",
  rect: "Rectangle",
  image: "Image",
  analog_clock: "Horloge analogique",
};

/** Chemins SVG (viewBox 24, trait) pour la palette. */
export const WIDGET_ICONS: Record<WidgetType, string[]> = {
  text: ["M5 6h14", "M12 6v13", "M9 19h6"],
  value: ["M4 17l4-6 4 3 4-7 4 5", "M4 20h16"],
  gauge_arc: ["M5 17a8 8 0 1 1 14 0", "M12 17l4-5"],
  bar: ["M4 9h16v6H4z", "M4 9h9v6H4z"],
  graph: ["M3 18l5-7 4 4 5-8 4 5", "M3 21h18"],
  rect: ["M4 6h16v12H4z"],
  image: ["M4 5h16v14H4z", "M4 16l5-5 4 4 3-3 4 4", "M9 9h.01"],
  analog_clock: ["M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0 -18 0", "M12 7v5l3 2"],
};

/** Polices suggérées (le moteur accepte aussi une police de la médiathèque). */
export const FONTS = ["Inter", "Roboto Mono", "Rajdhani"];

export interface SourceDef {
  key: string;
  label: string;
  unit: string;
  group: string;
}

export const SOURCES: SourceDef[] = [
  { key: "cpu.temp", label: "CPU température", unit: "°C", group: "Processeur" },
  { key: "cpu.load", label: "CPU charge", unit: "%", group: "Processeur" },
  { key: "cpu.clock", label: "CPU fréquence", unit: "MHz", group: "Processeur" },
  { key: "cpu.power", label: "CPU puissance", unit: "W", group: "Processeur" },
  { key: "cpu.fan", label: "CPU ventilateur", unit: "tr/min", group: "Processeur" },
  { key: "gpu.temp", label: "GPU température", unit: "°C", group: "Carte graphique" },
  { key: "gpu.load", label: "GPU charge", unit: "%", group: "Carte graphique" },
  { key: "gpu.clock", label: "GPU fréquence", unit: "MHz", group: "Carte graphique" },
  { key: "gpu.power", label: "GPU puissance", unit: "W", group: "Carte graphique" },
  { key: "gpu.vram_used_pct", label: "GPU VRAM utilisée", unit: "%", group: "Carte graphique" },
  { key: "gpu.fan", label: "GPU ventilateur", unit: "tr/min", group: "Carte graphique" },
  { key: "ram.used_pct", label: "RAM utilisée", unit: "%", group: "Mémoire" },
  { key: "ram.used_gb", label: "RAM utilisée", unit: "Go", group: "Mémoire" },
  { key: "ram.total_gb", label: "RAM totale", unit: "Go", group: "Mémoire" },
  { key: "cpu.voltage", label: "CPU tension", unit: "V", group: "Processeur" },
  { key: "gpu.hotspot", label: "GPU point chaud", unit: "°C", group: "Carte graphique" },
  { key: "gpu.mem_clock", label: "GPU fréquence mémoire", unit: "MHz", group: "Carte graphique" },
  { key: "gpu.vram_used_gb", label: "GPU VRAM utilisée", unit: "Go", group: "Carte graphique" },
  { key: "ram.free_gb", label: "RAM libre", unit: "Go", group: "Mémoire" },
  { key: "mobo.temp", label: "Carte mère température", unit: "°C", group: "Carte mère" },
  { key: "chipset.temp", label: "Chipset température", unit: "°C", group: "Carte mère" },
  { key: "vrm.temp", label: "VRM température", unit: "°C", group: "Carte mère" },
  { key: "ssd.1.temp", label: "SSD 1 température (n° = 1…)", unit: "°C", group: "Stockage" },
  { key: "ssd.1.used_pct", label: "SSD 1 occupé (n° = 1…)", unit: "%", group: "Stockage" },
  { key: "disk.C.used_pct", label: "Disque C: occupé (lettre = C…)", unit: "%", group: "Stockage" },
  { key: "disk.C.free_gb", label: "Disque C: libre (lettre = C…)", unit: "Go", group: "Stockage" },
  { key: "disk.C.total_gb", label: "Disque C: taille (lettre = C…)", unit: "Go", group: "Stockage" },
  { key: "disk.read", label: "Disques lecture", unit: "Mo/s", group: "Stockage" },
  { key: "disk.write", label: "Disques écriture", unit: "Mo/s", group: "Stockage" },
  { key: "fan.1.rpm", label: "Ventilateur 1 (n° = 1…)", unit: "tr/min", group: "Refroidissement" },
  { key: "liquid.temp", label: "Liquide température", unit: "°C", group: "Refroidissement" },
  { key: "pump.rpm", label: "Pompe", unit: "tr/min", group: "Refroidissement" },
  { key: "net.up", label: "Réseau envoi", unit: "Mo/s", group: "Réseau" },
  { key: "net.down", label: "Réseau réception", unit: "Mo/s", group: "Réseau" },
  { key: "time", label: "Heure", unit: "", group: "Horloge" },
  { key: "date", label: "Date", unit: "", group: "Horloge" },
  { key: "uptime", label: "Temps de fonctionnement", unit: "", group: "Horloge" },
];

export function sourceLabel(key: string): string {
  return SOURCES.find((s) => s.key === key)?.label ?? key;
}

export const DEFAULT_RAMP: ColorRamp = [
  [0, "#2ee06e"],
  [0.7, "#ffb020"],
  [1, "#ff3b30"],
];

export const FORMAT_HELP = "Ex. {:.0}°C · {:.1} GHz · {:.0}% — pour l'heure/date : %H:%M, %d/%m/%Y";

export function newId(): string {
  return `w_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

function textStyle(size: number, color = "#ffffff"): TextStyle {
  return { font: "Inter", size, color, align: "center", bold: true };
}

/** Widget par défaut, centré dans le canevas. */
export function makeWidget(type: WidgetType, canvas: { width: number; height: number }): Widget {
  const base = { id: newId(), x: 0, y: 0, w: 120, h: 60, rotation: 0, opacity: 1 };
  const m = Math.min(canvas.width, canvas.height);
  let w: Widget;
  switch (type) {
    case "text":
      w = { ...base, w: m * 0.4, h: m * 0.1, type, text: "Texte", style: textStyle(Math.round(m * 0.07)) };
      break;
    case "value":
      w = {
        ...base,
        w: m * 0.4,
        h: m * 0.18,
        type,
        source: "cpu.temp",
        format: "{:.0}°C",
        style: textStyle(Math.round(m * 0.14)),
      };
      break;
    case "gauge_arc":
      w = {
        ...base,
        w: m * 0.4,
        h: m * 0.4,
        type,
        source: "cpu.load",
        min: 0,
        max: 100,
        start_deg: 135,
        sweep_deg: 270,
        thickness: Math.round(m * 0.04),
        track: "#2a2a36",
        colors: DEFAULT_RAMP.map(([p, c]) => [p, c]),
      };
      break;
    case "bar":
      w = {
        ...base,
        w: m * 0.5,
        h: m * 0.06,
        type,
        source: "gpu.load",
        min: 0,
        max: 100,
        vertical: false,
        radius: Math.round(m * 0.03),
        track: "#2a2a36",
        colors: DEFAULT_RAMP.map(([p, c]) => [p, c]),
      };
      break;
    case "graph":
      w = {
        ...base,
        w: m * 0.5,
        h: m * 0.25,
        type,
        source: "cpu.temp",
        min: 20,
        max: 100,
        seconds: 60,
        line: "#ff5000",
        fill: "#ff500033",
        thickness: 2,
      };
      break;
    case "rect":
      w = { ...base, w: m * 0.4, h: m * 0.2, type, fill: "#1c1c24", stroke: null, stroke_width: 2, radius: 12 };
      break;
    case "image":
      w = { ...base, w: m * 0.3, h: m * 0.3, type, media_id: "", fit: "contain" };
      break;
    case "analog_clock":
      w = { ...base, w: m * 0.5, h: m * 0.5, type, face: "#16161c", hands: "#ffffff", accent: "#ff5000" };
      break;
  }
  w.w = Math.round(w.w);
  w.h = Math.round(w.h);
  w.x = Math.round((canvas.width - w.w) / 2);
  w.y = Math.round((canvas.height - w.h) / 2);
  return w;
}

export function blankLayout(width: number, height: number, name = "Nouveau design"): Layout {
  return {
    version: LAYOUT_VERSION,
    name,
    width,
    height,
    background: { kind: "solid", color: "#0d0d10" },
    widgets: [],
  };
}

export function cloneLayout(l: Layout): Layout {
  return JSON.parse(JSON.stringify(l)) as Layout;
}
