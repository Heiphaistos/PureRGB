"""Colonnes CPU | GPU | Système | Disques (2x2 sur les carrés) : titres, filets, lignes à barres."""
from content import CONTENT
from lib import Canvas, grid, min_text, split
from parts import row
from layouts.cap_common import GROUPS, densest

NAME, TITLE = "cap-colonnes", "Colonnes par famille"
CONTENTS = ["cap-complet", "cap-stockage", "cap-refroid"]
FORMATS = ["s480", "s320", "w169", "w1280", "u1920", "rb640"]


def build(p, fmt, content):
    ms, groups = CONTENT[content][1], GROUPS[content]
    mt = min_text(fmt)

    def mk(k):
        c = Canvas(p, fmt)
        gap = fmt.short * 0.05
        boxes = split(fmt.zone, len(groups), gap) if fmt.wide else grid(fmt.zone, len(groups), gap, 1.2)
        for g, ((title, a, b), (x, y, w, h)) in enumerate(zip(groups, boxes)):
            if fmt.wide and g:
                c.rect(x - gap / 2 - 1, y + h * 0.05, 2, h * 0.9, p.line, 1, pre="sep")
            hh = max(1.4 * mt, h * 0.12)
            c.text(x, y, w, hh, title, p.acc[g % 3], bold=True, pre="h", group="qh")
            c.rect(x, y + hh * 1.05, w, 2, p.acc[g % 3], 1, pre="ln")
            rh = min((h - hh * 1.25) / k, fmt.short * 0.13)
            for r, m in enumerate(ms[a:b][:k]):
                row(c, (x, y + hh * 1.25 + r * rh, w, rh * 0.94), m, g + r)
        return c.finish()
    return densest(mk, fmt, range(max(b - a for _, a, b in groups), 1, -1))
