"""Frise de jauges verticales : valeur, barre haute, valeur secondaire (charge), libelle ; socle CPU/GPU/RAM/SSD puis extras."""
from content import CONTENT
from lib import Canvas, split

NAME, TITLE = "uw_frise", "Frise de jauges"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["u1920", "w1280", "w169", "rb640"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    n = 5 if fmt.w < 900 else 8 if fmt.w < 1500 else 10
    ms = CONTENT[content][1][:n]
    n = len(ms)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.025
    for i, (cx, cy, cw, ch) in enumerate(split((x, y, w, h), n, gap)):
        m = ms[i]
        with c.m(m):
            c.card(cx, cy, cw, ch)
            ix, iw = cx + cw * 0.06, cw * 0.88
            sub = m.sub is not None
            c.value(m.src, m.fmt, ix, cy + ch * 0.04, iw, ch * 0.18, m.sample, p.text, bold=True, align="center", pre="v", group="fv")
            c.bar(cx + cw * 0.33, cy + ch * 0.25, cw * 0.34, ch * (0.4 if sub else 0.52), m.src, m.lo, m.hi, m.colors(p, i), vertical=True, pre="b")
            if sub:
                c.value(m.sub[0], m.sub[1], ix, cy + ch * 0.67, iw, ch * 0.14, m.sub[2], p.text2, align="center", pre="s", group="fs")
            c.text(ix, cy + ch * 0.82, iw, ch * 0.14, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="fl")
    return c.finish()
