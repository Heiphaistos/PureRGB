"""Tableau de bord de jauges : une jauge 270 deg par mesure (CPU / GPU : charge dans l'ouverture), extras en lignes."""
from content import CONTENT
from lib import Canvas, grid, split
from layouts.r2_common import SETS_B, counts, first_ok, gauge, rows

NAME, TITLE = "jauges", "Tableau de bord à jauges"
CONTENTS = SETS_B


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.02

    def all_mk(n, thick, one_row):
        def f():
            c = Canvas(p, fmt)
            cells = split(fmt.zone, n, gap) if one_row else grid(fmt.zone, n, gap, target=1.0)
            for i, (bx, by, bw, bh) in enumerate(cells):
                r = min(bw, bh) / 2
                gauge(c, bx + bw / 2, by + bh / 2, r, ms[i], i, sub=i < 2, thick=r * thick, gs="b" if i < 5 else "x")
            return c.finish()
        return f

    def hero_mk(n, cols, rh):
        """Deux grandes jauges (CPU, GPU) puis le reste en lignes : formats carres / petits."""
        def f():
            c = Canvas(p, fmt)
            hh = h * rh
            for i, (bx, by, bw, bh) in enumerate(split((x, y, w, hh), 2, gap * 2)):
                r = min(bw, bh) / 2
                gauge(c, bx + bw / 2, by + bh / 2, r, ms[i], i, sub=True, thick=r * 0.14)
            rows(c, (x, y + hh + gap * 2, w, h - hh - gap * 2), [(ms[i], i) for i in range(2, n)], cols, gap * 1.5)
            return c.finish()
        return f

    ns = counts(ms, (8, 7, 6, 5) if fmt.strip else (9, 8, 6, 5))
    makers = [all_mk(n, t, r) for n in ns for r in ((True, False) if fmt.ratio >= 1.7 else (False,)) for t in (0.17, 0.11)]
    if fmt.ratio < 2.2:
        makers += [hero_mk(n, cols, rh) for n in ns for rh in (0.5, 0.42) for cols in (1, 2)]
    return first_ok(fmt, makers)
