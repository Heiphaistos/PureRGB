import type { Color, EffectConfig } from "./types";

export type ThemeCategory =
  | "neon"
  | "nature"
  | "retro"
  | "seasonal"
  | "gaming"
  | "pastel"
  | "pro"
  | "dynamic";

export const THEME_CATEGORY_LABELS: Record<ThemeCategory, string> = {
  neon: "Néon / Cyberpunk",
  nature: "Nature",
  retro: "Rétro / Synthwave",
  seasonal: "Fêtes / Saisonnier",
  gaming: "Gaming / Compétitif",
  pastel: "Pastel / Doux",
  pro: "Sobre / Pro",
  dynamic: "Effets dynamiques",
};

export interface Theme {
  id: string;
  name: string;
  emoji: string;
  description: string;
  category: ThemeCategory;
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
  // --- Néon / Cyberpunk ---
  { id: "cyberpunk", name: "Cyberpunk", emoji: "🌃", description: "Magenta et cyan néon", category: "neon", config: mk("gradient", [c(255, 0, 200), c(0, 220, 255)]) },
  { id: "matrix", name: "Matrix", emoji: "💻", description: "Comète verte", category: "neon", config: mk("comet", [c(0, 255, 70)], 1.4) },
  { id: "night", name: "Nuit étoilée", emoji: "✨", description: "Comète blanche discrète", category: "neon", config: mk("comet", [c(255, 255, 255)], 0.6, 0.5) },
  { id: "neon-pink", name: "Rose néon", emoji: "💗", description: "Rose vif fixe", category: "neon", config: mk("static", [c(255, 20, 147)]) },
  { id: "tron-blue", name: "Tron bleu", emoji: "🔷", description: "Bleu électrique pulsé", category: "neon", config: mk("breathing", [c(20, 180, 255)], 1.1) },
  { id: "laser-green", name: "Laser vert", emoji: "🟢", description: "Comète verte rapide", category: "neon", config: mk("comet", [c(60, 255, 60)], 2.0) },
  { id: "hacker-red", name: "Hacker rouge", emoji: "🔴", description: "Vagues rouge-noir agressives", category: "neon", config: mk("color_wave", [c(255, 0, 0), c(40, 0, 0)], 1.0) },

  // --- Nature ---
  { id: "ocean", name: "Océan", emoji: "🌊", description: "Vagues bleu-cyan", category: "nature", config: mk("color_wave", [c(0, 60, 255), c(0, 220, 210)], 0.7) },
  { id: "forest", name: "Forêt", emoji: "🌲", description: "Respiration verte apaisante", category: "nature", config: mk("breathing", [c(20, 200, 80)], 0.5) },
  { id: "aurora", name: "Aurore boréale", emoji: "🌌", description: "Vagues vert-violet polaires", category: "nature", config: mk("color_wave", [c(0, 255, 140), c(150, 60, 255)], 0.5) },
  { id: "sunset", name: "Coucher de soleil", emoji: "🌅", description: "Dégradé orange-rose", category: "nature", config: mk("gradient", [c(255, 110, 0), c(255, 60, 130)]) },
  { id: "ice", name: "Glace", emoji: "❄️", description: "Respiration blanc-bleuté", category: "nature", config: mk("breathing", [c(180, 220, 255)], 0.4) },
  { id: "lava", name: "Lave", emoji: "🌋", description: "Dégradé rouge-orange incandescent", category: "nature", config: mk("gradient", [c(255, 20, 0), c(255, 160, 0)]) },
  { id: "campfire", name: "Feu de camp", emoji: "🔥", description: "Respiration orangée chaleureuse", category: "nature", config: mk("breathing", [c(255, 120, 20)], 1.6) },
  { id: "autumn", name: "Automne", emoji: "🍂", description: "Dégradé orange-marron feuillage", category: "nature", config: mk("gradient", [c(210, 100, 20), c(140, 60, 10)]) },
  { id: "spring", name: "Printemps", emoji: "🌷", description: "Vagues vert tendre-rose", category: "nature", config: mk("color_wave", [c(120, 220, 100), c(255, 150, 200)], 0.5) },
  { id: "desert", name: "Désert", emoji: "🏜️", description: "Dégradé sable-orange", category: "nature", config: mk("gradient", [c(230, 190, 110), c(200, 100, 40)]) },
  { id: "coral-reef", name: "Récif corail", emoji: "🐠", description: "Vagues corail-turquoise", category: "nature", config: mk("color_wave", [c(255, 100, 90), c(0, 200, 190)], 0.6) },

