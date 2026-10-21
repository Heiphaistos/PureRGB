"""16:9 : grande jauge heros a gauche, heure et liste a droite ; 4:3 : heure + 4 lignes."""
from content import CONTENT
from lib import Canvas
from parts import ring, row

NAME, TITLE = "uw_cinema", "Cinéma 16:9 et 4:3"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["w169", "r43"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    if fmt.w > 800:
        d = min(h, w * 0.4)
        ring(c, x + d / 2, y + h / 2, d / 2, ms[0], 0, sub=True)
        lx = x + d + gap * 1.5
        lw = w - d - gap * 1.5
        c.value("time", "%H:%M", lx, y, lw * 0.4, h * 0.2, "88:88", p.text, bold=True, pre="time")
        c.value("date", "%d/%m/%Y", lx + lw * 0.45, y + h * 0.04, lw * 0.55, h * 0.12, "88/88/8888", p.text2, align="right", pre="date")
        ry = y + h * 0.26
        rh = (h * 0.74 - gap) / 5
        for i in range(5):
            row(c, (lx, ry + i * (rh + gap * 0.35), lw, rh), ms[i + 1], i + 1)
        return c.finish()
    hh = h * 0.2
    c.value("time", "%H:%M", x, y, w * 0.4, hh, "88:88", p.text, bold=True, pre="time")
    c.value("date", "%d/%m/%Y", x + w * 0.45, y + hh * 0.15, w * 0.55, hh * 0.65, "88/88/8888", p.text2, align="right", pre="date")
    rh = (h - hh - gap) / 4
    for i in range(4):
        row(c, (x, y + hh + gap + i * rh, w, rh * 0.9), ms[i], i)
    return c.finish()
