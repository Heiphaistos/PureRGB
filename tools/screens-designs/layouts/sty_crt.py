"""Ecran CRT : bezel arrondi, vignettage, balayage, grande valeur et lignes d'invite avec barres."""
from content import CONTENT
from lib import Canvas, measure, split
from layouts.sty_common import need, scan

NAME, TITLE = "sty-crt", "Écran CRT"
CONTENTS = ["sty-pc", "sty-therm", "sty-air"]
MONO = "Roboto Mono"


def accepts(p):
    return p.dark and p.font != "Inter" or "terminal" in p.tags


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
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
    nrows = 4 if fmt.short >= 240 else 3
    if fmt.strip or fmt.wide:
        hero_w, hero = aw * 0.38, ms[0]
        rows = ms[1:1 + (5 if fmt.strip and fmt.w > 1500 else 4)]
        rx = ax + hero_w + aw * 0.04
        rw = aw - hero_w - aw * 0.04
        c.text(ax, ay, hero_w, need(fmt), hero.label.lower(), p.text2, MONO, pre="hl", maxsize=need(fmt))
        c.value(hero.src, hero.fmt, ax, ay + ah * 0.18, hero_w, ah * 0.7, hero.sample, p.text, MONO, bold=True, pre="hv")
        c.bar(ax, ay + ah * 0.92, hero_w, max(5, ah * 0.07), hero.src, hero.lo, hero.hi, hero.colors(p), radius=0, pre="hb")
    else:
        hh2 = ah * (0.34 if not fmt.round else 0.3)
        hero, rows = ms[0], ms[1:1 + nrows - 1 if fmt.round else 1 + nrows]
        c.text(ax, ay, aw * 0.5, need(fmt), hero.label.lower(), p.text2, MONO, pre="hl", maxsize=need(fmt))
        c.value(hero.src, hero.fmt, ax, ay + need(fmt), aw, hh2 - need(fmt), hero.sample, p.text, MONO, bold=True, pre="hv")
        rx, rw = ax, aw
        ay, ah = ay + hh2 * 1.08, ah - hh2 * 1.08
    k = len(rows)
    for i, (bx, by, bw, bh) in enumerate(split((rx, ay, rw, ah), k, ah * 0.02, horizontal=False)):
        m = rows[i]
        fs = bh * 0.62
        lw = bw * 0.36
        c.text(bx, by, lw, bh * 0.85, "> " + m.label.lower(), p.text2, MONO, pre="l", group="rl", maxsize=fs)
        c.bar(bx + lw + bw * 0.01, by + bh * 0.28, bw * 0.28, bh * 0.36, m.src, m.lo, m.hi, m.colors(p, i), radius=0, pre="b")
        c.value(m.src, m.fmt, bx + lw + bw * 0.3, by, bw - lw - bw * 0.3, bh * 0.85, m.sample, p.text, MONO, bold=True, align="right", pre="v", group="rv", maxsize=fs)
    scan(c, inner if not fmt.round else (x, y, w, h), max(4.0, fmt.h / 80), 0.16)
    c.rect(inner[0], inner[1] + inner[3] * 0.5, inner[2], inner[3] * 0.1, p.text, 0, opacity=0.04, pre="beam") if not fmt.round else None
    return c.finish()
