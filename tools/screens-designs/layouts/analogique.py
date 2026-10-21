"""Horloge analogique avec trois capteurs (a cote ou dessous selon le format)."""
from content import CONTENT
from lib import Canvas, split
from parts import hero_list, mini_col, row

NAME, TITLE = "analogique", "Horloge analogique"
CONTENTS = ["cpu-gpu", "quad"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:3]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    face = p.card if p.dark else "#ffffff"
    if fmt.ratio >= 1.5:
        d = min(h, w * 0.42)
        rx, lx, lw = hero_list(fmt.zone, d, gap)
        c.clock(rx, y + (h - d) / 2, d, d, face, p.text, p.acc[0])
        for i, box in enumerate(split((lx, y + h * 0.04, lw, h * 0.92), 3, gap, horizontal=False)):
            row(c, box, ms[i], i)
        return c.finish()
    d = min(w, h * 0.62)
    c.clock(x + (w - d) / 2, y, d, d, face, p.text, p.acc[0])
    for i, box in enumerate(split((x, y + d + gap * 1.5, w, y + h - (y + d + gap * 1.5)), 3, gap)):
        mini_col(c, box, ms[i], i)
    return c.finish()
