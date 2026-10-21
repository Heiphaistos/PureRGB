"""Ligne de temps : heure et date en tete, puis une rangee de courbes d'historique (libelle, charge et valeur au-dessus)."""
from content import CONTENT
from lib import Canvas, split, inset

NAME, TITLE = "uw_timeline", "Ligne de temps"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    n = 5 if fmt.w < 1000 else 6 if fmt.w < 1500 else 8
    ms = CONTENT[content][1][:n]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    hh = h * (0.3 if fmt.strip else 0.22)
    c.value("time", "%H:%M", x, y, w * 0.2, hh, "88:88", p.text, bold=True, align="left", pre="time")
    c.value("date", "%A %d %B %Y", x + w * 0.22, y + hh * 0.2, w * 0.5, hh * 0.6, "mercredi 30 septembre 2026", p.text2, pre="date")
    c.rect(x, y + hh + gap * 0.4, w, 2, p.line, 1, pre="sep")
    for i, box in enumerate(split((x, y + hh + gap, w, h - hh - gap), n, gap)):
        m = ms[i]
        bx, by, bw, bh = box
        with c.m(m):
            c.card(*box)
            ix, iy, iw, ih = inset(box, min(bw, bh) * 0.05)
            lh = ih * 0.15
            c.text(ix, iy, iw, lh, m.label, p.acc[i % 3], bold=True, pre="l", group="tl")
            c.value(m.src, m.fmt, ix, iy + lh * 1.05, iw, ih * 0.25, m.sample, p.text, bold=True, align="left", pre="v", group="tv")
            if m.sub:
                c.value(m.sub[0], m.sub[1], ix, iy + lh * 1.05 + ih * 0.26, iw, lh, m.sub[2], p.text2, align="left", pre="s", group="ts")
            lh = ih * 0.6
            c.graph(ix, iy + lh, iw, ih - lh, m.src, m.lo, m.hi, p.acc[i % 3], p.acc[i % 3] + "40", thick=max(2, ih * 0.04), pre="g")
    return c.finish()
