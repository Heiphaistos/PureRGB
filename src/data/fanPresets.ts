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
}

export const FAN_PRESETS: FanPreset[] = [
  // Cooler Master — https://www.coolermaster.com/en-global/products/masterfan-mf120-prismatic.html
  // "tri-loop Addressable RGB" : 24 ARGB LEDs (12/côté) + 6 ARGB additionnelles = 30
  { brand: "Cooler Master", model: "MF120 Prismatic (tri-loop)", ledsPerUnit: 30 },
  // Corsair — https://www.corsair.com/us/en/p/case-fans/co-9050097-ww/icue-ql120-rgb-120mm-pwm-single-fan-co-9050097-ww
  // "34 individually addressable RGB LEDs across four distinct light loops"
  { brand: "Corsair", model: "QL120 RGB", ledsPerUnit: 34 },
  // Corsair — https://www.corsair.com/ww/en/p/case-fans/co-9050091-ww/ll120-rgb-120mm-dual-light-loop-white-rgb-led-pwm-fan-single-pack-co-9050091-ww
  // "16 individually addressable RGB LEDs spread across two separate light loops"
  { brand: "Corsair", model: "LL120 RGB", ledsPerUnit: 16 },
  // Corsair — https://www.corsair.com/us/en/p/case-fans/co-9050075-ww/ml120-pro-rgb-led-120mm-pwm-premium-magnetic-levitation-fan-single-pack-co-9050075-ww
  // "Four hub-mounted RGB LEDs"
  { brand: "Corsair", model: "ML120 PRO RGB", ledsPerUnit: 4 },
  // NZXT — https://nzxt.com/en-US/product/f120-core-rgb
  // "8 individually addressable RGB LEDs mounted on the fan hub"
  { brand: "NZXT", model: "F120 RGB Core", ledsPerUnit: 8 },
  // Lian Li — https://lian-li.com/product/uni-fan-sl/
  // "32 addressable RGB LEDs" (16 sur chaque face du cadre)
  { brand: "Lian Li", model: "UNI FAN SL120", ledsPerUnit: 32 },
  // Entrée générique pour tout le reste (bandeau, ventilo non listé) :
  // l'utilisateur renseigne lui-même les LEDs/unité depuis la fiche produit.
  { brand: "Générique", model: "Autre (LEDs/unité personnalisé)", ledsPerUnit: 1 },
];

export function ledsFor(preset: FanPreset, qty: number): number {
  if (!Number.isFinite(qty) || qty <= 0) return 0;
  return Math.round(preset.ledsPerUnit * qty);
}
