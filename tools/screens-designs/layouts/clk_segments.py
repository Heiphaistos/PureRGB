"""Afficheur a segments : chiffres fantomes « 88:88 » sous l'heure, police a chasse fixe."""
from content import CONTENT
from lib import Canvas
from parts import row
from layouts.clk_common import date_line

NAME, TITLE = "clk-segments", "Heure à segments"
CONTENTS = ["clk-systeme", "clk-temps", "clk-etat"]
MONO = "Roboto Mono"


def _time(c, x, y, w, h):
    p = c.p
    c.value("time", "%H:%M", x, y, w, h, "88:88", p.text, font=MONO, bold=True, align="center", pre="ghost")
    c.out[-1]["opacity"] = 0.1
    c.out.inks.pop()
    c.value("time", "%H:%M", x, y, w, h, "88:88", p.acc[0], font=MONO, bold=True, align="center", pre="time")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if fmt.wide:
        tw = w * (0.5 if fmt.strip else 0.58)
        c.card(x, y, tw, h, radius=h * 0.08)
        _time(c, x + tw * 0.05, y + h * 0.1, tw * 0.9, h * 0.6)
        date_line(c, (x + tw * 0.05, y + h * 0.76, tw * 0.9, h * 0.14), p.text2, short=not fmt.strip)
        rx = x + tw + g
        n = 4 if fmt.strip else 3
        bh = (h - g * (n - 1)) / n
        for i in range(n):
            row(c, (rx, y + i * (bh + g), x + w - rx, bh), ms[i], i)
        return c.finish()
    c.card(x, y, w, h * 0.5, radius=h * 0.05)
    _time(c, x + w * 0.04, y + h * 0.04, w * 0.92, h * 0.32)
    date_line(c, (x + w * 0.04, y + h * 0.37, w * 0.92, h * 0.09), p.text2, short=True)
    n = 3
    bh = (h * 0.46 - g * (n - 1)) / n
    for i in range(n):
        row(c, (x, y + h * 0.54 + i * (bh + g), w, bh), ms[i], i)
    return c.finish()
