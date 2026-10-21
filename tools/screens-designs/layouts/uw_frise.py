"""Frise de jauges verticales : valeur au-dessus, barre haute, libelle dessous, sur toute la largeur."""
from content import CONTENT
from lib import Canvas, split

NAME, TITLE = "uw_frise", "Frise de jauges"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["u1920", "w1280", "w169", "rb640"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    n = 6 if fmt.w < 900 else 8
    ms = CONTENT[content][1][:n]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.025
    for i, (cx, cy, cw, ch) in enumerate(split((x, y, w, h), n, gap)):
        m = ms[i]
        c.card(cx, cy, cw, ch)
        ix, iw = cx + cw * 0.08, cw * 0.84
        c.value(m.src, m.fmt, ix, cy + ch * 0.04, iw, ch * 0.17, m.sample, p.text, bold=True, align="center", pre="v", group="fv")
        c.bar(cx + cw * 0.3, cy + ch * 0.24, cw * 0.4, ch * 0.54, m.src, m.lo, m.hi, m.colors(p, i), vertical=True, pre="b")
        c.text(ix, cy + ch * 0.8, iw, ch * 0.16, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="fl")
    return c.finish()
