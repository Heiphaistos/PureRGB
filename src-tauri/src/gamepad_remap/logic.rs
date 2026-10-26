//! Logique pure du remappage : boutons, zones mortes (radiale/axiale), courbes, seuils de gâchettes,
//! anti-dérive. Aucune E/S : testée sans matériel.

use super::model::*;

const FULL: f32 = 32767.0;

fn norm(v: i16) -> f32 {
    (v as f32 / FULL).clamp(-1.0, 1.0)
}

fn to_i16(v: f32) -> i16 {
    (v * FULL).round().clamp(-FULL, FULL) as i16
}

/// Zone morte + saturation + courbe sur une amplitude 0..1.
fn shape(m: f32, c: &StickCfg) -> f32 {
    if m <= c.inner {
        return 0.0;
    }
    // Pas de plafond à 1 : le « carré » d'un stick (diagonales > 1) reste intact, `to_i16` borne chaque axe.
    ((m - c.inner) / (1.0 - c.inner - c.outer)).powf(c.exponent)
}

fn stick(c: &StickCfg, x: i16, y: i16) -> (i16, i16) {
    let mut x = norm(x) - c.center_x;
    let mut y = norm(y) - c.center_y;
    if c.invert_x {
        x = -x;
    }
    if c.invert_y {
        y = -y;
    }
    let (ox, oy) = match c.mode {
        DeadzoneMode::Radial => {
            let m = x.hypot(y);
            if m <= c.inner || m == 0.0 {
                (0.0, 0.0)
            } else {
                let k = shape(m, c) / m;
                (x * k, y * k)
            }
        }
        DeadzoneMode::Axial => (shape(x.abs(), c).copysign(x), shape(y.abs(), c).copysign(y)),
    };
    (to_i16(ox), to_i16(oy))
}

fn trigger(c: &TriggerCfg, v: u8) -> u8 {
    if v <= c.low {
        return 0;
    }
    let s = ((v - c.low) as f32 / (c.high.saturating_sub(c.low).max(1)) as f32).clamp(0.0, 1.0);
    (s.powf(c.exponent) * 255.0).round() as u8
}

/// Applique un profil à l'état physique ; toutes les sources sont lues sur l'état d'entrée
/// (une permutation A<->B fonctionne).
pub fn transform(p: &Profile, s: &PadState) -> PadState {
    let (lx, ly) = stick(&p.left_stick, s.lx, s.ly);
    let (rx, ry) = stick(&p.right_stick, s.rx, s.ry);
    let mut out = PadState { buttons: 0, lt: trigger(&p.left_trigger, s.lt), rt: trigger(&p.right_trigger, s.rt), lx, ly, rx, ry };
    for b in Button::ALL {
        if s.buttons & b.bit() == 0 {
            continue;
        }
        match p.buttons.get(&b).copied().unwrap_or(Target::Button(b)) {
            Target::Button(t) => out.buttons |= t.bit(),
            Target::LeftTrigger => out.lt = 255,
            Target::RightTrigger => out.rt = 255,
            Target::Disabled => {}
        }
    }
    out
}

/// Anti-dérive : centre de repos d'un stick d'après des échantillons. `None` si trop peu
/// d'échantillons ou si le stick a bougé pendant la mesure (écart > 25 % de la course).
pub fn center_from_samples(samples: &[(i16, i16)]) -> Option<(f32, f32)> {
    if samples.len() < 30 {
        return None;
    }
    let n = samples.len() as f32;
    let (sx, sy) = samples.iter().fold((0.0, 0.0), |a, &(x, y)| (a.0 + norm(x), a.1 + norm(y)));
    let (mx, my) = (sx / n, sy / n);
    let steady = samples.iter().all(|&(x, y)| (norm(x) - mx).hypot(norm(y) - my) <= 0.25);
    (steady && mx.abs() <= 0.5 && my.abs() <= 0.5).then_some((mx, my))
}

