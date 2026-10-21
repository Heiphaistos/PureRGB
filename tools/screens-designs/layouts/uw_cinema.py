"""16:9 et large : heure, deux grandes jauges CPU/GPU a gauche, liste (RAM, SSD, extras) a droite ; 4:3 : heure + 5 lignes du socle."""
from content import CONTENT
from lib import Canvas, split
from layouts.uw_common import line, ring2

NAME, TITLE = "uw_cinema", "Cinéma 16:9 et 4:3"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["w169", "r43", "w1280", "u1920"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    hh = h * 0.2
    c.value("time", "%H:%M", x, y, w * 0.3, hh, "88:88", p.text, bold=True, pre="time")
    if fmt.ratio < 1.5:
        c.value("date", "%d/%m/%Y", x + w * 0.4, y + hh * 0.15, w * 0.6, hh * 0.7, "88/88/8888", p.text2, align="right", pre="date")
    else:
        c.value("date", "%A %d %B %Y", x + w * 0.35, y + hh * 0.15, w * 0.65, hh * 0.7, "mercredi 30 septembre 2026", p.text2, align="right", pre="date")
    by, bh = y + hh + gap, h - hh - gap
    if fmt.w > 800:
        d = min(bh, w * 0.24)
        for i in range(2):
            ring2(c, x + d / 2 + i * (d + gap), by + bh / 2, d / 2, ms[i], i, thick=d * 0.06)
        lx = x + 2 * d + 3 * gap
        n = 6
        for i, box in enumerate(split((lx, by, min(x + w - lx, bh * 2.6), bh), n, gap * 0.4, horizontal=False)):
            line(c, box, ms[i + 2], i + 2, g="cl")
        return c.finish()
    if fmt.ratio < 1.5:   # 4:3 : cinq lignes
        for i, box in enumerate(split((x, by, w, bh), 5, gap * 0.3, horizontal=False)):
            line(c, box, ms[i], i, g="cl")
        return c.finish()
    return None
