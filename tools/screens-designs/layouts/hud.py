"""HUD gaming : equerres aux coins, grande jauge centrale doublee d'un fin anneau, stats sur les cotes."""
from content import CONTENT
from lib import Canvas, split
from parts import corners, mini_col, ring, row

NAME, TITLE = "hud", "HUD gaming"
CONTENTS = ["gpu", "cpu", "quad"]


def accepts(p):
    return p.dark


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    mg = fmt.margin * 0.5
    side = fmt.short * 0.68 if fmt.round else None
    box = ((fmt.w - side) / 2, (fmt.h - side) / 2, side, side) if side else (mg, mg, fmt.w - 2 * mg, fmt.h - 2 * mg)
    corners(c, box, p.acc[1], fmt.short * (0.06 if side else 0.09), max(2, fmt.short * 0.006))
    x, y, w, h = fmt.zone
    if fmt.ratio >= 1.5:
        r = min(h / 2, w * 0.22)
        cx, cy = x + w / 2, y + h / 2
        c.arc(cx, cy, r, ms[0].src, 0, 0, 2, p.temp_ramp, track=p.acc[1] + "66", pre="halo")
        ring(c, cx, cy, r * 0.9, ms[0], 0, thick=r * 0.15, sub=True)
        sw = (w - 2 * r) / 2 - w * 0.05
        for side, picks in ((0, ms[1:3]), (1, ms[3:5] if len(ms) > 4 else ms[3:4] + ms[1:2])):
            bx = x if side == 0 else x + w - sw
            for i, box in enumerate(split((bx, y + h * 0.12, sw, h * 0.76), 2, h * 0.1, horizontal=False)):
                row(c, box, picks[i % len(picks)], i + side * 2)
        return c.finish()
    r = min(w / 2, h * 0.32)
    cx, cy = x + w / 2, y + r * 1.02
    c.arc(cx, cy, r, ms[0].src, 0, 0, 2, p.temp_ramp, track=p.acc[1] + "66", pre="halo")
    ring(c, cx, cy, r * 0.9, ms[0], 0, thick=r * 0.15, sub=True)
    for i, box in enumerate(split((x, y + h * 0.74, w, h * 0.26), 3, fmt.short * 0.03)):
        mini_col(c, box, ms[1 + i], i + 1)
    return c.finish()
