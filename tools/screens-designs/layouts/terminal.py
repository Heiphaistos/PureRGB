"""Terminal retro : texte monospace, lignes de balayage CRT, barres fines (palettes retro/terminal)."""
from content import CONTENT
from lib import Canvas, measure

NAME, TITLE = "terminal", "Terminal rétro"
CONTENTS = ["quad", "carte-mere", "refroidissement"]


def accepts(p):
    return "terminal" in p.tags or "rétro" in p.tags


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:6]
    x, y, w, h = fmt.zone
    mono = "Roboto Mono"
    cols = 1 if fmt.ratio < 1.4 else 2 if fmt.ratio < 2.6 else 3
    per = -(-len(ms) // cols)
    head = fmt.ratio < 2.6
    rows = per + (1 if head else 0)
    rh = min(h / rows, fmt.short * 0.2)
    y += (h - rh * rows) / 2
    cw = w / cols
    if head:
        s = c.text(x, y, w * 0.8, rh * 0.9, "> pure@rgb:~$ stats", p.text2, mono, pre="hd", maxsize=rh * 0.6)
        c.rect(x + measure("> pure@rgb:~$ stats", mono, False, s) + 4, y + rh * 0.45 - s * 0.36, s * 0.55, s * 0.72, p.text, 0, pre="cur")
    for i, m in enumerate(ms):
        col, r = divmod(i, per)
        bx = x + col * cw
        by = y + (r + (1 if head else 0)) * rh
        gap = cw * 0.04
        lw = cw * (0.34 if cols == 1 else 0.3)
        c.text(bx, by, lw, rh * 0.8, m.label.lower(), p.text2, mono, pre="l", group="tx", maxsize=rh * 0.6)
        c.bar(bx + lw, by + rh * 0.3, cw * 0.32, rh * 0.3, m.src, m.lo, m.hi, m.colors(p, i), radius=0, pre="b")
        c.value(m.src, m.fmt, bx + lw + cw * 0.32 + gap, by, cw - lw - cw * 0.32 - gap * 2, rh * 0.8, m.sample, p.text, mono, bold=True, align="right", pre="v", group="tx", maxsize=rh * 0.65)
    step = max(6.0, fmt.h / 34)
    yy = 0.0
    while yy < fmt.h - 2:
        half = fmt.w / 2
        if fmt.round:   # ligne de balayage = corde du cercle
            half = max(0.0, (half * half - (yy + 0.75 - fmt.h / 2) ** 2)) ** 0.5 - 3
        if half > 4:
            c.rect(fmt.w / 2 - half, yy, 2 * half, 1.5, "#000000", 0, opacity=0.18, pre="sc")
        yy += step
    return c.finish()
