"""Mosaïque de mini-jauges circulaires : valeur dans l'anneau, libellé dessous."""
from content import CONTENT
from lib import Canvas, grid, min_text
from parts import ring
from layouts.cap_common import densest

NAME, TITLE = "cap-minijauges", "Mini-jauges multiples"
CONTENTS = ["cap-systeme", "cap-thermique", "cap-complet"]


def build(p, fmt, content):
    ms = CONTENT[content][1]
    mt = min_text(fmt)

    def mk(n):
        c = Canvas(p, fmt)
        for i, (x, y, w, h) in enumerate(grid(fmt.zone, n, fmt.short * 0.02, 1.0)):
            lb = max(1.3 * mt, h * 0.16)
            r = min(w, h - lb) / 2 * 0.96
            ring(c, x + w / 2, y + (h - lb) / 2, r, ms[i], i, label=False)
            c.text(x, y + h - lb, w, lb, ms[i].label, p.text2, bold=True, align="center", pre="l", group="mj")
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 12), 2, -1))
