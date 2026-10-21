"""Calendrier-carte : mois, numero du jour geant, jour de la semaine, annee ; heure et socle a cote."""
from layouts.clk_common import CONTENTS, attempt, panel

NAME, TITLE = "clk-calendrier", "Heure et calendrier"


def _calendar(c, x, y, w, h):
    p = c.p
    r = min(w, h) * 0.08
    c.card(x, y, w, h, radius=r)
    bh = h * 0.22
    c.rect(x, y, w, bh, p.acc[0], r, pre="band")
    c.rect(x, y + bh * 0.5, w, bh * 0.5, p.acc[0], 0, pre="band")
    c.value("date", "%B", x + w * 0.06, y + bh * 0.08, w * 0.88, bh * 0.84, "septembre", p.bg0, bold=True, align="center", pre="mois")
    c.value("date", "%d", x + w * 0.06, y + bh + h * 0.01, w * 0.88, h * 0.46, "88", p.text, bold=True, align="center", pre="jour")
    c.value("date", "%A", x + w * 0.04, y + h * 0.7, w * 0.56, h * 0.13, "mercredi", p.text2, bold=True, align="right", pre="sem", group="cw")
    c.value("date", "%Y", x + w * 0.64, y + h * 0.7, w * 0.32, h * 0.13, "2026", p.acc[1], bold=True, align="left", pre="an", group="cw")


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.04
    if fmt.wide:
        cw = min(h * 0.95, w * 0.3)
        _calendar(c, x, y + (h - cw) / 2, cw, cw)
        tw = w * (0.3 if fmt.strip else 0.32)
        tx = x + cw + g * 2
        c.value("time", "%H:%M", tx, y + h * 0.2, tw, h * 0.6, "88:88", p.text, bold=True, align="center", pre="time")
        rx = tx + tw + g * 2
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g)
        return
    cw, ch = w * 0.46, h * 0.46
    _calendar(c, x, y, cw, ch)
    c.value("time", "%H:%M", x + cw + g, y + ch * 0.2, w - cw - g, ch * 0.6, "88:88", p.text, bold=True, align="center", pre="time")
    panel(c, (x, y + ch + g * 1.5, w, h - ch - g * 1.5), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
