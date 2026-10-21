"""Calendrier-carte : bandeau du mois, numero du jour geant, nom du jour ; heure et infos a cote."""
from content import CONTENT
from lib import Canvas
from layouts.clk_common import cells

NAME, TITLE = "clk-calendrier", "Calendrier et heure"
CONTENTS = ["clk-systeme", "clk-temps", "clk-etat"]


def _calendar(c, x, y, w, h):
    p = c.p
    c.card(x, y, w, h, radius=min(w, h) * 0.08)
    bh = h * 0.22
    c.rect(x, y, w, bh, p.acc[0], min(w, h) * 0.08, pre="band")
    c.rect(x, y + bh * 0.5, w, bh * 0.5, p.acc[0], 0, pre="band")
    c.value("date", "%B", x + w * 0.06, y + bh * 0.1, w * 0.88, bh * 0.8, "septembre", p.bg0, bold=True, align="center", pre="mois")
    c.value("date", "%d", x + w * 0.06, y + bh + h * 0.02, w * 0.88, h * 0.5, "88", p.text, bold=True, align="center", pre="jour")
    c.value("date", "%A", x + w * 0.06, y + h * 0.82, w * 0.88, h * 0.13, "mercredi", p.text2, bold=True, align="center", pre="sem")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if fmt.wide:
        cw = min(h * 0.95, w * 0.3)
        _calendar(c, x, y, cw, h)
        tw = w * (0.3 if fmt.strip else 0.36)
        tx = x + cw + g * 2
        c.value("time", "%H:%M", tx, y + h * 0.1, tw, h * 0.6, "88:88", p.text, bold=True, align="center", pre="time")
        c.value("date", "%Y", tx, y + h * 0.75, tw, h * 0.15, "2026", p.text2, align="center", pre="an")
        rx = tx + tw + g * 2
        cells(c, (rx, y, x + w - rx, h), ms[:4 if fmt.strip else 3], horizontal=fmt.strip, gap=g, card=True)
        return c.finish()
    cw = w * 0.5
    _calendar(c, x, y, cw, h * 0.62)
    c.value("time", "%H:%M", x + cw + g, y + h * 0.08, w - cw - g, h * 0.3, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%Y", x + cw + g, y + h * 0.42, w - cw - g, h * 0.12, "2026", p.text2, align="center", pre="an")
    cells(c, (x, y + h * 0.7, w, h * 0.3), ms[:3], gap=g, card=True)
    return c.finish()
