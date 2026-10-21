"""Blocs de cartes aux deux coins (heure en haut a gauche, socle CPU/GPU/RAM/SSD + extras), centre LIBRE pour un fond video/GIF."""
from content import CONTENT
from lib import Canvas, grid, inset
from parts import stat_card

NAME, TITLE = "uw_coins", "Coins, centre libre"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair", "uw-lpuis", "uw-lgpu", "uw-lcpu", "uw-lreseau", "uw-ldisq", "uw-lmix"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    big = fmt.w > 1000
    bw = w * (0.3 if big else 0.34)
    n = 4 if big else 3   # cartes par flanc (heure comprise a gauche)
    k = 0
    for side, sx in enumerate((x, x + w - bw)):
        cells = grid((sx, y, bw, h), n, gap, target=1.3) if big else [(sx, y + r * (h + gap) / n, bw, (h - gap * (n - 1)) / n) for r in range(n)]
        for j, box in enumerate(cells):
            if side == 0 and j == 0:
                c.card(*box)
                bx, by, bwid, bh = inset(box, min(box[2], box[3]) * 0.08)
                c.value("time", "%H:%M", bx, by, bwid, bh * 0.56, "88:88", p.text, bold=True, align="center", pre="time")
                c.value("date", "%d/%m/%Y", bx, by + bh * 0.62, bwid, bh * 0.3, "88/88/8888", p.text2, align="center", pre="date")
                continue
            with c.m(ms[k]):
                stat_card(c, box, ms[k], k)
            k += 1
    return c.finish()
