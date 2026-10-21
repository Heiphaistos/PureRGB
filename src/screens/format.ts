// Formatage de valeurs et couleurs (pur). Sert au mode démo et à l'inspecteur.
import type { ColorRamp } from "./types.ts";

const pad = (n: number, w = 2): string => String(n).padStart(w, "0");

/** Sous-ensemble de strftime : %H %M %S %d %m %Y %y. */
export function strftime(fmt: string, d: Date): string {
  return fmt.replace(/%([HMSdmYy])/g, (_, c: string) => {
    switch (c) {
      case "H":
        return pad(d.getHours());
      case "M":
        return pad(d.getMinutes());
      case "S":
        return pad(d.getSeconds());
      case "d":
        return pad(d.getDate());
      case "m":
        return pad(d.getMonth() + 1);
      case "Y":
        return String(d.getFullYear());
      default:
        return pad(d.getFullYear() % 100);
    }
  });
}

/** Applique un format `{:.1}` à une valeur ; `{}` = entier arrondi. */
export function formatValue(format: string, value: number): string {
  const m = format.match(/\{:\.(\d)\}/);
  if (!m) return format.replace("{}", String(Math.round(value)));
  return format.replace(m[0], value.toFixed(Number(m[1])));
}

export function isTimeSource(key: string): boolean {
  return key === "time" || key === "date" || key === "uptime";
}

export function parseHex(color: string): [number, number, number, number] | null {
  const m = color.trim().match(/^#([0-9a-f]{3,8})$/i);
  if (!m) return null;
  let h = m[1];
  if (h.length === 3 || h.length === 4) h = [...h].map((c) => c + c).join("");
  if (h.length !== 6 && h.length !== 8) return null;
  const n = (i: number): number => parseInt(h.slice(i, i + 2), 16);
  return [n(0), n(2), n(4), h.length === 8 ? n(6) / 255 : 1];
}

/** Partie `#rrggbb` utilisable par <input type=color>. */
export function toColorInput(color: string): string {
  const p = parseHex(color);
  if (!p) return "#000000";
  return `#${p
    .slice(0, 3)
    .map((v) => Math.round(v).toString(16).padStart(2, "0"))
    .join("")}`;
}

/** Couleur de la rampe à la position normalisée t (0..1), interpolée en RGB. */
export function sampleRamp(ramp: ColorRamp, t: number): string {
  if (ramp.length === 0) return "#ffffff";
  const sorted = [...ramp].sort((a, b) => a[0] - b[0]);
  const x = Math.min(1, Math.max(0, t));
  if (x <= sorted[0][0]) return sorted[0][1];
  for (let i = 1; i < sorted.length; i++) {
    const [p1, c1] = sorted[i];
    const [p0, c0] = sorted[i - 1];
    if (x <= p1) {
      const a = parseHex(c0);
      const b = parseHex(c1);
      if (!a || !b) return c1;
      const k = p1 === p0 ? 1 : (x - p0) / (p1 - p0);
      const ch = (j: number): number => Math.round(a[j] + (b[j] - a[j]) * k);
      return `rgb(${ch(0)}, ${ch(1)}, ${ch(2)})`;
    }
  }
  return sorted[sorted.length - 1][1];
}

/** CSS linear-gradient d'une rampe (aperçu dans l'inspecteur). */
export function rampCss(ramp: ColorRamp): string {
  if (ramp.length === 0) return "none";
  const sorted = [...ramp].sort((a, b) => a[0] - b[0]);
  return `linear-gradient(90deg, ${sorted.map(([p, c]) => `${c} ${Math.round(p * 100)}%`).join(", ")})`;
}
