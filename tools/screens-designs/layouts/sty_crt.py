"""Ecran CRT : bezel arrondi, vignettage, balayage, grande valeur et lignes d'invite avec barres."""
from lib import Canvas, measure, split
from layouts.sty_common import ALL8, REQ, fit, need, scan

NAME, TITLE = "sty-crt", "Écran CRT"
CONTENTS = ALL8
MONO = "Roboto Mono"


def accepts(p):
    return p.dark and p.font != "Inter" or "terminal" in p.tags


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    e = fmt.short * 0.02
    bz = (x - e, y - e, w + 2 * e, h + 2 * e) if fmt.round else (fmt.short * 0.025, fmt.short * 0.025, fmt.w - fmt.short * 0.05, fmt.h - fmt.short * 0.05)
    rad = bz[2] / 2 if fmt.round else fmt.short * 0.09
    c.rect(*bz, "#00000000", rad, stroke=p.text2, sw=max(3, fmt.short * 0.012), opacity=0.5, pre="bz")
    inner = (bz[0] + fmt.short * 0.02, bz[1] + fmt.short * 0.02, bz[2] - fmt.short * 0.04, bz[3] - fmt.short * 0.04)
    # vignettage : bords assombris
    vw = fmt.short * 0.05
    for k in range(3):
        d = vw * (1 - k / 3)
        c.rect(inner[0], inner[1], inner[2], d, "#000000", 0, opacity=0.12, pre="vg") if not fmt.round else None
    head = fmt.short > 240 and not fmt.strip
    hh = h * 0.1 if head else 0
    if head:
        s = c.text(x, y, w * 0.55, hh, "> état --direct", p.text2, MONO, pre="hd", maxsize=hh * 0.6)
        c.rect(x + measure("> état --direct", MONO, False, s) + 4, y + hh * 0.5 - s * 0.36, s * 0.55, s * 0.72, p.text, 0, pre="cur")
        c.value("time", "%H:%M:%S", x + w * 0.6, y, w * 0.4, hh, "88:88:88", p.text, MONO, align="right", pre="tm", maxsize=hh * 0.7)
    area = (x, y + hh * 1.2, w, h - hh * 1.2)
    ax, ay, aw, ah = area
    hero, rows = ms[0], ms[1:]
    if fmt.strip or fmt.wide:
        hero_w = aw * 0.34
        rx = ax + hero_w + aw * 0.04
        rw = aw - hero_w - aw * 0.04
        c.text(ax, ay, hero_w, need(fmt), hero.label.lower(), p.text2, MONO, pre="hl", maxsize=need(fmt))
        c.value(hero.src, hero.fmt, ax, ay + ah * 0.18, hero_w, ah * 0.5, hero.sample, p.text, MONO, bold=True, pre="hv")
        c.value(hero.sub[0], hero.sub[1], ax, ay + ah * 0.68, hero_w, ah * 0.2, hero.sub[2], p.text2, MONO, pre="hs")
        c.bar(ax, ay + ah * 0.92, hero_w, max(5, ah * 0.07), hero.src, hero.lo, hero.hi, hero.colors(p), radius=0, pre="hb")
    else:
        hh2 = ah * (0.3 if not fmt.round else 0.28)
        c.text(ax, ay, aw * 0.5, need(fmt), hero.label.lower(), p.text2, MONO, pre="hl", maxsize=need(fmt))
        c.value(hero.src, hero.fmt, ax, ay + need(fmt), aw * 0.6, hh2 - need(fmt), hero.sample, p.text, MONO, bold=True, pre="hv")
        c.value(hero.sub[0], hero.sub[1], ax + aw * 0.62, ay + need(fmt), aw * 0.38, (hh2 - need(fmt)) * 0.6, hero.sub[2], p.text2, MONO, align="right", pre="hs", maxsize=(hh2 - need(fmt)) * 0.45)
        rx, rw = ax, aw
        ay, ah = ay + hh2 * 1.08, ah - hh2 * 1.08
    k = len(rows)
    for i, (bx, by, bw, bh) in enumerate(split((rx, ay, rw, ah), k, ah * 0.015, horizontal=False)):
        m = rows[i]
        fs = bh * 0.62
        lw = bw * 0.27
        sub = m.sub and (m.src in REQ or bw > bh * 8)
        vw = bw * (0.27 if sub else 0.34)
        bw_ = bw - lw - vw - (bw * 0.26 if sub else 0) - bw * 0.04
        with c.m(m):
            c.text(bx, by, lw, bh * 0.85, "> " + m.label.lower(), p.text2, MONO, pre="l", group="rl", maxsize=fs)
            c.bar(bx + lw + bw * 0.01, by + bh * 0.28, bw_, bh * 0.36, m.src, m.lo, m.hi, m.colors(p, i), radius=0, pre="b")
            vx = bx + lw + bw_ + bw * 0.03
            c.value(m.src, m.fmt, vx, by, vw - bw * 0.03, bh * 0.85, m.sample, p.text, MONO, bold=True, align="right", pre="v", group="rv", maxsize=fs)
            if sub:
                c.value(m.sub[0], m.sub[1], vx + vw, by, bx + bw - vx - vw, bh * 0.85, m.sub[2], p.text2, MONO, align="right", pre="s", group="rs", maxsize=fs)
    scan(c, inner if not fmt.round else (x, y, w, h), max(4.0, fmt.h / 80), 0.16)
    c.rect(inner[0], inner[1] + inner[3] * 0.5, inner[2], inner[3] * 0.1, p.text, 0, opacity=0.04, pre="beam") if not fmt.round else None
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
