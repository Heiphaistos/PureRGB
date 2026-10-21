"""Militaire tactique : bandes de danger, fiches numerotees a barre laterale, entete d'etat, regle graduee."""
from content import CONTENT
from lib import Canvas, grid, split
from layouts.sty_common import cols_for, hazard, need, ticks_line

NAME, TITLE = "sty-tactique", "Tactique militaire"
CONTENTS = ["sty-pc", "sty-therm", "sty-puiss"]


def accepts(p):
    return p.dark


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.035
    band = max(10.0, fmt.short * 0.04)
    if not fmt.round:
        hazard(c, (x, y - band * 0.2, w, band), p.acc[1], band * 0.45)
        hazard(c, (x, y + h - band * 0.8, w, band), p.acc[1], band * 0.45)
        y, h = y + band * 1.1, h - band * 2.0
    hh = 0.0
    if not fmt.strip and fmt.short > 240:
        hh = need(fmt) * 1.1
        c.text(x, y, w * 0.6, hh, "ÉTAT SYSTÈME", p.acc[0], bold=True, pre="hd", maxsize=hh)
        c.value("time", "%H:%M:%S", x + w * 0.64, y, w * 0.36, hh, "88:88:88", p.text, bold=True, align="right", pre="tm", maxsize=hh)
        ticks_line(c, x, y + hh * 1.12, w, 40, hh * 0.3, p.text2, 0.7)
        hh *= 1.55
    n = cols_for(fmt, 4)
    zone = (x, y + hh, w, h - hh)
    for i, b in enumerate(split(zone, n, gap) if fmt.ratio >= 1.7 else grid(zone, n, gap, 1.7)):
        bx, by, bw, bh = b
        m = ms[i]
        col = p.acc[i % 3]
        c.rect(bx, by, bw, bh, p.card, 0, stroke=p.line, sw=1, pre="fc")
        c.rect(bx, by, max(5.0, bw * 0.035), bh, col, 0, pre="sb")
        pd = min(bw, bh) * 0.1
        ix, iw = bx + bw * 0.035 + pd, bw - bw * 0.035 - 2 * pd
        lh = max(bh * 0.2, min(need(fmt), bh * 0.3))
        bar = max(4.0, bh * 0.07)
        c.text(ix, by + pd * 0.6, iw, lh, f"{i + 1:02d} · {m.label}", col, bold=True, pre="l", group="tl", maxsize=lh)
        c.value(m.src, m.fmt, ix, by + pd * 0.6 + lh, iw, bh - lh - pd * 2 - bar, m.sample, p.text, bold=True, pre="v", group="tv", font=p.font)
        c.bar(ix, by + bh - pd - bar, iw, bar, m.src, m.lo, m.hi, m.colors(p, i), radius=0, pre="b")
    return c.finish()
