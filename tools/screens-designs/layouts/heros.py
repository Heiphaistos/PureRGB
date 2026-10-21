"""Heros + liste : grande jauge de la mesure principale (selon le contenu), liste complete des autres."""
from content import CONTENT
from lib import Canvas
from parts import ring
from layouts.r1_common import CONTENTS_ALL, block, fit, hero_split, pick, colopts, side_by_side, sizes

NAME, TITLE = "heros", "Jauge héros et liste"
CONTENTS = CONTENTS_ALL


def build(p, fmt, content):
    hero, rest = hero_split(content, CONTENT[content][1])
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04

    def mk(n, frac, cols):
        c = Canvas(p, fmt)
        wide = side_by_side(fmt)
        d = min(h, w * frac) if wide else min(w, h * frac)
        rx, ry = (x, y + (h - d) / 2) if wide else (x + (w - d) / 2, y)
        ring(c, rx + d / 2, ry + d / 2, d / 2, hero, 0, sub=True)
        box = (rx + d + gap, y, w - d - gap, h) if wide else (x, ry + d + gap, w, y + h - ry - d - gap)
        block(c, box, pick(rest, n), cols, gap, maxrow=fmt.short * 0.14)
        return c.finish()
    return fit(mk, fmt, sizes(rest), (0.42, 0.36, 0.5), colopts(fmt))
