export interface Color {
  r: number;
  g: number;
  b: number;
}

export type EffectKind =
  | "off"
  | "static"
  | "breathing"
  | "rainbow_cycle"
  | "rainbow_wave"
  | "color_wave"
  | "comet"
  | "blink"
  | "gradient";

export interface EffectConfig {
  kind: EffectKind;
  colors: Color[];
  speed: number;
  brightness: number;
  reverse: boolean;
}

export interface ZoneInfo {
  name: string;
  led_count: number;
}

export interface FanChannel {
  index: number;
  name: string;
  duty_percent: number | null;
  rpm: number | null;
}

export interface DeviceInfo {
  id: string;
  name: string;
  vendor: string;
  backend: string;
  device_type: string;
  zones: ZoneInfo[];
  led_count: number;
  fan_channels: FanChannel[];
  controllable: boolean;
  note: string;
}

export interface BackendStatus {
  name: string;
  available: boolean;
}

export interface ConflictingSoftware {
  name: string;
  process: string;
  affects: string[];
}

export interface ConflictReport {
  conflicts: ConflictingSoftware[];
  openrgb_running: boolean;
}

export interface Settings {
  openrgb_host: string;
  openrgb_port: number;
  auto_start_openrgb: boolean;
  native_drivers_enabled: boolean;
  fps: number;
  start_minimized: boolean;
  effects: Record<string, EffectConfig>;
}

export interface OpenRgbStatus {
  exe_path: string | null;
  server_reachable: boolean;
  managed: boolean;
}

export const EFFECT_LABELS: Record<EffectKind, string> = {
  off: "Éteint",
  static: "Couleur fixe",
  breathing: "Respiration",
  rainbow_cycle: "Arc-en-ciel (cycle)",
  rainbow_wave: "Arc-en-ciel (vague)",
  color_wave: "Vague bicolore",
  comet: "Comète",
  blink: "Clignotement",
  gradient: "Dégradé",
};

export const DEVICE_TYPE_LABELS: Record<string, string> = {
  motherboard: "Carte mère",
  dram: "RAM",
  gpu: "Carte graphique",
  cooler: "Refroidissement",
  led_strip: "Bande LED",
  keyboard: "Clavier",
  mouse: "Souris",
  mousemat: "Tapis de souris",
  headset: "Casque",
  headset_stand: "Support casque",
  gamepad: "Manette",
  light: "Lampe",
  speaker: "Enceinte",
  virtual: "Virtuel",
  storage: "Stockage",
  case: "Boîtier",
  microphone: "Micro",
  accessory: "Accessoire",
  keypad: "Pavé",
  fan: "Ventilateur",
  hub: "Hub",
  aio: "AIO / Watercooling",
  unknown: "Inconnu",
};

export function colorToHex(c: Color): string {
  const h = (n: number) => n.toString(16).padStart(2, "0");
  return `#${h(c.r)}${h(c.g)}${h(c.b)}`;
}

export function hexToColor(hex: string): Color {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return { r: 255, g: 80, b: 0 };
  const v = parseInt(m[1], 16);
  return { r: (v >> 16) & 255, g: (v >> 8) & 255, b: v & 255 };
}