  // --- Rétro / Synthwave ---
  { id: "synthwave", name: "Synthwave", emoji: "🕶️", description: "Comète magenta rétro", category: "retro", config: mk("comet", [c(255, 40, 180)], 1.2) },
  { id: "vaporwave", name: "Vaporwave", emoji: "🌴", description: "Respiration rose-violet", category: "retro", config: mk("breathing", [c(200, 120, 255)], 0.6) },
  { id: "miami-nights", name: "Miami Nights", emoji: "🌆", description: "Dégradé rose-cyan", category: "retro", config: mk("gradient", [c(255, 60, 160), c(0, 220, 255)]) },
  { id: "outrun", name: "Outrun", emoji: "🚗", description: "Vagues violet-orange rapides", category: "retro", config: mk("color_wave", [c(150, 0, 200), c(255, 130, 0)], 1.3) },
  { id: "retro-arcade", name: "Arcade rétro", emoji: "🕹️", description: "Clignotement multicolore", category: "retro", config: mk("blink", [c(255, 0, 100), c(0, 200, 255), c(255, 220, 0)], 1.0) },
  { id: "vhs-glitch", name: "VHS Glitch", emoji: "📼", description: "Comète cyan-magenta saccadée", category: "retro", config: mk("comet", [c(0, 255, 255)], 2.2) },

  // --- Fêtes / Saisonnier ---
  { id: "halloween", name: "Halloween", emoji: "🎃", description: "Vagues orange-violet", category: "seasonal", config: mk("color_wave", [c(255, 120, 0), c(120, 0, 200)], 0.8) },
  { id: "christmas", name: "Noël", emoji: "🎄", description: "Vagues rouge-vert", category: "seasonal", config: mk("color_wave", [c(255, 20, 20), c(0, 200, 60)], 0.6) },
  { id: "france", name: "France", emoji: "🇫🇷", description: "Dégradé bleu-rouge", category: "seasonal", config: mk("gradient", [c(0, 60, 255), c(255, 30, 30)]) },
  { id: "valentine", name: "Saint-Valentin", emoji: "💘", description: "Respiration rouge-rose", category: "seasonal", config: mk("breathing", [c(255, 20, 90)], 0.7) },
  { id: "st-patrick", name: "Saint-Patrick", emoji: "☘️", description: "Vert fixe intense", category: "seasonal", config: mk("static", [c(0, 200, 80)]) },
  { id: "easter", name: "Pâques", emoji: "🐣", description: "Dégradé pastel jaune-violet", category: "seasonal", config: mk("gradient", [c(255, 240, 150), c(200, 150, 255)]) },
  { id: "new-year", name: "Nouvel An", emoji: "🎆", description: "Clignotement or-blanc festif", category: "seasonal", config: mk("blink", [c(255, 215, 0), c(255, 255, 255)], 1.4) },
  { id: "summer", name: "Été", emoji: "☀️", description: "Dégradé jaune-turquoise", category: "seasonal", config: mk("gradient", [c(255, 220, 0), c(0, 210, 200)]) },

  // --- Gaming / Compétitif ---
  { id: "gaming-red", name: "Rouge gaming", emoji: "🎮", description: "Rouge intense fixe", category: "gaming", config: mk("static", [c(255, 10, 10)]) },
  { id: "blue-team", name: "Équipe bleue", emoji: "🔵", description: "Bleu fixe intense", category: "gaming", config: mk("static", [c(20, 90, 255)]) },
  { id: "red-team", name: "Équipe rouge", emoji: "🟥", description: "Rouge respirant", category: "gaming", config: mk("breathing", [c(255, 20, 20)], 1.0) },
  { id: "esport-purple", name: "Violet esport", emoji: "🟣", description: "Violet fixe saturé", category: "gaming", config: mk("static", [c(140, 0, 255)]) },
  { id: "stealth-green", name: "Vert furtif", emoji: "🥷", description: "Vert sombre discret", category: "gaming", config: mk("static", [c(10, 90, 30)], 1.0, 0.6) },
  { id: "victory-gold", name: "Or victoire", emoji: "🏅", description: "Doré pulsé", category: "gaming", config: mk("breathing", [c(255, 190, 20)], 1.2) },

