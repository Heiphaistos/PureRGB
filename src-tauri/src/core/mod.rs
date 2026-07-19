pub mod registry;

use serde::{Deserialize, Serialize};

/// Couleur RGB 8 bits par canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// HSV -> RGB. h en [0, 360), s et v en [0, 1].
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h.rem_euclid(360.0);
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u32 {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Color {
            r: ((r + m) * 255.0).round() as u8,
            g: ((g + m) * 255.0).round() as u8,
            b: ((b + m) * 255.0).round() as u8,
        }
    }

    pub fn scale(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 * f) as u8,
            g: (self.g as f32 * f) as u8,
            b: (self.b as f32 * f) as u8,
        }
    }

    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8,
            g: (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8,
            b: (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8,
        }
    }
}

/// Type d'appareil, aligné sur les catégories OpenRGB + extensions ventilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Motherboard,
    Dram,
    Gpu,
    Cooler,
    LedStrip,
    Keyboard,
    Mouse,
    Mousemat,
    Headset,
    HeadsetStand,
    Gamepad,
    Light,
    Speaker,
    Virtual,
    Storage,
    Case,
    Microphone,
    Accessory,
    Keypad,
    Fan,
    Hub,
    Aio,
    Unknown,
}

impl DeviceType {
    /// Mapping depuis l'enum device_type du protocole OpenRGB.
    pub fn from_openrgb(v: i32) -> Self {
        match v {
            0 => DeviceType::Motherboard,
            1 => DeviceType::Dram,
            2 => DeviceType::Gpu,
            3 => DeviceType::Cooler,
            4 => DeviceType::LedStrip,
            5 => DeviceType::Keyboard,
            6 => DeviceType::Mouse,
            7 => DeviceType::Mousemat,
            8 => DeviceType::Headset,
            9 => DeviceType::HeadsetStand,
            10 => DeviceType::Gamepad,
            11 => DeviceType::Light,
            12 => DeviceType::Speaker,
            13 => DeviceType::Virtual,
            14 => DeviceType::Storage,
            15 => DeviceType::Case,
            16 => DeviceType::Microphone,
            17 => DeviceType::Accessory,
            18 => DeviceType::Keypad,
            _ => DeviceType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneInfo {
    pub name: String,
    pub led_count: u32,
    /// Type OpenRGB : 0 = single, 1 = linear, 2 = matrix.
    #[serde(default)]
    pub zone_type: i32,
    /// Bornes de redimensionnement. leds_min != leds_max = zone redimensionnable
    /// (typiquement un connecteur ARGB de carte mère ou un canal de hub, où
    /// OpenRGB ne peut pas deviner combien de LEDs sont branchées).
    #[serde(default)]
    pub leds_min: u32,
    #[serde(default)]
    pub leds_max: u32,
}

impl ZoneInfo {
    /// Zone à taille fixe (drivers natifs : pas de redimensionnement).
    pub fn fixed(name: impl Into<String>, led_count: u32) -> Self {
        ZoneInfo {
            name: name.into(),
            led_count,
            zone_type: 1,
            leds_min: led_count,
            leds_max: led_count,
        }
    }

    pub fn resizable(&self) -> bool {
        self.leds_min != self.leds_max
    }
}

/// Flag OpenRGB : le mode accepte des couleurs choisies par l'utilisateur.
/// (Autres flags décodés côté UI depuis `ModeInfo.flags`.)
pub const MODE_FLAG_HAS_MODE_SPECIFIC_COLOR: u32 = 1 << 6;

/// Mode matériel natif d'un contrôleur OpenRGB (Rainbow, Breathing, Direct…)
/// avec ses bornes et réglages courants — pilotable tel quel par le firmware.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeInfo {
    pub index: u32,
    pub name: String,
    pub value: i32,
    pub flags: u32,
    pub speed_min: u32,
    pub speed_max: u32,
    pub colors_min: u32,
    pub colors_max: u32,
    pub speed: u32,
    pub direction: u32,
    pub color_mode: u32,
    pub colors: Vec<Color>,
}

/// Canal ventilateur exposé par un appareil (hub, AIO...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanChannel {
    pub index: u8,
    pub name: String,
    /// Dernier duty cycle appliqué en %, None si inconnu.
    pub duty_percent: Option<u8>,
    /// Dernier RPM lu, None si non supporté.
    pub rpm: Option<u32>,
}

fn default_active_mode() -> i32 {
    -1
}

/// Appareil unifié, quel que soit le backend d'origine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Id global unique: "<backend>:<id local>" ex "openrgb:2", "hid:1b1c:0c10:0".
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub backend: String,
    pub device_type: DeviceType,
    pub zones: Vec<ZoneInfo>,
    pub led_count: u32,
    pub fan_channels: Vec<FanChannel>,
    /// true = contrôle réel possible; false = détecté mais non pilotable (driver manquant).
    pub controllable: bool,
    /// true = écran LCD pilotable (Kraken Z / 2023 via liquidctl).
    #[serde(default)]
    pub has_lcd: bool,
    /// Modes matériels natifs (OpenRGB uniquement, vide sinon).
    #[serde(default)]
    pub modes: Vec<ModeInfo>,
    /// Index du mode matériel actif, -1 si inconnu.
    #[serde(default = "default_active_mode")]
    pub active_mode: i32,
    /// Note affichée à l'utilisateur (ex: "pilotable via OpenRGB", "driver expérimental").
    pub note: String,
}
