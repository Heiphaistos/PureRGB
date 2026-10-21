"""Bandeau ultra-large : heure et date a gauche, quatre capteurs en cartes a droite."""
from content import CONTENT
from lib import Canvas
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-bandeau", "Bandeau heure et capteurs"
CONTENTS = ["clk-systeme", "clk-temps", "clk-reseau", "clk-etat"]
FORMATS = ["w1280", "u1920", "rb640"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:4]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    tw = w * 0.38
    c.value("time", "%H:%M", x, y, tw, h * 0.68, "88:88", p.text, bold=True, align="left", pre="time")
    date_line(c, (x, y + h * 0.72, tw, h * 0.26), p.acc[0], align="left", short=True)
    c.rect(x + tw + g * 0.5, y + h * 0.08, 2, h * 0.84, p.line, 1, pre="sep")
    rx = x + tw + g * 2
    cells(c, (rx, y, x + w - rx, h), ms, gap=g, card=True)
    return c.finish()
