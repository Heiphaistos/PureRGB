// Remappage de manette (miroir de src-tauri/src/gamepad_remap/model.rs) : types, libellés, aperçu de courbe.

export type Button =
  | "up" | "down" | "left" | "right" | "start" | "back" | "left_thumb" | "right_thumb"
  | "left_shoulder" | "right_shoulder" | "guide" | "a" | "b" | "x" | "y";

export type Target =
  | { kind: "button"; button: Button }
  | { kind: "left_trigger" }
  | { kind: "right_trigger" }
  | { kind: "disabled" };

export interface StickCfg {
  mode: "radial" | "axial";
  inner: number;
  outer: number;
  exponent: number;
  invert_x: boolean;
  invert_y: boolean;
  center_x: number;
  center_y: number;
}

export interface TriggerCfg {
  low: number;
  high: number;
  exponent: number;
}

export interface Profile {
  buttons: Partial<Record<Button, Target>>;
  left_stick: StickCfg;
  right_stick: StickCfg;
  left_trigger: TriggerCfg;
  right_trigger: TriggerCfg;
}

export interface PadState {
  buttons: number;
  lt: number;
  rt: number;
  lx: number;
  ly: number;
  rx: number;
  ry: number;
}

/** Boutons physiques remappables (XInput ne rapporte jamais « Guide »). */
export const SOURCES: Button[] = [
  "a", "b", "x", "y", "left_shoulder", "right_shoulder", "back", "start",
  "left_thumb", "right_thumb", "up", "down", "left", "right",
];

export const BUTTON_LABELS: Record<Button, string> = {
  a: "A", b: "B", x: "X", y: "Y",
  left_shoulder: "LB", right_shoulder: "RB", back: "Retour (Back)", start: "Menu (Start)",
  left_thumb: "Clic stick gauche", right_thumb: "Clic stick droit",
  up: "Croix haut", down: "Croix bas", left: "Croix gauche", right: "Croix droite", guide: "Guide (Xbox)",
};

/** Cibles proposées dans les listes déroulantes : valeur textuelle <-> Target. */
export const TARGET_CHOICES: string[] = [
  ...(Object.keys(BUTTON_LABELS) as Button[]).map((b) => `button:${b}`),
  "left_trigger", "right_trigger", "disabled",
];

export function targetToChoice(t: Target | undefined, source: Button): string {
  if (!t) return `button:${source}`;
  return t.kind === "button" ? `button:${t.button}` : t.kind;
}

export function choiceToTarget(c: string): Target {
  if (c.startsWith("button:")) return { kind: "button", button: c.slice(7) as Button };
  return { kind: c as "left_trigger" | "right_trigger" | "disabled" };
}

export function targetLabel(c: string): string {
  if (c.startsWith("button:")) return BUTTON_LABELS[c.slice(7) as Button] ?? c;
  return { left_trigger: "Gâchette gauche (à fond)", right_trigger: "Gâchette droite (à fond)", disabled: "Désactivé" }[c] ?? c;
}

export const defaultStick = (): StickCfg => ({
  mode: "radial", inner: 0, outer: 0, exponent: 1, invert_x: false, invert_y: false, center_x: 0, center_y: 0,
});
export const defaultTrigger = (): TriggerCfg => ({ low: 0, high: 255, exponent: 1 });
export const defaultProfile = (): Profile => ({
  buttons: {}, left_stick: defaultStick(), right_stick: defaultStick(), left_trigger: defaultTrigger(), right_trigger: defaultTrigger(),
});

/** Réponse d'un stick sur une amplitude 0..1 (même formule que logic.rs, pour l'aperçu de la courbe). */
export function response(x: number, inner: number, outer: number, exponent: number): number {
  if (x <= inner) return 0;
  const span = Math.max(1 - inner - outer, 0.05);
  return Math.pow(Math.min(1, (x - inner) / span), exponent);
}

/** Points « x y » d'un tracé SVG de la courbe (repère 0..1, y vers le haut). */
export function curvePath(inner: number, outer: number, exponent: number, steps = 32): string {
  return Array.from({ length: steps + 1 }, (_, i) => {
    const x = i / steps;
    return `${x.toFixed(3)},${(1 - response(x, inner, outer, exponent)).toFixed(3)}`;
  }).join(" ");
}

/** Enregistrement d'un profil : retire les entrées identité (le backend le fait aussi). */
export function withoutIdentity(p: Profile): Profile {
  const buttons: Profile["buttons"] = {};
  for (const [k, t] of Object.entries(p.buttons) as [Button, Target][]) {
    if (!(t.kind === "button" && t.button === k)) buttons[k] = t;
  }
  return { ...p, buttons };
}
