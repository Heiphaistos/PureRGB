"""Deux bandeaux translucides (haut : heure + 4 mesures, bas : 4 mesures a barres), milieu LIBRE pour une video/GIF."""
from content import CONTENT
from lib import Canvas, split
from parts import mini_col

NAME, TITLE = "uw_bandeaux", "Bandeaux haut et bas"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    bh = h * (0.27 if fmt.w > 1000 else 0.3)
    pad = bh * 0.12
    gap = fmt.short * 0.03
    for top in (True, False):
        by = y if top else y + h - bh
        c.card(x, by, w, bh, radius=min(bh * 0.2, 24))
        ix, iw = x + pad, w - 2 * pad
        if top:
            tw = iw * 0.24
            c.value("time", "%H:%M", ix, by + pad, tw, bh * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
            c.value("date", "%d/%m/%Y", ix, by + bh * 0.62, tw, bh * 0.22, "88/88/8888", p.text2, align="center", pre="date")
            c.rect(ix + tw + gap * 0.5, by + bh * 0.15, 2, bh * 0.7, p.line, 1, pre="sep")
            zone = (ix + tw + gap, by + pad, iw - tw - gap, bh - 2 * pad)
            mm = ms[:4]
        else:
            zone = (ix, by + pad, iw, bh - 2 * pad)
            mm = ms[4:8]
        for i, (cx, cy, cw, ch) in enumerate(split(zone, 4, gap)):
            m = mm[i]
            mini_col(c, (cx, cy, cw, ch * 0.82), m, i + (0 if top else 4))
            c.bar(cx + cw * 0.1, cy + ch * 0.9, cw * 0.8, max(4, ch * 0.08), m.src, m.lo, m.hi, m.colors(p, i), pre="b")
    return c.finish()
