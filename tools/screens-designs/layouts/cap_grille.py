"""Grille dense de mini-cartes (2x3, 3x3, 4x3...) : le plus de capteurs possible."""
from content import CONTENT
from lib import Canvas, grid
from parts import stat_card
from layouts.cap_common import densest

NAME, TITLE = "cap-grille", "Grille de capteurs"
CONTENTS = ["cap-systeme", "cap-thermique", "cap-complet"]


def build(p, fmt, content):
    ms = CONTENT[content][1]

    def mk(n):
        c = Canvas(p, fmt)
        for i, cell in enumerate(grid(fmt.zone, n, fmt.short * 0.025, 1.3)):
            stat_card(c, cell, ms[i], i, sub=False)
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 12), 3, -1))
