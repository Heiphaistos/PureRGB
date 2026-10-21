"""Double affichage : horloge analogique + heure numerique + date."""
from content import CONTENT
from lib import Canvas, split
from parts import row
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-double", "Analogique et numérique"
CONTENTS = ["clk-systeme", "clk-reseau", "clk-etat"]


def _digital(c, x, y, w, h):
    p = c.p
    c.value("time", "%H:%M", x, y, w, h * 0.62, "88:88", p.text, bold=True, align="center", pre="time")
    date_line(c, (x, y + h * 0.68, w, h * 0.28), p.acc[0], short=True)


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    face = p.card if p.dark else "#ffffff"
    if fmt.wide:
        d = h
        c.clock(x, y, d, d, face, p.text, p.acc[0])
        mw = w * 0.34
        _digital(c, x + d + g, y, mw, h)
        rx = x + d + mw + g * 3
        n = 4 if fmt.strip else 3
        for i, b in enumerate(split((rx, y, x + w - rx, h), n, g, horizontal=False)):
            row(c, b, ms[i], i)
        return c.finish()
    d = h * 0.52
    c.clock(x, y, d, d, face, p.text, p.acc[0])
    _digital(c, x + d + g, y, w - d - g, d)
    cells(c, (x, y + d + g * 2, w, h - d - g * 2), ms[:3], gap=g, card=True)
    return c.finish()
