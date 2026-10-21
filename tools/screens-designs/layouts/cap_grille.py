"""Grille dense de mini-cartes (2x3, 3x3, 4x3...) : le plus de capteurs possible."""
from lib import Canvas, grid
from parts import stat_card
from layouts.cap_common import ALL, densest, metrics

NAME, TITLE = "cap-grille", "Grille de capteurs"
CONTENTS = ALL


def build(p, fmt, content):
    ms = metrics(content)

    def mk(n):
        c = Canvas(p, fmt)
        for i, cell in enumerate(grid(fmt.zone, n, fmt.short * 0.025, 1.3)):
            with c.m(ms[i]):
                stat_card(c, cell, ms[i], i, sub=False)
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 12), 6, -1))
