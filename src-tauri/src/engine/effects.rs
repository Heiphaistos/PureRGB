use crate::core::Color;
use serde::{Deserialize, Serialize};

/// Configuration d'un effet appliqué à un appareil.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectConfig {
    pub kind: EffectKind,
    /// Couleurs utilisateur (1 à 3 selon l'effet).
    pub colors: Vec<Color>,
    /// Vitesse 0.1 - 5.0 (multiplicateur temporel).
    pub speed: f32,
    /// Luminosité 0.0 - 1.0.
    pub brightness: f32,
    /// Inverse le sens des effets directionnels.
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Off,
    Static,
    Breathing,
    RainbowCycle,
    RainbowWave,
    ColorWave,
    Comet,
    Blink,
    Gradient,
}

impl Default for EffectConfig {
    fn default() -> Self {
        EffectConfig {
            kind: EffectKind::Static,
            colors: vec![Color::new(255, 80, 0)],
            speed: 1.0,
            brightness: 1.0,
            reverse: false,
        }
    }
}

impl EffectConfig {
    /// true si l'effet est figé dans le temps => une seule application suffit,
    /// le moteur peut se mettre en veille (0% CPU).
    pub fn is_static(&self) -> bool {
        matches!(self.kind, EffectKind::Off | EffectKind::Static | EffectKind::Gradient)
    }

    fn color(&self, i: usize) -> Color {
        self.colors.get(i).copied().unwrap_or(Color::new(255, 80, 0))
    }
}

/// Rend l'état des LEDs à l'instant `t` (secondes). Fonction pure, testable.
pub fn render(cfg: &EffectConfig, t: f32, led_count: usize) -> Vec<Color> {
    if led_count == 0 {
        return Vec::new();
    }
    let bt = cfg.brightness.clamp(0.0, 1.0);
    let ts = t * cfg.speed.clamp(0.1, 5.0);
    let n = led_count as f32;

    let pos = |i: usize| -> f32 {
        let p = i as f32 / n;
        if cfg.reverse {
            1.0 - p
        } else {
            p
        }
    };

    match cfg.kind {
        EffectKind::Off => vec![Color::BLACK; led_count],
        EffectKind::Static => vec![cfg.color(0).scale(bt); led_count],
        EffectKind::Breathing => {
            let phase = (ts * std::f32::consts::TAU / 4.0).sin() * 0.5 + 0.5;
            vec![cfg.color(0).scale(bt * phase); led_count]
        }
        EffectKind::RainbowCycle => {
            let hue = (ts * 60.0) % 360.0;
            vec![Color::from_hsv(hue, 1.0, bt); led_count]
        }
        EffectKind::RainbowWave => (0..led_count)
            .map(|i| {
                let hue = (ts * 90.0 + pos(i) * 360.0) % 360.0;
                Color::from_hsv(hue, 1.0, bt)
            })
            .collect(),
        EffectKind::ColorWave => {
            let a = cfg.color(0);
            let b = cfg.color(1);
            (0..led_count)
                .map(|i| {
                    let phase =
                        ((ts + pos(i) * 2.0) * std::f32::consts::TAU / 2.0).sin() * 0.5 + 0.5;
                    Color::lerp(a, b, phase).scale(bt)
                })
                .collect()
        }
        EffectKind::Comet => {
            let head = (ts * 0.5).fract();
            let tail_len = 0.35;
            (0..led_count)
                .map(|i| {
                    let mut d = head - pos(i);
                    if d < 0.0 {
                        d += 1.0;
                    }
                    if d <= tail_len {
                        cfg.color(0).scale(bt * (1.0 - d / tail_len))
                    } else {
                        Color::BLACK
                    }
                })
                .collect()
        }
        EffectKind::Blink => {
            let on = (ts * 2.0).fract() < 0.5;
            if on {
                vec![cfg.color(0).scale(bt); led_count]
            } else {
                vec![Color::BLACK; led_count]
            }
        }
        EffectKind::Gradient => {
            let a = cfg.color(0);
            let b = cfg.color(1);
            (0..led_count)
                .map(|i| Color::lerp(a, b, pos(i)).scale(bt))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(kind: EffectKind) -> EffectConfig {
        EffectConfig {
            kind,
            colors: vec![Color::new(255, 0, 0), Color::new(0, 0, 255)],
            speed: 1.0,
            brightness: 1.0,
            reverse: false,
        }
    }

    #[test]
    fn static_fills_all_leds_with_color() {
        let out = render(&cfg(EffectKind::Static), 12.34, 10);
        assert_eq!(out.len(), 10);
        assert!(out.iter().all(|c| *c == Color::new(255, 0, 0)));
    }

    #[test]
    fn off_is_black() {
        let out = render(&cfg(EffectKind::Off), 0.0, 5);
        assert!(out.iter().all(|c| *c == Color::BLACK));
    }

    #[test]
    fn zero_leds_returns_empty() {
        assert!(render(&cfg(EffectKind::RainbowWave), 1.0, 0).is_empty());
    }

    #[test]
    fn brightness_scales_output() {
        let mut c = cfg(EffectKind::Static);
        c.brightness = 0.5;
        let out = render(&c, 0.0, 1);
        assert_eq!(out[0].r, 127);
    }

    #[test]
    fn rainbow_wave_varies_across_leds() {
        let out = render(&cfg(EffectKind::RainbowWave), 0.0, 30);
        assert_ne!(out[0], out[15]);
    }

    #[test]
    fn rainbow_wave_varies_over_time() {
        let a = render(&cfg(EffectKind::RainbowWave), 0.0, 10);
        let b = render(&cfg(EffectKind::RainbowWave), 1.0, 10);
        assert_ne!(a, b);
    }

    #[test]
    fn gradient_endpoints_match_colors() {
        let out = render(&cfg(EffectKind::Gradient), 0.0, 100);
        assert_eq!(out[0], Color::new(255, 0, 0));
        // dernier LED proche de la couleur b (pos = 99/100)
        assert!(out[99].b > 240 && out[99].r < 15);
    }

    #[test]
    fn reverse_flips_gradient() {
        let mut c = cfg(EffectKind::Gradient);
        c.reverse = true;
        let out = render(&c, 0.0, 100);
        assert!(out[0].b > 240);
    }

    #[test]
    fn comet_has_dark_and_lit_zones() {
        let out = render(&cfg(EffectKind::Comet), 0.25, 50);
        let lit = out.iter().filter(|c| c.r > 0).count();
        assert!(lit > 0 && lit < 50);
    }

    #[test]
    fn static_kinds_flagged_static() {
        assert!(cfg(EffectKind::Static).is_static());
        assert!(cfg(EffectKind::Gradient).is_static());
        assert!(!cfg(EffectKind::Breathing).is_static());
    }

    #[test]
    fn hsv_primary_colors() {
        assert_eq!(Color::from_hsv(0.0, 1.0, 1.0), Color::new(255, 0, 0));
        assert_eq!(Color::from_hsv(120.0, 1.0, 1.0), Color::new(0, 255, 0));
        assert_eq!(Color::from_hsv(240.0, 1.0, 1.0), Color::new(0, 0, 255));
    }
}
