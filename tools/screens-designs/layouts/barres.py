"""Barres horizontales : libelle, grosse barre arrondie, valeur et valeur secondaire ; colonnes selon le format."""
from content import CONTENT
from lib import Canvas, measure
from layouts.r1_common import CONTENTS_ALL, fit, pick, sizes

NAME, TITLE = "barres", "Barres horizontales"
CONTENTS = CONTENTS_ALL


def build(p, fmt, content):
    ms = CONTENT[content][1]
    z = fmt.zone

    def mk(n, cols):
        items = pick(ms, n)
        rows = -(-n // cols)
        gap = fmt.short * 0.025
        gx = fmt.short * 0.05
        cw = (z[2] - gx * (cols - 1)) / cols
        rh = min((z[3] - gap * (rows - 1)) / rows, fmt.short * 0.17)
        s = rh * 0.6
        f = p.font
        lw = max(measure(m.label, f, True, s) for m in items) + 4
        vw = max(measure(m.sample, f, True, s) for m in items) + 4
        sw = max((measure(m.sub[2], f, False, s * 0.8) for m in items if m.sub), default=0) + 4
        bw = cw - lw - vw - sw - 12
        if bw < cw * 0.2:
            return None
        c = Canvas(p, fmt)
        top = z[1] + (z[3] - rows * rh - gap * (rows - 1)) / 2
        for k, m in enumerate(items):
            col, r = divmod(k, rows)
            x, y = z[0] + col * (cw + gx), top + r * (rh + gap)
            bh = min(rh * 0.42, fmt.short * 0.06)
            with c.m(m):
                c.text(x, y, lw, rh, m.label, p.acc[k % 3], bold=True, pre="l", group="bl", font=f, maxsize=s)
                c.bar(x + lw + 4, y + (rh - bh) / 2, bw, bh, m.src, m.lo, m.hi, m.colors(p, k), radius=bh * 0.35, pre="b")
                c.value(m.src, m.fmt, x + cw - vw - sw, y, vw, rh, m.sample, p.text, bold=True, align="right", pre="v", group="bv", font=f, maxsize=s)
                if m.sub:
                    c.value(m.sub[0], m.sub[1], x + cw - sw, y, sw, rh, m.sub[2], p.text2, align="right", pre="s", group="bs", maxsize=s * 0.8)
        return c.finish()
    return fit(mk, fmt, sizes(ms), (1, 2, 3))
