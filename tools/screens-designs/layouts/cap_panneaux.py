"""Panneaux par famille : titre, valeur vedette + barre, puis lignes de détail dans une carte."""
from content import CONTENT
from lib import Canvas, grid, min_text, split
from parts import row
from layouts.cap_common import GROUPS, densest

NAME, TITLE = "cap-panneaux", "Panneaux détaillés"
CONTENTS = ["cap-complet", "cap-stockage", "cap-refroid"]
FORMATS = ["s480", "w169", "w1280", "u1920", "r43"]


def build(p, fmt, content):
    ms, groups = CONTENT[content][1], GROUPS[content]
    mt = min_text(fmt)

    def mk(k):
        c = Canvas(p, fmt)
        gap = fmt.short * 0.03
        boxes = split(fmt.zone, len(groups), gap) if fmt.wide else grid(fmt.zone, len(groups), gap, 1.0)
        for g, ((title, a, b), box) in enumerate(zip(groups, boxes)):
            c.card(*box)
            pad = min(box[2], box[3]) * 0.07
            x, y, w, h = box[0] + pad, box[1] + pad, box[2] - 2 * pad, box[3] - 2 * pad
            th = max(1.3 * mt, h * 0.1)
            c.text(x, y, w, th, title, p.acc[g % 3], bold=True, pre="h", group="ph")
            m, bh = ms[a], max(4.0, h * 0.03)
            avail = h - th * 1.1 - bh - h * 0.1
            vh = min(avail - k * 2.0 * mt, h * 0.5)
            if vh < h * 0.2:
                return None
            c.value(m.src, m.fmt, x, y + th * 1.1, w, vh, m.sample, p.text, bold=True, pre="v", group="pv")
            c.bar(x, y + th * 1.1 + vh * 1.05, w, bh, m.src, m.lo, m.hi, m.colors(p, g), pre="b")
            top = y + th * 1.1 + vh * 1.05 + bh + h * 0.06
            if k:
                rh = min((y + h - top) / k, fmt.short * 0.12)
                for r, mm in enumerate(ms[a + 1:b][:k]):
                    row(c, (x, top + r * rh, w, rh * 0.94), mm, g + r, bar=False)
        return c.finish()
    return densest(mk, fmt, range(max(b - a for _, a, b in groups) - 1, -1, -1))
