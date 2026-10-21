"""Grand chiffre unique : une seule mesure, tres lisible de loin."""
from content import CONTENT
from lib import Canvas
from parts import glow

NAME, TITLE = "chiffre", "Grand chiffre"
CONTENTS = ["cpu", "gpu", "liquide", "ram"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    m = CONTENT[content][1][0]
    x, y, w, h = fmt.zone
    if p.dark:
        glow(c, x + w * (0.68 if fmt.wide else 0.5), y + h / 2, min(w, h) * 0.5, p.acc[0])
    if fmt.wide:
        lw = w * 0.3
        c.text(x, y + h * 0.18, lw, h * 0.2, m.label, p.acc[0], bold=True, pre="l")
        if m.sub:
            c.value(m.sub[0], m.sub[1], x, y + h * 0.42, lw, h * 0.16, m.sub[2], p.text2, pre="s")
        c.bar(x, y + h * 0.7, lw, max(6, h * 0.07), m.src, m.lo, m.hi, m.colors(p), pre="b")
        c.value(m.src, m.fmt, x + w * 0.34, y, w * 0.66, h, m.sample, p.text, bold=True, align="right", pre="v")
        return c.finish()
    c.text(x, y, w, h * 0.12, m.label, p.acc[0], bold=True, align="center", pre="l")
    c.value(m.src, m.fmt, x, y + h * 0.15, w, h * 0.5, m.sample, p.text, bold=True, align="center", pre="v")
    if m.sub:
        c.value(m.sub[0], m.sub[1], x, y + h * 0.68, w, h * 0.12, m.sub[2], p.text2, align="center", pre="s")
    c.bar(x + w * 0.15, y + h * 0.88, w * 0.7, max(6, h * 0.05), m.src, m.lo, m.hi, m.colors(p), pre="b")
    return c.finish()
