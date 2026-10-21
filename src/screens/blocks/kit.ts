// Briques communes des blocs prêts à l'emploi : constructeurs de widgets (sans id) et mise à l'échelle.
import type { ColorRamp, Fit, Layout, Widget } from "../types.ts";
import { MAX_WIDGETS } from "../types.ts";
import { newId } from "../widgets.ts";

/** Widget sans identifiant : l'id est attribué à l'insertion. */
export type Proto = Widget extends infer W ? (W extends Widget ? Omit<W, "id"> : never) : never;

export interface BlockParam {
  label: string;
  options: string[];
  default: string;
}

export interface BlockCtx {
  accent: string;
  /** Valeur du paramètre choisi (lettre de disque, n° de SSD ou de ventilateur). */
  param: string;
}

export interface BlockDef {
  id: string;
  label: string;
  category: string;
  /** Taille de conception du bloc, en unités ; mise à l'échelle à l'insertion. */
  w: number;
  h: number;
  param?: BlockParam;
  build: (ctx: BlockCtx) => Proto[];
}

export const CARD = "#16161c";
export const TRACK = "#2a2a36";
export const DIM = "#9a9aa8";
export const DANGER = "#ff3b30";

const base = { rotation: 0, opacity: 1 };

export const ramp = (accent: string): ColorRamp => [
  [0, accent],
  [0.75, accent],
  [1, DANGER],
];

export interface TextOpts {
  align?: "left" | "center" | "right";
  bold?: boolean;
}

const style = (size: number, color: string, o: TextOpts) => ({ font: "Inter", size, color, align: o.align ?? "center", bold: o.bold ?? true });

export const txt = (x: number, y: number, w: number, h: number, text: string, size: number, color = "#ffffff", o: TextOpts = {}): Proto => ({
  ...base, x, y, w, h, type: "text", text, style: style(size, color, o),
});

export const val = (x: number, y: number, w: number, h: number, source: string, format: string, size: number, color = "#ffffff", o: TextOpts = {}): Proto => ({
  ...base, x, y, w, h, type: "value", source, format, style: style(size, color, o),
});

export const card = (x: number, y: number, w: number, h: number, radius = 16): Proto => ({
  ...base, x, y, w, h, type: "rect", fill: CARD, stroke: "#ffffff14", stroke_width: 1, radius,
});

export const arc = (x: number, y: number, s: number, source: string, accent: string, thickness: number, min = 0, max = 100): Proto => ({
  ...base, x, y, w: s, h: s, type: "gauge_arc", source, min, max, start_deg: 135, sweep_deg: 270, thickness, track: TRACK, colors: ramp(accent),
});

export const bar = (x: number, y: number, w: number, h: number, source: string, accent: string, min = 0, max = 100, vertical = false): Proto => ({
  ...base, x, y, w, h, type: "bar", source, min, max, vertical, radius: Math.min(w, h) / 2, track: TRACK, colors: ramp(accent),
});

export const graph = (x: number, y: number, w: number, h: number, source: string, line: string, min: number, max: number, seconds = 60): Proto => ({
  ...base, x, y, w, h, type: "graph", source, min, max, seconds, line, fill: `${line}33`, thickness: 2,
});

export const image = (x: number, y: number, w: number, h: number, mediaId = "", fit: Fit = "contain"): Proto => ({
  ...base, x, y, w, h, type: "image", media_id: mediaId, fit,
});

/** Mise à l'échelle uniforme de toutes les dimensions d'un widget. */
function scaled(p: Proto, k: number): Proto {
  const out = { ...p, x: p.x * k, y: p.y * k, w: p.w * k, h: p.h * k } as Proto;
  if (out.type === "text" || out.type === "value") out.style = { ...out.style, size: Math.max(6, Math.round(out.style.size * k)) };
  else if (out.type === "gauge_arc") out.thickness = Math.max(1, out.thickness * k);
  else if (out.type === "bar") out.radius *= k;
  else if (out.type === "graph") out.thickness = Math.max(1, out.thickness * k);
  else if (out.type === "rect") {
    out.radius *= k;
    out.stroke_width = Math.max(1, out.stroke_width * k);
  }
  return out;
}

export const defaultParam = (def: BlockDef): string => def.param?.default ?? "";

/** Widgets d'un bloc, mis à l'échelle (au plus la moitié du canevas dans chaque sens) et centrés sur `at` (ou le canevas). */
export function instantiateBlock(def: BlockDef, ctx: BlockCtx, canvas: Pick<Layout, "width" | "height">, at?: { x: number; y: number }): Widget[] {
  const k = Math.min((canvas.width * 0.5) / def.w, (canvas.height * 0.5) / def.h);
  const ox = (at ? at.x : canvas.width / 2) - (def.w * k) / 2;
  const oy = (at ? at.y : canvas.height / 2) - (def.h * k) / 2;
  return def
    .build(ctx)
    .slice(0, MAX_WIDGETS)
    .map((p) => {
      const s = scaled(p, k);
      return { ...s, id: newId(), x: Math.round(s.x + ox), y: Math.round(s.y + oy), w: Math.round(s.w), h: Math.round(s.h) } as Widget;
    });
}
