//! Fonds (uni, dégradé, média) et dessin d'image avec ajustement cover/contain/stretch.

use super::{shapes, Ctx};
use crate::screens::layout::{Background, Fit};
use tiny_skia::{Color, FillRule, FilterQuality, GradientStop, LinearGradient, Paint, Pattern, Point, SpreadMode, Transform};

pub fn draw_background(ctx: &mut Ctx, bg: &Background) -> Result<(), String> {
    let (w, h) = (ctx.w, ctx.h);
    match bg {
        Background::Solid { color } => {
            let c = ctx.col(color)?;
            if let Some(p) = shapes::rrect(0.0, 0.0, w, h, 0.0) {
                ctx.fill(&p, c, FillRule::Winding);
            }
        }
        Background::Gradient { from, to, angle } => {
            let (a, b) = (ctx.col(from)?, ctx.col(to)?);
            let rad = if angle.is_finite() { angle.to_radians() } else { 0.0 };
            let (dx, dy) = (rad.cos(), rad.sin());
            // Longueur de projection du rectangle sur la direction : le dégradé couvre exactement les coins.
            let half = (w * dx.abs() + h * dy.abs()) / 2.0;
            let (cx, cy) = (w / 2.0, h / 2.0);
            let shader = LinearGradient::new(
                Point::from_xy(cx - dx * half, cy - dy * half),
                Point::from_xy(cx + dx * half, cy + dy * half),
                vec![GradientStop::new(0.0, a), GradientStop::new(1.0, b)],
                SpreadMode::Pad,
                Transform::identity(),
            )
            .ok_or("dégradé invalide")?;
            let paint = Paint { shader, anti_alias: false, ..Paint::default() };
            if let Some(p) = shapes::rrect(0.0, 0.0, w, h, 0.0) {
                ctx.fill_paint(&p, &paint, FillRule::Winding);
            }
        }
        Background::Media { media_id, fit } => {
            if let Some(p) = shapes::rrect(0.0, 0.0, w, h, 0.0) {
                ctx.fill(&p, Color::BLACK, FillRule::Winding);
            }
            draw_image(ctx, media_id, *fit)?;
        }
    }
    Ok(())
}

/// Dessine le média `media_id` dans la boîte (0,0,w,h) du contexte.
pub fn draw_image(ctx: &mut Ctx, media_id: &str, fit: Fit) -> Result<(), String> {
    let img = ctx.media.frame_at(media_id, ctx.t_ms).ok_or_else(|| format!("média introuvable : {media_id}"))?;
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let (sx, sy) = match fit {
        Fit::Stretch => (ctx.w / iw, ctx.h / ih),
        Fit::Cover => {
            let s = (ctx.w / iw).max(ctx.h / ih);
            (s, s)
        }
        Fit::Contain => {
            let s = (ctx.w / iw).min(ctx.h / ih);
            (s, s)
        }
    };
    let (dw, dh) = (iw * sx, ih * sy);
    let (dx, dy) = ((ctx.w - dw) / 2.0, (ctx.h - dh) / 2.0);
    // Zone remplie : l'image elle-même (contain) ou la boîte (cover/stretch, l'image la déborde).
    let area = if fit == Fit::Contain { (dx, dy, dw, dh) } else { (0.0, 0.0, ctx.w, ctx.h) };
    let quality = if sx.max(sy) >= 1.0 { FilterQuality::Bilinear } else { FilterQuality::Bicubic };
    let pattern = Pattern::new((*img).as_ref(), SpreadMode::Pad, quality, ctx.opacity, Transform::from_row(sx, 0.0, 0.0, sy, dx, dy));
    let paint = Paint { shader: pattern, anti_alias: true, ..Paint::default() };
    let path = shapes::rrect(area.0, area.1, area.2, area.3, 0.0).ok_or("zone image vide")?;
    ctx.fill_paint(&path, &paint, FillRule::Winding);
    Ok(())
}
