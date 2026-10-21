//! Texte : polices libres embarquées (OFL) et tracé des glyphes en chemins
//! tiny-skia (anti-aliasing, rotation et mise à l'échelle gratuites).
//!
//! Polices : "Inter" (défaut), "Roboto Mono", "Rajdhani" (affichage).
//! `size` = taille d'em en unités du canevas (comme `font-size` en CSS).

use super::Ctx;
use crate::screens::layout::{Align, TextStyle};
use ab_glyph::{Font, FontRef, OutlineCurve, VariableFont};
use std::sync::OnceLock;
use tiny_skia::{Color, FillRule, PathBuilder};

struct Face {
    name: &'static str,
    bold: bool,
    font: FontRef<'static>,
}

static FACES: OnceLock<Vec<Face>> = OnceLock::new();

fn load() -> Vec<Face> {
    let mut faces = Vec::new();
    let mut add = |name: &'static str, bold: bool, data: &'static [u8], wght: Option<f32>| match FontRef::try_from_slice(data) {
        Ok(mut font) => {
            if let Some(w) = wght {
                font.set_variation(b"wght", w);
            }
            faces.push(Face { name, bold, font });
        }
        Err(e) => log::error!("police embarquée {name} illisible: {e}"),
    };
    const INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter.ttf");
    const MONO: &[u8] = include_bytes!("../../../assets/fonts/RobotoMono.ttf");
    add("Inter", false, INTER, Some(400.0));
    add("Inter", true, INTER, Some(700.0));
    add("Roboto Mono", false, MONO, Some(400.0));
    add("Roboto Mono", true, MONO, Some(700.0));
    add("Rajdhani", false, include_bytes!("../../../assets/fonts/Rajdhani-Medium.ttf"), None);
    add("Rajdhani", true, include_bytes!("../../../assets/fonts/Rajdhani-Bold.ttf"), None);
    faces
}

/// Police par nom (insensible à la casse) ; repli sur la première police embarquée.
fn font(name: &str, bold: bool) -> Option<&'static FontRef<'static>> {
    let faces = FACES.get_or_init(load);
    let pick = |n: &str| faces.iter().find(|f| f.name.eq_ignore_ascii_case(n) && f.bold == bold);
    pick(name.trim()).or_else(|| faces.iter().find(|f| f.bold == bold)).or_else(|| faces.first()).map(|f| &f.font)
}

fn em(font: &FontRef<'static>) -> f32 {
    font.units_per_em().unwrap_or_else(|| font.height_unscaled()).max(1.0)
}

/// Largeur du texte à la taille `size`.
fn measure(font: &FontRef<'static>, text: &str, size: f32) -> f32 {
    let k = size / em(font);
    let mut x = 0.0;
    let mut prev = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(p) = prev {
            x += font.kern_unscaled(p, id) * k;
        }
        x += font.h_advance_unscaled(id) * k;
        prev = Some(id);
    }
    x
}

/// Dessine `text` dans la boîte du widget. Le texte trop large est réduit pour tenir.
pub fn draw(ctx: &mut Ctx, text: &str, style: &TextStyle, color: Color) -> Result<(), String> {
    if !(style.size.is_finite() && style.size > 0.0) || text.is_empty() {
        return Ok(());
    }
    let font = font(&style.font, style.bold).ok_or("aucune police embarquée")?;
    let mut size = style.size.min(2000.0);
    let width = measure(font, text, size);
    if width > ctx.w && ctx.w > 0.0 {
        size *= ctx.w / width;
    }
    let width = measure(font, text, size);
    let k = size / em(font);
    // Centrage vertical sur la hauteur des capitales (les chiffres y sont alignés).
    let cap = font.outline(font.glyph_id('H')).map(|o| (o.bounds.max.y - o.bounds.min.y).abs() * k).unwrap_or(size * 0.7);
    let x0 = match style.align {
        Align::Left => 0.0,
        Align::Center => (ctx.w - width) / 2.0,
        Align::Right => ctx.w - width,
    };
    let baseline = ctx.h / 2.0 + cap / 2.0;

    let mut pb = PathBuilder::new();
    let mut pen = x0;
    let mut prev = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(p) = prev {
            pen += font.kern_unscaled(p, id) * k;
        }
        if let Some(outline) = font.outline(id) {
            let mut last: Option<(f32, f32)> = None;
            let pt = |p: ab_glyph::Point| (pen + p.x * k, baseline - p.y * k);
            for curve in &outline.curves {
                let start = match curve {
                    OutlineCurve::Line(a, _) | OutlineCurve::Quad(a, _, _) | OutlineCurve::Cubic(a, _, _, _) => pt(*a),
                };
                if last.is_none_or(|l| (l.0 - start.0).abs() > 0.001 || (l.1 - start.1).abs() > 0.001) {
                    if last.is_some() {
                        pb.close();
                    }
                    pb.move_to(start.0, start.1);
                }
                last = Some(match curve {
                    OutlineCurve::Line(_, b) => {
                        let b = pt(*b);
                        pb.line_to(b.0, b.1);
                        b
                    }
                    OutlineCurve::Quad(_, c1, b) => {
                        let (c1, b) = (pt(*c1), pt(*b));
                        pb.quad_to(c1.0, c1.1, b.0, b.1);
                        b
                    }
                    OutlineCurve::Cubic(_, c1, c2, b) => {
                        let (c1, c2, b) = (pt(*c1), pt(*c2), pt(*b));
                        pb.cubic_to(c1.0, c1.1, c2.0, c2.1, b.0, b.1);
                        b
                    }
                });
            }
            if last.is_some() {
                pb.close();
            }
        }
        pen += font.h_advance_unscaled(id) * k;
        prev = Some(id);
    }
    if let Some(path) = pb.finish() {
        ctx.fill(&path, color, FillRule::Winding);
    }
    Ok(())
}
