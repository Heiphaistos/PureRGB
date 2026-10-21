"""Minimal japonais : beaucoup de vide, anneau enso fin pour la valeur principale, lignes aerees, pastille rouge."""
from lib import Canvas, grid
from layouts.sty_common import ALL8, body, fit, need

NAME, TITLE = "sty-japon", "Minimal japonais"
CONTENTS = ALL8


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    hero = ms[0]
    wide = fmt.ratio >= 1.5
    if wide:
        R = min(h * 0.48, w * 0.16)
        cx, cy = x + R + w * 0.02, y + h / 2
    else:
        R = min(w * 0.3, h * 0.25) if not fmt.round else w * 0.28
        cx, cy = x + w / 2, y + R + h * 0.02
    t = max(3.0, R * 0.045)
    c.arc(cx, cy, R, hero.src, hero.lo, hero.hi, t, [[0, p.acc[0]], [1, p.acc[0]]], start=-90, sweep=360)
    vw = R * 1.4
    c.text(cx - vw / 2, cy - R * 0.66, vw, R * 0.28, hero.label.lower(), p.text2, align="center", pre="hl")
    c.value(hero.src, hero.fmt, cx - vw / 2, cy - R * 0.36, vw, R * 0.6, hero.sample, p.text, align="center", pre="hv")
    c.value(hero.sub[0], hero.sub[1], cx - vw / 2, cy + R * 0.28, vw, R * 0.28, hero.sub[2], p.text2, align="center", pre="hs")
    c.rect(cx - R * 0.05, cy + R * 0.66, R * 0.1, R * 0.1, p.acc[0], R * 0.05, pre="dot")
    gap = fmt.short * 0.03
    if wide:
        rx = cx + R + w * 0.05
        c.rect(rx - w * 0.025, y + h * 0.1, 1.5, h * 0.8, p.text2, 0, opacity=0.5, pre="vl")
        rest = (rx, y + h * 0.04, x + w - rx, h * 0.92)
    else:
        top = cy + R + h * 0.06
        c.rect(x + w * 0.3, top - h * 0.03, w * 0.4, 1.5, p.text2, 0, opacity=0.6, pre="hr")
        rest = (x, top, w, y + h - top)
    line = lambda c_, x_, y_, w_, h_, m_, i_: c_.rect(x_, y_ + h_ / 2, w_, 1.2, p.text2, 0, opacity=0.4, pre="ln")
    for i, b in enumerate(grid(rest, len(ms) - 1, gap, 2.6)):
        body(c, b, ms[1 + i], i, p.text2, bar=line, g="j", bold=False, lower=True)
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
