"""Egaliseur : barres verticales, une par mesure ; valeur et valeur secondaire au-dessus, libelle dessous."""
from content import CONTENT
from lib import Canvas, split
from layouts.r1_common import CONTENTS_ALL, fit, pick, sizes

NAME, TITLE = "egaliseur", "Égaliseur vertical"
CONTENTS = CONTENTS_ALL


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone

    def mk(n, k):
        items = pick(ms, n)
        c = Canvas(p, fmt)
        th = h * k
        for i, (box, m) in enumerate(zip(split(fmt.zone, n, fmt.short * 0.02), items)):
            bx, by, bw, bh = box
            top = by + th * (1.9 if m.sub else 1.1)
            barw = min(bw * 0.62, h * 0.4)
            with c.m(m):
                c.value(m.src, m.fmt, bx - 3, by, bw + 6, th, m.sample, p.text, bold=True, align="center", pre="v", group="ev")
                if m.sub:
                    c.value(m.sub[0], m.sub[1], bx - 3, by + th * 0.95, bw + 6, th * 0.85, m.sub[2], p.text2, align="center", pre="s", group="es")
                c.rect(bx + (bw - barw) / 2 - 3, top - 3, barw + 6, by + bh - th * 1.15 - top + 6, p.card, barw * 0.3 + 3, pre="f")
                c.bar(bx + (bw - barw) / 2, top, barw, by + bh - th * 1.15 - top, m.src, m.lo, m.hi, m.colors(p, i), vertical=True, radius=barw * 0.3, pre="b")
                c.text(bx - 3, by + bh - th, bw + 6, th, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="el", maxsize=th * 0.8)
        return c.finish()
    return fit(mk, fmt, sizes(ms), (0.13, 0.11))
