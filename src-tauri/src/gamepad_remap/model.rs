//! Modèle de données du remappage de manette (pur, sérialisé dans settings.json).
//! Disposition de référence : XInput / manette Xbox 360 (mêmes bits que `XINPUT_GAMEPAD.wButtons`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Boutons d'une manette Xbox 360 ; `bit()` = masque XInput (documenté par Microsoft, xinput.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    Start,
    Back,
    LeftThumb,
    RightThumb,
    LeftShoulder,
    RightShoulder,
    /// Cible seulement : `XInputGetState` ne rapporte jamais ce bouton.
    Guide,
    A,
    B,
    X,
    Y,
}

impl Button {
    pub const ALL: [Button; 15] = [
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
        Button::Start,
        Button::Back,
        Button::LeftThumb,
        Button::RightThumb,
        Button::LeftShoulder,
        Button::RightShoulder,
        Button::Guide,
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
    ];

    pub const fn bit(self) -> u16 {
        match self {
            Button::Up => 0x0001,
            Button::Down => 0x0002,
            Button::Left => 0x0004,
            Button::Right => 0x0008,
            Button::Start => 0x0010,
            Button::Back => 0x0020,
            Button::LeftThumb => 0x0040,
            Button::RightThumb => 0x0080,
            Button::LeftShoulder => 0x0100,
            Button::RightShoulder => 0x0200,
            Button::Guide => 0x0400,
            Button::A => 0x1000,
            Button::B => 0x2000,
            Button::X => 0x4000,
            Button::Y => 0x8000,
        }
    }
}

/// Ce que devient un bouton physique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "button", rename_all = "snake_case")]
pub enum Target {
    Button(Button),
    /// Appui = gâchette à fond.
    LeftTrigger,
    RightTrigger,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeadzoneMode {
    /// Zone morte circulaire (conserve les diagonales).
    #[default]
    Radial,
    /// Zone morte par axe (croix).
    Axial,
}

/// Réglages d'un stick. Les valeurs par défaut sont transparentes (aucun effet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StickCfg {
    pub mode: DeadzoneMode,
    /// Zone morte intérieure, 0..0,9 (fraction de la course).
    pub inner: f32,
    /// Zone de saturation extérieure : la course est atteinte à `1 - outer`.
    pub outer: f32,
    /// Courbe de réponse : sortie = entrée^exposant (1 = linéaire, >1 = précision au centre, <1 = réactif).
    pub exponent: f32,
    pub invert_x: bool,
    pub invert_y: bool,
    /// Anti-dérive : décalage de repos mesuré (calibration), soustrait avant la zone morte.
    pub center_x: f32,
    pub center_y: f32,
}

impl Default for StickCfg {
    fn default() -> Self {
        StickCfg {
            mode: DeadzoneMode::Radial,
            inner: 0.0,
            outer: 0.0,
            exponent: 1.0,
            invert_x: false,
            invert_y: false,
            center_x: 0.0,
            center_y: 0.0,
        }
    }
}

/// Réglages d'une gâchette (0..255).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TriggerCfg {
    /// Sous ce seuil (inclus) la sortie est 0.
    pub low: u8,
    /// À partir de ce seuil la sortie est 255.
    pub high: u8,
    pub exponent: f32,
}

impl Default for TriggerCfg {
    fn default() -> Self {
        TriggerCfg { low: 0, high: 255, exponent: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    /// Bouton physique -> cible ; absent = identité.
    pub buttons: BTreeMap<Button, Target>,
    pub left_stick: StickCfg,
    pub right_stick: StickCfg,
    pub left_trigger: TriggerCfg,
    pub right_trigger: TriggerCfg,
}

/// État d'une manette (entrée physique ou sortie virtuelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct PadState {
    pub buttons: u16,
    pub lt: u8,
    pub rt: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

fn fin(v: f32, lo: f32, hi: f32, dflt: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        dflt
    }
}

impl StickCfg {
    fn sanitize(&mut self) {
        self.inner = fin(self.inner, 0.0, 0.9, 0.0);
        self.outer = fin(self.outer, 0.0, 0.5, 0.0);
        if self.inner + self.outer > 0.95 {
            self.outer = 0.95 - self.inner;
        }
        self.exponent = fin(self.exponent, 0.3, 4.0, 1.0);
        self.center_x = fin(self.center_x, -0.5, 0.5, 0.0);
        self.center_y = fin(self.center_y, -0.5, 0.5, 0.0);
    }
}

impl TriggerCfg {
    fn sanitize(&mut self) {
        self.exponent = fin(self.exponent, 0.3, 4.0, 1.0);
        if self.high <= self.low {
            if self.low == 255 {
                self.low = 254;
            }
            self.high = self.low + 1;
        }
    }
}

impl Profile {
    /// Entrée venue de l'interface (frontière de confiance) : borne toutes les valeurs.
    pub fn sanitize(&mut self) {
        self.left_stick.sanitize();
        self.right_stick.sanitize();
        self.left_trigger.sanitize();
        self.right_trigger.sanitize();
        // Une entrée identité est inutile à stocker.
        self.buttons.retain(|k, t| *t != Target::Button(*k));
    }
}

/// Nom de profil : 1..40 caractères, sans caractère de contrôle.
pub fn valid_profile_name(n: &str) -> bool {
    let t = n.trim();
    !t.is_empty() && t.chars().count() <= 40 && !t.chars().any(char::is_control)
}
