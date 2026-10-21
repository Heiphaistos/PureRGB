"""Cyberpunk / HUD sci-fi : equerres, quadrillage, bandeau de titre, panneaux a languette et barres segmentees."""
from lib import Canvas, grid
from parts import corners
from layouts.sty_common import ALL8, body, fit, grid_lines, need, scan, seg_bar

NAME, TITLE = "sty-cyberhud", "Cyberpunk HUD"
CONTENTS = ALL8


def accepts(p):
    return p.dark


def _seg(c, x, y, w, h, m, i):
    seg_bar(c, x, y, w, h, m, i, max(6, min(12, int(w / 16))))


def _cell(c, box, m, i):
    p = c.p
    x, y, w, h = box
    pad = min(w, h) * 0.09
    col = p.acc[i % 3]
    with c.m(m):
        c.rect(x, y, w, h, p.card, 0, stroke=col, sw=1, opacity=0.9, pre="pn")
        c.rect(x, y, w * 0.38, max(3, h * 0.03), col, 0, pre="tab")
        c.rect(x + w - pad, y + h - pad * 0.5, pad * 0.9, pad * 0.35, col, 0, pre="nt")
    body(c, (x + pad, y + pad * 0.8, w - 2 * pad, h - pad * 1.6), m, i, col, font=p.font, g="c", bar=_seg)


def _draw(p, fmt, ms):
    if fmt.round:
        return None
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    mg = fmt.margin * 0.45
    box = (x - mg * 0.5, y - mg * 0.5, w + mg, h + mg) if fmt.round else (mg, mg, fmt.w - 2 * mg, fmt.h - 2 * mg)
    if not fmt.round:
        grid_lines(c, box, max(4, round(box[2] / 60)), max(3, round(box[3] / 60)), p.acc[1], 0.09)
    corners(c, box, p.acc[1], fmt.short * 0.07, max(2, fmt.short * 0.007))
    hh = 0.0
    if not fmt.strip and fmt.short > 240:
        hh = h * (0.11 if not fmt.round else 0.12)
        c.text(x, y, w * 0.5, hh * 0.8, "SYSTÈME EN LIGNE", p.acc[1], bold=True, pre="hd", maxsize=hh * 0.62, font=p.font)
        if not fmt.round:
            c.value("date", "%d/%m/%Y", x + w * 0.52, y, w * 0.26, hh * 0.8, "88/88/8888", p.text2, align="right", pre="dt", maxsize=hh * 0.5, font=p.font)
        c.value("time", "%H:%M", x + w * 0.8, y, w * 0.2, hh * 0.8, "88:88", p.acc[0], bold=True, align="right", pre="tm", maxsize=hh * 0.7, font=p.font)
        c.rect(x, y + hh * 0.88, w, 2, p.acc[1], 0, opacity=0.6, pre="hr")
        hh *= 1.1
    gap = fmt.short * 0.03
    for i, b in enumerate(grid((x, y + hh, w, h - hh), len(ms), gap, 1.3)):
        _cell(c, b, ms[i], i)
    if not fmt.round:
        scan(c, box, max(8, fmt.h / 40), 0.07)
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
