"""Tableau de bord auto / avion : deux gros cadrans a graduations, voyants et bandeau central."""
import math

from content import CONTENT
from lib import Canvas, grid, split
from parts import ring
from layouts.sty_common import cols_for, need

NAME, TITLE = "sty-cockpit", "Tableau de bord"
CONTENTS = ["sty-pc", "sty-therm", "sty-air"]
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
    ring(c, cx, cy, R * 0.88, m, i, thick=R * 0.1, label=False)
    lh = need(c.f)
    c.text(cx - R * 0.5, cy + R * 0.5, R, lh, m.label, p.text2, bold=True, align="center", pre="dl", maxsize=lh, group="dl")


def _light(c, box, m, i):
    p = c.p
    x, y, w, h = box
    c.rect(x, y, w, h, p.card, min(h * 0.2, 12), stroke=p.line, sw=1, pre="vy")
    d = min(h * 0.3, 26.0)
    c.rect(x + h * 0.2, y + (h - d) / 2, d, d, p.acc[i % 3], d / 2, pre="lamp")
    c.rect(x + h * 0.2 - d * 0.35, y + (h - d) / 2 - d * 0.35, d * 1.7, d * 1.7, p.acc[i % 3], d * 0.85, opacity=0.18, pre="lh")
    lx = x + h * 0.2 + d * 1.5
    if w > 2.4 * h:
        c.text(lx, y, w * 0.36, h, m.label, p.text2, bold=True, pre="l", group="vl", maxsize=h * 0.5)
        c.value(m.src, m.fmt, x + w * 0.5, y, w * 0.47, h, m.sample, p.text, bold=True, align="right", pre="v", group="vv", maxsize=h * 0.6)
    else:
        c.text(lx, y + h * 0.04, x + w - lx - 4, h * 0.38, m.label, p.text2, bold=True, pre="l", group="vl", maxsize=h * 0.3)
        c.value(m.src, m.fmt, lx, y + h * 0.42, x + w - lx - 4, h * 0.54, m.sample, p.text, bold=True, pre="v", group="vv", maxsize=h * 0.5)


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    if fmt.ratio >= 1.5:
        R = min(h * 0.46, w * 0.14)
        cy = y + h / 2
        _dial(c, x + R * 1.1, cy, R, ms[0], 0)
        _dial(c, x + w - R * 1.1, cy, R, ms[1], 1)
        mx, mw = x + R * 2.4, w - R * 4.8
        if mw < 60:
            return None
        hh = h * 0.22 if not fmt.strip else 0
        if hh:
            c.value("time", "%H:%M", mx, y, mw, hh * 1.4, "88:88", p.text, bold=True, align="center", pre="tm")
        cells = grid((mx, y + hh * 1.5, mw, h - hh * 1.5), 2 if fmt.strip else 4, gap, 1.2 if fmt.strip else 3.0)
        for i, b in enumerate(cells):
            _light(c, b, ms[2 + i], i)
        return c.finish()
    if fmt.round:
        R = w * 0.31
        _dial(c, x + w / 2, y + R * 1.05, R, ms[0], 0)
        top = y + R * 2.2
        for i, b in enumerate(split((x, top, w, y + h - top), 2, gap, horizontal=False)):
            _light(c, b, ms[1 + i], i)
        return c.finish()
    R = min(w * 0.22, h * 0.25)
    if fmt.short > 240:
        c.value("time", "%H:%M", x + w * 0.3, y, w * 0.4, h * 0.14, "88:88", p.text, bold=True, align="center", pre="tm", maxsize=h * 0.13)
    cy = y + h * 0.13 + R * 1.1
    _dial(c, x + w * 0.26 + R * 0.04, cy, R, ms[0], 0)
    _dial(c, x + w * 0.74 - R * 0.04, cy, R, ms[1], 1)
    top = cy + R * 1.2
    bh_ = y + h - top
    if fmt.short >= 400:
        cells = grid((x, top, w, bh_), 4, gap, 3.0)
    else:
        cells = split((x, top, w, bh_), 2, gap, horizontal=False)
    for i, b in enumerate(cells):
        _light(c, b, ms[2 + i], i)
    return c.finish()