/// Manette physique à lire : la première connectée qui n'est pas notre manette virtuelle.
pub fn pick_physical(connected: &[bool; 4], virtual_slot: Option<u32>) -> Option<u32> {
    (0..4u32).find(|&i| connected[i as usize] && Some(i) != virtual_slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st() -> PadState {
        PadState::default()
    }

    #[test]
    fn default_profile_is_transparent() {
        let p = Profile::default();
        for v in [-32767i16, -20000, -1, 0, 1, 777, 20000, 32767] {
            let s = PadState { buttons: 0xF3FF, lt: (v & 0xFF) as u8, rt: 255, lx: v, ly: -v, rx: v / 2, ry: 5 };
            assert_eq!(transform(&p, &s), s, "v={v}");
        }
    }

    #[test]
    fn button_swap_is_simultaneous() {
        let mut p = Profile::default();
        p.buttons.insert(Button::A, Target::Button(Button::B));
        p.buttons.insert(Button::B, Target::Button(Button::A));
        let s = PadState { buttons: Button::A.bit(), ..st() };
        assert_eq!(transform(&p, &s).buttons, Button::B.bit());
        let s = PadState { buttons: Button::A.bit() | Button::B.bit(), ..st() };
        assert_eq!(transform(&p, &s).buttons, Button::A.bit() | Button::B.bit());
    }

    #[test]
    fn button_to_trigger_and_disabled() {
        let mut p = Profile::default();
        p.buttons.insert(Button::LeftShoulder, Target::LeftTrigger);
        p.buttons.insert(Button::Y, Target::Disabled);
        let o = transform(&p, &PadState { buttons: Button::LeftShoulder.bit() | Button::Y.bit() | Button::X.bit(), ..st() });
        assert_eq!((o.lt, o.buttons), (255, Button::X.bit()));
    }

    #[test]
    fn radial_deadzone_zeroes_small_and_keeps_direction() {
        let mut p = Profile::default();
        p.left_stick.inner = 0.2;
        assert_eq!(transform(&p, &PadState { lx: 4000, ly: 4000, ..st() }).lx, 0); // magnitude 0,17
        let o = transform(&p, &PadState { lx: 16000, ly: 16000, ..st() });
        assert_eq!(o.lx, o.ly); // diagonale conservée
        assert!(o.lx > 0);
        assert_eq!(transform(&p, &PadState { lx: 32767, ..st() }).lx, 32767); // pleine course atteinte
    }

    #[test]
    fn axial_deadzone_is_per_axis() {
        let mut p = Profile::default();
        p.left_stick.mode = DeadzoneMode::Axial;
        p.left_stick.inner = 0.2;
        let o = transform(&p, &PadState { lx: 3000, ly: 20000, ..st() });
        assert_eq!(o.lx, 0);
        assert!(o.ly > 0);
        let o = transform(&p, &PadState { lx: -3000, ly: -20000, ..st() });
        assert!(o.ly < 0);
    }

    #[test]
    fn outer_zone_saturates_early() {
        let mut p = Profile::default();
        p.right_stick.outer = 0.1;
        assert_eq!(transform(&p, &PadState { rx: 29500, ..st() }).rx, 32767); // 0,9 de course
    }

    #[test]
    fn curve_exponent_shapes_response() {
        let mut p = Profile::default();
        p.left_stick.exponent = 2.0;
        let half = transform(&p, &PadState { lx: 16384, ..st() }).lx; // 0,5 -> 0,25
        assert!((half - 8192).abs() <= 2, "{half}");
        p.left_stick.exponent = 0.5;
        assert!(transform(&p, &PadState { lx: 16384, ..st() }).lx > 16384);
    }

    #[test]
    fn invert_and_center() {
        let mut p = Profile::default();
        p.left_stick.invert_y = true;
        assert_eq!(transform(&p, &PadState { ly: 1000, ..st() }).ly, -1000);
        let mut p = Profile::default();
        p.left_stick.center_x = 1000.0 / 32767.0; // dérive mesurée
        p.left_stick.inner = 0.05;
        assert_eq!(transform(&p, &PadState { lx: 1000, ..st() }).lx, 0);
    }

    #[test]
    fn trigger_thresholds() {
        let mut p = Profile::default();
        p.left_trigger.low = 50;
        p.left_trigger.high = 200;
        let t = |v| transform(&p, &PadState { lt: v, ..st() }).lt;
        assert_eq!((t(0), t(50), t(200), t(255)), (0, 0, 255, 255));
        assert!((t(125) as i32 - 128).abs() <= 1);
    }

    #[test]
    fn sanitize_bounds_everything() {
        let mut p = Profile::default();
        p.left_stick.inner = f32::NAN;
        p.left_stick.outer = 5.0;
        p.right_stick.inner = 0.9;
        p.right_stick.outer = 0.5;
        p.right_stick.exponent = 100.0;
        p.left_trigger.low = 200;
        p.left_trigger.high = 100;
        p.right_trigger.low = 255;
        p.right_trigger.high = 255;
        p.buttons.insert(Button::A, Target::Button(Button::A));
        p.sanitize();
        assert_eq!(p.left_stick.inner, 0.0);
        assert_eq!(p.left_stick.outer, 0.5);
        assert!(p.right_stick.inner + p.right_stick.outer <= 0.95 + 1e-6);
        assert_eq!(p.right_stick.exponent, 4.0);
        assert!(p.left_trigger.high > p.left_trigger.low);
        assert!(p.right_trigger.high > p.right_trigger.low);
        assert!(p.buttons.is_empty());
        transform(&p, &PadState { lx: 30000, lt: 255, rt: 255, ..st() });
    }

    #[test]
    fn calibration_center() {
        let rest: Vec<(i16, i16)> = (0..40).map(|i| (800 + (i % 3) as i16, -500)).collect();
        let (cx, cy) = center_from_samples(&rest).unwrap();
        assert!((cx - 801.0 / 32767.0).abs() < 1e-3 && (cy + 500.0 / 32767.0).abs() < 1e-3);
        assert!(center_from_samples(&rest[..10]).is_none());
        let moving: Vec<(i16, i16)> = (0..40).map(|i| (if i % 2 == 0 { 20000 } else { -20000 }, 0)).collect();
        assert!(center_from_samples(&moving).is_none());
    }

    #[test]
    fn physical_pad_skips_virtual() {
        assert_eq!(pick_physical(&[true, true, false, false], Some(0)), Some(1));
        assert_eq!(pick_physical(&[true, false, false, false], None), Some(0));
        assert_eq!(pick_physical(&[true, false, false, false], Some(0)), None);
        assert_eq!(pick_physical(&[false; 4], None), None);
    }

    #[test]
    fn profile_roundtrips_json() {
        let mut p = Profile::default();
        p.buttons.insert(Button::A, Target::Button(Button::X));
        p.buttons.insert(Button::Y, Target::Disabled);
        let j = serde_json::to_string(&p).unwrap();
        assert!(j.contains(r#""a":{"kind":"button","button":"x"}"#), "{j}");
        assert_eq!(serde_json::from_str::<Profile>(&j).unwrap(), p);
        assert_eq!(serde_json::from_str::<Profile>("{}").unwrap(), Profile::default());
        assert!(valid_profile_name("Défaut") && !valid_profile_name("  ") && !valid_profile_name("a\nb"));
    }
}
