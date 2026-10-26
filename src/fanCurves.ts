// Préréglages de courbes ventilateur (température °C -> vitesse %). Jamais sous 20 % (arrêt de pompe AIO).
import type { CurvePoint } from "./types";

export interface CurvePreset {
  id: string;
  label: string;
  points: CurvePoint[];
}

export const CURVE_PRESETS: CurvePreset[] = [
  { id: "silent", label: "Silencieux", points: [{ temp: 40, duty: 20 }, { temp: 60, duty: 30 }, { temp: 75, duty: 55 }, { temp: 88, duty: 100 }] },
  { id: "balanced", label: "Équilibré", points: [{ temp: 30, duty: 30 }, { temp: 50, duty: 45 }, { temp: 70, duty: 70 }, { temp: 85, duty: 100 }] },
  { id: "performance", label: "Performance", points: [{ temp: 30, duty: 40 }, { temp: 45, duty: 60 }, { temp: 60, duty: 80 }, { temp: 75, duty: 100 }] },
];

/** Copie profonde d'un préréglage (l'éditeur modifie ses points). */
export function presetPoints(id: string): CurvePoint[] {
  const p = CURVE_PRESETS.find((x) => x.id === id);
  return p ? p.points.map((q) => ({ ...q })) : [];
}
