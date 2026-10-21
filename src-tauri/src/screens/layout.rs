//! Modèle JSON d'un design d'écran. Sérialisé tel quel dans les profils et dans
//! la bibliothèque embarquée. Coordonnées en pixels du canevas du design ; le
//! rendu met à l'échelle vers la résolution réelle de l'écran.

use serde::{Deserialize, Serialize};

pub const LAYOUT_VERSION: u32 = 1;
/// Garde-fous sur du JSON importé.
pub const MAX_WIDGETS: usize = 200;
pub const MAX_CANVAS: u32 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub name: String,
    /// Taille de conception (ex. 480×480). Le rendu met à l'échelle.
    pub width: u32,
    pub height: u32,
    pub background: Background,
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Background {
    Solid { color: String },
    /// Dégradé linéaire, angle en degrés.
    Gradient { from: String, to: String, angle: f32 },
    /// Image/GIF/vidéo de la médiathèque (`media_id`), avec mode d'ajustement.
    Media { media_id: String, fit: Fit },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    Cover,
    Contain,
    Stretch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Degrés, autour du centre du widget.
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(flatten)]
    pub kind: WidgetKind,
}

fn one() -> f32 {
    1.0
}

/// Source de données : voir `bindings.rs` pour la liste des clés (`cpu.temp`, `gpu.load`, `ram.used_pct`, `time`, …).
pub type Source = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WidgetKind {
    /// Texte fixe.
    Text { text: String, style: TextStyle },
    /// Valeur live formatée (`format` : "{:.0}°C", "{:.1} GHz", "%H:%M" pour l'heure/date).
    Value { source: Source, format: String, style: TextStyle },
    /// Jauge circulaire (arc) ou semi-circulaire.
    GaugeArc { source: Source, min: f32, max: f32, start_deg: f32, sweep_deg: f32, thickness: f32, track: String, colors: ColorRamp },
    /// Barre horizontale/verticale.
    Bar { source: Source, min: f32, max: f32, vertical: bool, radius: f32, track: String, colors: ColorRamp },
    /// Courbe d'historique (dernières `seconds` secondes).
    Graph { source: Source, min: f32, max: f32, seconds: u32, line: String, fill: Option<String>, thickness: f32 },
    /// Rectangle (fond de carte, séparateur).
    Rect { fill: String, stroke: Option<String>, stroke_width: f32, radius: f32 },
    /// Image de la médiathèque (logo, icône).
    Image { media_id: String, fit: Fit },
    /// Horloge analogique.
    AnalogClock { face: String, hands: String, accent: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    /// Nom de police embarquée ou de la médiathèque.
    pub font: String,
    pub size: f32,
    pub color: String,
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub bold: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Rampe de couleurs selon la valeur normalisée (0..1) : `[(0.0,"#0f0"),(0.7,"#fa0"),(1.0,"#f00")]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorRamp(pub Vec<(f32, String)>);

impl Layout {
    /// Refuse un layout hors bornes (JSON importé non fiable).
    pub fn validate(&self) -> Result<(), String> {
        if self.version != LAYOUT_VERSION {
            return Err(format!("version de design non supportée : {}", self.version));
        }
        if self.width == 0 || self.height == 0 || self.width > MAX_CANVAS || self.height > MAX_CANVAS {
            return Err("taille de canevas invalide".into());
        }
        if self.widgets.len() > MAX_WIDGETS {
            return Err(format!("trop de widgets (max {MAX_WIDGETS})"));
        }
        Ok(())
    }
}
