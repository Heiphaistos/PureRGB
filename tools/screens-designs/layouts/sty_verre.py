"""Verre depoli : taches de couleur floues derriere des cartes de verre (reflet haut, liseré clair)."""
from content import CONTENT
from lib import Canvas, grid, split
from parts import ring
from layouts.sty_common import cols_for, lv, need

NAME, TITLE = "sty-verre", "Verre dépoli"
CONTENTS = ["sty-pc", "sty-therm", "sty-puiss"]


def _glass(c, box, r, hi=True):
    p = c.p
    x, y, w, h = box
    c.rect(x, y, w, h, "#ffffff" if p.dark else "#ffffff", r, stroke=p.text, sw=1, opacity=0.2 if p.dark else 0.55, pre="gl")
    if hi and h > 30:
        c.rect(x + r * 0.6, y + 1.5, w - r * 1.2, 1.5, "#ffffff", 0, opacity=0.55, pre="hi")


def _blobs(c, fmt):
    p = c.p
    s = fmt.short
    pts = [(0.18, 0.3, 0.5, 0), (0.78, 0.25, 0.42, 1), (0.55, 0.8, 0.5, 2)]
    for fx, fy, k, i in pts:
        r = min(s * k * 0.5, fmt.w / 2, fmt.h / 2)
        cx = min(max(fmt.w * fx, r), fmt.w - r)
        cy = min(max(fmt.h * fy, r), fmt.h - r)
        for st in range(7):   # flou simule : disques concentriques tres transparents
            rr = r * (1 - st * 0.12)
            c.rect(cx - rr, cy - rr, 2 * rr, 2 * rr, p.acc[i], rr, opacity=0.07 + 0.025 * st, pre="bl")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.035
    if not fmt.round:
        _blobs(c, fmt)
    else:
        for i, (dx, dy) in enumerate([(-0.18, -0.12), (0.2, -0.1), (0.0, 0.22)]):
            r = fmt.short * 0.2
            cx, cy = fmt.w / 2 + dx * fmt.w, fmt.h / 2 + dy * fmt.h
            for st in range(7):
                rr = r * (1 - st * 0.12)
                c.rect(cx - rr, cy - rr, 2 * rr, 2 * rr, p.acc[i], rr, opacity=0.07 + 0.025 * st, pre="bl")
    rad = min(fmt.short * 0.05, 26)
    if fmt.ratio >= 1.6:
        hw = min(h, w * 0.3)
        hero = (x, y, hw, h)
        rest = (x + hw + gap, y, w - hw - gap, h)
    elif fmt.round or fmt.short <= 240:
        hero, rest = (x, y, w, h * 0.5), (x, y + h * 0.5 + gap, w, h * 0.5 - gap)
    else:
        hero, rest = (x, y, w, h * 0.48), (x, y + h * 0.48 + gap, w, h * 0.52 - gap)
    _glass(c, hero, rad)
    hx, hy, hw_, hh_ = hero
    lh = need(fmt)
    c.text(hx + hw_ * 0.08, hy + hh_ * 0.05, hw_ * 0.84, lh, ms[0].label, p.text2, bold=True, align="center", pre="hl", maxsize=lh)
    r = min(hw_ * 0.45, (hh_ - lh * 1.4) / 2 * 1.0)
    ring(c, hx + hw_ / 2, hy + lh * 1.2 + hh_ * 0.03 + (hh_ - lh * 1.4) / 2, r, ms[0], 0, label=False)
    n = cols_for(fmt, 3)
    cells = grid(rest, n, gap, 2.4 if fmt.ratio < 1.6 else 1.5)
    if fmt.ratio >= 1.6 and not fmt.strip:
        cells = grid(rest, n, gap, 1.6)
    for i, b in enumerate(cells):
        _glass(c, b, rad)
        bx, by, bw, bh = b
        pd = min(bw, bh) * 0.1
        lv(c, (bx + pd, by + pd, bw - 2 * pd, bh - 2 * pd), ms[1 + i], p.acc[i % 3], valign="center")
    return c.finish()
