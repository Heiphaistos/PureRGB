//! Contrôle qualité des designs générés (`gen-*`) : géométrie, textes, ids, taille.
//! Le rendu raccourcit un texte trop large ; ici on vérifie que le design ne le demande pas
//! (estimation de largeur par police, large tolérance : le générateur Python mesure pour de vrai).

use super::{builtin, parse};
use crate::screens::layout::{Layout, WidgetKind};

const TOL: f32 = 2.0;

/// Largeur moyenne d'un caractère / taille d'em (estimation prudente, texte gras).
fn char_w(font: &str, bold: bool) -> f32 {
    match (font, bold) {
        ("Rajdhani", _) => 0.44,
        ("Roboto Mono", _) => 0.6,
        (_, true) => 0.62,
        _ => 0.56,
    }
}

/// Longueur affichée estimée d'un format (`{:.1}` -> 4 caractères, `%A` -> 8, etc.).
fn shown_len(fmt: &str) -> usize {
    let (mut n, mut it) = (0, fmt.chars());
    while let Some(c) = it.next() {
        match c {
            '{' => {
                // `{:.0}` -> 3 chiffres (100) ; `{:.N}` -> 2 chiffres + point + N décimales.
                let spec: String = it.by_ref().take_while(|d| *d != '}').collect();
                n += match spec.strip_prefix(":.").and_then(|d| d.parse::<usize>().ok()) {
                    Some(d) if d > 0 => 3 + d,
                    _ => 3,
                };
            }
            '%' => n += if matches!(it.next(), Some('A') | Some('B')) { 8 } else { 2 },
            _ => n += 1,
        }
    }
    n
}

fn bbox(w: &crate::screens::layout::Widget) -> (f32, f32, f32, f32) {
    let (cx, cy) = (w.x + w.w / 2.0, w.y + w.h / 2.0);
    let a = w.rotation.to_radians();
    let ex = (w.w * a.cos()).abs() / 2.0 + (w.h * a.sin()).abs() / 2.0;
    let ey = (w.w * a.sin()).abs() / 2.0 + (w.h * a.cos()).abs() / 2.0;
    (cx - ex, cy - ey, cx + ex, cy + ey)
}

fn problems(l: &Layout) -> Vec<String> {
    let mut out = Vec::new();
    if l.widgets.len() > 200 {
        out.push(format!("{} widgets", l.widgets.len()));
    }
    let ids: std::collections::HashSet<_> = l.widgets.iter().map(|w| w.id.as_str()).collect();
    if ids.len() != l.widgets.len() {
        out.push("ids de widgets dupliqués".into());
    }
    let (cw, ch) = (l.width as f32, l.height as f32);
    for w in &l.widgets {
        let (x0, y0, x1, y1) = bbox(w);
        if x0 < -TOL || y0 < -TOL || x1 > cw + TOL || y1 > ch + TOL {
            out.push(format!("{} sort du canevas ({x0:.0},{y0:.0})-({x1:.0},{y1:.0})", w.id));
        }
        let text = match &w.kind {
            WidgetKind::Text { text, style } => Some((text.chars().count(), style)),
            WidgetKind::Value { format, style, .. } => Some((shown_len(format), style)),
            _ => None,
        };
        if let Some((len, st)) = text {
            let est = len as f32 * st.size * char_w(&st.font, st.bold);
            if est > w.w * 1.2 + 4.0 {
                out.push(format!("{} : texte estimé {est:.0} px > boîte {:.0}", w.id, w.w));
            }
            if st.size > w.h * 1.3 + 2.0 {
                out.push(format!("{} : police {} dépasse la boîte de {:.0}", w.id, st.size, w.h));
            }
        }
    }
    out
}

#[test]
fn generated_designs_pass_quality_checks() {
    let mut count = 0;
    let mut bad = Vec::new();
    for b in builtin::all().filter(|b| b.id.starts_with("gen-")) {
        count += 1;
        let l = parse(b.json).unwrap_or_else(|e| panic!("{} : {e:#}", b.id));
        if b.id.len() > 64 || b.category.is_empty() || b.tags.is_empty() {
            bad.push(format!("{} : id/catégorie/tags invalides", b.id));
        }
        for p in problems(&l) {
            bad.push(format!("{} : {p}", b.id));
        }
    }
    assert!(count >= 100, "seulement {count} designs générés");
    assert!(bad.is_empty(), "{} problèmes :\n{}", bad.len(), bad.join("\n"));
}
