"""Afficheur a segments : chiffres fantomes « 88:88 » sous l'heure, police a chasse fixe, date et socle."""
from layouts.clk_common import CONTENTS, attempt, date_line, panel

NAME, TITLE = "clk-segments", "Heure à segments"
MONO = "Roboto Mono"


def _time(c, x, y, w, h):
    p = c.p
    c.value("time", "%H:%M", x, y, w, h, "88:88", p.text, font=MONO, bold=True, align="center", pre="ghost")
    c.out[-1]["opacity"] = 0.1
    c.out.inks.pop()
    c.value("time", "%H:%M", x, y, w, h, "88:88", p.acc[0], font=MONO, bold=True, align="center", pre="time")


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if fmt.wide:
        tw = w * (0.46 if fmt.strip else 0.5)
        c.card(x, y, tw, h, radius=h * 0.08)
        _time(c, x + tw * 0.05, y + h * 0.1, tw * 0.9, h * 0.55)
        date_line(c, (x + tw * 0.05, y + h * 0.74, tw * 0.9, h * 0.14), p.text2, short=tw < 600)
        rx = x + tw + g
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)
        return
    c.card(x, y, w, h * 0.34, radius=h * 0.05)
    _time(c, x + w * 0.04, y + h * 0.02, w * 0.92, h * 0.22)
    date_line(c, (x + w * 0.04, y + h * 0.25, w * 0.92, h * 0.07), p.text2, short=True)
    panel(c, (x, y + h * 0.38, w, h * 0.62), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
