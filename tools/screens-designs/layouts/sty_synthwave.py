"""Synthwave / retro 80s : soleil raye, horizon, sol en perspective, cartes sombres en pied."""
from lib import Canvas, grid
from parts import glow
from layouts.sty_common import ALL8, body, fit, persp

NAME, TITLE = "sty-synthwave", "Synthwave rétro"
CONTENTS = ALL8


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


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.025
    th = max(h * (0.12 if fmt.ratio < 1.5 else 0.15), 22.0)
    R = min(h * (0.13 if fmt.ratio < 1.5 else 0.17 if fmt.strip else 0.2), w * 0.18)
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
        c.value("date", "%A %d %B", x + w * 0.5, y, w * 0.5, th * 0.7, "mercredi 30 septembre", p.acc[1], align="right", pre="dt")
    else:
        c.value("time", "%H:%M", x, y, w * 0.5, th, "88:88", p.text, bold=True, pre="tm", maxsize=th * 0.95)
        c.value("date", "%d/%m", x + w * 0.6, y, w * 0.4, th * 0.6, "88/88", p.acc[1], bold=True, align="right", pre="dt")
    top = hy + gap
    for i, (bx, by, bw, bh) in enumerate(grid((x, top, w, y + h - top), len(ms), gap, 2.0)):
        col = p.acc[i % 2]
        with c.m(ms[i]):
            c.rect(bx, by, bw, bh, "#000000", min(bh * 0.14, 14), stroke=col, sw=1.5, opacity=0.82, pre="cd")
        pd = min(bw, bh) * 0.1
        body(c, (bx + pd, by + pd * 0.6, bw - 2 * pd, bh - 1.2 * pd), ms[i], i, col, g="s")
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
