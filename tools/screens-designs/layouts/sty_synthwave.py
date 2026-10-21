"""Synthwave / retro 80s : soleil raye, horizon, sol en perspective, cartes sombres en pied."""
from content import CONTENT
from lib import Canvas, grid, split
from parts import glow
from layouts.sty_common import cols_for, lv, persp

NAME, TITLE = "sty-synthwave", "Synthwave rétro"
CONTENTS = ["sty-pc", "sty-therm", "sty-io"]


def accepts(p):
    return p.dark


def _sun(c, cx, cy, R):
    p = c.p
    c.rect(cx - R, cy - R, 2 * R, 2 * R, p.acc[2], R, opacity=0.95, pre="sun")
    for k in range(5):   # stries : de plus en plus epaisses vers le bas
        t = R * (0.05 + 0.045 * k)
        yy = cy + R * (0.05 + 0.19 * k)
        half = (R * R - (yy + t / 2 - cy) ** 2) ** 0.5
        c.rect(cx - half, yy, 2 * half, t, p.bg1, 0, pre="st")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.025
    th = max(h * 0.15, 22.0)
    R = min(h * 0.2, w * 0.18)
    hy = y + th + 2 * R + gap
    cx = x + w / 2
    persp(c, (x, hy, w, y + h - hy), hy, p.acc[0])
    c.rect(x, hy - 1.5, w, 3, p.acc[1], 0, pre="hor")
    glow(c, cx, hy - gap - R, R * 1.9, p.acc[0], 5)
    _sun(c, cx, hy - gap - R, R)
    if fmt.strip:
        c.value("time", "%H:%M", x, y, w * 0.3, th * 1.4, "88:88", p.text, bold=True, pre="tm", maxsize=th * 1.2)
        c.value("date", "%d/%m", x + w * 0.7, y, w * 0.3, th * 1.0, "88/88", p.acc[1], bold=True, align="right", pre="dt")
    elif fmt.wide:
        c.value("time", "%H:%M", x, y, w * 0.3, th * 1.4, "88:88", p.text, bold=True, pre="tm", maxsize=th * 1.2)
        c.value("date", "%A %d %B", x + w * 0.7, y, w * 0.3, th * 0.7, "mercredi 30 septembre", p.acc[1], align="right", pre="dt")
    else:
        c.value("time", "%H:%M", x, y, w * 0.5, th, "88:88", p.text, bold=True, pre="tm", maxsize=th * 0.95)
        c.value("date", "%d/%m", x + w * 0.6, y, w * 0.4, th * 0.6, "88/88", p.acc[1], bold=True, align="right", pre="dt")
    n = cols_for(fmt, 4)
    top = y + h * (0.6 if fmt.ratio < 1.5 else 0.68)
    boxes = split((x, top, w, y + h - top), n, gap) if fmt.ratio >= 1.5 else grid((x, top, w, y + h - top), n, gap, 2.6)
    for i, (bx, by, bw, bh) in enumerate(boxes):
        col = p.acc[i % 2]
        c.rect(bx, by, bw, bh, "#000000", min(bh * 0.14, 14), stroke=col, sw=1.5, opacity=0.5, pre="cd")
        pd = min(bw, bh) * 0.1
        lv(c, (bx + pd, by + pd * 0.6, bw - 2 * pd, bh - 1.2 * pd), ms[i], col, valign="center")
    return c.finish()
