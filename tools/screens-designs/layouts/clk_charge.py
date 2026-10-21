"""Compte-tours stylise : arc de charge gradue, heure au centre, date et infos."""
import math
from content import CONTENT
from lib import Canvas
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-charge", "Compte-tours et heure"
CONTENTS = ["clk-systeme", "clk-temps", "clk-etat"]


def _tach(c, cx, cy, r, m):
    p = c.p
    t = max(6.0, r * 0.14)
    c.arc(cx, cy, r, m.src, m.lo, m.hi, t, m.colors(p, 0))
    L = t * 0.9
    for k in range(11):
        a = 135 + 27 * k
        rr = r - t - L * 0.8
        ar = math.radians(a)
        c.rect(cx + rr * math.cos(ar) - L / 2, cy + rr * math.sin(ar) - 1.25, L, 2.5, p.text if k % 5 == 0 else p.text2, 1, rotation=a, pre="tick")
    inner = r - t - L * 1.6
    c.value("time", "%H:%M", cx - inner * 0.95, cy - inner * 0.62, inner * 1.9, inner * 0.7, "88:88", p.text, bold=True, align="center", pre="time")
    date_line(c, (cx - inner * 0.95, cy + inner * 0.1, inner * 1.9, inner * 0.27), p.text2, short=True)
    lh = r * 0.18
    c.text(cx - r * 0.45, cy + r * 0.52, r * 0.9, lh, m.label, p.text2, bold=True, align="center", pre="l", maxsize=lh)
    c.value(m.src, m.fmt, cx - r * 0.45, cy + r * 0.52 + lh, r * 0.9, r * 0.3, m.sample, p.acc[0], bold=True, align="center", pre="v")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.ratio >= 1.5:
        r = h / 2
        side = w - 2 * r - g * 2
        if side < fmt.short * 0.4:
            return None
        _tach(c, x + r, y + r, r, ms[0])
        cells(c, (x + 2 * r + g * 2, y, side, h), ms[1:4], horizontal=False, gap=g, card=True)
        return c.finish()
    r = min(w, h * 0.68) / 2
    _tach(c, x + w / 2, y + r, r, ms[0])
    by = y + 2 * r + g
    cells(c, (x, by, w, y + h - by), ms[1:3], gap=g, card=True)
    return c.finish()
