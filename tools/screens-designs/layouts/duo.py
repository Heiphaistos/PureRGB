"""Duo de cartes : deux mesures cote a cote (ecrans larges) ou empilees (carres)."""
from content import CONTENT
from lib import Canvas, split
from parts import stat_card

NAME, TITLE = "duo", "Duo en cartes"
CONTENTS = ["cpu-gpu", "liquide", "reseau", "gpu"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:2]
    z = fmt.zone
    cols = z[2] / z[3] >= 1.5
    for i, box in enumerate(split(z, 2, fmt.short * 0.04, horizontal=cols)):
        stat_card(c, box, ms[i], i)
    return c.finish()
