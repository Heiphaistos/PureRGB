import { FAN_CATALOG } from "./fanCatalog.ts";

// Nombre de LEDs adressables par unité (1 ventilateur, ou 1m de bandeau pour
// les entrées génériques) — sourcé depuis la fiche produit officielle.
// Détection matérielle automatique impossible sur un header 3-pin passif
// (WS281x-like unidirectionnel, aucune voie de retour) : ceci est
// l'équivalent pratique — l'utilisateur choisit son modèle au lieu de
// compter les LEDs à la main.
export interface FanPreset {
  brand: string;
  model: string;
  ledsPerUnit: number;
  /** Page du constructeur où le nombre de LEDs est écrit. */
  source?: string;
}

/** Catalogue généré (documentation constructeur) + une entrée générique pour tout le reste. */
export const FAN_PRESETS: FanPreset[] = [
  ...FAN_CATALOG,
  // Entrée générique pour tout le reste (bandeau, ventilo non listé) :
  // l'utilisateur renseigne lui-même les LEDs/unité depuis la fiche produit.
  { brand: "Générique", model: "Autre (LEDs/unité personnalisé)", ledsPerUnit: 1 },
];

/** Modèles regroupés par marque, pour le sélecteur. */
export const FAN_GROUPS: { brand: string; items: FanPreset[] }[] = Object.values(
  FAN_PRESETS.reduce<Record<string, { brand: string; items: FanPreset[] }>>((acc, p) => {
    (acc[p.brand] ??= { brand: p.brand, items: [] }).items.push(p);
    return acc;
  }, {}),
);

export function ledsFor(preset: FanPreset, qty: number): number {
  if (!Number.isFinite(qty) || qty <= 0) return 0;
  return Math.round(preset.ledsPerUnit * qty);
}
