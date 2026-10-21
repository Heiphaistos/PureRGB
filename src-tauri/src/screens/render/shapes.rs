//! Chemins géométriques communs (rectangle arrondi, arc, cercle, segment).

use tiny_skia::{Path, PathBuilder, Rect};

const KAPPA: f32 = 0.552_284_75;

pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = if r.is_finite() { r.clamp(0.0, w.min(h) / 2.0) } else { 0.0 };
    if r < 0.01 {
        return Some(PathBuilder::from_rect(Rect::from_xywh(x, y, w, h)?));
    }
    let k = r * KAPPA;
    let (x2, y2) = (x + w, y + h);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x2 - r, y);
    pb.cubic_to(x2 - r + k, y, x2, y + r - k, x2, y + r);
    pb.line_to(x2, y2 - r);
    pb.cubic_to(x2, y2 - r + k, x2 - r + k, y2, x2 - r, y2);
    pb.line_to(x + r, y2);
    pb.cubic_to(x + r - k, y2, x, y2 - r + k, x, y2 - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Arc de cercle ; angles en degrés, sens horaire à l'écran depuis 3 h (0° = droite, 90° = bas).
pub fn arc(cx: f32, cy: f32, r: f32, start_deg: f32, sweep_deg: f32) -> Option<Path> {
    let sweep = sweep_deg.clamp(-360.0, 360.0);
    let n = ((sweep.abs() / 90.0).ceil() as usize).max(1);
    let step = (sweep / n as f32).to_radians();
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let mut pb = PathBuilder::new();
    let mut a0 = start_deg.to_radians();
    pb.move_to(cx + r * a0.cos(), cy + r * a0.sin());
    for _ in 0..n {
        let a1 = a0 + step;
        pb.cubic_to(
            cx + r * (a0.cos() - k * a0.sin()),
            cy + r * (a0.sin() + k * a0.cos()),
            cx + r * (a1.cos() + k * a1.sin()),
            cy + r * (a1.sin() - k * a1.cos()),
            cx + r * a1.cos(),
            cy + r * a1.sin(),
        );
        a0 = a1;
    }
    pb.finish()
}

pub fn circle(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r)
}

pub fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    pb.finish()
}
