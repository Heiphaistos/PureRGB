//! Courbe d'historique : ligne arrondie + dégradé de remplissage vers le bas.

use super::Ctx;
use tiny_skia::{Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder, Point, SpreadMode, Stroke, Transform};

#[allow(clippy::too_many_arguments)]
pub fn graph(ctx: &mut Ctx, source: &str, min: f32, max: f32, seconds: u32, line: &str, fill: Option<&str>, thickness: f32) -> Result<(), String> {
    let line = ctx.col(line)?;
    let fill = fill.map(|f| ctx.col(f)).transpose()?;
    let data = ctx.store.history(source, seconds.clamp(1, 600));
    if data.len() < 2 || !(min.is_finite() && max.is_finite()) || max <= min {
        return Ok(());
    }
    let pad = (thickness.max(0.0) / 2.0).min(ctx.h / 4.0);
    let (w, h) = (ctx.w, ctx.h);
    let n = data.len() as f32 - 1.0;
    let pts: Vec<(f32, f32)> = data
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = ((v - min) / (max - min)).clamp(0.0, 1.0);
            (i as f32 / n * w, pad + (h - 2.0 * pad) * (1.0 - t))
        })
        .collect();

    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    for p in &pts[1..] {
        pb.line_to(p.0, p.1);
    }
    let Some(line_path) = pb.finish() else { return Ok(()) };

    if let Some(top) = fill {
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, h);
        for p in &pts {
            pb.line_to(p.0, p.1);
        }
        pb.line_to(w, h);
        pb.close();
        let bottom = Color::from_rgba(top.red(), top.green(), top.blue(), 0.0).unwrap_or(top);
        let shader = LinearGradient::new(
            Point::from_xy(0.0, 0.0),
            Point::from_xy(0.0, h),
            vec![GradientStop::new(0.0, top), GradientStop::new(1.0, bottom)],
            SpreadMode::Pad,
            Transform::identity(),
        );
        if let (Some(path), Some(shader)) = (pb.finish(), shader) {
            let paint = Paint { shader, anti_alias: true, ..Paint::default() };
            ctx.fill_paint(&path, &paint, FillRule::Winding);
        }
    }
    let stroke = Stroke { width: thickness.max(0.5), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    ctx.stroke(&line_path, line, &stroke);
    Ok(())
}
