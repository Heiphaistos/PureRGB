"""Minimal : heure fine alignee a gauche, filet, date et infos en texte seul."""
from content import CONTENT
from lib import Canvas
from parts import row
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-minimal", "Heure minimale"
CONTENTS = ["clk-systeme", "clk-temps", "clk-reseau"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if fmt.wide:
        tw = w * 0.56
        c.value("time", "%H:%M", x, y, tw, h * 0.7, "88:88", p.text, align="left", pre="time")
        date_line(c, (x, y + h * 0.76, tw, h * 0.2), p.acc[0], align="left", short=not fmt.strip)
        c.rect(x + tw + g, y + h * 0.1, 2, h * 0.8, p.acc[0], 1, pre="sep")
        rx = x + tw + g * 3
        n = 4 if fmt.strip else 3
        bh = h / n
        for i in range(n):
            row(c, (rx, y + i * bh, x + w - rx, bh * 0.9), ms[i], i, bar=False)
        return c.finish()
    c.value("time", "%H:%M", x, y, w, h * 0.5, "88:88", p.text, align="left", pre="time")
    date_line(c, (x, y + h * 0.52, w, h * 0.1), p.acc[0], align="left", short=True)
    c.rect(x, y + h * 0.66, w, 2, p.line, 1, pre="rule")
    cells(c, (x, y + h * 0.72, w, h * 0.28), ms[:3], gap=g, bar=False)
    return c.finish()
