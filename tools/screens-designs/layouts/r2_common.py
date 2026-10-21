"""Briques communes du lot r2 : essai du plus riche au plus sobre, lignes / mini-blocs avec valeur secondaire.

Regle produit : BASE (ms[0:5] = CPU temp+charge, GPU temp+charge, RAM, SSD 1, SSD 2) est toujours affichee,
les extras (ms[5:]) seulement si la place le permet (le QA tranche)."""
import math

import qa
from lib import grid, split
from parts import ring


def first_ok(fmt, makers):
    """Premier design sans probleme QA parmi `makers` (callables -> widgets), du plus riche au plus sobre.
    Aucun ne passe : renvoie le dernier (le generateur le rejette) ; liste vide : None."""
    w = None
    for mk in makers:
        w = mk()
        if w is not None and not qa.problems(w, fmt):
            return w
    return w


def counts(ms, wanted, minimum=5):
    """Nombres d'items a essayer (decroissant), plafonnes a la taille du contenu, jamais sous BASE."""
    out = sorted({min(n, len(ms)) for n in wanted if min(n, len(ms)) >= minimum}, reverse=True)
    return out or [minimum]


def g(k):
    """Groupe de taille : BASE (pourcentages, degres) separe des extras (unites longues)."""
    return "b" if k < 5 else "x"


