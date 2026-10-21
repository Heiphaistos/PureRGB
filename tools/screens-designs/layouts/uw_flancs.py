"""Cartes translucides sur les deux flancs, centre LIBRE pour une video ou un GIF de fond (ultra-large, large)."""
from content import CONTENT
from lib import Canvas, split
from parts import stat_card

NAME, TITLE = "uw_flancs", "Flancs, centre libre"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair"]
FORMATS = ["u1920", "w1280"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    cw = w * (0.27 if fmt.w > 1500 else 0.31)
    gap = fmt.short * 0.03
    for side, sx in enumerate((x, x + w - cw)):
        for i, box in enumerate(split((sx, y, cw, h), 4, gap, horizontal=False)):
            stat_card(c, box, ms[side * 4 + i], i + side)
    return c.finish()
