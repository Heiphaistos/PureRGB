"""Bandeau ultra-large multi-colonnes : libellé, grande valeur, barre ; 1 ou 2 rangées."""
from content import CONTENT
from lib import Canvas, min_text
from layouts.cap_common import densest

NAME, TITLE = "cap-bandeau", "Bandeau multi-colonnes"
CONTENTS = ["cap-complet", "cap-systeme", "cap-thermique"]
FORMATS = ["rb640", "w169", "w1280", "u1920"]


def build(p, fmt, content):
    ms = CONTENT[content][1]
    mt = min_text(fmt)

    def mk(n):
        c = Canvas(p, fmt)
        z = fmt.zone
        rows = 1 if fmt.ratio >= 3 else (2 if n <= 8 else 3)
        cols = -(-n // rows)
        gx, gy = fmt.short * 0.05, fmt.short * 0.06
        cw, ch = (z[2] - gx * (cols - 1)) / cols, (z[3] - gy * (rows - 1)) / rows
        for i in range(n):
            r, col = divmod(i, cols)
            x, y, m = z[0] + col * (cw + gx), z[1] + r * (ch + gy), ms[i]
            if col:
                c.rect(x - gx / 2 - 1, y + ch * 0.05, 2, ch * 0.9, p.line, 1, pre="sep")
            lh = max(1.3 * mt, ch * 0.2)
            c.text(x, y, cw, lh, m.label, p.acc[i % 3], bold=True, pre="l", group="bl")
            bh = max(4.0, ch * 0.08)
            c.value(m.src, m.fmt, x, y + lh * 1.1, cw, ch - lh * 1.1 - bh * 2.2, m.sample, p.text, bold=True, pre="v", group="bv")
            c.bar(x, y + ch - bh, cw, bh, m.src, m.lo, m.hi, m.colors(p, i), pre="b")
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 20), 3, -1))
