"""Briques communes aux archetypes clk-* (pas de build : ignore par le generateur).

Regle produit : BASE (CPU temp+charge, GPU temp+charge, RAM, SSD 1, SSD 2 masque si absent) = ms[:5] est toujours
affiche, puis autant d'extras (ms[5:]) que le QA en accepte (`attempt`)."""
import qa
from content import CONTENT
from lib import LINE_H, Canvas, grid, measure, split
from parts import stat_card

CONTENTS = ["clk-systeme", "clk-temps", "clk-reseau", "clk-etat", "clk-air", "clk-watts", "clk-disques", "clk-gpu",
            "clk-cpu", "clk-ventil"]
WEEK = ("lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche")
MONTHS = ("janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre")
GAP = 0.28   # espace entre mots de la date (en tailles de police)


def attempt(draw, p, fmt, content, kmax=4):
    """Essaie `draw(c, ms, mode)` avec le plus d'extras possible (kmax..0) puis les modes de panneau ;
    renvoie le premier resultat sans defaut QA (ou None). `draw` renvoie False si le cas n'a pas de sens."""
    ms = CONTENT[content][1]
    modes = ("cards", "rows", "strip") if fmt.wide else ("rows", "cards", "strip")
    for k in range(min(kmax, len(ms) - 5), -1, -1):
        for mode in modes:
            c = Canvas(p, fmt)
            if draw(c, ms[:5 + k], mode) is False:
                continue
            w = c.finish()
            if not qa.problems(w, fmt):
                return w
    return None


# -- panneau de metriques -----------------------------------------------------
def line(c, box, m, i=0, bar=True):
    """Ligne : libelle, valeur, valeur secondaire (charge/temperature), fine barre."""
    x, y, w, h = box
    p = c.p
    th = h * (0.84 if bar else 1.0)
    sub = m.sub if m.sub and len(m.sub[2]) <= 8 else None
    lw, sw = w * 0.34, (w * 0.26 if sub else 0)
    with c.m(m):
        c.text(x, y, lw, th, m.label, p.acc[i % 3], bold=True, pre="l", group="rl", maxsize=th * 0.85)
        c.value(m.src, m.fmt, x + lw, y, w - lw - sw, th, m.sample, p.text, bold=True, align="right", pre="v", group="rv", maxsize=th * 0.9)
        if sub:
            c.value(sub[0], sub[1], x + w - sw, y, sw, th, sub[2], p.text2, align="right", pre="s", group="rs", maxsize=th * 0.8)
        if bar:
            c.bar(x, y + h * 0.9, w, max(3.0, h * 0.08), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def col(c, box, m, i=0, bar=True, card=False):
    """Colonne centree : libelle colore, valeur, valeur secondaire, barre."""
    x, y, w, h = box
    p = c.p
    if card:
        c.card(x, y, w, h)
        x, y, w, h = x + w * 0.06, y + h * 0.05, w * 0.88, h * 0.9
    sub = m.sub if m.sub and len(m.sub[2]) <= 8 else None
    hb = max(3.0, h * 0.07)
    with c.m(m):
        c.text(x, y, w, h * 0.2, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="cl", maxsize=h * 0.18)
        c.value(m.src, m.fmt, x, y + h * 0.24, w, h * 0.34, m.sample, p.text, bold=True, align="center", pre="v", group="cv")
        if sub:
            c.value(sub[0], sub[1], x, y + h * 0.6, w, h * 0.2, sub[2], p.text2, align="center", pre="s", group="cs", maxsize=h * 0.17)
        if bar:
            c.bar(x + w * 0.08, y + h - hb, w * 0.84, hb, m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def panel(c, box, ms, mode="cards", gap=None, bar=True, card=True, target=2.4):
    """Toutes les metriques `ms` dans `box` : cartes (grille), lignes (liste) ou colonnes (bande)."""
    g = min(box[2], box[3]) * 0.03 if gap is None else gap
    n = len(ms)
    if mode == "rows":
        for i, b in enumerate(split(box, n, g * 0.6, False)):
            line(c, b, ms[i], i, bar)
    elif mode == "strip":
        for i, b in enumerate(split(box, n, g, True)):
            col(c, b, ms[i], i, bar, card)
    else:
        for i, b in enumerate(grid(box, n, g, target)):
            with c.m(ms[i]):
                stat_card(c, b, ms[i], i)


# -- heure et date --------------------------------------------------------------
def _widest(p, items):
    return max(items, key=lambda s: measure(s, p.font, True, 100))


def date_line(c, box, color=None, align="center", short=False):
    """Date complete (jour, numero, mois, annee) en UN widget."""
    p = c.p
    wd, mo = _widest(p, WEEK), _widest(p, MONTHS)
    if short:
        c.value("date", "%a %d %b %Y", *box, "mer. 88 sept. 2026", color or p.text2, align=align, pre="date")
    else:
        c.value("date", "%A %d %B %Y", *box, f"{wd} 88 {mo} 2026", color or p.text2, align=align, pre="date")


def date_block(c, box, align="center", maxsize=None):
    """Date complete en QUATRE widgets separes (%A jour, %d numero, %B mois, %Y annee), sur une ou deux lignes
    selon la place ; taille commune calculee avec les vraies largeurs de police."""
    x, y, w, h = box
    p = c.p
    it = [("%A", _widest(p, WEEK), p.acc[0]), ("%d", "88", p.text), ("%B", _widest(p, MONTHS), p.acc[1]), ("%Y", "2026", p.text2)]

    def width(row):   # largeur a la taille 1
        return (sum(measure(s, p.font, True, 100) for _, s, _ in row) + 100 * GAP * (len(row) - 1)) / 100

    best = None
    for rows in ([it], [it[:2], it[2:]]):
        s = min((h / len(rows)) / LINE_H, *(w / width(r) for r in rows), maxsize or 1e9)
        if best is None or s > best[0] + 0.01:
            best = (round(s, 1), rows)
    s, rows = best
    rh = h / len(rows)
    for j, row in enumerate(rows):
        tw = width(row) * s
        x0 = x if align == "left" else x + w - tw if align == "right" else x + (w - tw) / 2
        for f, sample, color in row:
            iw = measure(sample, p.font, True, s)
            c.value("date", f, x0, y + j * rh + (rh - s * LINE_H) / 2, iw, s * LINE_H, sample, color, bold=True, align="center", size=s, pre="d")
            x0 += iw + GAP * s
