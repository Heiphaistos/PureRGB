"""Cartes a courbes : valeur et valeur secondaire en tete, historique glissant (sparkline) dessous."""
from content import CONTENT
from lib import Canvas, grid, inset
from layouts.r1_common import CONTENTS_ALL, fit, pick, sizes

NAME, TITLE = "courbes", "Cartes à courbes"
CONTENTS = CONTENTS_ALL


def card(c, box, m, i):
    p = c.p
    x, y, w, h = box
    ix, iy, iw, ih = inset(box, min(w, h) * 0.09)
    col = p.acc[i % 3]
    th = max(2, ih * 0.035)
    with c.m(m):
        c.card(x, y, w, h)
        if w > 2.0 * h:   # carte large : texte a gauche, courbe a droite
            tw = iw * 0.36
            c.text(ix, iy, tw, ih * 0.28, m.label, p.text2, bold=True, pre="l", group="kl", maxsize=ih * 0.26)
            c.value(m.src, m.fmt, ix, iy + ih * 0.3, tw, ih * 0.4, m.sample, p.text, bold=True, pre="v", group="kv")
            if m.sub:
                c.value(m.sub[0], m.sub[1], ix, iy + ih * 0.72, tw, ih * 0.28, m.sub[2], p.text2, pre="s", group="ks", maxsize=ih * 0.24)
            c.graph(ix + iw * 0.4, iy, iw * 0.6, ih, m.src, m.lo, m.hi, col, col + "40", thick=th, pre="g")
            return
        c.text(ix, iy, iw, ih * 0.16, m.label, p.text2, bold=True, pre="l", group="kl", maxsize=ih * 0.15)
        vw = iw * (0.58 if m.sub else 1)
        c.value(m.src, m.fmt, ix, iy + ih * 0.17, vw, ih * 0.27, m.sample, p.text, bold=True, pre="v", group="kv")
        if m.sub:
            c.value(m.sub[0], m.sub[1], ix + vw, iy + ih * 0.2, iw - vw, ih * 0.22, m.sub[2], p.text2, align="right", pre="s", group="ks")
        c.graph(ix, iy + ih * 0.5, iw, ih * 0.5, m.src, m.lo, m.hi, col, col + "40", thick=th, pre="g")


def build(p, fmt, content):
    ms = CONTENT[content][1]

    def mk(n, target):
        c = Canvas(p, fmt)
        for i, (box, m) in enumerate(zip(grid(fmt.zone, n, fmt.short * 0.03, target), pick(ms, n))):
            card(c, box, m, i)
        return c.finish()
    return fit(mk, fmt, [n for n in sizes(ms) if n <= 8], (1.5, 1.1, 2.2))
