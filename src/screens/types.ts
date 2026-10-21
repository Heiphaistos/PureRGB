// Miroir TypeScript de src-tauri/src/screens/layout.rs et driver.rs (JSON identique).

export type Fit = "cover" | "contain" | "stretch";
export type Align = "left" | "center" | "right";

export type Background =
  | { kind: "solid"; color: string }
  | { kind: "gradient"; from: string; to: string; angle: number }
  | { kind: "media"; media_id: string; fit: Fit };

/** Rampe de couleurs : [position normalisée 0..1, couleur]. */
export type ColorRamp = [number, string][];

export interface TextStyle {
  font: string;
  size: number;
  color: string;
  align: Align;
  bold: boolean;
}

export type WidgetKind =
  | { type: "text"; text: string; style: TextStyle }
  | { type: "value"; source: string; format: string; style: TextStyle }
  | {
      type: "gauge_arc";
      source: string;
      min: number;
      max: number;
      start_deg: number;
      sweep_deg: number;
      thickness: number;
      track: string;
      colors: ColorRamp;
    }
  | {
      type: "bar";
      source: string;
      min: number;
      max: number;
      vertical: boolean;
      radius: number;
      track: string;
      colors: ColorRamp;
    }
  | {
      type: "graph";
      source: string;
      min: number;
      max: number;
      seconds: number;
      line: string;
      fill: string | null;
      thickness: number;
    }
  | { type: "rect"; fill: string; stroke: string | null; stroke_width: number; radius: number }
  | { type: "image"; media_id: string; fit: Fit }
  | { type: "analog_clock"; face: string; hands: string; accent: string };

export type WidgetType = WidgetKind["type"];

export interface WidgetBase {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  rotation: number;
  opacity: number;
}

export type Widget = WidgetBase & WidgetKind;

export interface Layout {
  version: number;
  name: string;
  width: number;
  height: number;
  background: Background;
  widgets: Widget[];
}

export const LAYOUT_VERSION = 1;
export const MAX_WIDGETS = 200;
export const MAX_CANVAS = 4096;

export type Protocol =
  | "thermalright_scsi"
  | "thermalright_hid"
  | "thermalright_bulk"
  | "thermalright_ly"
  | "turing_serial"
  | "corsair_lcd"
  | "deepcool_lm"
  | "asus_ryujin"
  | "thermaltake_lcd"
  | "lian_li_hid"
  | "turzx_usb"
  | "simulated";

export interface ScreenInfo {
  id: string;
  vendor: string;
  model: string;
  protocol: Protocol;
  width: number;
  height: number;
  rotation: number;
  resolution_confirmed: boolean;
  verified: boolean;
}

export interface ScreenState {
  layout: Layout | null;
  brightness: number;
  rotation: number;
  fps: number;
  running: boolean;
  /** Faux : l'écran n'accepte que des images fixes (envoi manuel). */
  live: boolean;
  error: string | null;
}

export interface ScreenOptions {
  brightness: number;
  rotation: number;
  fps: number;
}

export interface DesignSummary {
  id: string;
  name: string;
  category: string;
  width: number;
  height: number;
  tags: string[];
}

export type MediaKind = "image" | "gif" | "video" | "font";

export interface MediaItem {
  id: string;
  name: string;
  kind: MediaKind;
  width: number;
  height: number;
  frames: number;
  duration_ms: number;
}

export type LiveValues = Record<string, number>;

/** Capteur brut tel que lu par sensord (LibreHardwareMonitor). */
export interface RawSensor {
  id: string;
  hardware: string;
  name: string;
  /** Temperature, Load, Fan, Power, Clock, Voltage, Data, SmallData, Control… */
  type: string;
  value: number;
  controllable: boolean;
}

export interface UsedSensor {
  /** Vide = valeur calculée (moyenne, ratio…). */
  id: string;
  hardware: string;
  name: string;
}

export interface SensorsReport {
  sensord_running: boolean;
  sensor_count: number;
  exe_path: string | null;
  sensors: RawSensor[];
  /** clé -> capteur utilisé. */
  used: Record<string, UsedSensor>;
}
