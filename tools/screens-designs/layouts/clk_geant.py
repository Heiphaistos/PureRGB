"""Heure numerique geante, date complete dessous, socle et extras en cartes."""
from parts import glow
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-geant", "Heure et calendrier"


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if p.dark:
        glow(c, x + w * 0.3, y + h * 0.3, min(w, h) * 0.4, p.acc[0])
    if fmt.wide:
        tw = w * (0.44 if fmt.strip else 0.5)
        c.value("time", "%H:%M", x, y + h * 0.02, tw, h * 0.58, "88:88", p.text, bold=True, align="center", pre="time")
        c.rect(x + tw * 0.3, y + h * 0.63, tw * 0.4, max(3, h * 0.015), p.acc[0], 2, pre="rule")
        date_block(c, (x, y + h * 0.68, tw, h * 0.3))
        rx = x + tw + g * 2
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)
        return
    c.value("time", "%H:%M", x, y, w, h * 0.32, "88:88", p.text, bold=True, align="center", pre="time")
    c.rect(x + w * 0.36, y + h * 0.34, w * 0.28, max(3, h * 0.012), p.acc[0], 2, pre="rule")
    date_block(c, (x, y + h * 0.37, w, h * 0.12))
    panel(c, (x, y + h * 0.52, w, h * 0.48), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
