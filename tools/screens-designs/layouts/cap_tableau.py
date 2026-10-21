"""Tableau façon moniteur matériel : en-tête, lignes zébrées, capteur | barre | secondaire | valeur."""
from lib import Canvas, min_text
from layouts.cap_common import ALL, densest, metrics, table_dims

NAME, TITLE = "cap-tableau", "Tableau moniteur"
CONTENTS = ALL
FORMATS = ["s480", "r43", "w169", "w1280", "u1920", "rond"]


def build(p, fmt, content):
    ms = metrics(content)
    mt = min_text(fmt)

    def mk(n):
        d = table_dims(fmt, n, 1.8 * mt, 14 * mt, extra=1)
        if not d:
            return None
        cols, rows = d
        c = Canvas(p, fmt)
        z, gx = fmt.zone, fmt.short * 0.05
        cw = (z[2] - gx * (cols - 1)) / cols
        rh = min(z[3] / rows, fmt.short * 0.12)
        top = z[1] + (z[3] - rh * rows) / 2
        sub = cw >= 24 * mt
        for col in range(cols):
            x = z[0] + col * (cw + gx)
            c.text(x + rh * 0.2, top, cw * 0.4, rh * 0.8, "CAPTEUR", p.text2, bold=True, pre="th", group="th")
            c.text(x + cw * 0.6, top, cw * 0.4 - rh * 0.2, rh * 0.8, "VALEUR", p.text2, bold=True, align="right", pre="th", group="th")
            c.rect(x, top + rh * 0.92, cw, 1.5, p.line, 0, pre="ln")
            for r in range(rows - 1):
                i = col * (rows - 1) + r
                if i >= n:
                    break
                m, y, pad = ms[i], top + (r + 1) * rh, rh * 0.2
                with c.m(m):
                    if r % 2 == 0:
                        c.rect(x, y, cw, rh, p.card, rh * 0.2, pre="z")
                    c.text(x + pad, y, cw * 0.34, rh, m.label, p.acc[i % 3], bold=True, pre="l", group="tl", maxsize=rh * 0.5)
                    bw = cw * (0.2 if sub else 0.34)
                    c.bar(x + cw * 0.38, y + rh * 0.39, bw, rh * 0.22, m.src, m.lo, m.hi, m.colors(p, i), pre="b")
                    if sub and m.sub:
                        c.value(m.sub[0], m.sub[1], x + cw * 0.6, y, cw * 0.14, rh, m.sub[2], p.text2, align="right", pre="s", group="ts", maxsize=rh * 0.42)
                    c.value(m.src, m.fmt, x + cw * 0.76, y, cw * 0.24 - pad, rh, m.sample, p.text, bold=True, align="right", pre="v", group="tv", maxsize=rh * 0.5)
        return c.finish()
    return densest(mk, fmt, range(len(ms), 6, -1))
