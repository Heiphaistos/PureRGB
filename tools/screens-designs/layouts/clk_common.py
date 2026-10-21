"""Briques communes aux archetypes clk-* (pas de build : ignore par le generateur)."""
from lib import split

DATE_L, DATE_LS = "%A %d %B", "mercredi 30 septembre"


def cell(c, box, m, i=0, bar=True, card=False, g="cl"):
    """Libelle colore, valeur, fine barre, le tout centre."""
    x, y, w, h = box
    p = c.p
    if card:
        c.card(x, y, w, h)
        x, y, w, h = x + w * 0.06, y + h * 0.06, w * 0.88, h * 0.88
    hb = max(3.0, h * 0.1)
    c.text(x, y, w, h * 0.3, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group=g + "l", maxsize=h * 0.26)
    c.value(m.src, m.fmt, x, y + h * 0.32, w, h * (0.5 if bar else 0.66), m.sample, p.text, bold=True, align="center", pre="v", group=g + "v")
    if bar:
        c.bar(x + w * 0.1, y + h - hb, w * 0.8, hb, m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def cells(c, box, ms, horizontal=True, gap=0, **kw):
    for i, b in enumerate(split(box, len(ms), gap, horizontal)):
        cell(c, b, ms[i], i, **kw)


def date_line(c, box, color=None, align="center", short=False):
    c.value("date", "%a %d %b" if short else DATE_L, *box, "mer. 30 sept." if short else DATE_LS, color or c.p.text2, align=align, pre="date")
