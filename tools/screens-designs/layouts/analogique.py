"""Horloge analogique, date complete et liste des mesures (a cote ou dessous selon le format)."""
from content import CONTENT
from lib import Canvas
from layouts.r1_common import CONTENTS_ALL, block, colopts, fit, pick, side_by_side, sizes

NAME, TITLE = "analogique", "Horloge analogique"
CONTENTS = CONTENTS_ALL


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04
    face = p.card if p.dark else "#ffffff"

    def mk(n, frac, cols, date):
        c = Canvas(p, fmt)
        wide = side_by_side(fmt)
        dh = (h if wide else h * frac) * 0.1 if date else 0
        avail = (h if wide else h * frac) - 2 * dh - (gap if date else 0)
        d = min(avail, w * frac) if wide else min(avail, w)
        rx, ry = (x, y + (h - d - 2 * dh) / 2) if wide else (x + (w - d) / 2, y)
        c.clock(rx, ry, d, d, face, p.text, p.acc[0])
        if date:
            bw = min(d * 1.3, w)
            bx = rx + d / 2 - bw / 2
            c.value("date", "%A %d", bx, ry + d + gap * 0.5, bw, dh, "mercredi 30", p.text, bold=True, align="center", pre="d", group="dd")
            c.value("date", "%B %Y", bx, ry + d + gap * 0.5 + dh, bw, dh, "septembre 2026", p.text2, align="center", pre="d", group="dd")
        bot = ry + d + 2 * dh + (gap if date else 0)
        box = (rx + d + gap, y, w - d - gap, h) if wide else (x, bot + gap, w, y + h - bot - gap)
        block(c, box, pick(ms, n), cols, gap, maxrow=fmt.short * 0.14)
        return c.finish()
    return fit(mk, fmt, sizes(ms), (0.4, 0.34, 0.46), colopts(fmt), (True, False))
