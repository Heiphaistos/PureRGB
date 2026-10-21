"""Neon : tubes lumineux (contour net + halo simule), valeurs colorees, enseigne de l'heure."""
from content import CONTENT
from lib import Canvas, grid, split
from layouts.sty_common import cols_for, halo, lv

NAME, TITLE = "sty-neon", "Néon"
CONTENTS = ["sty-pc", "sty-therm", "sty-air"]


def accepts(p):
    return p.dark


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.06
    hh = 0.0
    if not fmt.strip and fmt.short > 240 and not fmt.round:
        hh = h * 0.2
        c.value("time", "%H:%M", x, y, w * 0.5, hh, "88:88", p.acc[0], bold=True, pre="tm", maxsize=hh * 1.05)
        c.value("date", "%d/%m/%Y", x + w * 0.5, y + hh * 0.2, w * 0.5, hh * 0.5, "88/88/8888", p.acc[1], align="right", pre="dt", maxsize=hh * 0.5)
        c.rect(x, y + hh * 1.12, w, 3, p.acc[0], 1.5, pre="un")
        c.rect(x, y + hh * 1.12 - 5, w, 13, p.acc[0], 6, opacity=0.14, pre="unh")
        hh *= 1.3
    n = 2 if fmt.short <= 240 else cols_for(fmt, 4)
    e = fmt.short * 0.025
    zone = (x + e, y + hh + e, w - 2 * e, h - hh - 2 * e)
    for i, b in enumerate(split(zone, n, gap) if fmt.ratio >= 1.7 else grid(zone, n, gap, 1.5)):
        bx, by, bw, bh = b
        col = p.acc[i % 3]
        halo(c, b, col, min(bw, bh) * 0.22, max(2.0, fmt.short * 0.007))
        pd = min(bw, bh) * 0.12
        lv(c, (bx + pd, by + pd, bw - 2 * pd, bh - 2 * pd), ms[i], col, tcol=p.text, valign="center")
    return c.finish()
