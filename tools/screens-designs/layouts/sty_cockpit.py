"""Tableau de bord auto / avion : deux gros cadrans a graduations, voyants et bandeau central."""
import math

from lib import Canvas, grid
from layouts.sty_common import ALL8, body, fit

NAME, TITLE = "sty-cockpit", "Tableau de bord"
CONTENTS = ALL8
FORMATS = ["s480", "s320", "rond", "w169", "w1280", "u1920", "rb640"]


def _dial(c, cx, cy, R, m, i):
    p = c.p
    for k in range(37):   # graduations sur 270 deg
        a = math.radians(135 + k * 270 / 36)
        big = k % 6 == 0
        ln = R * (0.12 if big else 0.06)
        rt = R * 1.04 - ln / 2
        c.rect(cx + rt * math.cos(a) - ln / 2, cy + rt * math.sin(a) - R * 0.01, ln, R * 0.02, p.text if big else p.text2, 0,
               rotation=math.degrees(a), opacity=0.9 if big else 0.55, pre="tk")
    c.arc(cx, cy, R * 0.88, m.src, m.lo, m.hi, R * 0.1, m.colors(p, i))
    vw = R * 1.2
    c.value(m.src, m.fmt, cx - vw / 2, cy - R * 0.62, vw, R * 0.55, m.sample, p.text, bold=True, align="center", pre="dv", group="dv", font=p.font)
    c.value(m.sub[0], m.sub[1], cx - vw / 2, cy - R * 0.04, vw, R * 0.3, m.sub[2], p.text2, align="center", pre="ds", group="ds", font=p.font)
    c.text(cx - R * 0.5, cy + R * 0.42, R, R * 0.26, m.label, p.text2, bold=True, align="center", pre="dl", group="dl", font=p.font)


def _light(c, box, m, i):
    p = c.p
    x, y, w, h = box
    d = min(h * 0.24, 22.0)
    lx = x + h * 0.14 + d * 1.35
    with c.m(m):
        c.rect(x, y, w, h, p.card, min(h * 0.2, 12), stroke=p.line, sw=1, pre="vy")
        c.rect(x + h * 0.14, y + (h - d) / 2, d, d, p.acc[i % 3], d / 2, pre="lamp")
        c.rect(x + h * 0.14 - d * 0.35, y + (h - d) / 2 - d * 0.35, d * 1.7, d * 1.7, p.acc[i % 3], d * 0.85, opacity=0.18, pre="lh")
    body(c, (lx, y + h * 0.08, x + w - lx - h * 0.1, h * 0.84), m, i, p.text2, g="v")


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    k = len(ms) - 2
    if fmt.ratio >= 1.5:
        R = min(h * 0.46, w * 0.14)
        cy = y + h / 2
        _dial(c, x + R * 1.1, cy, R, ms[0], 0)
        _dial(c, x + w - R * 1.1, cy, R, ms[1], 1)
        mx, mw = x + R * 2.4, w - R * 4.8
        if mw < 60:
            return None
        hh = h * 0.16 if not fmt.strip else 0
        if hh:
            c.value("time", "%H:%M", mx, y, mw * 0.5, hh * 1.4, "88:88", p.text, bold=True, align="left", pre="tm")
            c.value("date", "%d/%m/%Y", mx + mw * 0.5, y, mw * 0.5, hh * 1.4, "88/88/8888", p.text2, align="right", pre="dt")
        cells = grid((mx, y + hh * 1.5, mw, h - hh * 1.5), k, gap, 2.4)
    elif fmt.round:
        return None
    else:
        hh = h * 0.12 if fmt.short >= 400 else 0
        R = min(w * 0.22, h * (0.22 if hh else 0.27))
        if hh:
            c.value("time", "%H:%M", x, y, w * 0.4, hh, "88:88", p.text, bold=True, align="left", pre="tm")
            c.value("date", "%d/%m/%Y", x + w * 0.4, y, w * 0.6, hh, "88/88/8888", p.text2, align="right", pre="dt")
        cy = y + hh + R * 1.1
        _dial(c, x + w * 0.26 + R * 0.04, cy, R, ms[0], 0)
        _dial(c, x + w * 0.74 - R * 0.04, cy, R, ms[1], 1)
        top = cy + R * 1.2
        cells = grid((x, top, w, y + h - top), k, gap, 2.6)
    for i, b in enumerate(cells):
        _light(c, b, ms[2 + i], i)
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
