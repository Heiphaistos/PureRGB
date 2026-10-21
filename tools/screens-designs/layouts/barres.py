"""Barres horizontales : libelle, grosse barre arrondie, valeur ; deux colonnes sur les ecrans larges."""
from content import CONTENT
from lib import Canvas

NAME, TITLE = "barres", "Barres horizontales"
CONTENTS = ["quad", "disques", "refroidissement", "reseau"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:6]
    z = fmt.zone
    cols = 1 if fmt.ratio < 2.6 else 2
    n = min(len(ms), 5 if cols == 1 else 6)
    rows = -(-n // cols)
    gap = fmt.short * 0.03
    gx = fmt.short * (0.03 if cols == 1 else 0.14)
    cw = (z[2] - gx * (cols - 1)) / cols
    rh = min((z[3] - gap * (rows - 1)) / rows, fmt.short * 0.17)
    top = z[1] + (z[3] - rows * rh - gap * (rows - 1)) / 2
    for i, m in enumerate(ms[:n]):
        col, r = divmod(i, rows)
        x, y, w, h = z[0] + col * (cw + gx), top + r * (rh + gap), cw, rh
        lw, vw = w * 0.27, w * 0.3
        c.text(x, y, lw, h, m.label, p.acc[i % 3], bold=True, pre="l", group="bl", font=p.font, maxsize=h * 0.62)
        bh = min(h * 0.42, fmt.short * 0.06)
        c.bar(x + lw + 4, y + (h - bh) / 2, w - lw - vw - 8, bh, m.src, m.lo, m.hi, m.colors(p, i), radius=bh * 0.35, pre="b")
        c.value(m.src, m.fmt, x + w - vw, y, vw, h, m.sample, p.text, bold=True, align="right", pre="v", group="bv", font=p.font, maxsize=h * 0.7)
    return c.finish()
