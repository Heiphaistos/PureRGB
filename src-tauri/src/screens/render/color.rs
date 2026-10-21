//! Couleurs "#rgb" / "#rrggbb" / "#rrggbbaa" et rampes (valeur normalisée -> couleur).

use crate::screens::layout::ColorRamp;
use tiny_skia::Color;

pub fn parse(s: &str) -> Result<Color, String> {
    let bad = || format!("couleur invalide : \"{}\" (attendu #rgb, #rrggbb ou #rrggbbaa)", s.chars().take(24).collect::<String>());
    let hex = s.trim().strip_prefix('#').ok_or_else(bad)?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());
    let nib = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).map(|n| n * 17).map_err(|_| bad());
    let (r, g, b, a) = match hex.len() {
        3 => (nib(0)?, nib(1)?, nib(2)?, 255),
        6 => (byte(0)?, byte(2)?, byte(4)?, 255),
        8 => (byte(0)?, byte(2)?, byte(4)?, byte(6)?),
        _ => return Err(bad()),
    };
    Ok(Color::from_rgba8(r, g, b, a))
}

pub fn with_opacity(mut c: Color, opacity: f32) -> Color {
    c.apply_opacity(opacity.clamp(0.0, 1.0));
    c
}

/// Couleur de la rampe à la position `t` (0..1), interpolation linéaire entre paliers.
pub fn ramp(r: &ColorRamp, t: f32) -> Result<Color, String> {
    let mut stops: Vec<(f32, Color)> = Vec::with_capacity(r.0.len());
    for (p, c) in &r.0 {
        if p.is_finite() {
            stops.push((*p, parse(c)?));
        }
    }
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = match (stops.first(), stops.last()) {
        (Some(f), Some(l)) => (*f, *l),
        _ => return Err("rampe de couleurs vide".into()),
    };
    let t = if t.is_finite() { t } else { 0.0 };
    if t <= first.0 {
        return Ok(first.1);
    }
    if t >= last.0 {
        return Ok(last.1);
    }
    for w in stops.windows(2) {
        let ((p0, c0), (p1, c1)) = (w[0], w[1]);
        if t <= p1 {
            let f = if p1 > p0 { (t - p0) / (p1 - p0) } else { 1.0 };
            let l = |a: f32, b: f32| a + (b - a) * f;
            return Ok(Color::from_rgba(l(c0.red(), c1.red()), l(c0.green(), c1.green()), l(c0.blue(), c1.blue()), l(c0.alpha(), c1.alpha()))
                .unwrap_or(c1));
        }
    }
    Ok(last.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_forms_and_rejects_garbage() {
        assert_eq!(parse("#f00").unwrap().to_color_u8().red(), 255);
        assert_eq!(parse("#00ff00").unwrap().to_color_u8().green(), 255);
        assert_eq!(parse("#0000ff80").unwrap().to_color_u8().alpha(), 128);
        for bad in ["", "#", "red", "#12", "#12345", "#gggggg", "#ééé", "#éé", "ff0000", "#1234567890"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn ramp_interpolates_and_clamps() {
        let r = ColorRamp(vec![(0.0, "#000000".into()), (1.0, "#ffffff".into())]);
        assert_eq!(ramp(&r, 0.5).unwrap().to_color_u8().red(), 128);
        assert_eq!(ramp(&r, -3.0).unwrap().to_color_u8().red(), 0);
        assert_eq!(ramp(&r, 9.0).unwrap().to_color_u8().red(), 255);
        assert!(ramp(&ColorRamp(vec![]), 0.5).is_err());
    }
}
