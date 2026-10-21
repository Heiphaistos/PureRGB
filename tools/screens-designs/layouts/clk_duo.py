"""Heures et minutes separees dans deux cartes, date complete et socle autour."""
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-duo", "Heure et calendrier"


def _pair(c, x, y, w, h):
    p, cw = c.p, w * 0.42
    dot = max(4.0, min(w * 0.04, h * 0.1))
    for k, (fmt_, bx) in enumerate((("%H", x), ("%M", x + w - cw))):
        c.card(bx, y, cw, h, radius=h * 0.1)
        c.value("time", fmt_, bx + cw * 0.06, y + h * 0.08, cw * 0.88, h * 0.84, "88", p.text if k == 0 else p.acc[0], bold=True, align="center", pre="time", group="hm")
    mx = x + w / 2 - dot / 2
    for fy in (0.36, 0.6):
        c.rect(mx, y + h * fy - dot / 2, dot, dot, p.acc[1], dot / 2, pre="dot")


def _draw(c, ms, mode):
    fmt = c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.wide:
        tw = w * (0.46 if fmt.strip else 0.5)
        _pair(c, x, y, tw, h * 0.62)
        date_block(c, (x, y + h * 0.68, tw, h * 0.32))
        rx = x + tw + g * 2
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)
        return
    _pair(c, x, y, w, h * 0.34)
    date_block(c, (x, y + h * 0.37, w, h * 0.12))
    panel(c, (x, y + h * 0.52, w, h * 0.48), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
