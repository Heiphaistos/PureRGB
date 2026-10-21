"""Anneaux concentriques (CPU, GPU, RAM) avec l'heure au centre, et la liste complete des mesures a cote ou dessous."""
from content import CONTENT
from lib import Canvas
from layouts.r1_common import CONTENTS_ALL, block, fit, pick, colopts, side_by_side, sizes

NAME, TITLE = "anneaux", "Anneaux concentriques"
CONTENTS = CONTENTS_ALL


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.04

    def mk(n, frac, cols):
        c = Canvas(p, fmt)
        wide = side_by_side(fmt)
        d = min(h, w * frac) if wide else min(w, h * frac)
        rx, ry = (x, y + (h - d) / 2) if wide else (x + (w - d) / 2, y)
        cx, cy, R = rx + d / 2, ry + d / 2, d / 2
        th = R * 0.11
        step = th * 1.5
        for i, m in enumerate(ms[:3]):
            c.arc(cx, cy, R - i * step, m.src, m.lo, m.hi, th, m.colors(p, i), start=120, sweep=300)
        inner = R - 3 * step + th * 0.6
        side = inner * 1.5
        c.value("time", "%H:%M", cx - side / 2, cy - inner * 0.3, side, inner * 0.6, "88:88", p.text, bold=True, align="center", pre="time")
        box = (rx + d + gap, y, w - d - gap, h) if wide else (x, ry + d + gap, w, y + h - ry - d - gap)
        block(c, box, pick(ms, n), cols, gap, colored=3, maxrow=fmt.short * 0.14)
        return c.finish()
    return fit(mk, fmt, sizes(ms), (0.42, 0.36, 0.5), colopts(fmt))
