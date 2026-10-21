// Designs de démonstration (mode démo uniquement ; la vraie bibliothèque est embarquée côté Rust).
import type { Layout, TextStyle, Widget } from "./types.ts";
import { blankLayout, makeWidget, DEFAULT_RAMP } from "./widgets.ts";

type Partial_ = Record<string, unknown>;

function add(l: Layout, type: Widget["type"], x: number, y: number, w: number, h: number, extra: Partial_ = {}): void {
  const wd = makeWidget(type, l);
  Object.assign(wd, { x, y, w, h }, extra);
  l.widgets.push(wd);
}

const style = (size: number, color = "#ffffff", align: TextStyle["align"] = "center"): TextStyle => ({
  font: "Inter",
  size,
  color,
  align,
  bold: true,
});

function moniteur(): Layout {
  const l = blankLayout(480, 480, "Moniteur CPU / GPU");
  l.background = { kind: "gradient", from: "#101018", to: "#1c1228", angle: 160 };
  add(l, "gauge_arc", 20, 60, 210, 210, { source: "cpu.load", thickness: 18 });
  add(l, "gauge_arc", 250, 60, 210, 210, { source: "gpu.load", thickness: 18 });
  add(l, "value", 20, 135, 210, 60, { source: "cpu.temp", format: "{:.0}°C", style: style(52) });
  add(l, "value", 250, 135, 210, 60, { source: "gpu.temp", format: "{:.0}°C", style: style(52) });
  add(l, "text", 20, 20, 210, 30, { text: "CPU", style: style(26, "#ff5000") });
  add(l, "text", 250, 20, 210, 30, { text: "GPU", style: style(26, "#3ea6ff") });
  add(l, "bar", 30, 310, 420, 22, { source: "ram.used_pct" });
  add(l, "text", 30, 280, 200, 26, { text: "Mémoire", style: style(20, "#8b8898", "left") });
  add(l, "value", 30, 380, 200, 70, { source: "time", format: "%H:%M", style: style(60, "#ffffff", "left") });
  add(l, "value", 250, 400, 200, 40, { source: "date", format: "%d/%m/%Y", style: style(28, "#8b8898", "right") });
  return l;
}

function minimal(): Layout {
  const l = blankLayout(480, 480, "Minimal horloge");
  l.background = { kind: "solid", color: "#000000" };
  add(l, "value", 40, 170, 400, 140, { source: "time", format: "%H:%M", style: style(130, "#ffffff") });
  add(l, "value", 40, 320, 400, 40, { source: "date", format: "%d/%m/%Y", style: style(30, "#8b8898") });
  return l;
}

function neon(): Layout {
  const l = blankLayout(480, 480, "Néon rétro");
  l.background = { kind: "gradient", from: "#1a0033", to: "#05001a", angle: 180 };
  add(l, "rect", 40, 40, 400, 400, { fill: "#00000000", stroke: "#ff2bd6", stroke_width: 4, radius: 24 });
  add(l, "gauge_arc", 90, 90, 300, 300, {
    source: "gpu.temp",
    min: 20,
    max: 100,
    thickness: 24,
    track: "#2a1050",
    colors: [[0, "#00f0ff"], [0.6, "#ff2bd6"], [1, "#ff2b2b"]],
  });
  add(l, "value", 90, 200, 300, 80, { source: "gpu.temp", format: "{:.0}°", style: style(80, "#00f0ff") });
  add(l, "text", 90, 290, 300, 30, { text: "GPU", style: style(24, "#ff2bd6") });
  return l;
}

function horloge(): Layout {
  const l = blankLayout(480, 480, "Horloge analogique");
  l.background = { kind: "solid", color: "#0d0d10" };
  add(l, "analog_clock", 60, 60, 360, 360);
  return l;
}

function courbes(): Layout {
  const l = blankLayout(480, 480, "Courbes température");
  l.background = { kind: "solid", color: "#0b1118" };
  add(l, "text", 30, 20, 420, 34, { text: "Températures", style: style(26, "#8b8898", "left") });
  add(l, "graph", 30, 70, 420, 160, { source: "cpu.temp", line: "#ff5000", fill: "#ff500033" });
  add(l, "graph", 30, 260, 420, 160, { source: "gpu.temp", line: "#3ea6ff", fill: "#3ea6ff33" });
  return l;
}

function barres(): Layout {
  const l = blankLayout(480, 480, "Barres pro");
  l.background = { kind: "solid", color: "#14141a" };
  const rows: [string, string][] = [["CPU", "cpu.load"], ["GPU", "gpu.load"], ["RAM", "ram.used_pct"], ["VRAM", "gpu.vram_used_pct"]];
  rows.forEach(([name, src], i) => {
    add(l, "text", 30, 40 + i * 100, 140, 30, { text: name, style: style(24, "#ffffff", "left") });
    add(l, "value", 330, 40 + i * 100, 120, 30, { source: src, format: "{:.0}%", style: style(24, "#8b8898", "right") });
    add(l, "bar", 30, 76 + i * 100, 420, 22, { source: src, colors: DEFAULT_RAMP });
  });
  return l;
}

const MAKERS: [string, string, () => Layout, string[]][] = [
  ["demo-moniteur", "Moniteur", moniteur, ["cpu", "gpu"]],
  ["demo-minimal", "Minimal", minimal, ["horloge"]],
  ["demo-neon", "Rétro / Néon", neon, ["gpu"]],
  ["demo-horloge", "Horloge", horloge, ["analogique"]],
  ["demo-courbes", "Moniteur", courbes, ["historique"]],
  ["demo-barres", "Pro / Sobre", barres, ["barres"]],
];

export const MOCK_DESIGNS: Map<string, { category: string; tags: string[]; layout: Layout }> = new Map(
  MAKERS.map(([id, category, make, tags]) => [id, { category, tags, layout: make() }]),
);
