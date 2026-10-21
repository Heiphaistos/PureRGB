"""Colonnes par famille (2x2 sur les carrés) : titres, filets, lignes à barres."""
from lib import Canvas, min_text
from layouts.cap_common import ALL, GROUPS, boxes, crow, densest, fracs

NAME, TITLE = "cap-colonnes", "Colonnes par famille"
CONTENTS = ALL
FORMATS = ["s480", "s320", "w169", "w1280", "u1920", "rb640", "r43"]


def build(p, fmt, content):
    groups = GROUPS[content]
    mt = min_text(fmt)

    def mk(k):
        c = Canvas(p, fmt)
        gap = fmt.short * 0.05
        bxs, cols = boxes(fmt, len(groups), gap)
        for g, ((title, ms), (x, y, w, h)) in enumerate(zip(groups, bxs)):
            if cols and g:
                c.rect(x - gap / 2 - 1, y + h * 0.05, 2, h * 0.9, p.line, 1, pre="sep")
            hh = max(1.4 * mt, h * 0.12)
            c.text(x, y, w, hh, title, p.acc[g % 3], bold=True, pre="h", group="qh")
            c.rect(x, y + hh * 1.05, w, 2, p.acc[g % 3], 1, pre="ln")
            fr = fracs(p, ms[:k])
            rh = min((h - hh * 1.25) / k, fmt.short * 0.13)
            for r, m in enumerate(ms[:k]):
                with c.m(m):
                    crow(c, (x, y + hh * 1.25 + r * rh, w, rh * 0.94), m, g + r, fr)
        return c.finish()
    return densest(mk, fmt, range(max(len(ms) for _, ms in groups), 1, -1), floor=0)
