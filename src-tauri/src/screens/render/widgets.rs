//! Répartition des widgets, rectangle et horloge analogique.

use super::{background, color, format, gauge, graph, shapes, text, Ctx};
use crate::screens::layout::WidgetKind;
use tiny_skia::{FillRule, LineCap, Stroke};

pub fn draw(ctx: &mut Ctx, kind: &WidgetKind) -> Result<(), String> {
    match kind {
        WidgetKind::Text { text: t, style } => {
            let c = ctx.col(&style.color)?;
            text::draw(ctx, t, style, c)
        }
        WidgetKind::Value { source, format: f, style } => {
            let c = ctx.col(&style.color)?;
            let s = ctx.value_text(source, f);
            text::draw(ctx, &s, style, c)
        }
        WidgetKind::GaugeArc { source, min, max, start_deg, sweep_deg, thickness, track, colors } => {
            gauge::gauge(ctx, source, *min, *max, *start_deg, *sweep_deg, *thickness, track, colors)
        }
        WidgetKind::Bar { source, min, max, vertical, radius, track, colors } => {
            gauge::bar(ctx, source, *min, *max, *vertical, *radius, track, colors)
        }
        WidgetKind::Graph { source, min, max, seconds, line, fill, thickness } => {
            graph::graph(ctx, source, *min, *max, *seconds, line, fill.as_deref(), *thickness)
        }
        WidgetKind::Rect { fill, stroke, stroke_width, radius } => rect(ctx, fill, stroke.as_deref(), *stroke_width, *radius),
        WidgetKind::Image { media_id, fit } => background::draw_image(ctx, media_id, *fit),
        WidgetKind::AnalogClock { face, hands, accent } => clock(ctx, face, hands, accent),
    }
}

fn rect(ctx: &mut Ctx, fill: &str, stroke: Option<&str>, stroke_width: f32, radius: f32) -> Result<(), String> {
    let fill = ctx.col(fill)?;
    let stroke_c = stroke.map(|s| ctx.col(s)).transpose()?;
    let (w, h) = (ctx.w, ctx.h);
    if let Some(p) = shapes::rrect(0.0, 0.0, w, h, radius) {
        ctx.fill(&p, fill, FillRule::Winding);
    }
    if let Some(c) = stroke_c {
        let sw = if stroke_width.is_finite() { stroke_width.clamp(0.0, w.min(h) / 2.0) } else { 0.0 };
        if sw > 0.0 {
            // Trait centré sur un rectangle rentré de sw/2 : le contour reste dans la boîte.
            let inset = sw / 2.0;
            if let Some(p) = shapes::rrect(inset, inset, w - sw, h - sw, (radius - inset).max(0.0)) {
                ctx.stroke(&p, c, &Stroke { width: sw, ..Stroke::default() });
            }
        }
    }
    Ok(())
}

fn clock(ctx: &mut Ctx, face: &str, hands: &str, accent: &str) -> Result<(), String> {
    let (face, hands, accent) = (ctx.col(face)?, ctx.col(hands)?, ctx.col(accent)?);
    let t = format::local_now();
    let (cx, cy) = (ctx.w / 2.0, ctx.h / 2.0);
    let r = ctx.w.min(ctx.h) / 2.0;
    if let Some(p) = shapes::circle(cx, cy, r) {
        ctx.fill(&p, face, FillRule::Winding);
    }
    let round = |width: f32| Stroke { width, line_cap: LineCap::Round, ..Stroke::default() };
    let at = |deg: f32, from: f32, to: f32| shapes::line(cx + r * from * deg.cos(), cy + r * from * deg.sin(), cx + r * to * deg.cos(), cy + r * to * deg.sin());
    for i in 0..12 {
        let deg = (i as f32 * 30.0 - 90.0).to_radians();
        let major = i % 3 == 0;
        let tick = color::with_opacity(hands, if major { 0.9 } else { 0.45 });
        if let Some(p) = at(deg, if major { 0.82 } else { 0.88 }, 0.94) {
            ctx.stroke(&p, tick, &round(r * if major { 0.045 } else { 0.025 }));
        }
    }
    let m = t.minute as f32 + t.second as f32 / 60.0;
    let hour_deg = ((t.hour % 12) as f32 + m / 60.0) * 30.0 - 90.0;
    let min_deg = m * 6.0 - 90.0;
    let sec_deg = t.second as f32 * 6.0 - 90.0;
    if let Some(p) = at(hour_deg.to_radians(), -0.08, 0.5) {
        ctx.stroke(&p, hands, &round(r * 0.06));
    }
    if let Some(p) = at(min_deg.to_radians(), -0.08, 0.76) {
        ctx.stroke(&p, hands, &round(r * 0.04));
    }
    if let Some(p) = at(sec_deg.to_radians(), -0.16, 0.86) {
        ctx.stroke(&p, accent, &round(r * 0.018));
    }
    if let Some(p) = shapes::circle(cx, cy, r * 0.055) {
        ctx.fill(&p, accent, FillRule::Winding);
    }
    Ok(())
}
