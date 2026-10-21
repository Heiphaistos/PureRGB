"""Plan technique : quadrillage, cotations a reperes, anneau en coupe a reticule, cartouche en bas a droite."""
from content import CONTENT
from lib import Canvas, split
from layouts.sty_common import grid_lines, need

NAME, TITLE = "sty-blueprint", "Plan technique"
CONTENTS = ["sty-pc", "sty-therm", "sty-air"]


def _dim(c, x, y, w, col):
    """Ligne de cote : trait + reperes d'extremite."""
    c.rect(x, y, w, 1.5, col, 0, opacity=0.7, pre="dm")
    for ex in (x, x + w - 1.5):
        c.rect(ex, y - 5, 1.5, 11, col, 0, opacity=0.9, pre="dm")


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    col, col2 = p.text2, p.acc[1]
    if not fmt.round:
        e = fmt.short * 0.03
        grid_lines(c, (e, e, fmt.w - 2 * e, fmt.h - 2 * e), max(8, round(fmt.w / 48)), max(6, round(fmt.h / 48)), p.text, 0.08)
        c.rect(e, e, fmt.w - 2 * e, fmt.h - 2 * e, "#00000000", 0, stroke=p.text, sw=2, opacity=0.6, pre="fr")
    title = not fmt.strip and not fmt.round and fmt.short > 240
    th = max(h * 0.13, need(fmt) * 1.3) if title else 0
    ax, ay, aw, ah = x, y, w, h - th * 1.15
    if fmt.ratio >= 1.5:
        R = min(ah * 0.44, aw * 0.16)
        cx, cy = ax + R * 1.2, ay + ah / 2
    else:
        R = min(aw * 0.3, ah * 0.27) if not fmt.round else aw * 0.27
        cx, cy = ax + aw / 2, ay + R * 1.12
    hero = ms[0]
    c.arc(cx, cy, R, hero.src, hero.lo, hero.hi, max(3, R * 0.07), [[0, col2], [1, p.acc[0]]])
    for s_ in (-1, 1):   # reticule : tenons hors de l'anneau
        c.rect(cx + s_ * R * 1.0 - (R * 0.3 if s_ < 0 else 0) + (R * 0.0 if s_ < 0 else 0), cy, R * 0.3, 1.2, p.text, 0, opacity=0.6, pre="rt")
        c.rect(cx, cy + s_ * R * 1.0 - (R * 0.3 if s_ < 0 else 0), 1.2, R * 0.3, p.text, 0, opacity=0.6, pre="rt")
    vw = R * 1.4
    c.value(hero.src, hero.fmt, cx - vw / 2, cy - R * 0.45, vw, R * 0.6, hero.sample, p.text, bold=True, align="center", pre="hv", font=p.font)
    lh = need(fmt)
    c.text(cx - R * 0.6, cy + R * 0.25, R * 1.2, lh, hero.label, col, bold=True, align="center", pre="hl", maxsize=lh)
    if fmt.ratio >= 1.5:
        rx = cx + R * 1.4
        if fmt.strip:
            rows = split((rx, ay + ah * 0.06, ax + aw - rx, ah * 0.88), 3, aw * 0.02)
        else:
            rows = split((rx, ay + ah * 0.06, ax + aw - rx, ah * 0.88), 4, ah * 0.04, horizontal=False)
    else:
        top = cy + R * 1.35
        rows = split((ax, top, aw, ay + ah - top), 3 if fmt.short >= 400 else 2, ah * 0.01, horizontal=False)
    for i, (bx, by, bw, bh) in enumerate(rows):
        m = ms[1 + i]
        tag = f"A{i + 1} · {m.label}" if fmt.short >= 400 else m.label
        if fmt.strip:
            c.text(bx, by, bw, bh * 0.28, tag, col2, bold=True, pre="l", group="bl", maxsize=bh * 0.28)
            _dim(c, bx, by + bh * 0.9, bw * 0.9, p.text)
            c.value(m.src, m.fmt, bx, by + bh * 0.32, bw, bh * 0.52, m.sample, p.text, bold=True, pre="v", group="bv")
            continue
        _dim(c, bx, by + bh * 0.92, bw, p.text)
        c.text(bx, by, bw * 0.46, bh * 0.85, tag, col2, bold=True, pre="l", group="bl", maxsize=bh * 0.55)
        c.value(m.src, m.fmt, bx + bw * 0.46, by, bw * 0.54, bh * 0.85, m.sample, p.text, bold=True, align="right", pre="v", group="bv", maxsize=bh * 0.75)
    if title:
        tx, ty, tw, tt = x + w * 0.4, y + h - th, w * 0.6, th
        c.rect(tx, ty, tw, tt, "#00000000", 0, stroke=p.text, sw=1.5, opacity=0.8, pre="cart")
        c.rect(tx + tw * 0.5, ty, 1.5, tt, p.text, 0, opacity=0.8, pre="cart")
        c.text(tx + tw * 0.03, ty + tt * 0.08, tw * 0.44, tt * 0.84, "PLAN 01", col, bold=True, pre="tl", maxsize=tt * 0.5)
        c.value("date", "%d/%m/%Y", tx + tw * 0.53, ty + tt * 0.08, tw * 0.44, tt * 0.84, "88/88/8888", p.text, bold=True, align="right", pre="td", maxsize=tt * 0.5)
    return c.finish()
