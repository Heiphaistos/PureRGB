"""Minimal typographique : un chiffre geant, quelques lignes de texte, aucun cadre."""
from content import CONTENT
from lib import Canvas, split

NAME, TITLE = "typo", "Minimal typographique"
CONTENTS = ["cpu-gpu", "gpu", "carte-mere", "cpu"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    hero, rest = ms[0], ms[1:4]
    x, y, w, h = fmt.zone
    if fmt.wide:
        lw = w * 0.5
        c.text(x, y, lw, h * 0.14, hero.label, p.acc[0], bold=True, pre="l")
        c.value(hero.src, hero.fmt, x, y + h * 0.16, lw, h * 0.84, hero.sample, p.text, bold=True, pre="v")
        c.rect(x + lw + w * 0.03, y + h * 0.1, 3, h * 0.8, p.acc[0], 1.5, pre="rule")
        for i, (bx, by, bw, bh) in enumerate(split((x + lw + w * 0.07, y + h * 0.1, w - lw - w * 0.07, h * 0.8), len(rest), h * 0.04, horizontal=False)):
            m = rest[i]
            c.text(bx, by, bw * 0.5, bh, m.label, p.text2, pre="l", group="yl", maxsize=bh * 0.55)
            c.value(m.src, m.fmt, bx + bw * 0.5, by, bw * 0.5, bh, m.sample, p.text, bold=True, align="right", pre="v", group="yv", maxsize=bh * 0.7)
        return c.finish()
    c.text(x, y, w, h * 0.09, hero.label, p.acc[0], bold=True, pre="l")
    c.value(hero.src, hero.fmt, x, y + h * 0.1, w, h * 0.42, hero.sample, p.text, bold=True, pre="v")
    c.rect(x, y + h * 0.55, w * 0.25, 3, p.acc[0], 1.5, pre="rule")
    for i, (bx, by, bw, bh) in enumerate(split((x, y + h * 0.62, w, h * 0.38), len(rest), h * 0.02, horizontal=False)):
        m = rest[i]
        c.text(bx, by, bw * 0.5, bh, m.label, p.text2, pre="l", group="yl", maxsize=bh * 0.7)
        c.value(m.src, m.fmt, bx + bw * 0.5, by, bw * 0.5, bh, m.sample, p.text, bold=True, align="right", pre="v", group="yv", maxsize=bh * 0.8)
    return c.finish()
