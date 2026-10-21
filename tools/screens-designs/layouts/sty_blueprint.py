"""Plan technique : quadrillage, cotations a reperes, anneau en coupe a reticule, cartouche en bas a droite."""
from lib import Canvas, grid
from layouts.sty_common import ALL8, body, fit, grid_lines, need

NAME, TITLE = "sty-blueprint", "Plan technique"
CONTENTS = ALL8


def _dim(c, x, y, w, col):
    """Ligne de cote : trait + reperes d'extremite."""
    c.rect(x, y, w, 1.5, col, 0, opacity=0.7, pre="dm")
    for ex in (x, x + w - 1.5):
        c.rect(ex, y - 5, 1.5, 11, col, 0, opacity=0.9, pre="dm")


def _draw(p, fmt, ms):
    if fmt.round:
        return None
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    col, col2 = p.text2, p.acc[1]
    if not fmt.round:
        e = fmt.short * 0.03
        grid_lines(c, (e, e, fmt.w - 2 * e, fmt.h - 2 * e), max(8, round(fmt.w / 48)), max(6, round(fmt.h / 48)), p.text, 0.08)
        c.rect(e, e, fmt.w - 2 * e, fmt.h - 2 * e, "#00000000", 0, stroke=p.text, sw=2, opacity=0.6, pre="fr")
    title = not fmt.strip and not fmt.round and fmt.short > 240
    th = max(h * 0.11, need(fmt) * 1.3) if title else 0
    ax, ay, aw, ah = x, y, w, h - th * 1.15
    if fmt.ratio >= 1.5:
        R = min(ah * 0.44, aw * 0.16)
        cx, cy = ax + R * 1.2, ay + ah / 2
        rest = (cx + R * 1.4, ay, ax + aw - cx - R * 1.4, ah)
    else:
        R = min(aw * 0.3, ah * 0.25)
        cx, cy = ax + aw / 2, ay + R * 1.1
        rest = (ax, cy + R * 1.35, aw, ay + ah - cy - R * 1.35)
    hero = ms[0]
    c.arc(cx, cy, R, hero.src, hero.lo, hero.hi, max(3, R * 0.07), [[0, col2], [1, p.acc[0]]])
    for s_ in (-1, 1):   # reticule : tenons hors de l'anneau
        c.rect(cx + s_ * R * 1.0 - (R * 0.3 if s_ < 0 else 0), cy, R * 0.3, 1.2, p.text, 0, opacity=0.6, pre="rt")
        c.rect(cx, cy + s_ * R * 1.0 - (R * 0.3 if s_ < 0 else 0), 1.2, R * 0.3, p.text, 0, opacity=0.6, pre="rt")
    vw = R * 1.3
    c.value(hero.src, hero.fmt, cx - vw / 2, cy - R * 0.62, vw, R * 0.55, hero.sample, p.text, bold=True, align="center", pre="hv", font=p.font)
    c.value(hero.sub[0], hero.sub[1], cx - vw / 2, cy - R * 0.04, vw, R * 0.28, hero.sub[2], p.text2, align="center", pre="hs", font=p.font)
    c.text(cx - R * 0.6, cy + R * 0.28, R * 1.2, R * 0.28, hero.label, col, bold=True, align="center", pre="hl", font=p.font)
    gap = fmt.short * 0.02
    for i, b in enumerate(grid(rest, len(ms) - 1, gap, 2.4)):
        b = (b[0], b[1], b[2] * 0.93, b[3])
        body(c, b, ms[1 + i], i, col2, bar=lambda c_, x_, y_, w_, h_, m_, i_: _dim(c_, x_, y_ + h_ / 2, w_, p.text), g="bl")
    if title:
        tx, ty, tw, tt = x + w * 0.4, y + h - th, w * 0.6, th
        c.rect(tx, ty, tw, tt, "#00000000", 0, stroke=p.text, sw=1.5, opacity=0.8, pre="cart")
        c.rect(tx + tw * 0.5, ty, 1.5, tt, p.text, 0, opacity=0.8, pre="cart")
        c.text(tx + tw * 0.03, ty + tt * 0.08, tw * 0.44, tt * 0.84, "PLAN 01", col, bold=True, pre="tl", maxsize=tt * 0.5)
        c.value("date", "%d/%m/%Y", tx + tw * 0.53, ty + tt * 0.08, tw * 0.44, tt * 0.84, "88/88/8888", p.text, bold=True, align="right", pre="td", maxsize=tt * 0.5)
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
