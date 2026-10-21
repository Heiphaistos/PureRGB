"""Cartes translucides sur les deux flancs, centre LIBRE pour une video ou un GIF de fond (ultra-large, large)."""
from content import CONTENT
from lib import Canvas, split
from parts import stat_card

NAME, TITLE = "uw_flancs", "Flancs, centre libre"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair", "uw-lpuis", "uw-lgpu", "uw-lcpu", "uw-lreseau", "uw-ldisq", "uw-lmix"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    big = fmt.w > 1000
    n = 4 if big else 3
    cw = w * (0.27 if fmt.w > 1500 else 0.31 if big else 0.34)
    gap = fmt.short * 0.03
    for side, sx in enumerate((x, x + w - cw)):
        for i, box in enumerate(split((sx, y, cw, h), n, gap, horizontal=False)):
            with c.m(ms[side * n + i]):
                stat_card(c, box, ms[side * n + i], i + side)
    return c.finish()
