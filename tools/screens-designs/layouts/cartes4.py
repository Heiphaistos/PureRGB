"""Quatre cartes : CPU / GPU / RAM / SSD (ou disques, carte mere) en grille adaptative."""
from content import CONTENT
from lib import Canvas, grid
from parts import stat_card

NAME, TITLE = "cartes4", "Quatre cartes"
CONTENTS = ["quad", "disques", "carte-mere", "puissance"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:4]
    for i, box in enumerate(grid(fmt.zone, 4, fmt.short * 0.035, target=1.2)):
        stat_card(c, box, ms[i], i)
    return c.finish()