def mrow(c, box, m, i=0, bar=True, sub=True, labw=0.3, subw=0.25, lab_color=None, lab_bold=True, k=None, subn=99):
    """Ligne : libelle | valeur (droite) | valeur secondaire (droite), fine barre dessous."""
    x, y, w, h = box
    p = c.p
    gg = g(i if k is None else k)
    th = h * (0.82 if bar else 1.0)
    has = bool(sub and m.sub and i < subn)   # subn : seules les `subn` premieres metriques montrent leur valeur secondaire
    with c.m(m):
        c.text(x, y, w * labw, th, m.label, lab_color or p.text2, bold=lab_bold, pre="l", group="ml" + gg, font=p.font, maxsize=th * 0.8)
        vw = w * (1 - labw - (subw if has else 0)) - 2
        c.value(m.src, m.fmt, x + w * labw, y, vw, th, m.sample, p.text, bold=True, align="right", pre="v", group="mv" + gg, font=p.font, maxsize=th * 0.9)
        if has:
            c.value(m.sub[0], m.sub[1], x + w * (1 - subw), y + th * 0.1, w * subw, th * 0.8, m.sub[2], p.text2, align="right", pre="s", group="ms" + gg, maxsize=th * 0.7)
        if bar:
            c.bar(x, y + h * 0.9, w, max(3.0, h * 0.09), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def rows(c, zone, items, cols, gap, **kw):
    """Range `items` [(metrique, indice)] en `cols` colonnes de lignes `mrow` (remplissage colonne par colonne)."""
    per = math.ceil(len(items) / cols)
    for ci, col in enumerate(split(zone, cols, gap * 1.5 + (zone[2] * 0.03 if cols > 1 else 0))):
        chunk = items[ci * per:(ci + 1) * per]
        if not chunk:
            continue
        # hauteur de ligne : celle d'une colonne pleine
        rh = (col[3] - gap * (per - 1)) / per
        for ri, (m, i) in enumerate(chunk):
            mrow(c, (col[0], col[1] + ri * (rh + gap), col[2], rh), m, i, k=i, **kw)


def rcards(c, zone, items, cols, gap, **kw):
    """Comme `rows`, mais chaque ligne est posee sur une carte translucide (liseré d'accent a gauche)."""
    per = math.ceil(len(items) / cols)
    for ci, col in enumerate(split(zone, cols, gap)):
        chunk = items[ci * per:(ci + 1) * per]
        rh = (col[3] - gap * (per - 1)) / per
        for ri, (m, i) in enumerate(chunk):
            bx, by, bw, bh = col[0], col[1] + ri * (rh + gap), col[2], rh
            pad = min(bw, bh) * 0.12
            with c.m(m):
                c.card(bx, by, bw, bh, radius=bh * 0.18)
                c.rect(bx, by + bh * 0.2, 3, bh * 0.6, c.p.acc[i % 3], 1.5, pre="st")
            mrow(c, (bx + pad * 1.4, by + pad, bw - pad * 2.4, bh - 2 * pad), m, i, k=i, **kw)


def mini_stat(c, box, m, i=0, sub=True, k=None, subn=99):
    """Bloc vertical centre : libelle colore, valeur, valeur secondaire dessous."""
    x, y, w, h = box
    p = c.p
    gg = g(i if k is None else k)
    has = bool(sub and m.sub and i < subn)
    hl = h * 0.27
    hs = h * 0.27 if has else 0
    with c.m(m):
        c.text(x, y, w, hl, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="nl" + gg, font=p.font, maxsize=hl * 0.9)
        c.value(m.src, m.fmt, x, y + hl, w, h - hl - hs, m.sample, p.text, bold=True, align="center", pre="v", group="nv" + gg, font=p.font)
        if has:
            c.value(m.sub[0], m.sub[1], x, y + h - hs, w, hs, m.sub[2], p.text2, align="center", pre="s", group="ns" + gg, maxsize=hs * 0.85)


def tile(c, box, m, i=0, sub=True, k=None, subn=99):
    """Mini-bloc dans une carte translucide."""
    x, y, w, h = box
    with c.m(m):
        c.card(x, y, w, h, radius=min(w, h) * 0.12)
    pad = min(w, h) * 0.06
    mini_stat(c, (x + pad, y + pad, w - 2 * pad, h - 2 * pad), m, i, sub, k, subn)


def tiles(c, zone, items, gap, target=1.3, **kw):
    for box, (m, i) in zip(grid(zone, len(items), gap, target), items):
        tile(c, box, m, i, k=i, **kw)


def bottom(c, zone, items, gap, kind):
    """Zone basse des designs « heros + liste » : `kind` = (style, colonnes, subn) avec style 't' tuiles verticales,
    'r' lignes sur cartes ; `subn` : nombre de metriques qui montrent leur valeur secondaire."""
    style, cols, subn = kind
    if style == "t":
        tiles(c, zone, items, gap, target=cols, subn=subn)
    else:
        rcards(c, zone, items, cols, gap, subn=subn)


BOTTOMS = [(s, c, n) for n in (99, 2) for s, c in (("t", 1.4), ("r", 2), ("r", 1), ("t", 2.2))]


def date_line(c, box, two=False, color=None, align="center", pre="date"):
    """Date complete : jour de la semaine, numero, mois, annee (une ligne, ou deux : jour / numero mois annee)."""
    x, y, w, h = box
    col = color or c.p.text2
    if not two:
        c.value("date", "%A %d %B %Y", x, y, w, h, "mercredi 30 septembre 2026", col, align=align, pre=pre)
        return
    c.value("date", "%A", x, y, w, h * 0.48, "mercredi", c.p.acc[0], bold=True, align=align, pre=pre + "j", group="dj")
    c.value("date", "%d %B %Y", x, y + h * 0.52, w, h * 0.48, "30 septembre 2026", col, align=align, pre=pre + "n", group="dn")


# jeux de contenu (cles de content.py) a parcourir : 8 jeux x 4 formats >= 30 designs par archetype avec --fpc 4
SETS = ["quad", "disques", "carte-mere", "reseau", "refroidissement", "gpu", "cpu", "puissance"]
SETS_B = ["puissance", "liquide", "cpu", "gpu", "refroidissement", "reseau", "carte-mere", "disques"]
SETS_C = ["disques", "quad", "ram", "puissance", "gpu", "carte-mere", "cpu", "liquide"]


def gauge(c, cx, cy, r, m, i=0, sub=False, thick=None, gs=""):
    """Jauge 270 deg. `sub` : la valeur secondaire (charge) se range dans l'ouverture a cote du libelle
    (libelle | valeur) au lieu d'ecraser la valeur centrale ; sinon libelle centre dans l'ouverture."""
    p = c.p
    with c.m(m):
        if not (sub and m.sub):
            ring(c, cx, cy, r, m, i, thick=thick, g=gs)
            return
        t = thick or max(5.0, r * 0.17)
        c.arc(cx, cy, r, m.src, m.lo, m.hi, t, m.colors(p, i))
        inner = r - t
        vw, vh = inner * 1.45, inner * 0.8
        c.value(m.src, m.fmt, cx - vw / 2, cy - vh * 0.6, vw, vh, m.sample, p.text, bold=True, align="center", pre="v", group="gv" + gs, font=p.font)
        ry, rh = cy + r * 0.66, r * 0.3
        c.text(cx - r * 0.62, ry, r * 0.58, rh, m.label, p.text2, bold=True, align="right", pre="l", group="gl" + gs, font=p.font, maxsize=rh * 0.9)
        c.value(m.sub[0], m.sub[1], cx + r * 0.04, ry, r * 0.58, rh, m.sub[2], p.text, bold=True, align="left", pre="s", group="gs" + gs, maxsize=rh * 0.9)
