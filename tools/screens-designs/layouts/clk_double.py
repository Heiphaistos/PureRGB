"""Double affichage : horloge analogique + heure numerique + date complete + socle."""
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-double", "Analogique et numérique"


def _digital(c, x, y, w, h):
    p = c.p
    c.value("time", "%H:%M", x, y, w, h * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
    date_block(c, (x, y + h * 0.56, w, h * 0.42))


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    face = p.card if p.dark else "#ffffff"
    if fmt.wide:
        d = h
        c.clock(x, y, d, d, face, p.text, p.acc[0])
        mw = w * 0.3
        _digital(c, x + d + g, y, mw, h)
        rx = x + d + mw + g * 3
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)
        return
    d = min(h * 0.4, w * 0.45)
    c.clock(x, y, d, d, face, p.text, p.acc[0])
    _digital(c, x + d + g, y, w - d - g, d)
    panel(c, (x, y + d + g * 2, w, h - d - g * 2), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
