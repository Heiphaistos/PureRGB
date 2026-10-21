"""Tableau de bord de jauges : une jauge 270 deg par mesure, valeur au centre."""
from content import CONTENT
from lib import Canvas, grid
from parts import ring

NAME, TITLE = "jauges", "Tableau de bord à jauges"
CONTENTS = ["quad", "gpu", "cpu", "refroidissement"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    n = 4 if fmt.ratio < 1.5 or fmt.strip else 3
    for i, (x, y, w, h) in enumerate(grid(fmt.zone, n, fmt.short * 0.02, target=1.0)):
        r = min(w, h) / 2
        ring(c, x + w / 2, y + h / 2, r, ms[i], i)
    return c.finish()
