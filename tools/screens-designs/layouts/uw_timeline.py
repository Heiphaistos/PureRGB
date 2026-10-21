"""Ligne de temps : heure et date en tete, puis une rangee de courbes d'historique (libelle + valeur au-dessus)."""
from content import CONTENT
from lib import Canvas, split, inset

NAME, TITLE = "uw_timeline", "Ligne de temps"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["u1920", "w1280", "w169", "rb640"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    n = 4 if fmt.w > 1000 else 3 if fmt.w > 800 else 2
    ms = CONTENT[content][1][:n]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    hh = h * (0.3 if fmt.strip else 0.22)
    c.value("time", "%H:%M", x, y, w * 0.2, hh, "88:88", p.text, bold=True, align="left", pre="time")
    c.value("date", "%A %d %B", x + w * 0.22, y + hh * 0.2, w * 0.4, hh * 0.6, "mercredi 30 septembre", p.text2, pre="date")
    c.rect(x, y + hh + gap * 0.4, w, 2, p.line, 1, pre="sep")
    for i, box in enumerate(split((x, y + hh + gap, w, h - hh - gap), n, gap)):
        m = ms[i]
        bx, by, bw, bh = box
        c.card(*box)
        ix, iy, iw, ih = inset(box, min(bw, bh) * 0.08)
        lh = ih * 0.26
        c.text(ix, iy, iw * 0.45, lh, m.label, p.acc[i % 3], bold=True, pre="l", group="tl")
        c.value(m.src, m.fmt, ix + iw * 0.45, iy, iw * 0.55, lh, m.sample, p.text, bold=True, align="right", pre="v", group="tv")
        c.graph(ix, iy + lh * 1.2, iw, ih - lh * 1.2, m.src, m.lo, m.hi, p.acc[i % 3], p.acc[i % 3] + "40", thick=max(2, ih * 0.04), pre="g")
    return c.finish()
