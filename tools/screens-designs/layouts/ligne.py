"""Ligne de mesures separees par des puces (CPU / GPU : temperature + charge, RAM, SSD 1-2, extras), barre pleine largeur.
Les formats tres larges tiennent sur une ligne ; sinon les mesures se repartissent sur 2 ou 3 lignes."""
import math

from content import CONTENT
from lib import Canvas, measure
from layouts.r2_common import SETS_B, counts, first_ok

NAME, TITLE = "ligne", "Ligne unique"
CONTENTS = SETS_B
FORMATS = ["w1280", "u1920", "rb640", "w169"]


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    f = p.font

    def mk(n, L):
        def run():
            per = math.ceil(n / L)
            lines = [ms[k * per:(k + 1) * per] for k in range(L) if ms[k * per:(k + 1) * per]]
            lh = h * 0.84 / len(lines)

            def width(line, vs):
                ls, ss, gap = vs * 0.6, vs * 0.7, vs * 0.9
                t = gap * (len(line) - 1)
                for m in line:
                    t += vs * 0.5 + measure(m.label, f, True, ls) + ls * 0.5 + measure(m.sample, f, True, vs)
                    if m.sub:
                        t += ss * 0.4 + measure(m.sub[2], f, False, ss)
                return t

            vs = lh * 0.72
            while max(width(l, vs) for l in lines) > w and vs > 8:
                vs *= 0.97
            ls, ss, gap = vs * 0.6, vs * 0.7, vs * 0.9
            c = Canvas(p, fmt)
            k = 0
            for li, line in enumerate(lines):
                cx = x + (w - width(line, vs)) / 2
                cy = y + li * lh + (lh - vs * 1.15) / 2
                for m in line:
                    with c.m(m):
                        c.rect(cx, cy + vs * 0.575 - vs * 0.16, vs * 0.32, vs * 0.32, p.acc[k % 3], vs * 0.16, pre="dot")
                        cx += vs * 0.5
                        lw = measure(m.label, f, True, ls)
                        c.text(cx, cy + vs * 0.1, lw, vs * 1.05, m.label, p.text2, bold=True, size=ls, pre="l")
                        cx += lw + ls * 0.5
                        vw = measure(m.sample, f, True, vs)
                        c.value(m.src, m.fmt, cx, cy, vw, vs * 1.15, m.sample, p.text, bold=True, size=vs, pre="v")
                        cx += vw
                        if m.sub:
                            cx += ss * 0.4
                            sw = measure(m.sub[2], f, False, ss)
                            c.value(m.sub[0], m.sub[1], cx, cy + vs * 0.1, sw, ss * 1.15, m.sub[2], p.text2, size=ss, pre="s")
                            cx += sw
                        cx += gap
                    k += 1
            by = y + h - max(6, h * 0.09)
            c.bar(x, by, w, max(6, h * 0.09), ms[0].src, ms[0].lo, ms[0].hi, ms[0].colors(p), pre="b")
            return c.finish()
        return run

    return first_ok(fmt, [mk(n, L) for n in counts(ms, (9, 8, 7, 5)) for L in (1, 2, 3)])
