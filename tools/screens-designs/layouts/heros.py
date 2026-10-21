"""Heros + liste : grande jauge de la mesure principale, trois lignes secondaires."""
from content import CONTENT
from lib import Canvas, split
from parts import hero_list, ring, row

NAME, TITLE = "heros", "Jauge héros et liste"
CONTENTS = ["quad", "cpu", "gpu", "disques"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    if fmt.ratio >= 1.5:
        d = min(h, w * 0.45)
        rx, lx, lw = hero_list(fmt.zone, d, gap)
        ring(c, rx + d / 2, y + h / 2, d / 2, ms[0], 0, sub=True)
        for i, box in enumerate(split((lx, y + h * 0.06, lw, h * 0.88), 3, gap, horizontal=False)):
            row(c, box, ms[1 + i], i + 1)
        return c.finish()
    r = min(w / 2, h * 0.3)
    ring(c, x + w / 2, y + r, r, ms[0], 0, sub=True)
    top = y + r * 2 + gap * 0.5
    for i, box in enumerate(split((x, top, w, y + h - top), 3, gap * 0.6, horizontal=False)):
        row(c, box, ms[1 + i], i + 1)
    return c.finish()
