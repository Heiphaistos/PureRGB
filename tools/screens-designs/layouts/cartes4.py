"""Cartes en grille adaptative : CPU / GPU (temperature + charge), RAM, SSD 1-2, puis les extras du contenu."""
from content import CONTENT
from lib import Canvas, grid
from parts import stat_card
from layouts.r2_common import SETS, counts, first_ok, rcards

NAME, TITLE = "cartes4", "Quatre cartes"
CONTENTS = SETS


def build(p, fmt, content):
    ms = CONTENT[content][1]

    def mk(n, target):
        def f():
            c = Canvas(p, fmt)
            for i, box in enumerate(grid(fmt.zone, n, fmt.short * 0.035, target=target)):
                with c.m(ms[i]):
                    stat_card(c, box, ms[i], i, g="b" if i < 5 else "x")
            return c.finish()
        return f

    def rc(n, cols):
        def f():
            c = Canvas(p, fmt)
            rcards(c, fmt.zone, [(ms[i], i) for i in range(n)], cols, fmt.short * 0.03)
            return c.finish()
        return f

    ns = counts(ms, (10, 9, 8, 6, 5))
    makers = [mk(n, t) for n in ns for t in (1.2, 1.7, 3.0)] + [rc(n, cols) for n in ns for cols in (1, 2)]
    return first_ok(fmt, makers)
