"""Grand chiffre : une mesure en tres grand (selon le contenu), les autres en lignes compactes."""
from content import CONTENT
from lib import Canvas
from parts import glow
from layouts.r1_common import CONTENTS_ALL, block, fit, hero_split, pick, colopts, side_by_side, sizes

NAME, TITLE = "chiffre", "Grand chiffre"
CONTENTS = [k for k in CONTENTS_ALL if k != "quad"]


def build(p, fmt, content):
    hero, rest = hero_split(content, CONTENT[content][1])
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04

    def mk(n, frac, cols):
        c = Canvas(p, fmt)
        wide = side_by_side(fmt)
        hx, hy, hw, hh = (x, y, w * frac, h) if wide else (x, y, w, h * frac)
        if p.dark:
            glow(c, hx + hw / 2, hy + hh / 2, min(hw, hh) * 0.55, p.acc[0])
        c.text(hx, hy + hh * 0.04, hw, hh * 0.14, hero.label, p.acc[0], bold=True, align="center", pre="l")
        c.value(hero.src, hero.fmt, hx, hy + hh * 0.2, hw, hh * 0.5, hero.sample + ("0" if p.font == "Roboto Mono" else ""), p.text, bold=True, align="center", pre="v")  # le test Rust estime la chasse mono plus large
        if hero.sub:
            c.value(hero.sub[0], hero.sub[1], hx, hy + hh * 0.72, hw, hh * 0.12, hero.sub[2], p.text2, align="center", pre="s")
        c.bar(hx + hw * 0.1, hy + hh * 0.9, hw * 0.8, max(6, hh * 0.05), hero.src, hero.lo, hero.hi, hero.colors(p), pre="b")
        box = (x + hw + gap, y, w - hw - gap, h) if wide else (x, y + hh + gap, w, h - hh - gap)
        block(c, box, pick(rest, n), cols, gap, maxrow=fmt.short * 0.14)
        return c.finish()
    return fit(mk, fmt, sizes(rest), (0.46, 0.4, 0.55), colopts(fmt))
