"""Minimal : heure fine alignee a gauche, filet, date complete et socle en texte seul."""
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-minimal", "Heure et calendrier"


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if mode == "cards":
        return False
    if fmt.wide:
        tw = w * 0.5
        c.value("time", "%H:%M", x, y, tw, h * 0.55, "88:88", p.text, align="left", pre="time")
        date_block(c, (x, y + h * 0.62, tw, h * 0.34), align="left")
        c.rect(x + tw + g, y + h * 0.06, 2, h * 0.88, p.acc[0], 1, pre="sep")
        rx = x + tw + g * 3
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g, bar=False, card=False)
        return
    c.value("time", "%H:%M", x, y, w, h * 0.26, "88:88", p.text, align="left", pre="time")
    date_block(c, (x, y + h * 0.28, w, h * 0.12), align="left")
    c.rect(x, y + h * 0.43, w, 2, p.line, 1, pre="rule")
    panel(c, (x, y + h * 0.47, w, h * 0.53), ms, mode, gap=g, bar=False, card=False)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
