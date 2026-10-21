"""Briques decoratives du lot styles : balayage, grilles, barres segmentees, halo neon, texte fantome."""
import math

import qa
from content import CONTENT
from lib import fit_size, measure, min_text

REQ = ("cpu.temp", "gpu.temp")   # leur valeur secondaire (charge) est obligatoire : jamais omise


def ink(col, al):
    """Couleur #rrggbb + alpha hexa."""
    return col[:7] + al


def scan(c, box, step, op=0.16, color="#000000", th=1.5):
    """Lignes de balayage horizontales dans `box`."""
    x, y, w, h = box
    yy = y
    while yy < y + h - th:
        c.rect(x, yy, w, th, color, 0, opacity=op, pre="sc")
        yy += step


def grid_lines(c, box, nx, ny, color, op=0.14, th=1):
    """Quadrillage fin de nx x ny cases."""
    x, y, w, h = box
    for i in range(nx + 1):
        c.rect(x + i * (w - th) / nx, y, th, h, color, 0, opacity=op, pre="gx")
    for j in range(ny + 1):
        c.rect(x, y + j * (h - th) / ny, w, th, color, 0, opacity=op, pre="gy")


def seg_bar(c, x, y, w, h, m, i=0, n=14, gap_col="#000000"):
    """Barre segmentee : barre normale + fines coupures sombres."""
    c.bar(x, y, w, h, m.src, m.lo, m.hi, m.colors(c.p, i), radius=0, pre="b")
    g = max(1.5, w / n * 0.16)
    for k in range(1, n):
        c.rect(x + k * w / n - g / 2, y - 1, g, h + 2, gap_col, 0, opacity=0.85, pre="sg")


def ghost(c, x, y, w, h, txt, color, op=0.12, font=None, bold=True, align="right", size=None):
    """Texte decoratif sans controle de chevauchement (ex. segments eteints d'un LCD)."""
    font = font or c.p.font
    s = fit_size(txt, font, bold, w, h) if size is None else size
    c._add("gh", x, y, w, h, type="text", text=txt, opacity=op, style=c._style(font, s, color, align, bold))
    return s


def halo(c, box, color, r, th=2, steps=3):
    """Tube neon : contour net + lueurs concentriques tres transparentes (reste dans le canevas)."""
    x, y, w, h = box
    for k in range(steps, 0, -1):
        e = th * k * 1.6
        e = min(e, x, y, c.f.w - x - w, c.f.h - y - h)
        c.rect(x - e, y - e, w + 2 * e, h + 2 * e, "#00000000", r + e, stroke=color, sw=th + e, opacity=0.07 + 0.03 * (steps - k), pre="hl")
    c.rect(x, y, w, h, "#00000000", r, stroke=color, sw=th, pre="tb")


def ticks_line(c, x, y, w, n, ln, color, op=0.5, th=1.5):
    """Regle graduee horizontale (n traits)."""
    for k in range(n + 1):
        big = k % 5 == 0
        c.rect(x + k * (w - th) / n, y, th, ln * (1 if big else 0.55), color, 0, opacity=op if big else op * 0.7, pre="tk")


def cols_for(fmt, n=4):
    """Nombre de metriques adapte au format."""
    if fmt.round or fmt.short <= 240:
        return min(n, 3)
    if fmt.strip:
        return min(n, 5 if fmt.w > 1500 else 4)
    return n


def ring_pts(c, cx, cy, r, n, size, color, op=1.0, start=0.0):
    """n petits carres sur un cercle (pointilles)."""
    for k in range(n):
        a = start + 2 * math.pi * k / n
        c.rect(cx + r * math.cos(a) - size / 2, cy + r * math.sin(a) - size / 2, size, size, color, size / 2, opacity=op, pre="pt")


def line(c, x0, y0, x1, y1, color, th=1.5, op=1.0):
    """Segment quelconque (rect fin tourne)."""
    L = math.hypot(x1 - x0, y1 - y0)
    if L < 1:
        return
    a = math.degrees(math.atan2(y1 - y0, x1 - x0))
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    c.rect(cx - L / 2, cy - th / 2, L, th, color, 0, opacity=op, rotation=a, pre="ln")


def persp(c, box, hy, color, op=0.7, nh=6, nv=7):
    """Sol en perspective synthwave : horizontales resserrees vers l'horizon, fuyantes vers (cx, hy)."""
    x, y, w, h = box
    bot = y + h
    for j in range(1, nh + 1):
        yy = hy + (bot - hy) * (j / nh) ** 2
        c.rect(x, min(yy, bot - 2), w, 2, color, 0, opacity=op, pre="hz")
    cx = x + w / 2
    for k in range(-nv, nv + 1):
        line(c, cx, hy, cx + k * w / (2 * nv) * 1.0, bot, color, 1.6, op * 0.8)


def hazard(c, box, color, t):
    """Bande de chevrons de danger (traits a 45 deg alternes)."""
    x, y, w, h = box
    L = max(1.0, (h - t * 0.7) * 1.414)
    c.rect(x, y, w, h, color, 0, opacity=0.9, pre="hzb")
    k = 0.0
    step = max(t * 2.2, w / 30)
    while k < w - t:
        c.rect(x + k + t - L / 2, y + h / 2 - t / 2, L, t, "#000000", 0, opacity=0.85, rotation=-45, pre="hz")
        k += step


def need(fmt):
    """Hauteur de boite pour un libelle a la taille mini lisible (+ marge)."""
    return min_text(fmt) * 1.25


