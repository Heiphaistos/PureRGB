"""Colonnes separees (ecrans larges) ou lignes separees (carres) : sans cartes, filets fins ; BASE + extras."""
from content import CONTENT
from lib import Canvas, split
from layouts.r2_common import SETS_C, counts, first_ok, g, rows

NAME, TITLE = "colonnes", "Colonnes séparées"
CONTENTS = SETS_C


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone

    def cols_mk(n):
        def f():
            c = Canvas(p, fmt)
            gap = fmt.short * 0.06
            for i, (bx, by, bw, bh) in enumerate(split(fmt.zone, n, gap)):
                m = ms[i]
                gg = g(i)
                with c.m(m):
                    if i:
                        c.rect(bx - gap / 2 - 1, by + bh * 0.06, 2, bh * 0.88, p.line, 1, pre="sep")
                    c.text(bx, by, bw, bh * 0.16, m.label, p.acc[i % 3], bold=True, pre="l", group="cl" + gg, font=p.font)
                    c.value(m.src, m.fmt, bx, by + bh * 0.2, bw, bh * 0.4, m.sample, p.text, bold=True, pre="v", group="cv" + gg, font=p.font)
                    if m.sub:
                        c.value(m.sub[0], m.sub[1], bx, by + bh * 0.63, bw, bh * 0.15, m.sub[2], p.text2, pre="s", group="cs" + gg)
                    c.bar(bx, by + bh * 0.88, bw, max(5, bh * 0.06), m.src, m.lo, m.hi, m.colors(p, i), pre="b")
            return c.finish()
        return f

    def rows_mk(n, cols):
        def f():
            c = Canvas(p, fmt)
            gap = fmt.short * 0.03
            rows(c, fmt.zone, [(ms[i], i) for i in range(n)], cols, gap, bar=True, lab_color=p.acc[0])
            return c.finish()
        return f

    ns = counts(ms, (8, 7, 6, 5))
    if fmt.ratio >= 1.7:
        return first_ok(fmt, [cols_mk(n) for n in ns])
    return first_ok(fmt, [rows_mk(n, cols) for n in ns for cols in (1, 2)])
