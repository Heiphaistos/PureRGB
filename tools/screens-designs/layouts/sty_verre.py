"""Verre depoli : taches de couleur floues derriere des cartes de verre (reflet haut, liseré clair)."""
from lib import Canvas, grid
from layouts.sty_common import ALL8, body, dial, fit

NAME, TITLE = "sty-verre", "Verre dépoli"
CONTENTS = ALL8


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
            c.rect(cx - rr, cy - rr, 2 * rr, 2 * rr, p.acc[i], rr, opacity=0.04 + 0.015 * st, pre="bl")


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
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
                c.rect(cx - rr, cy - rr, 2 * rr, 2 * rr, p.acc[i], rr, opacity=0.04 + 0.015 * st, pre="bl")
    rad = min(fmt.short * 0.05, 26)
    if fmt.ratio >= 1.6:
        hw = min(h, w * 0.3)
        hero = (x, y, hw, h)
        rest = (x + hw + gap, y, w - hw - gap, h)
    else:
        hero, rest = (x, y, w, h * 0.42), (x, y + h * 0.42 + gap, w, h * 0.58 - gap)
    _glass(c, hero, rad)
    hx, hy, hw_, hh_ = hero
    r = min(hw_, hh_) * 0.45
    dial(c, hx + hw_ / 2, hy + hh_ / 2, r, ms[0], 0)
    for i, b in enumerate(grid(rest, len(ms) - 1, gap, 1.7)):
        with c.m(ms[1 + i]):
            _glass(c, b, rad)
        bx, by, bw, bh = b
        pd = min(bw, bh) * 0.1
        body(c, (bx + pd, by + pd, bw - 2 * pd, bh - 2 * pd), ms[1 + i], i, p.acc[i % 3], g="g")
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
