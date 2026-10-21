"""Egaliseur : barres verticales, une par mesure, valeur au-dessus et libelle dessous."""
from content import CONTENT
from lib import Canvas, split

NAME, TITLE = "egaliseur", "Égaliseur vertical"
CONTENTS = ["quad", "disques", "refroidissement", "carte-mere"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:6]
    x, y, w, h = fmt.zone
    n = len(ms)
    th = h * 0.15
    for i, box in enumerate(split(fmt.zone, n, fmt.short * 0.025)):
        bx, by, bw, bh = box
        m = ms[i]
        c.value(m.src, m.fmt, bx - 4, by, bw + 8, th, m.sample, p.text, bold=True, align="center", pre="v", group="ev")
        barw = min(bw * 0.62, h * 0.4)
        c.rect(bx + (bw - barw) / 2 - 3, by + th * 1.15 - 3, barw + 6, bh - 2.3 * th + 6, p.card, barw * 0.3 + 3, pre="f")
        c.bar(bx + (bw - barw) / 2, by + th * 1.15, barw, bh - 2.3 * th, m.src, m.lo, m.hi, m.colors(p, i), vertical=True, radius=barw * 0.3, pre="b")
        c.text(bx - 4, by + bh - th, bw + 8, th, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="el", maxsize=th * 0.8)
    return c.finish()
