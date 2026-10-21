//! Jauge en arc et barre de progression.

use super::{color, shapes, Ctx};
use crate::screens::layout::ColorRamp;
use tiny_skia::{LineCap, Stroke};

/// Valeur normalisée 0..1 ; `None` si la valeur est absente.
fn norm(ctx: &Ctx, source: &str, min: f32, max: f32) -> Option<f32> {
    let v = ctx.value(source)? as f32;
    if !(min.is_finite() && max.is_finite()) || max <= min {
        return Some(0.0);
    }
    Some(((v - min) / (max - min)).clamp(0.0, 1.0))
}

/// Angles en degrés depuis 3 h, sens horaire (135° + 270° = jauge ouverte vers le bas).
#[allow(clippy::too_many_arguments)]
pub fn gauge(ctx: &mut Ctx, source: &str, min: f32, max: f32, start: f32, sweep: f32, thickness: f32, track: &str, colors: &ColorRamp) -> Result<(), String> {
    let track = ctx.col(track)?;
    let t = norm(ctx, source, min, max);
    let fill = match t {
        Some(t) => Some(ctx.apply(color::ramp(colors, t)?)),
        None => None,
    };
    if ![start, sweep, thickness].iter().all(|v| v.is_finite()) || thickness <= 0.0 {
        return Err("jauge : angles ou épaisseur invalides".into());
    }
    let r = (ctx.w.min(ctx.h) / 2.0 - thickness / 2.0).max(0.5);
    let (cx, cy) = (ctx.w / 2.0, ctx.h / 2.0);
    let stroke = Stroke { width: thickness, line_cap: LineCap::Round, ..Stroke::default() };
    if let Some(p) = shapes::arc(cx, cy, r, start, sweep) {
        ctx.stroke(&p, track, &stroke);
    }
    if let (Some(t), Some(fill)) = (t, fill) {
        if t > 0.0 {
            if let Some(p) = shapes::arc(cx, cy, r, start, sweep * t) {
                ctx.stroke(&p, fill, &stroke);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn bar(ctx: &mut Ctx, source: &str, min: f32, max: f32, vertical: bool, radius: f32, track: &str, colors: &ColorRamp) -> Result<(), String> {
    let track = ctx.col(track)?;
    let t = norm(ctx, source, min, max);
    let fill = match t {
        Some(t) => Some(ctx.apply(color::ramp(colors, t)?)),
        None => None,
    };
    let (w, h) = (ctx.w, ctx.h);
    if let Some(p) = shapes::rrect(0.0, 0.0, w, h, radius) {
        ctx.fill(&p, track, tiny_skia::FillRule::Winding);
    }
    if let (Some(t), Some(fill)) = (t, fill) {
        let rect = if vertical { (0.0, h * (1.0 - t), w, h * t) } else { (0.0, 0.0, w * t, h) };
        if rect.2 > 0.01 && rect.3 > 0.01 {
            if let Some(p) = shapes::rrect(rect.0, rect.1, rect.2, rect.3, radius) {
                ctx.fill(&p, fill, tiny_skia::FillRule::Winding);
            }
        }
    }
    Ok(())
}
