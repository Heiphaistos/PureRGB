"""Tachymetre : jauge a graduations (reglettes en rotation) et statistiques annexes."""
import math

from content import CONTENT
from lib import Canvas, split
from parts import hero_list, mini_col, ring, row

NAME, TITLE = "tachymetre", "Tachymètre"
CONTENTS = ["cpu", "gpu", "liquide", "quad"]


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
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    if fmt.ratio >= 1.5:
        R = min(h / 2, w * 0.3)
        rx, lx, lw = hero_list(fmt.zone, 2 * R, gap)
        cx, cy = rx + R, y + h / 2
        _ticks(c, cx, cy, R)
        ring(c, cx, cy, R * 0.83, ms[0], 0, thick=R * 0.12, sub=True)
        for i, box in enumerate(split((lx, y + h * 0.08, lw, h * 0.84), 3, gap, horizontal=False)):
            row(c, box, ms[1 + i], i + 1)
        return c.finish()
    R = min(w / 2, h * 0.36)
    cx, cy = x + w / 2, y + R
    _ticks(c, cx, cy, R)
    ring(c, cx, cy, R * 0.83, ms[0], 0, thick=R * 0.12, sub=True)
    for i, box in enumerate(split((x, y + 2 * R + gap, w, y + h - (y + 2 * R + gap)), 3, gap)):
        mini_col(c, box, ms[1 + i], i + 1)
    return c.finish()
