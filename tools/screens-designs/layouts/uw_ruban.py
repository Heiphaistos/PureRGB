"""Ruban : carte heure/date a gauche puis une rangee de jauges circulaires (petits bandeaux, large, ultra-large)."""
from content import CONTENT
from lib import Canvas, inset
from parts import ring

NAME, TITLE = "uw_ruban", "Ruban à jauges"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["rb640", "w1280", "u1920"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04 + 4
    tw = min(w * 0.3, h * 1.9)
    c.card(x, y, tw, h)
    bx, by, bw, bh = inset((x, y, tw, h), h * 0.08)
    c.value("time", "%H:%M", bx, by + bh * 0.08, bw, bh * 0.56, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%d/%m/%Y", bx, by + bh * 0.7, bw, bh * 0.2, "88/88/8888", p.text2, align="center", pre="date")
    avail = w - tw - gap
    n = max(1, min(5, round(avail / (0.95 * h + gap))))
    d = min(h, (avail - gap * (n - 1)) / n)
    x0 = x + tw + gap + (avail - n * d - gap * (n - 1)) / 2
    for i in range(n):
        ring(c, x0 + d / 2 + i * (d + gap), y + h / 2, d / 2, ms[i], i, thick=d * 0.055)
    return c.finish()
