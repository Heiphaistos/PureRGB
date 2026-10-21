"""Heure numerique geante, date dessous, infos avec barres."""
from content import CONTENT
from lib import Canvas
from parts import glow
from layouts.clk_common import cell, cells, date_line

NAME, TITLE = "clk-geant", "Heure géante"
CONTENTS = ["clk-systeme", "clk-temps", "clk-etat"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if p.dark:
        glow(c, x + w * 0.3, y + h * 0.4, min(w, h) * 0.4, p.acc[0])
    if fmt.wide:
        tw = w * (0.46 if fmt.strip else 0.56)
        c.value("time", "%H:%M", x, y + h * 0.04, tw, h * 0.66, "88:88", p.text, bold=True, align="center", pre="time")
        c.rect(x + tw * 0.3, y + h * 0.72, tw * 0.4, max(3, h * 0.015), p.acc[0], 2, pre="rule")
        date_line(c, (x, y + h * 0.78, tw, h * 0.18))
        k = 4 if fmt.strip else 3
        rx = x + tw + g * 2
        cells(c, (rx, y, x + w - rx, h), ms[:k], horizontal=fmt.strip, gap=g, card=True)
        return c.finish()
    c.value("time", "%H:%M", x, y, w, h * 0.44, "88:88", p.text, bold=True, align="center", pre="time")
    c.rect(x + w * 0.36, y + h * 0.46, w * 0.28, max(3, h * 0.012), p.acc[0], 2, pre="rule")
    date_line(c, (x, y + h * 0.49, w, h * 0.1))
    cells(c, (x, y + h * 0.66, w, h * 0.34), ms[:3], gap=g, card=True)
    return c.finish()
