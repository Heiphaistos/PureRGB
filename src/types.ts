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
  zone_type: number;
  leds_min: number;
  leds_max: number;
}

export function zoneResizable(z: ZoneInfo): boolean {
  return z.leds_min !== z.leds_max;
}

export interface FanChannel {
  index: number;
  name: string;
  duty_percent: number | null;
  rpm: number | null;
}

export interface ModeInfo {
  index: number;
  name: string;
  value: number;
  flags: number;
  speed_min: number;
  speed_max: number;
  colors_min: number;
  colors_max: number;
  speed: number;
  direction: number;
  color_mode: number;
  colors: Color[];
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
  has_lcd: boolean;
  modes: ModeInfo[];
  active_mode: number;
  note: string;
}

/** Faux si l'appareil n'a aucune zone RGB (ex. ventilo mobo/AIO liquidctl —
 * pilotage PWM/vitesse uniquement, RGB détecté séparément par OpenRGB si présent). */
export function deviceHasRgb(d: DeviceInfo): boolean {
  return d.led_count > 0 || d.zones.length > 0;
}

export interface Sensor {
  id: string;
  hardware: string;
  name: string;
  type: string;
  value: number;
}

export interface CurvePoint {
  temp: number;
  duty: number;
}

export interface CurveConfig {
  sensor_id: string;
  points: CurvePoint[];
  enabled: boolean;
}

export interface BackendStatus {
  name: string;
  available: boolean;
}

export interface ServiceInfo {
  name: string;
  display_name: string;
  state: string;
  start_mode: string;
}

export interface ConflictingSoftware {
  family: string;
  name: string;
  processes: string[];
  services: ServiceInfo[];
  affects: string[];
  active: boolean;
}

export interface ConflictReport {
  conflicts: ConflictingSoftware[];
  openrgb_running: boolean;
  guarded_families: string[];
}

export interface Settings {
  openrgb_host: string;
  openrgb_port: number;
  auto_start_openrgb: boolean;
  native_drivers_enabled: boolean;
  fps: number;
  start_minimized: boolean;
  effects: Record<string, EffectConfig>;
  disabled_services: Record<string, string>;
  curves: Record<string, CurveConfig>;
  hw_modes: Record<
    string,
    { mode_index: number; speed: number | null; direction: number | null; colors: Color[] | null }
  >;
  autostart: boolean;
  zone_sizes: Record<string, number>;
  network_devices: NetworkDevice[];
  auto_manage_conflicts: boolean;
  telemetry_opt_in: boolean;
}

export type NetworkDeviceKind =
  | "hue"
  | "nanoleaf"
  | "yeelight"
  | "lifx"
  | "govee"
  | "wiz"
  | "elgato_key_light"
  | "elgato_light_strip"
  | "kasa"
  | "e131";

/// Union taguée alignée sur l'enum Rust NetworkDevice (tag "kind").
export interface NetworkDevice {
  kind: NetworkDeviceKind;
  ip: string;
  mac?: string;
  entertainment?: boolean;
  port?: number;
  auth_token?: string;
  music_mode?: boolean;
  name?: string;
  multizone?: boolean;
  extended_multizone?: boolean;
  num_leds?: number;
  start_universe?: number;
  start_channel?: number;
  universe_size?: number;
  keepalive_time?: number;
}

export const NETWORK_KIND_LABELS: Record<NetworkDeviceKind, string> = {
  hue: "Philips Hue (pont)",
  nanoleaf: "Nanoleaf (panneaux)",
  yeelight: "Yeelight",
  lifx: "LIFX",
  govee: "Govee",
  wiz: "Philips Wiz",
  elgato_key_light: "Elgato Key Light",
  elgato_light_strip: "Elgato Light Strip",
  kasa: "TP-Link Kasa",
  e131: "WLED / E1.31 (sACN)",
};

export type DiagResult = { Ok: string } | { Err: string };

export function diagOk(r: DiagResult): boolean {
  return "Ok" in r;
}

export function diagText(r: DiagResult): string {
  return "Ok" in r ? r.Ok : r.Err;
}

export interface LiquidctlDiag {
  exe_path: string | null;
  version: DiagResult;
  list: DiagResult;
  initialize: DiagResult;
  status: DiagResult;
}

export interface SensorDiag {
  exe_path: string | null;
  running: boolean;
  sensor_count: number;
}

export interface RawHidDevice {
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
  recognized: boolean;
  has_native_driver: boolean;
}

export interface HardwareDiagnostics {
  liquidctl: LiquidctlDiag;
  sensord: SensorDiag;
  openrgb: OpenRgbStatus;
  hid_raw: RawHidDevice[];
}

export interface OpenRgbStatus {
  exe_path: string | null;
  server_reachable: boolean;
  managed: boolean;
  pawnio_installed: boolean;
  pawnio_ready: boolean;
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
