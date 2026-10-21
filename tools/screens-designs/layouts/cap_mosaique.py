"""Héros + mosaïque : grande jauge centrale et mini-cartes de tous les autres capteurs."""
from content import CONTENT
from lib import Canvas, grid
from parts import ring, stat_card
from layouts.cap_common import densest

NAME, TITLE = "cap-mosaique", "Héros et mosaïque"
CONTENTS = ["cap-systeme", "cap-complet"]
FORMATS = ["s480", "s320", "r43", "w169", "w1280", "u1920"]


def build(p, fmt, content):
    ms = CONTENT[content][1]

    def mk(n):
        c = Canvas(p, fmt)
        x, y, w, h = fmt.zone
        gap = fmt.short * 0.03
        if fmt.ratio >= 1.5:
            d = h
            ring(c, x + d / 2, y + d / 2, d / 2 * 0.97, ms[0], 0, sub=True)
            rest = (x + d + gap * 2, y, w - d - gap * 2, h)
        else:
            d = min(w, h * 0.42)
            ring(c, x + w / 2, y + d / 2, d / 2 * 0.97, ms[0], 0)
            rest = (x, y + d + gap, w, h - d - gap)
        for i, cell in enumerate(grid(rest, n, gap, 1.4)):
            stat_card(c, cell, ms[i + 1], i + 1, sub=False)
        return c.finish()
    return densest(mk, fmt, range(min(len(ms) - 1, 9), 1, -1))
