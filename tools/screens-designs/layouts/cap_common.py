"""Aides communes aux archétypes cap : métriques ordonnées (obligatoires d'abord) et densité maximale qui passe le QA."""
import math

import qa
from lib import measure, split
from content_ext_cap import GROUPS, flat  # noqa: F401  (GROUPS réexporté pour les archétypes)

ALL = list(GROUPS)   # tous les contenus cap
MIN_N = 7   # CPU temp+charge, GPU temp+charge, RAM, SSD 1, SSD 2 : jamais moins


def metrics(content):
    return flat(content)


def densest(mk, fmt, ns, floor=MIN_N):
    """Premier résultat propre en essayant n décroissant (jamais sous `floor`) ; sinon le dernier (le QA du générateur le rejettera)."""
    last = None
    for n in ns:
        if n < floor:
            break
        w = mk(n)
        if w is None:
            continue
        last = w
        if not qa.problems(w, fmt):
            return w
    return last


def table_dims(fmt, n, row_min, col_min, extra=0):
    """(colonnes, lignes) : moins de colonnes possible tout en gardant des lignes >= row_min."""
    z = fmt.zone
    for cols in range(1, 7):
        if z[2] / cols < col_min:
            break
        rows = math.ceil(n / cols) + extra
        if z[3] / rows >= row_min:
            return cols, rows
    return None


def fracs(p, ms, gap=0.06):
    """Part de la largeur de ligne réservée au libellé : proportionnelle à ce que libellés et valeurs mesurent vraiment."""
    lab = max(measure(m.label, p.font, True, 100) for m in ms)
    val = max(measure(m.sample, p.font, True, 100) for m in ms)
    return lab / (lab + val) * (1 - gap) + gap / 2


def crow(c, box, m, i, frac, bar=True, grp="c"):
    """Ligne libellé | valeur dont la largeur du libellé s'adapte au texte (cf. `fracs`), fine barre dessous."""
    x, y, w, h = box
    p = c.p
    th = h * (0.78 if bar else 1.0)
    c.text(x, y, w * frac, th, m.label, p.text2, bold=True, pre="l", group=grp + "l", maxsize=th * 0.8)
    c.value(m.src, m.fmt, x + w * frac, y, w * (1 - frac), th, m.sample, p.text, bold=True, align="right", pre="v", group=grp + "v", maxsize=th)
    if bar:
        c.bar(x, y + h * 0.86, w, max(3.0, h * 0.1), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def boxes(fmt, n, gap):
    """Boîtes des `n` familles : colonnes (écrans larges), 2x2 (4 familles) ou 3 bandes empilées sur les carrés.
    Retourne (boîtes, colonnes ?)."""
    z = fmt.zone
    if fmt.ratio >= 2.2 or (n != 4 and fmt.ratio >= 1.5):
        return split(z, n, gap), True
    if n == 4:
        return [b for r in split(z, 2, gap, False) for b in split(r, 2, gap)], False
    return split(z, n, gap, False), False
