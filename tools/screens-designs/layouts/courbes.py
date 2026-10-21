"""Cartes a courbes : valeur en tete, historique glissant (sparkline) dessous."""
from content import CONTENT
from lib import Canvas, grid, inset

NAME, TITLE = "courbes", "Cartes à courbes"
CONTENTS = ["quad", "reseau", "carte-mere", "gpu"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:4 if fmt.ratio < 2.6 else 3]
    n = len(ms)
    for i, box in enumerate(grid(fmt.zone, n, fmt.short * 0.035, target=1.5 if fmt.ratio < 2.6 else 1.0)):
        x, y, w, h = box
        m = ms[i]
        c.card(x, y, w, h)
        ix, iy, iw, ih = inset(box, min(w, h) * 0.09)
        col = p.acc[i % 3]
        if w > 2.0 * h:   # carte large : texte a gauche, courbe a droite
            c.text(ix, iy, iw * 0.34, ih * 0.4, m.label, p.text2, bold=True, pre="l", group="kl", maxsize=ih * 0.3)
            c.value(m.src, m.fmt, ix, iy + ih * 0.42, iw * 0.34, ih * 0.58, m.sample, p.text, bold=True, pre="v", group="kv")
            c.graph(ix + iw * 0.38, iy, iw * 0.62, ih, m.src, m.lo, m.hi, col, col + "40", thick=max(2, ih * 0.04), pre="g")
            continue
        c.text(ix, iy, iw, ih * 0.16, m.label, p.text2, bold=True, pre="l", group="kl", maxsize=ih * 0.15)
        c.value(m.src, m.fmt, ix, iy + ih * 0.17, iw, ih * 0.27, m.sample, p.text, bold=True, pre="v", group="kv")
        c.graph(ix, iy + ih * 0.5, iw, ih * 0.5, m.src, m.lo, m.hi, col, col + "40", thick=max(2, ih * 0.03), pre="g")
    return c.finish()
