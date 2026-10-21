"""Heures et minutes separees dans deux cartes, date et infos autour."""
from content import CONTENT
from lib import Canvas
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-duo", "Heures et minutes"
CONTENTS = ["clk-systeme", "clk-reseau", "clk-etat"]


def _pair(c, x, y, w, h):
    p, cw = c.p, w * 0.42
    dot = max(4.0, min(w * 0.04, h * 0.1))
    for k, (fmt_, bx) in enumerate((("%H", x), ("%M", x + w - cw))):
        c.card(bx, y, cw, h, radius=h * 0.1)
        c.value("time", fmt_, bx + cw * 0.06, y + h * 0.08, cw * 0.88, h * 0.84, "88", p.text if k == 0 else p.acc[0], bold=True, align="center", pre="time", group="hm")
    mx = x + w / 2 - dot / 2
    for fy in (0.36, 0.6):
        c.rect(mx, y + h * fy - dot / 2, dot, dot, p.acc[1], dot / 2, pre="dot")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.wide:
        tw = w * (0.5 if fmt.strip else 0.58)
        _pair(c, x, y, tw, h)
        rx = x + tw + g * 2
        date_line(c, (rx, y, x + w - rx, h * 0.2), c.p.text, short=not fmt.strip)
        cells(c, (rx, y + h * 0.26, x + w - rx, h * 0.74), ms[:4 if fmt.strip else 3], horizontal=fmt.strip, gap=g)
        return c.finish()
    _pair(c, x, y, w, h * 0.5)
    date_line(c, (x, y + h * 0.53, w, h * 0.1), p.text)
    cells(c, (x, y + h * 0.68, w, h * 0.32), ms[:3], gap=g, card=True)
    return c.finish()
