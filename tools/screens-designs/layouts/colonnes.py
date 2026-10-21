"""Colonnes separees (ecrans larges) ou lignes separees (carres) : sans cartes, filets fins."""
from content import CONTENT
from lib import Canvas, split

NAME, TITLE = "colonnes", "Colonnes séparées"
CONTENTS = ["quad", "carte-mere", "refroidissement", "disques"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    z = fmt.zone
    if fmt.ratio >= 1.7:
        n = 4
        for i, (x, y, w, h) in enumerate(split(z, n, fmt.short * 0.06)):
            m = ms[i]
            if i:
                c.rect(x - fmt.short * 0.03 - 1, y + h * 0.08, 2, h * 0.84, p.line, 1, pre="sep")
            c.text(x, y, w, h * 0.16, m.label, p.acc[i % 3], bold=True, pre="l", group="cl")
            c.value(m.src, m.fmt, x, y + h * 0.2, w, h * 0.42, m.sample, p.text, bold=True, pre="v", group="cv")
            if m.sub:
                c.value(m.sub[0], m.sub[1], x, y + h * 0.64, w, h * 0.14, m.sub[2], p.text2, pre="s", group="cs")
            c.bar(x, y + h * 0.88, w, max(5, h * 0.06), m.src, m.lo, m.hi, m.colors(p, i), pre="b")
        return c.finish()
    n = 3
    for i, (x, y, w, h) in enumerate(split(z, n, fmt.short * 0.04, horizontal=False)):
        m = ms[i]
        if i:
            c.rect(x, y - fmt.short * 0.02 - 1, w, 2, p.line, 1, pre="sep")
        c.text(x, y, w * 0.4, h * 0.45, m.label, p.acc[i % 3], bold=True, pre="l", group="cl", maxsize=h * 0.3)
        c.value(m.src, m.fmt, x + w * 0.45, y, w * 0.55, h * 0.7, m.sample, p.text, bold=True, align="right", pre="v", group="cv")
        c.bar(x, y + h * 0.82, w, max(4, h * 0.08), m.src, m.lo, m.hi, m.colors(p, i), pre="b")
    return c.finish()
