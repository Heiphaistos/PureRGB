"""Horloge numerique + date complete en heros ; BASE (CPU/GPU temp+charge, RAM, SSD 1-2) et extras en lignes / tuiles."""
from content import CONTENT
from lib import Canvas
from parts import glow
from layouts.r2_common import SETS, counts, date_line, first_ok, rows, tiles

NAME, TITLE = "horloge", "Horloge et capteurs"
CONTENTS = SETS


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    wide = fmt.ratio >= 1.7

    def mk(n, two, tfrac, cols):
        def f():
            c = Canvas(p, fmt)
            items = [(ms[i], i) for i in range(n)]
            if p.dark:
                glow(c, x + w * (0.25 if wide else 0.5), y + h * (0.4 if wide else 0.2), min(w, h) * 0.4, p.acc[1])
            if wide:   # heure et date a gauche, capteurs en lignes a droite
                tw = w * tfrac
                c.value("time", "%H:%M", x, y + h * 0.04, tw, h * 0.58, "88:88", p.text, bold=True, align="center", pre="time")
                date_line(c, (x, y + h * 0.66, tw, h * (0.3 if two else 0.16)), two)
                c.rect(x + tw + gap, y + h * 0.06, 2, h * 0.88, p.line, 1, pre="sep")
                rows(c, (x + tw + gap * 3, y, w - tw - gap * 3, h), items, cols, gap, bar=True)
                return c.finish()
            th = h * tfrac
            c.value("time", "%H:%M", x, y, w, th * 0.74, "88:88", p.text, bold=True, align="center", pre="time")
            date_line(c, (x, y + th * 0.76, w, th * (0.5 if two else 0.24)), two)
            top = y + th * (1.3 if two else 1.04)
            if cols == 0:
                tiles(c, (x, top, w, y + h - top), items, gap, target=1.3)
            else:
                rows(c, (x, top, w, y + h - top), items, cols, gap, bar=True)
            return c.finish()
        return f

    makers = []
    for tfrac in ((0.45, 0.4) if wide else (0.4, 0.34, 0.28)):
        for n in counts(ms, (9, 8, 6, 5)):
            for two in (False, True):
                makers += [mk(n, two, tfrac, cols) for cols in ((1, 2) if wide else (0, 1, 2))]
    return first_ok(fmt, makers)
