"""Duo de cartes heros (CPU, GPU : temperature + charge) et mosaique de cartes pour RAM, SSD 1-2 et extras."""
from content import CONTENT
from lib import Canvas, grid, split
from parts import stat_card
from layouts.r2_common import SETS_B, counts, first_ok, rcards

NAME, TITLE = "duo", "Duo en cartes"
CONTENTS = SETS_B


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.035

    def mk(k, frac, cols=0):
        def f():
            c = Canvas(p, fmt)
            if w / h >= 1.7:   # heros a gauche, mosaique a droite
                hw = w * frac
                heroes = split((x, y, hw, h), 2, gap)
                rest = grid((x + hw + gap, y, w - hw - gap, h), k - 2, gap, 1.5)
            else:              # heros en haut, mosaique dessous
                hh = h * frac
                heroes = split((x, y, w, hh), 2, gap)
                rest = grid((x, y + hh + gap, w, h - hh - gap), k - 2, gap, 1.5)
            if cols:   # reste en lignes sur cartes (carres : plus lisible que de petites cartes)
                rz = heroes_end(frac)
                rcards(c, rz, [(ms[i], i) for i in range(2, k)], cols, gap)
                rest = []
            for i, box in enumerate(heroes + rest):
                with c.m(ms[i]):
                    stat_card(c, box, ms[i], i, g="h" if i < 2 else "b" if i < 5 else "x")
            return c.finish()
        return f

    def heroes_end(frac):
        if w / h >= 1.7:
            return (x + w * frac + gap, y, w - w * frac - gap, h)
        return (x, y + h * frac + gap, w, h - h * frac - gap)

    wide = w / h >= 1.7
    fracs = (0.42, 0.5) if wide else (0.4, 0.48)
    ks = counts(ms, (9, 8, 7, 5))
    return first_ok(fmt, [mk(k, f, cols) for k in ks for f in fracs for cols in (0, 1, 2)])
