"""Minimal typographique : un chiffre geant (CPU, charge en dessous), des lignes de texte (BASE + extras), aucun cadre."""
from content import CONTENT
from lib import Canvas
from layouts.r2_common import SETS_C, counts, first_ok, rows

NAME, TITLE = "typo", "Minimal typographique"
CONTENTS = SETS_C[::-1]


def build(p, fmt, content):
    ms = CONTENT[content][1]
    hero = ms[0]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.02

    def hero_block(c, bx, by, bw, bh):
        """Libelle, grand chiffre, puis « charge » + valeur secondaire."""
        c.text(bx, by, bw, bh * 0.13, hero.label, p.acc[0], bold=True, pre="l")
        c.value(hero.src, hero.fmt, bx, by + bh * 0.15, bw, bh * 0.58, hero.sample, p.text, bold=True, pre="v")
        c.text(bx, by + bh * 0.77, bw * 0.4, bh * 0.2, "charge", p.text2, pre="hs", group="hc")
        c.value(hero.sub[0], hero.sub[1], bx + bw * 0.42, by + bh * 0.77, bw * 0.4, bh * 0.2, hero.sub[2], p.text, bold=True, pre="hv", group="hc")

    def mk(n, cols, frac):
        def f():
            c = Canvas(p, fmt)
            items = [(ms[i], i) for i in range(1, n)]
            if fmt.ratio >= 1.7:
                lw = w * frac
                hero_block(c, x, y, lw, h)
                c.rect(x + lw + w * 0.02, y + h * 0.08, 3, h * 0.84, p.acc[0], 1.5, pre="rule")
                rows(c, (x + lw + w * 0.05, y + h * 0.04, w - lw - w * 0.05, h * 0.92), items, cols, gap, bar=False, lab_bold=False)
            else:
                hh = h * frac
                hero_block(c, x, y, w, hh)
                c.rect(x, y + hh + gap, w * 0.25, 3, p.acc[0], 1.5, pre="rule")
                top = y + hh + gap * 3
                rows(c, (x, top, w, y + h - top), items, cols, gap, bar=False, lab_bold=False)
            return c.finish()
        return f

    fr = (0.42, 0.35) if fmt.ratio >= 1.7 else (0.4, 0.34, 0.28)
    return first_ok(fmt, [mk(n, cols, f) for n in counts(ms, (9, 8, 7, 5)) for f in fr for cols in (1, 2)])
