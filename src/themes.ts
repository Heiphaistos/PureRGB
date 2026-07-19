import type { Color, EffectConfig } from "./types";

export interface Theme {
  id: string;
  name: string;
  emoji: string;
  description: string;
  config: EffectConfig;
}

const c = (r: number, g: number, b: number): Color => ({ r, g, b });
const mk = (
  kind: EffectConfig["kind"],
  colors: Color[],
  speed = 1.0,
  brightness = 1.0,
): EffectConfig => ({ kind, colors, speed, brightness, reverse: false });

/** Profils prédéfinis appliqués à tous les appareils d'un coup. */
export const THEMES: Theme[] = [
  { id: "cyberpunk", name: "Cyberpunk", emoji: "🌃", description: "Magenta et cyan néon", config: mk("gradient", [c(255, 0, 200), c(0, 220, 255)]) },
  { id: "synthwave", name: "Synthwave", emoji: "🕶️", description: "Comète magenta rétro", config: mk("comet", [c(255, 40, 180)], 1.2) },
  { id: "vaporwave", name: "Vaporwave", emoji: "🌴", description: "Respiration rose-violet", config: mk("breathing", [c(200, 120, 255)], 0.6) },
  { id: "ocean", name: "Océan", emoji: "🌊", description: "Vagues bleu-cyan", config: mk("color_wave", [c(0, 60, 255), c(0, 220, 210)], 0.7) },
  { id: "lava", name: "Lave", emoji: "🌋", description: "Dégradé rouge-orange incandescent", config: mk("gradient", [c(255, 20, 0), c(255, 160, 0)]) },
  { id: "forest", name: "Forêt", emoji: "🌲", description: "Respiration verte apaisante", config: mk("breathing", [c(20, 200, 80)], 0.5) },
  { id: "aurora", name: "Aurore boréale", emoji: "🌌", description: "Vagues vert-violet polaires", config: mk("color_wave", [c(0, 255, 140), c(150, 60, 255)], 0.5) },
  { id: "sunset", name: "Coucher de soleil", emoji: "🌅", description: "Dégradé orange-rose", config: mk("gradient", [c(255, 110, 0), c(255, 60, 130)]) },
  { id: "ice", name: "Glace", emoji: "❄️", description: "Respiration blanc-bleuté", config: mk("breathing", [c(180, 220, 255)], 0.4) },
  { id: "matrix", name: "Matrix", emoji: "💻", description: "Comète verte", config: mk("comet", [c(0, 255, 70)], 1.4) },
  { id: "rainbow-wave", name: "Arc-en-ciel fluide", emoji: "🌈", description: "Vague multicolore", config: mk("rainbow_wave", [], 1.0) },
  { id: "rainbow-calm", name: "Arc-en-ciel calme", emoji: "🫧", description: "Cycle lent de toutes les teintes", config: mk("rainbow_cycle", [], 0.3) },
  { id: "gaming-red", name: "Rouge gaming", emoji: "🎮", description: "Rouge intense fixe", config: mk("static", [c(255, 10, 10)]) },
  { id: "deep-blue", name: "Bleu profond", emoji: "🌀", description: "Bleu nuit fixe", config: mk("static", [c(10, 40, 200)]) },
  { id: "royal-purple", name: "Violet royal", emoji: "👑", description: "Respiration violette", config: mk("breathing", [c(140, 30, 255)], 0.6) },
  { id: "gold", name: "Or", emoji: "🏆", description: "Doré fixe", config: mk("static", [c(255, 180, 20)]) },
  { id: "sakura", name: "Rose sakura", emoji: "🌸", description: "Dégradé rose-blanc", config: mk("gradient", [c(255, 130, 190), c(255, 230, 240)]) },
  { id: "halloween", name: "Halloween", emoji: "🎃", description: "Vagues orange-violet", config: mk("color_wave", [c(255, 120, 0), c(120, 0, 200)], 0.8) },
  { id: "christmas", name: "Noël", emoji: "🎄", description: "Vagues rouge-vert", config: mk("color_wave", [c(255, 20, 20), c(0, 200, 60)], 0.6) },
  { id: "france", name: "France", emoji: "🇫🇷", description: "Dégradé bleu-rouge", config: mk("gradient", [c(0, 60, 255), c(255, 30, 30)]) },
  { id: "pure-white", name: "Blanc pur", emoji: "💡", description: "Éclairage blanc neutre", config: mk("static", [c(255, 255, 255)]) },
  { id: "campfire", name: "Feu de camp", emoji: "🔥", description: "Respiration orangée chaleureuse", config: mk("breathing", [c(255, 120, 20)], 1.6) },
  { id: "night", name: "Nuit étoilée", emoji: "✨", description: "Comète blanche discrète", config: mk("comet", [c(255, 255, 255)], 0.6, 0.5) },
  { id: "pastel", name: "Pastel", emoji: "🍬", description: "Dégradé menthe-lilas doux", config: mk("gradient", [c(150, 255, 220), c(220, 180, 255)], 1.0, 0.8) },
];
