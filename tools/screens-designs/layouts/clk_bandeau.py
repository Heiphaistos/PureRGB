"""Bandeau ultra-large : heure et date a gauche, socle CPU/GPU/RAM/SSD + extras a droite."""
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-bandeau", "Heure et calendrier"
FORMATS = ["w169", "w1280", "u1920", "rb640"]


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    hw = w * (0.34 if fmt.short < 200 or not fmt.strip else 0.3 if fmt.ratio < 3 else 0.27)
    c.value("time", "%H:%M", x, y, hw, h * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
    date_block(c, (x, y + h * 0.56, hw, h * 0.42))
    c.rect(x + hw + g * 0.5, y + h * 0.06, 2, h * 0.88, p.line, 1, pre="sep")
    rx = x + hw + g * 2
    panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
