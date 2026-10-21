"""Compte-tours stylise : arc de temperature CPU gradue, heure au centre, date et socle a cote."""
import math
from layouts.clk_common import CONTENTS, attempt, date_block, date_line, panel

NAME, TITLE = "clk-charge", "Compte-tours et heure"


def _tach(c, cx, cy, r, m, date):
    p = c.p
    t = max(6.0, r * 0.14)
    c.arc(cx, cy, r, m.src, m.lo, m.hi, t, m.colors(p, 0))
    L = t * 0.9
    rr = r - t - L * 0.8
    for k in range(11):
        a = 135 + 27 * k
        ar = math.radians(a)
        c.rect(cx + rr * math.cos(ar) - L / 2, cy + rr * math.sin(ar) - 1.25, L, 2.5, p.text if k % 5 == 0 else p.text2, 1, rotation=a, pre="tick")
    inner = r - t - L * 1.6
    if date:
        c.value("time", "%H:%M", cx - inner * 0.95, cy - inner * 0.6, inner * 1.9, inner * 0.75, "88:88", p.text, bold=True, align="center", pre="time")
        date_line(c, (cx - inner * 0.95, cy + inner * 0.2, inner * 1.9, inner * 0.3), p.text2, short=True)
    else:
        c.value("time", "%H:%M", cx - inner * 0.95, cy - inner * 0.4, inner * 1.9, inner * 0.8, "88:88", p.text, bold=True, align="center", pre="time")


def _draw(c, ms, mode):
    fmt = c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    if fmt.ratio >= 1.5:
        r = h / 2
        side = w - 2 * r - g * 2
        if side < fmt.short * 0.4:
            return False
        _tach(c, x + r, y + r, r, ms[0], True)
        panel(c, (x + 2 * r + g * 2, y, side, h), ms, mode, gap=g)
        return
    d = min(w * 0.5, h * 0.4)
    _tach(c, x + d / 2, y + d / 2, d / 2, ms[0], False)
    date_block(c, (x + d + g * 2, y, w - d - g * 2, d))
    panel(c, (x, y + d + g * 2, w, h - d - g * 2), ms, mode, gap=g)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
