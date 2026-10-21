"""Tachymetre : jauge CPU a graduations (temperature + charge) et statistiques annexes (BASE + extras)."""
import math

from content import CONTENT
from lib import Canvas
from parts import hero_list
from layouts.r2_common import SETS_C, counts, first_ok, rows, bottom, BOTTOMS, gauge

NAME, TITLE = "tachymetre", "Tachymètre"
CONTENTS = SETS_C


def _ticks(c, cx, cy, R, n=28):
    p = c.p
    for k in range(n):
        a = 135 + k * 270 / (n - 1)
        major = k % 3 == 0
        ln = R * (0.1 if major else 0.06)
        rt = R * 0.97 - ln / 2
        rad = math.radians(a)
        c.rect(cx + rt * math.cos(rad) - ln / 2, cy + rt * math.sin(rad) - R * 0.011, ln, R * 0.022,
               p.text if major else p.text2, 0, rotation=a, opacity=0.9 if major else 0.55, pre="tk")


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04

    def wide_mk(n, cols, rfrac, sb, sn):
        def f():
            c = Canvas(p, fmt)
            R = min(h / 2, w * rfrac)
            rx, lx, lw = hero_list(fmt.zone, 2 * R, gap)
            cx, cy = rx + R, y + h / 2
            _ticks(c, cx, cy, R)
            gauge(c, cx, cy, R * 0.83, ms[0], 0, sub=True, thick=R * 0.12)
            rows(c, (lx, y + h * 0.04, lw, h * 0.92), [(ms[i], i) for i in range(1, n)], cols, gap * 0.6, subw=sb, subn=sn)
            return c.finish()
        return f

    def tall_mk(n, rfrac, kind):
        def f():
            c = Canvas(p, fmt)
            R = min(w / 2, h * rfrac)
            cx, cy = x + w / 2, y + R
            _ticks(c, cx, cy, R)
            gauge(c, cx, cy, R * 0.83, ms[0], 0, sub=True, thick=R * 0.12)
            top = y + 2 * R + gap
            bottom(c, (x, top, w, y + h - top), [(ms[i], i) for i in range(1, n)], gap * 0.6, kind)
            return c.finish()
        return f

    ns = counts(ms, (9, 8, 6, 5))
    if fmt.ratio >= 1.5:
        return first_ok(fmt, [wide_mk(n, cols, rf, sb, sn) for n in ns for rf in (0.3, 0.24) for cols in (1, 2) for sb in (0.25, 0.34) for sn in (99, 2)])
    return first_ok(fmt, [tall_mk(n, rf, k) for n in ns for rf in (0.38, 0.34, 0.28, 0.22) for k in BOTTOMS])
