"""Ruban : carte heure/date a gauche (sauf ruban etroit) puis une rangee de jauges circulaires (socle CPU/GPU/RAM/SSD + extras)."""
from content import CONTENT
from lib import Canvas, inset
from layouts.uw_common import ring2

NAME, TITLE = "uw_ruban", "Ruban à jauges"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["rb640", "w1280", "u1920", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04 + 4
    if fmt.w < 1000:   # ruban etroit : pas de place pour l'heure, les 5 jauges du socle seules
        tw = 0
    else:
        tw = min(w * 0.2, h * 1.9)
        c.card(x, y, tw, h)
        bx, by, bw, bh = inset((x, y, tw, h), h * 0.08)
        c.value("time", "%H:%M", bx, by + bh * 0.08, bw, bh * 0.56, "88:88", p.text, bold=True, align="center", pre="time")
        c.value("date", "%d/%m/%Y", bx, by + bh * 0.7, bw, bh * 0.2, "88/88/8888", p.text2, align="center", pre="date")
        tw += gap
    avail = w - tw
    n = 6 if (avail - gap * 5) / 6 >= 160 else 5
    d = min(h, (avail - gap * (n - 1)) / n)
    x0 = x + tw + (avail - n * d - gap * (n - 1)) / 2
    for i in range(n):
        ring2(c, x0 + d / 2 + i * (d + gap), y + h / 2, d / 2, ms[i], i, thick=d * 0.055)
    return c.finish()
