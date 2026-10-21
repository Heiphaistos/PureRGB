"""Cyberpunk / HUD sci-fi : equerres, quadrillage, bandeau de titre, panneaux a languette et barres segmentees."""
from content import CONTENT
from lib import Canvas, grid, split
from parts import corners
from layouts.sty_common import cols_for, need, grid_lines, seg_bar, scan

NAME, TITLE = "sty-cyberhud", "Cyberpunk HUD"
CONTENTS = ["sty-pc", "sty-therm", "sty-puiss"]


def accepts(p):
    return p.dark


def _cell(c, box, m, i, fmt):
    p = c.p
    x, y, w, h = box
    pad = min(w, h) * 0.09
    col = p.acc[i % 3]
    c.rect(x, y, w, h, p.card, 0, stroke=col, sw=1, opacity=0.9, pre="pn")
    c.rect(x, y, w * 0.38, max(3, h * 0.03), col, 0, pre="tab")
    c.rect(x + w - pad, y + h - pad * 0.5, pad * 0.9, pad * 0.35, col, 0, pre="nt")
    lh = max(h * 0.15, min(need(fmt), h * 0.3))
    c.text(x + pad, y + pad * 0.7, w * 0.7, lh, m.label, col, bold=True, pre="l", group="cl", font=p.font, maxsize=lh)
    c.value(m.src, m.fmt, x + pad, y + h * 0.27, w - 2 * pad, h * 0.42, m.sample, p.text, bold=True, pre="v", group="cv", font=p.font)
    seg_bar(c, x + pad, y + h * 0.77, w - 2 * pad, max(5.0, h * 0.09), m, i, 12)


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    mg = fmt.margin * 0.45
    box = (x - mg * 0.5, y - mg * 0.5, w + mg, h + mg) if fmt.round else (mg, mg, fmt.w - 2 * mg, fmt.h - 2 * mg)
    if not fmt.round:
        grid_lines(c, box, max(4, round(box[2] / 60)), max(3, round(box[3] / 60)), p.acc[1], 0.09)
    corners(c, box, p.acc[1], fmt.short * 0.07, max(2, fmt.short * 0.007))
    hh = 0.0
    if not fmt.strip and fmt.short > 240:
        hh = h * (0.13 if not fmt.round else 0.12)
        c.text(x, y, w * 0.6, hh * 0.8, "SYSTÈME EN LIGNE", p.acc[1], bold=True, pre="hd", maxsize=hh * 0.62, font=p.font)
        c.value("time", "%H:%M", x + w * 0.62, y, w * 0.38, hh * 0.8, "88:88", p.acc[0], bold=True, align="right", pre="tm", maxsize=hh * 0.7, font=p.font)
        c.rect(x, y + hh * 0.88, w, 2, p.acc[1], 0, opacity=0.6, pre="hr")
        hh *= 1.1
    n = cols_for(fmt, 4)
    gap = fmt.short * 0.03
    zone = (x, y + hh, w, h - hh)
    for i, b in enumerate(split(zone, n, gap) if fmt.ratio >= 1.7 else grid(zone, n, gap, 1.35)):
        _cell(c, b, ms[i], i, fmt)
    if not fmt.round:
        scan(c, box, max(4, fmt.h / 90), 0.07)
    return c.finish()
