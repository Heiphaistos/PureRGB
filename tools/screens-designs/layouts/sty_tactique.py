"""Militaire tactique : bandes de danger, fiches numerotees a barre laterale, entete d'etat, regle graduee."""
from lib import Canvas, grid
from layouts.sty_common import ALL8, body, fit, hazard, need, ticks_line

NAME, TITLE = "sty-tactique", "Tactique militaire"
CONTENTS = ALL8


def accepts(p):
    return p.dark


def _draw(p, fmt, ms):
    if fmt.round:
        return None
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.035
    band = max(10.0, fmt.short * 0.04)
    if not fmt.round:
        hazard(c, (x, y - band * 0.2, w, band), p.acc[1], band * 0.45)
        hazard(c, (x, y + h - band * 0.8, w, band), p.acc[1], band * 0.45)
        y, h = y + band * 1.1, h - band * 2.0
    hh = 0.0
    if not fmt.strip and fmt.short >= 400:
        hh = need(fmt) * 1.1
        c.text(x, y, w * 0.6, hh, "ÉTAT SYSTÈME", p.acc[0], bold=True, pre="hd", maxsize=hh)
        c.value("time", "%H:%M:%S", x + w * 0.64, y, w * 0.36, hh, "88:88:88", p.text, bold=True, align="right", pre="tm", maxsize=hh)
        ticks_line(c, x, y + hh * 1.12, w, 40, hh * 0.3, p.text2, 0.7)
        hh *= 1.55
    zone = (x, y + hh, w, h - hh)
    for i, b in enumerate(grid(zone, len(ms), gap, 1.7)):
        bx, by, bw, bh = b
        m = ms[i]
        col = p.acc[i % 3]
        with c.m(m):
            c.rect(bx, by, bw, bh, p.card, 0, stroke=p.line, sw=1, pre="fc")
            c.rect(bx, by, max(5.0, bw * 0.035), bh, col, 0, pre="sb")
        pd = min(bw, bh) * 0.1
        ix = bx + bw * 0.035 + pd
        body(c, (ix, by + pd * 0.7, bx + bw - pd - ix, bh - pd * 1.4), m, i, col, label=f"{i + 1:02d} {m.label}" if fmt.short >= 400 else None, g="t", bar=lambda c_, x_, y_, w_, h_, m_, i_: c_.bar(x_, y_, w_, h_, m_.src, m_.lo, m_.hi, m_.colors(c_.p, i_), radius=0, pre="b"))
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
