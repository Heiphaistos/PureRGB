"""Briques communes du lot uw : colonne compacte et ligne a trois valeurs (libelle, valeur, valeur secondaire)."""


def col(c, box, m, i=0, g="uc", bar=True):
    """Colonne : libelle colore, grande valeur, valeur secondaire (charge...), fine barre. Masquee avec la metrique."""
    x, y, w, h = box
    p = c.p
    with c.m(m):
        sub = m.sub is not None
        hv = 0.38 if sub else 0.56
        c.text(x, y, w, h * 0.24, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group=g + "l", font=p.font)
        c.value(m.src, m.fmt, x, y + h * 0.25, w, h * hv, m.sample, p.text, bold=True, align="center", pre="v", group=g + "v", font=p.font)
        if sub:
            c.value(m.sub[0], m.sub[1], x, y + h * (0.26 + hv), w, h * 0.24, m.sub[2], p.text2, align="center", pre="s", group=g + "s")
        if bar:
            c.bar(x + w * 0.1, y + h * 0.95, w * 0.8, max(3.0, h * 0.05), m.src, m.lo, m.hi, m.colors(p, i), pre="b")


def line(c, box, m, i=0, g="ul", sub=True):
    """Ligne : libelle a gauche, valeur puis valeur secondaire a droite."""
    x, y, w, h = box
    p = c.p
    with c.m(m):
        c.text(x, y, w * 0.3, h, m.label, p.acc[i % 3], bold=True, pre="l", group=g + "l", font=p.font)
        has = sub and m.sub is not None
        vw = w * (0.38 if has else 0.7)
        c.value(m.src, m.fmt, x + w * 0.3, y, vw, h, m.sample, p.text, bold=True, align="right", pre="v", group=g + "v", font=p.font)
        if has:
            c.value(m.sub[0], m.sub[1], x + w * 0.68, y, w * 0.32, h, m.sub[2], p.text2, align="right", pre="s", group=g + "s")


def ring2(c, cx, cy, r, m, i=0, thick=None, g="ur"):
    """Jauge 270 deg : valeur au centre, valeur secondaire dessous, libelle (large) dans l'ouverture du bas."""
    p = c.p
    t = thick or max(5.0, r * 0.12)
    inner = r - t
    with c.m(m):
        c.arc(cx, cy, r, m.src, m.lo, m.hi, t, m.colors(p, i))
        c.value(m.src, m.fmt, cx - inner * 0.75, cy - inner * 0.72, inner * 1.5, inner * 0.66, m.sample, p.text, bold=True, align="center", pre="v", group=g + "v", font=p.font)
        if m.sub:
            c.value(m.sub[0], m.sub[1], cx - inner * 0.7, cy - inner * 0.04, inner * 1.4, inner * 0.38, m.sub[2], p.text2, align="center", pre="s", group=g + "s")
        c.text(cx - r * 0.75, cy + r * 0.58, r * 1.5, inner * 0.38, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group=g + "l", font=p.font)