  // --- Pastel / Doux ---
  { id: "sakura", name: "Rose sakura", emoji: "🌸", description: "Dégradé rose-blanc", category: "pastel", config: mk("gradient", [c(255, 130, 190), c(255, 230, 240)]) },
  { id: "pastel", name: "Pastel", emoji: "🍬", description: "Dégradé menthe-lilas doux", category: "pastel", config: mk("gradient", [c(150, 255, 220), c(220, 180, 255)], 1.0, 0.8) },
  { id: "cotton-candy", name: "Barbe à papa", emoji: "🍭", description: "Vagues rose-bleu pastel", category: "pastel", config: mk("color_wave", [c(255, 180, 220), c(180, 210, 255)], 0.4) },
  { id: "baby-blue", name: "Bleu bébé", emoji: "🩵", description: "Bleu pastel fixe", category: "pastel", config: mk("static", [c(170, 210, 255)], 1.0, 0.7) },
  { id: "lavender", name: "Lavande", emoji: "💜", description: "Respiration violette douce", category: "pastel", config: mk("breathing", [c(200, 170, 255)], 0.4, 0.7) },
  { id: "peach", name: "Pêche", emoji: "🍑", description: "Dégradé pêche-abricot", category: "pastel", config: mk("gradient", [c(255, 200, 170), c(255, 170, 130)], 1.0, 0.8) },

  // --- Sobre / Pro ---
  { id: "deep-blue", name: "Bleu profond", emoji: "🌀", description: "Bleu nuit fixe", category: "pro", config: mk("static", [c(10, 40, 200)]) },
  { id: "royal-purple", name: "Violet royal", emoji: "👑", description: "Respiration violette", category: "pro", config: mk("breathing", [c(140, 30, 255)], 0.6) },
  { id: "gold", name: "Or", emoji: "🏆", description: "Doré fixe", category: "pro", config: mk("static", [c(255, 180, 20)]) },
  { id: "pure-white", name: "Blanc pur", emoji: "💡", description: "Éclairage blanc neutre", category: "pro", config: mk("static", [c(255, 255, 255)]) },
  { id: "monochrome", name: "Monochrome", emoji: "⚪", description: "Blanc respirant discret", category: "pro", config: mk("breathing", [c(220, 220, 220)], 0.3, 0.6) },
  { id: "graphite", name: "Graphite", emoji: "⚫", description: "Gris-bleu sobre fixe", category: "pro", config: mk("static", [c(90, 100, 110)], 1.0, 0.5) },
  { id: "ivory", name: "Ivoire", emoji: "🤍", description: "Blanc chaud discret", category: "pro", config: mk("static", [c(245, 235, 210)], 1.0, 0.5) },
  { id: "midnight", name: "Minuit", emoji: "🌑", description: "Bleu très sombre respirant", category: "pro", config: mk("breathing", [c(10, 15, 50)], 0.3, 0.4) },

  // --- Effets dynamiques ---
  { id: "rainbow-wave", name: "Arc-en-ciel fluide", emoji: "🌈", description: "Vague multicolore", category: "dynamic", config: mk("rainbow_wave", [], 1.0) },
  { id: "rainbow-calm", name: "Arc-en-ciel calme", emoji: "🫧", description: "Cycle lent de toutes les teintes", category: "dynamic", config: mk("rainbow_cycle", [], 0.3) },
  { id: "strobe-party", name: "Strobe fête", emoji: "🪩", description: "Clignotement multicolore rapide", category: "dynamic", config: mk("blink", [c(255, 0, 0), c(0, 255, 0), c(0, 100, 255), c(255, 255, 0)], 2.5) },
  { id: "police-lights", name: "Gyrophare police", emoji: "🚨", description: "Clignotement bleu-rouge alterné", category: "dynamic", config: mk("blink", [c(255, 0, 0), c(0, 60, 255)], 3.0) },
  { id: "fire-flicker", name: "Feu vacillant", emoji: "🔥", description: "Comète orange-rouge instable", category: "dynamic", config: mk("comet", [c(255, 90, 0)], 1.8) },
  { id: "lightning", name: "Éclair", emoji: "⚡", description: "Clignotement blanc-bleu violent", category: "dynamic", config: mk("blink", [c(255, 255, 255), c(150, 180, 255)], 3.5) },
  { id: "pulse-wave", name: "Pulse", emoji: "💓", description: "Respiration rouge-rose rapide", category: "dynamic", config: mk("breathing", [c(255, 30, 60)], 2.0) },
  { id: "kaleidoscope", name: "Kaléidoscope", emoji: "🔮", description: "Vagues arc-en-ciel rapides", category: "dynamic", config: mk("color_wave", [c(255, 0, 150), c(0, 200, 255), c(255, 220, 0)], 1.5) },
];
