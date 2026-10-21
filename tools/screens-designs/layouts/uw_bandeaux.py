"""Deux bandeaux translucides (haut : heure + socle CPU/GPU/RAM/SSD, bas : extras), milieu LIBRE pour une video/GIF."""
from content import CONTENT
from lib import Canvas, split
from layouts.uw_common import col

NAME, TITLE = "uw_bandeaux", "Bandeaux haut et bas"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair", "uw-lpuis", "uw-lgpu", "uw-lcpu", "uw-lreseau", "uw-ldisq", "uw-lmix"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    bh = h * (0.33 if fmt.w > 1000 else 0.36)
    pad = bh * 0.1
    gap = fmt.short * 0.03
    nb = 6 if fmt.w > 1000 else 5   # extras en bas
    for top in (True, False):
        by = y if top else y + h - bh
        c.card(x, by, w, bh, radius=min(bh * 0.2, 24))
        ix, iw = x + pad, w - 2 * pad
        if top:
            tw = iw * (0.2 if fmt.w > 1000 else 0.17)
            c.value("time", "%H:%M", ix, by + pad, tw, bh * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
            c.value("date", "%d/%m/%Y", ix, by + bh * 0.62, tw, bh * 0.22, "88/88/8888", p.text2, align="center", pre="date")
            c.rect(ix + tw + gap * 0.5, by + bh * 0.15, 2, bh * 0.7, p.line, 1, pre="sep")
            zone = (ix + tw + gap, by + pad, iw - tw - gap, bh - 2 * pad)
            idx = range(5)
        else:
            zone = (ix, by + pad, iw, bh - 2 * pad)
            idx = range(5, 5 + nb)
        idx = [k for k in idx if k < len(ms)]
        for k, (cx, cy, cw, ch) in zip(idx, split(zone, len(idx), gap)):
            col(c, (cx, cy, cw, ch), ms[k], k, g="bt" if top else "bb")
    return c.finish()
