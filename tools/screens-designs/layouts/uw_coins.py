"""Quatre blocs de cartes aux coins (heure en haut a gauche), centre LIBRE pour un fond video/GIF."""
from content import CONTENT
from lib import Canvas, grid, inset
from parts import stat_card

NAME, TITLE = "uw_coins", "Coins, centre libre"
CONTENTS = ["uw-lsys", "uw-ltherm", "uw-lstock", "uw-lair"]
FORMATS = ["u1920", "w1280", "w169"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    cols = 2 if fmt.w > 1000 else 1
    bw = w * (0.3 if cols == 2 else 0.27)
    k = 0
    for side, sx in enumerate((x, x + w - bw)):
        if cols == 2:
            cells = grid((sx, y, bw, h), 4, gap, target=1.3)
        else:
            cells = [(sx, y + r * (h + gap) / 2, bw, (h - gap) / 2) for r in range(2)]
        for j, box in enumerate(cells):
            if side == 0 and j == 0:
                c.card(*box)
                bx, by, bwid, bh = inset(box, min(box[2], box[3]) * 0.08)
                c.value("time", "%H:%M", bx, by, bwid, bh * 0.62, "88:88", p.text, bold=True, align="center", pre="time")
                c.value("date", "%d/%m/%Y", bx, by + bh * 0.68, bwid, bh * 0.2, "88/88/8888", p.text2, align="center", pre="date")
                continue
            stat_card(c, box, ms[k], k)
            k += 1
    return c.finish()
