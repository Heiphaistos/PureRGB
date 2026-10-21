// Validation d'un layout reçu ou construit côté UI (miroir de Layout::validate en Rust + structure).
import type { Layout } from "./types.ts";
import { LAYOUT_VERSION, MAX_CANVAS, MAX_WIDGETS } from "./types.ts";

const WIDGET_TYPES = new Set(["text", "value", "gauge_arc", "bar", "graph", "rect", "image", "analog_clock"]);
const BG_KINDS = new Set(["solid", "gradient", "media"]);

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isNum(v: unknown): v is number {
  return typeof v === "number" && Number.isFinite(v);
}

/** Renvoie un message d'erreur en français, ou null si le layout est acceptable. */
export function validateLayout(input: unknown): string | null {
  if (!isObj(input)) return "design invalide (objet attendu)";
  if (input.version !== LAYOUT_VERSION) return `version de design non supportée : ${String(input.version)}`;
  if (typeof input.name !== "string") return "nom du design manquant";
  const { width, height } = input;
  if (!isNum(width) || !isNum(height) || width < 1 || height < 1 || width > MAX_CANVAS || height > MAX_CANVAS) {
    return "taille de canevas invalide";
  }
  if (!isObj(input.background) || typeof input.background.kind !== "string" || !BG_KINDS.has(input.background.kind)) {
    return "fond invalide";
  }
  if (!Array.isArray(input.widgets)) return "liste de widgets manquante";
  if (input.widgets.length > MAX_WIDGETS) return `trop de widgets (max ${MAX_WIDGETS})`;
  const ids = new Set<string>();
  for (const w of input.widgets) {
    if (!isObj(w)) return "widget invalide";
    if (typeof w.id !== "string" || w.id === "") return "widget sans identifiant";
    if (ids.has(w.id)) return `identifiant de widget en double : ${w.id}`;
    ids.add(w.id);
    if (typeof w.type !== "string" || !WIDGET_TYPES.has(w.type)) return `type de widget inconnu : ${String(w.type)}`;
    for (const k of ["x", "y", "w", "h", "rotation", "opacity"]) {
      if (!isNum(w[k])) return `widget ${w.id} : champ « ${k} » invalide`;
    }
  }
  return null;
}

export function isLayout(input: unknown): input is Layout {
  return validateLayout(input) === null;
}
