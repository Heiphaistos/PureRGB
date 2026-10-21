"""Briques composees reutilisables par les archetypes : carte de stat, anneau, lueur, coins HUD."""
from lib import inset


def stat_card(c, box, m, i=0, sub=True, stripe=True):
    """Carte translucide : libelle, grande valeur, valeur secondaire, barre. S'adapte a la forme de la boite."""
    x, y, w, h = box
    p = c.p
    c.card(x, y, w, h)
    pad = min(w, h) * 0.1
    ix, iy, iw, ih = inset(box, pad)
    if stripe and w >= 40:
        c.rect(x, y + h * 0.22, 3, h * 0.56, p.acc[i % 3], 1.5, pre="st")
    has_sub = sub and m.sub is not None
    if w > 1.9 * h:   # carte large et basse : libelle a gauche, valeur a droite
        lw = iw * 0.38
        c.text(ix, iy, lw, ih * 0.5, m.label, p.text2, bold=True, pre="l", group="sl", font=p.font)
        if has_sub:
            c.value(m.sub[0], m.sub[1], ix, iy + ih * 0.5, lw, ih * 0.4, m.sub[2], p.text2, pre="s", group="ss")
        c.value(m.src, m.fmt, ix + lw + pad * 0.5, iy, iw - lw - pad * 0.5, ih * 0.72, m.sample, p.text, bold=True, align="right", pre="v", group="sv", font=p.font)
        c.bar(ix + lw + pad * 0.5, iy + ih * 0.84, iw - lw - pad * 0.5, ih * 0.16, m.src, m.lo, m.hi, m.colors(p, i), pre="b")
        return
    hl, hb = ih * 0.17, max(4.0, ih * 0.08)
    c.text(ix, iy, iw, hl, m.label, p.text2, bold=True, pre="l", group="sl", font=p.font, maxsize=hl / LINE)
    top, bot = iy + hl, iy + ih - hb
    hs = ih * 0.17 if has_sub else 0
    hv = (bot - top - hs) * 0.92
    c.value(m.src, m.fmt, ix, top + (bot - top - hs - hv) / 2, iw, hv, m.sample, p.text, bold=True, pre="v", group="sv", font=p.font)
    if has_sub:
        c.value(m.sub[0], m.sub[1], ix, bot - hs, iw, hs * 0.9, m.sub[2], p.text2, pre="s", group="ss", maxsize=hs * 0.8)
    c.bar(ix, iy + ih - hb, iw, hb, m.src, m.lo, m.hi, m.colors(p, i), pre="b")


LINE = 1.15


def ring(c, cx, cy, r, m, i=0, thick=None, label=True, sub=False, vscale=1.0):
    """Jauge 270 deg ouverte vers le bas : valeur au centre, libelle dans l'ouverture."""
    p = c.p
    t = thick or max(5.0, r * 0.17)
    c.arc(cx, cy, r, m.src, m.lo, m.hi, t, m.colors(p, i))
    inner = r - t
    vw, vh = inner * 1.45, inner * 0.72 * vscale
    c.value(m.src, m.fmt, cx - vw / 2, cy - vh * 0.62, vw, vh, m.sample, p.text, bold=True, align="center", pre="v", group="rv", font=p.font)
    if label:
        lh = inner * 0.3
        c.text(cx - r * 0.5, cy + r * 0.58, r, lh, m.label, p.text2, bold=True, align="center", pre="l", group="rl", font=p.font, maxsize=lh)
    if sub and m.sub:
        sh = inner * 0.26
        c.value(m.sub[0], m.sub[1], cx - inner * 0.7, cy + inner * 0.12, inner * 1.4, sh, m.sub[2], p.text2, align="center", pre="s", group="rs")


def hero_list(zone, d, gap):
    """Anneau de diametre `d` + liste a droite, le tout centre : retourne (x de l'anneau, x de la liste, largeur de la liste)."""
    x, _, w, _ = zone
    lw = min(w - d - 2 * gap, d * 2.6)
    x0 = x + (w - (d + 2 * gap + lw)) / 2
    return x0, x0 + d + 2 * gap, lw


def glow(c, cx, cy, r, color, steps=4):
    """Lueur douce : cercles concentriques tres transparents (reste dans le canevas)."""
    r = min(r, c.f.w / 2, c.f.h / 2)
    cx, cy = min(max(cx, r), c.f.w - r), min(max(cy, r), c.f.h - r)
    for k in range(steps):
        rr = r * (1 - k / steps)
        c.rect(cx - rr, cy - rr, 2 * rr, 2 * rr, color, rr, opacity=0.07 + 0.03 * k, pre="gl")


def corners(c, box, color, size, thick=2):
    """Quatre equerres de HUD aux coins de `box`."""
    x, y, w, h = box
    for sx, sy, ox, oy in [(0, 0, 0, 0), (1, 0, -size, 0), (0, 1, 0, -size), (1, 1, -size, -size)]:
        px, py = x + sx * w, y + sy * h
        c.rect(px + (ox if sx else 0), py + (0 if not sy else -thick), size, thick, color, 0, pre="cn")
        c.rect(px + (-thick if sx else 0), py + (oy if sy else 0), thick, size, color, 0, pre="cn")


def row(c, box, m, i=0, bar=True):
    """Ligne de stat : libelle a gauche, valeur a droite, fine barre dessous."""
    x, y, w, h = box
    p = c.p
    th = h * (0.78 if bar else 1.0)
    c.text(x, y, w * 0.45, th, m.label, p.text2, bold=True, pre="l", group="wl", font=p.font, maxsize=th * 0.8)
    c.value(m.src, m.fmt, x + w * 0.45, y, w * 0.55, th, m.sample, p.text, bold=True, align="right", pre="v", group="wv", font=p.font, maxsize=th)
    if bar:
        c.bar(x, y + h * 0.86, w, max(3.0, h * 0.1), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def mini_col(c, box, m, i=0):
    """Petite colonne centree : libelle colore au-dessus, valeur dessous."""
    x, y, w, h = box
    p = c.p
    c.text(x, y, w, h * 0.34, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="ml", font=p.font, maxsize=h * 0.3)
    c.value(m.src, m.fmt, x, y + h * 0.4, w, h * 0.6, m.sample, p.text, bold=True, align="center", pre="v", group="mv", font=p.font)
