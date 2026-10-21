"""Neon : tubes lumineux (contour net + halo simule), valeurs colorees, enseigne de l'heure."""
from lib import Canvas, grid
from layouts.sty_common import ALL8, body, fit, halo

NAME, TITLE = "sty-neon", "Néon"
CONTENTS = ALL8


def accepts(p):
    return p.dark


def _draw(p, fmt, ms):
    if fmt.round:
        return None
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.045
    hh = 0.0
    if not fmt.strip and fmt.short > 240 and not fmt.round:
        hh = h * 0.16
        c.value("time", "%H:%M", x, y, w * 0.5, hh, "88:88", p.acc[0], bold=True, pre="tm", maxsize=hh * 1.05)
        c.value("date", "%d/%m/%Y", x + w * 0.5, y + hh * 0.2, w * 0.5, hh * 0.5, "88/88/8888", p.acc[1], align="right", pre="dt", maxsize=hh * 0.5)
        c.rect(x, y + hh * 1.12, w, 3, p.acc[0], 1.5, pre="un")
        c.rect(x, y + hh * 1.12 - 5, w, 13, p.acc[0], 6, opacity=0.14, pre="unh")
        hh *= 1.3
    e = fmt.short * 0.025
    zone = (x + e, y + hh + e, w - 2 * e, h - hh - 2 * e)
    for i, b in enumerate(grid(zone, len(ms), gap, 1.5)):
        bx, by, bw, bh = b
        col = p.acc[i % 3]
        with c.m(ms[i]):
            halo(c, b, col, min(bw, bh) * 0.22, max(2.0, fmt.short * 0.007))
        pd = min(bw, bh) * 0.12
        body(c, (bx + pd, by + pd * 0.8, bw - 2 * pd, bh - 1.6 * pd), ms[i], i, col, align="center", g="n")
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
