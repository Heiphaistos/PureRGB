"""Cadran de montre sport : horloge analogique cerclee de quatre arcs de capteurs."""
import math
from content import CONTENT
from lib import Canvas
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-sport", "Cadran sport"
CONTENTS = ["clk-systeme", "clk-temps", "clk-etat"]
CENTERS = (180, 0, 270, 90)   # O, E, N, S (degres, sens horaire depuis l'est)


def _dial(c, cx, cy, R, ms):
    p = c.p
    t = max(5.0, R * 0.09)
    for i, m in enumerate(ms):
        c.arc(cx, cy, R, m.src, m.lo, m.hi, t, m.colors(p, i), start=CENTERS[i] - 38, sweep=76)
    d = 2 * (R - t * 1.8) * 0.96
    c.clock(cx - d / 2, cy - d / 2, d, d, p.card if p.dark else "#ffffff", p.text, p.acc[0])
    for k in range(12):   # index des heures
        a = math.radians(k * 30)
        r = R - t * 1.0
        c.rect(cx + r * math.cos(a) - t * 0.35, cy + r * math.sin(a) - t * 0.35, t * 0.7, t * 0.7, p.text2, t * 0.35, pre="tk")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:4]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.ratio >= 1.5:
        R = h / 2
        cx = x + w / 2
        side = cx - R - g * 2 - x
        if side < fmt.short * 0.25:
            return None
        _dial(c, cx, y + R, R, ms)
        lx, rx = x, cx + R + g * 2
        date_line(c, (lx, y, side, h * 0.3), p.text, align="center", short=True)
        cells(c, (lx, y + h * 0.4, side, h * 0.6), [ms[0], ms[2]], horizontal=False, gap=g, card=True)
        cells(c, (rx, y, side, h), [ms[1], ms[3]], horizontal=False, gap=g, card=True)
        return c.finish()
    R = min(w, h * 0.58) / 2
    ms = ms[:2]   # carre : deux arcs (O, E) et deux cellules, sinon libelles illisibles
    _dial(c, x + w / 2, y + R, R, ms)
    by = y + 2 * R + g
    date_line(c, (x, by, w, (y + h - by) * 0.34), p.text, short=True)
    cells(c, (x, by + (y + h - by) * 0.34, w, (y + h - by) * 0.66), ms, gap=g, bar=False)
    return c.finish()
