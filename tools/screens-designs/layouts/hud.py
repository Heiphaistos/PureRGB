"""HUD gaming : equerres aux coins, grande jauge centrale CPU (temperature + charge), stats BASE et extras sur les cotes."""
from content import CONTENT
from lib import Canvas, split
from parts import corners
from layouts.r2_common import BOTTOMS, SETS_B, bottom, counts, first_ok, gauge, mrow

NAME, TITLE = "hud", "HUD gaming"
CONTENTS = SETS_B


def accepts(p):
    return p.dark


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    mg = fmt.margin * 0.5
    side = fmt.short * 0.68 if fmt.round else None
    box = ((fmt.w - side) / 2, (fmt.h - side) / 2, side, side) if side else (mg, mg, fmt.w - 2 * mg, fmt.h - 2 * mg)

    def frame(c):
        corners(c, box, p.acc[1], fmt.short * (0.06 if side else 0.09), max(2, fmt.short * 0.006))

    def hero(c, cx, cy, r):
        c.arc(cx, cy, r, ms[0].src, 0, 0, 2, p.temp_ramp, track=p.acc[1] + "66", pre="halo")
        gauge(c, cx, cy, r * 0.9, ms[0], 0, sub=True, thick=r * 0.15)

    def wide_mk(n, rfrac, sb, sn):
        def f():
            c = Canvas(p, fmt)
            frame(c)
            r = min(h / 2, w * rfrac)
            hero(c, x + w / 2, y + h / 2, r)
            others = [(ms[i], i) for i in range(1, n)]
            left = (len(others) + 1) // 2
            sw = (w - 2 * r) / 2 - w * 0.04
            gap = h * 0.04
            for bx, picks in ((x, others[:left]), (x + w - sw, others[left:])):
                rh = (h * 0.9 - gap * (left - 1)) / left
                for k, (m, i) in enumerate(picks):
                    mrow(c, (bx, y + h * 0.05 + k * (rh + gap), sw, rh), m, i, k=i, subw=sb, subn=sn)
            return c.finish()
        return f

    def tall_mk(n, rfrac, kind):
        def f():
            c = Canvas(p, fmt)
            frame(c)
            r = min(w / 2, h * rfrac)
            hero(c, x + w / 2, y + r * 1.02, r)
            top = y + 2.1 * r
            bottom(c, (x, top, w, y + h - top), [(ms[i], i) for i in range(1, n)], fmt.short * 0.025, kind)
            return c.finish()
        return f

    ns = counts(ms, (9, 8, 6, 5))
    if fmt.ratio >= 1.5:
        return first_ok(fmt, [wide_mk(n, rf, sb, sn) for n in ns for rf in (0.2, 0.16) for sb in (0.25, 0.34) for sn in (99, 2)])
    return first_ok(fmt, [tall_mk(n, rf, k) for n in ns for rf in (0.34, 0.3, 0.24, 0.2) for k in BOTTOMS])
