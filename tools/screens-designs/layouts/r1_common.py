"""Aides communes du lot r1 : BASE/extras, heros, lignes compactes (libelle, valeur, valeur secondaire, barre)."""
import itertools

import qa
from content import BASE
from lib import measure

CONTENTS_ALL = ["cpu-gpu", "quad", "disques", "reseau", "refroidissement", "carte-mere", "gpu", "cpu", "puissance", "liquide", "ram"]
HERO_IDX = {"cpu-gpu": 0, "cpu": 0, "gpu": 1, "ram": 2, "refroidissement": 6}   # defaut : 1er extra (ms[5])
CAP = 10   # nombre maximal d'elements affiches


def hero_split(content, ms):
    """(heros, reste) : le heros depend du contenu (ms[0:5] = BASE, ms[5:] = extras)."""
    i = HERO_IDX.get(content, 5)
    h = ms[i] if i < len(ms) else ms[0]
    return h, [m for m in ms if m is not h]


def pick(ms, n):
    """n elements : tout BASE d'abord, puis les extras ; les metriques conditionnelles (SSD 2) en dernier
    pour que leur absence ne laisse aucun trou au milieu."""
    must = [m for m in ms if m in BASE]
    out = must + [m for m in ms if m not in BASE][:max(0, n - len(must))]
    return [m for m in out if not m.cond] + [m for m in out if m.cond]


def sizes(ms):
    """Nombres d'elements essayes, du plus dense au minimum (BASE complet)."""
    lo = sum(m in BASE for m in ms)
    return list(range(min(len(ms), CAP), lo - 1, -1))


def fit(mk, fmt, *axes):
    """Premier resultat sans defaut QA du produit cartesien des axes (ordre = priorite) ; None sinon."""
    for a in itertools.product(*axes):
        w = mk(*a)
        if w and not qa.problems(w, fmt):
            return w
    return None


def fracs(p, items):
    """Fractions de largeur (libelle, valeur, secondaire) communes a toutes les lignes."""
    f = p.font
    lab = max(measure(m.label, f, True, 100) for m in items)
    val = max(measure(m.sample, f, True, 100) for m in items)
    sub = max((measure(m.sub[2], f, False, 100) * 0.8 for m in items if m.sub), default=0)
    val, sub = val * 1.12, sub * 1.2   # aere : la valeur et sa secondaire ne se touchent pas
    t = lab + val + sub
    return lab / t, val / t, sub / t


def srow(c, box, m, i, fr, bar=True, lcolor=None):
    """Ligne : libelle | valeur | valeur secondaire, fine barre dessous. Cree dans `c.m(m)` (SSD 2 conditionnel)."""
    x, y, w, h = box
    p = c.p
    th = h * (0.8 if bar else 1.0)
    fl, fv, fs = fr
    with c.m(m):
        c.text(x, y, w * fl, th, m.label, lcolor or p.text2, bold=True, pre="l", group="rl", font=p.font, maxsize=th * 0.8)
        c.value(m.src, m.fmt, x + w * fl, y, w * fv, th, m.sample, p.text, bold=True, align="right", pre="v", group="rv", font=p.font, maxsize=th * 0.9)
        if m.sub:
            c.value(m.sub[0], m.sub[1], x + w * (fl + fv), y, w * fs, th, m.sub[2], p.text2, align="right", pre="s", group="rs", maxsize=th * 0.75)
        if bar:
            c.bar(x, y + h * 0.88, w, max(3.0, h * 0.09), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def block(c, box, items, cols, gap, bar=True, colored=0, maxrow=None):
    """Lignes `srow` en `cols` colonnes (remplissage par colonne). `colored` : nb de premieres lignes au libelle colore."""
    x, y, w, h = box
    rows = -(-len(items) // cols)
    cw = (w - gap * (cols - 1)) / cols
    rh = min((h - gap * 0.5 * (rows - 1)) / rows, maxrow or 1e9)
    top = y + (h - rh * rows - gap * 0.5 * (rows - 1)) / 2
    fr = fracs(c.p, items)
    for k, m in enumerate(items):
        col, r = divmod(k, rows)
        srow(c, (x + col * (cw + gap), top + r * (rh + gap * 0.5), cw, rh), m, k, fr, bar, c.p.acc[k % 3] if k < colored else None)


def colopts(fmt):
    """Nombres de colonnes de liste essayes : les ecrans tres larges en veulent plus."""
    return (3, 4, 2) if fmt.ratio >= 3 else (2, 1, 3) if fmt.ratio >= 2.2 else (1, 2)


def side_by_side(fmt):
    """Mise en page : visuel a cote de la liste (ratio >= 1,5) ou visuel au-dessus."""
    return fmt.ratio >= 1.5
