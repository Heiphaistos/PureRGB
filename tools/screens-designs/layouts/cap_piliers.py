"""Histogramme de capteurs : jusqu'à 12 piliers verticaux, valeur au-dessus, libellé dessous."""
from content import CONTENT
from lib import Canvas, min_text, split
from layouts.cap_common import densest

NAME, TITLE = "cap-piliers", "Piliers de capteurs"
CONTENTS = ["cap-refroid", "cap-thermique", "cap-stockage"]
FORMATS = ["s480", "w169", "w1280", "u1920", "rb640"]


def build(p, fmt, content):
    ms = CONTENT[content][1]
    mt = min_text(fmt)

    def mk(n):
        c = Canvas(p, fmt)
        for i, (x, y, w, h) in enumerate(split(fmt.zone, n, fmt.short * 0.012)):
            m, th = ms[i], max(1.4 * mt, h * 0.12)
            c.value(m.src, m.fmt, x, y, w, th, m.sample, p.text, bold=True, align="center", pre="v", group="pv")
            bw = min(w * 0.7, h * 0.4)
            c.rect(x + (w - bw) / 2 - 3, y + th * 1.15 - 3, bw + 6, h - 2.3 * th + 6, p.card, bw * 0.3 + 3, pre="f")
            c.bar(x + (w - bw) / 2, y + th * 1.15, bw, h - 2.3 * th, m.src, m.lo, m.hi, m.colors(p, i), vertical=True, radius=bw * 0.3, pre="b")
            c.text(x, y + h - th, w, th, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="pl")
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 12), 3, -1))
