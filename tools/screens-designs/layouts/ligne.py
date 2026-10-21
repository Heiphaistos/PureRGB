"""Ligne unique : mesures en ligne separees par des puces, barre de charge pleine largeur (formats tres larges)."""
from content import CONTENT
from lib import Canvas, measure

NAME, TITLE = "ligne", "Ligne unique"
CONTENTS = ["quad", "carte-mere", "refroidissement"]
FORMATS = ["w1280", "u1920", "rb640"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    allm = CONTENT[content][1][:5]
    x, y, w, h = fmt.zone
    for count in range(len(allm), 1, -1):   # autant d'items que la largeur le permet sans petits caracteres
        ms = allm[:count]
        vs = h * 0.42
        gap = vs * 0.9
        while True:
            ls = vs * 0.5
            parts = [(measure(m.label, p.font, True, ls), measure(m.sample, p.font, True, vs)) for m in ms]
            total = sum(a + b + ls * 0.6 for a, b in parts) + gap * (len(ms) - 1) + vs * 0.5 * len(ms)
            if total <= w or vs < 10:
                break
            vs *= 0.96
            gap = vs * 0.9
        if ls >= 15:
            break
    cx = x + (w - total) / 2
    cy = y + h * 0.5 - vs * 0.9
    for i, m in enumerate(ms):
        lw, vw = parts[i]
        c.rect(cx, cy + vs * 0.5 - vs * 0.16, vs * 0.32, vs * 0.32, p.acc[i % 3], vs * 0.16, pre="dot")
        cx += vs * 0.5
        c.text(cx, cy, lw, vs * 1.15, m.label, p.text2, bold=True, size=ls, pre="l")
        cx += lw + ls * 0.6
        c.value(m.src, m.fmt, cx, cy, vw, vs * 1.15, m.sample, p.text, bold=True, size=vs, pre="v")
        cx += vw + gap
    by = y + h - max(6, h * 0.1)
    c.bar(x, by, w, max(6, h * 0.1), ms[0].src, ms[0].lo, ms[0].hi, ms[0].colors(p), pre="b")
    return c.finish()