def lv(c, box, m, col, tcol=None, g="lv", font=None, bold=True, valign="left"):
    """Libelle + valeur dans `box` : cote a cote si la boite est large, empiles sinon."""
    x, y, w, h = box
    p = c.p
    tcol = tcol or p.text
    if w > 1.7 * h:
        c.text(x, y, w * 0.42, h, m.label, col, bold=True, pre="l", group=g + "l", font=font, maxsize=h * 0.55)
        c.value(m.src, m.fmt, x + w * 0.44, y, w * 0.56, h, m.sample, tcol, bold=bold, align="right", pre="v", group=g + "v", font=font, maxsize=h * 0.8)
        return
    lh = max(h * 0.26, min(need(c.f), h * 0.45))
    c.text(x, y, w, lh, m.label, col, bold=True, align=valign, pre="l", group=g + "l", font=font, maxsize=lh)
    c.value(m.src, m.fmt, x, y + lh, w, h - lh, m.sample, tcol, bold=bold, align=valign, pre="v", group=g + "v", font=font)


def order(ms, n):
    """n premieres metriques, SSD 2 (conditionnel) en dernier : le trou eventuel reste en fin de grille."""
    sh = list(ms[:n])
    if n >= 5:
        sh.append(sh.pop(4))
    return sh


def fit(p, fmt, content, draw, nmax=9):
    """Plus grand nombre de metriques (>= 5, BASE d'abord) pour lequel `draw(p, fmt, metriques)` passe le QA ; sinon None."""
    ms = CONTENT[content][1]
    for n in range(min(nmax, len(ms)), 4, -1):
        w = draw(p, fmt, order(ms, n))
        if w is not None and not qa.problems(w, fmt):
            return w
    return None


def body(c, box, m, i, col, tcol=None, bar=None, font=None, g="b", align="left", label=None, lsize=None, bold=True, lower=False):
    """Libelle + valeur + valeur secondaire (+ barre) dans `box` ; tout dans la condition de la metrique.
    Boite large et basse : libelle/secondaire a gauche, valeur a droite ; sinon empile."""
    x, y, w, h = box
    p = c.p
    tcol = tcol or p.text
    mt = min_text(c.f)
    label = label or (m.label.lower() if lower else m.label)
    vg = g + "v" + ("w" if len(m.sample) > 6 else "n")
    with c.m(m):
        bh = max(3.0, h * 0.09) if bar else 0.0
        hh = h - (bh + h * 0.05 if bar else 0.0)
        if w > 1.35 * h:
            sub = m.sub and (m.src in REQ or hh * 0.5 >= mt * 1.25)
            ts = mt * 1.1
            fnt = font or p.font
            need_lw = max(measure(label, fnt, True, ts), measure(m.sub[2], fnt, False, ts) if sub else 0.0)
            lw = min(max(need_lw, w * 0.34), w * 0.56)
            lh = hh * 0.5 if sub else hh
            c.text(x, y, lw, lh, label, col, bold=bold, align=align, pre="l", group=g + "l", font=font, maxsize=lsize)
            if sub:
                c.value(m.sub[0], m.sub[1], x, y + lh, lw, hh - lh, m.sub[2], p.text2, align=align, pre="s", group=g + "s", font=font)
            c.value(m.src, m.fmt, x + lw + w * 0.03, y, w - lw - w * 0.03, hh, m.sample, tcol, bold=bold, align="right", pre="v", group=vg, font=font, maxsize=hh * 0.72)
        else:
            sub = m.sub and (m.src in REQ or hh * 0.2 >= mt * 1.25)
            lh, sh = hh * 0.24, (hh * 0.22 if sub else 0.0)
            c.text(x, y, w, lh, label, col, bold=bold, align=align, pre="l", group=g + "l", font=font, maxsize=lsize)
            c.value(m.src, m.fmt, x, y + lh, w, hh - lh - sh, m.sample, tcol, bold=bold, align=align, pre="v", group=vg, font=font)
            if sub:
                c.value(m.sub[0], m.sub[1], x, y + hh - sh, w, sh, m.sub[2], p.text2, align=align, pre="s", group=g + "s", font=font)
        if bar:
            if callable(bar):
                bar(c, x, y + h - bh, w, bh, m, i)
            else:
                c.bar(x, y + h - bh, w, bh, m.src, m.lo, m.hi, m.colors(p, i), pre="b")


ALL8 = ["sty-pc", "sty-therm", "sty-io", "sty-air", "sty-puiss", "sty-gpu", "sty-cpu", "sty-mem"]


def dial(c, cx, cy, R, m, i, thick=None, g="d"):
    """Jauge 270 deg : valeur au centre, valeur secondaire dessous, libelle dans l'ouverture."""
    p = c.p
    c.arc(cx, cy, R, m.src, m.lo, m.hi, thick or R * 0.11, m.colors(p, i))
    vw = R * 1.3
    c.value(m.src, m.fmt, cx - vw / 2, cy - R * 0.7, vw, R * 0.62, m.sample, p.text, bold=True, align="center", pre="dv", group=g + "v", font=p.font)
    c.value(m.sub[0], m.sub[1], cx - vw / 2, cy - R * 0.05, vw, R * 0.32, m.sub[2], p.text2, align="center", pre="ds", group=g + "s", font=p.font)
    c.text(cx - R * 0.5, cy + R * 0.42, R, R * 0.33, m.label, p.text2, bold=True, align="center", pre="dl", group=g + "l", font=p.font)
