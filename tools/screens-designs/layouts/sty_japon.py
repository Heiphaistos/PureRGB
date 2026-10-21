"""Minimal japonais : beaucoup de vide, anneau enso fin pour la valeur principale, lignes aerees, pastille rouge."""
from content import CONTENT
from lib import Canvas, split
from layouts.sty_common import need

NAME, TITLE = "sty-japon", "Minimal japonais"
CONTENTS = ["sty-pc", "sty-therm", "sty-io"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    hero = ms[0]
    if fmt.ratio >= 1.5:
        R = min(h * 0.48, w * 0.16)
        cx, cy = x + R + w * 0.06, y + h / 2
    else:
        R = min(w * 0.4, h * 0.26) if not fmt.round else w * 0.28
        cx, cy = x + w / 2, y + R + h * 0.02
    t = max(3.0, R * 0.045)
    c.arc(cx, cy, R, hero.src, hero.lo, hero.hi, t, [[0, p.acc[0]], [1, p.acc[0]]], start=-90, sweep=360)
    vw = R * 1.5
    c.value(hero.src, hero.fmt, cx - vw / 2, cy - R * 0.3, vw, R * 0.7, hero.sample, p.text, align="center", pre="hv")
    c.text(cx - R * 0.6, cy - R * 0.7, R * 1.2, need(fmt), hero.label.lower(), p.text2, align="center", pre="hl", maxsize=need(fmt))
    c.rect(cx - R * 0.05, cy + R * 0.6, R * 0.1, R * 0.1, p.acc[0], R * 0.05, pre="dot")
    if fmt.ratio >= 1.5:
        rx, ry, rw, rh = x + w * 0.4, y + h * 0.06, w * 0.6, h * 0.88
        c.rect(x + w * 0.37, y + h * 0.1, 1.5, h * 0.8, p.text2, 0, opacity=0.5, pre="vl")
        k = 4 if not fmt.strip else (4 if fmt.w > 1500 else 3)
        rows = split((rx, ry, rw, rh), min(k, 4) if fmt.ratio < 2.5 else 3, rh * 0.06, horizontal=False)
        if fmt.strip:
            rows = split((rx, ry, rw, rh), 3, rw * 0.04)
    else:
        top = cy + R + h * 0.07
        rows = split((x, top, w, y + h - top), 3, h * 0.02, horizontal=False)
        c.rect(x + w * 0.3, top - h * 0.035, w * 0.4, 1.5, p.text2, 0, opacity=0.6, pre="hr")
    strip = fmt.strip
    for i, (bx, by, bw, bh) in enumerate(rows):
        m = ms[1 + i]
        if strip:
            c.text(bx, by + bh * 0.12, bw, bh * 0.25, m.label.lower(), p.text2, pre="l", group="jl", maxsize=bh * 0.25)
            c.value(m.src, m.fmt, bx, by + bh * 0.4, bw, bh * 0.55, m.sample, p.text, pre="v", group="jv")
            continue
        c.text(bx, by, bw * 0.4, bh * 0.8, m.label.lower(), p.text2, pre="l", group="jl", maxsize=bh * 0.6)
        c.value(m.src, m.fmt, bx + bw * 0.4, by, bw * 0.6, bh * 0.8, m.sample, p.text, align="right", pre="v", group="jv", maxsize=bh * 0.7)
        c.rect(bx, by + bh * 0.9, bw, 1.2, p.text2, 0, opacity=0.4, pre="ln")
    return c.finish()
