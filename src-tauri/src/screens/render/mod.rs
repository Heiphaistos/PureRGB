//! Moteur de rendu des designs d'écran : `Layout` + valeurs + médias -> `Frame`
//! RGBA (tiny-skia, anti-aliasing). Même moteur pour l'aperçu de l'éditeur et
//! pour l'écran réel.
//!
//! Le canevas de conception est mis à l'échelle vers la résolution de sortie
//! (échelles X et Y indépendantes : garder le ratio du design). Un widget
//! invalide (couleur, média, nombre non fini) est ignoré et journalisé, jamais
//! de panique.

mod background;
mod color;
mod format;
mod gauge;
mod graph;
mod shapes;
mod text;
mod widgets;
#[cfg(test)]
mod tests;

use crate::screens::bindings::ValueStore;
use crate::screens::driver::Frame;
use crate::screens::layout::Layout;
use crate::screens::media::MediaStore;
use anyhow::{Context, Result};
use image::ImageEncoder;
use tiny_skia::{Color, FillRule, Paint, Path, Pixmap, Stroke, Transform};

/// Taille de sortie maximale acceptée.
pub const MAX_OUTPUT: u32 = 8192;

/// Contexte de dessin d'un widget : `ts` convertit l'espace local du widget
/// (0..w, 0..h en unités du canevas) en pixels de sortie.
pub(crate) struct Ctx<'a> {
    pub pm: &'a mut Pixmap,
    pub store: &'a ValueStore,
    pub media: &'a MediaStore,
    pub t_ms: u64,
    pub ts: Transform,
    pub w: f32,
    pub h: f32,
    pub opacity: f32,
}

impl Ctx<'_> {
    /// Couleur "#hex" avec l'opacité du widget appliquée.
    pub fn col(&self, s: &str) -> Result<Color, String> {
        color::parse(s).map(|c| self.apply(c))
    }

    pub fn apply(&self, c: Color) -> Color {
        color::with_opacity(c, self.opacity)
    }

    pub fn value(&self, source: &str) -> Option<f64> {
        self.store.get(source)
    }

    /// Texte d'une valeur : `time`/`date` en heure locale (strftime), le reste numérique.
    pub fn value_text(&self, source: &str, fmt: &str) -> String {
        match source {
            "time" | "date" => format::strftime(fmt, &format::local_now()),
            _ => format::format_value(fmt, self.value(source)),
        }
    }

    pub fn fill(&mut self, path: &Path, color: Color, rule: FillRule) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        self.fill_paint(path, &paint, rule);
    }

    pub fn fill_paint(&mut self, path: &Path, paint: &Paint, rule: FillRule) {
        self.pm.fill_path(path, paint, rule, self.ts, None);
    }

    pub fn stroke(&mut self, path: &Path, color: Color, stroke: &Stroke) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        self.pm.stroke_path(path, &paint, stroke, self.ts, None);
    }
}

fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Rend `layout` en `out_w` x `out_h` à l'instant `t_ms` (animation des GIF/fonds média).
pub fn render(layout: &Layout, store: &ValueStore, media: &MediaStore, out_w: u32, out_h: u32, t_ms: u64) -> Frame {
    let (out_w, out_h) = (out_w.min(MAX_OUTPUT), out_h.min(MAX_OUTPUT));
    let black = || Frame { width: out_w, height: out_h, rgba: [0, 0, 0, 255].repeat(out_w as usize * out_h as usize) };
    if let Err(e) = layout.validate() {
        log::warn!("design refusé : {e}");
        return black();
    }
    let Some(mut pm) = Pixmap::new(out_w, out_h) else { return black() };
    pm.fill(Color::BLACK);
    let (cw, ch) = (layout.width as f32, layout.height as f32);
    let base = Transform::from_scale(out_w as f32 / cw, out_h as f32 / ch);

    let mut bg = Ctx { pm: &mut pm, store, media, t_ms, ts: base, w: cw, h: ch, opacity: 1.0 };
    if let Err(e) = background::draw_background(&mut bg, &layout.background) {
        log::warn!("fond ignoré : {e}");
    }

    for wd in &layout.widgets {
        if wd.visible_if.as_deref().is_some_and(|k| store.get(k).is_none()) {
            continue;
        }
        let opacity = if wd.opacity.is_finite() { wd.opacity.clamp(0.0, 1.0) } else { 1.0 };
        if !finite(&[wd.x, wd.y, wd.w, wd.h, wd.rotation]) || wd.w <= 0.0 || wd.h <= 0.0 || opacity <= 0.0 {
            if opacity > 0.0 {
                log::warn!("widget {} ignoré : géométrie invalide", wd.id);
            }
            continue;
        }
        let ts = base
            .pre_translate(wd.x + wd.w / 2.0, wd.y + wd.h / 2.0)
            .pre_rotate(wd.rotation)
            .pre_translate(-wd.w / 2.0, -wd.h / 2.0);
        let mut ctx = Ctx { pm: &mut pm, store, media, t_ms, ts, w: wd.w, h: wd.h, opacity };
        if let Err(e) = widgets::draw(&mut ctx, &wd.kind) {
            log::warn!("widget {} ignoré : {e}", wd.id);
        }
    }
    Frame { width: out_w, height: out_h, rgba: pm.take() }
}

/// PNG de la frame encodé en base64 (sans préfixe `data:`), pour l'aperçu de l'éditeur.
pub fn frame_to_png_base64(frame: &Frame) -> Result<String> {
    anyhow::ensure!(frame.is_valid() && frame.width > 0 && frame.height > 0, "frame invalide");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&frame.rgba, frame.width, frame.height, image::ExtendedColorType::Rgba8)
        .context("encodage PNG")?;
    Ok(base64(&png))
}

fn base64(data: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, b)| acc | (*b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHA[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
