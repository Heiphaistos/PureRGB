"""Cadran de montre sport : horloge analogique cerclee de quatre arcs (CPU, GPU, RAM, SSD), date et socle."""
import math
from layouts.clk_common import CONTENTS, attempt, date_block, panel

NAME, TITLE = "clk-sport", "Cadran sport"
CENTERS = (180, 0, 270, 90)   # O, E, N, S (degres, sens horaire depuis l'est)


def _dial(c, cx, cy, R, ms):
    p = c.p
    t = max(5.0, R * 0.09)
    for i, m in enumerate(ms[:4]):
        c.arc(cx, cy, R, m.src, m.lo, m.hi, t, m.colors(p, i), start=CENTERS[i] - 38, sweep=76)
    d = 2 * (R - t * 1.8) * 0.96
    c.clock(cx - d / 2, cy - d / 2, d, d, p.card if p.dark else "#ffffff", p.text, p.acc[0])
    for k in range(12):   # index des heures
        a = math.radians(k * 30)
        r = R - t * 1.0
        c.rect(cx + r * math.cos(a) - t * 0.35, cy + r * math.sin(a) - t * 0.35, t * 0.7, t * 0.7, p.text2, t * 0.35, pre="tk")


def _draw(c, ms, mode):
    fmt = c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.ratio >= 1.5:
        R = h / 2
        cx = x + w / 2
        side = cx - R - g * 2 - x
        if side < fmt.short * 0.4:
            return False
        _dial(c, cx, y + R, R, ms)
        date_block(c, (x, y, side, h * 0.26))
        panel(c, (x, y + h * 0.3, side, h * 0.7), ms[0::2], mode, gap=g)
        panel(c, (cx + R + g * 2, y, side, h), ms[1::2], mode, gap=g)
        return
    d = min(w * 0.5, h * 0.4)
    _dial(c, x + d / 2, y + d / 2, d / 2, ms)
    date_block(c, (x + d + g * 2, y, w - d - g * 2, d))
    panel(c, (x, y + d + g * 2, w, h - d - g * 2), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
