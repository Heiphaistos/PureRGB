"""Terminal retro : texte monospace, lignes de balayage CRT, barres fines ; BASE (CPU/GPU temp+charge...) puis extras."""
from content import CONTENT
from lib import Canvas, measure
from layouts.r2_common import SETS, counts, first_ok

NAME, TITLE = "terminal", "Terminal rétro"
CONTENTS = SETS


def accepts(p):
    return "terminal" in p.tags or "rétro" in p.tags


def build(p, fmt, content):
    ms = CONTENT[content][1]
    x0, y0, w, h = fmt.zone
    mono = "Roboto Mono"

    def mk(n, cols, head):
        def f():
            c = Canvas(p, fmt)
            per = -(-n // cols)
            rows = per + (1 if head else 0)
            rh = min(h / rows, fmt.short * 0.2)
            y = y0 + (h - rh * rows) / 2
            cw0 = w / cols
            cw = cw0 - (cw0 * 0.07 if cols > 1 else 0)   # gouttiere entre colonnes
            if head:
                s = c.text(x0, y, w * 0.8, rh * 0.9, "> pure@rgb:~$ stats", p.text2, mono, pre="hd", maxsize=rh * 0.6)
                c.rect(x0 + measure("> pure@rgb:~$ stats", mono, False, s) + 4, y + rh * 0.45 - s * 0.36, s * 0.55, s * 0.72, p.text, 0, pre="cur")
            for i in range(n):
                m = ms[i]
                col, r = divmod(i, per)
                bx, by = x0 + col * cw0, y + (r + (1 if head else 0)) * rh
                gap = cw * 0.03
                lw = cw * 0.24
                has = bool(m.sub)
                sw = cw * 0.27 if has else 0
                bw = cw * 0.12 if cols > 1 else cw * 0.16
                vw = cw - lw - bw - sw - gap * 3
                gg = "b" if i < 5 else "x"
                with c.m(m):
                    c.text(bx, by, lw * 0.9, rh * 0.8, m.label.lower(), p.text2, mono, pre="l", group="tl" + gg, maxsize=rh * 0.6)
                    c.bar(bx + lw, by + rh * 0.3, bw, rh * 0.3, m.src, m.lo, m.hi, m.colors(p, i), radius=0, pre="b")
                    c.value(m.src, m.fmt, bx + lw + bw + gap, by, vw, rh * 0.8, m.sample, p.text, mono, bold=True, align="right", pre="v", group="tv" + gg, maxsize=rh * 0.65)
                    if has:
                        c.value(m.sub[0], m.sub[1], bx + cw - sw - gap, by, sw, rh * 0.8, m.sub[2], p.text2, mono, align="right", pre="s", group="ts" + gg, maxsize=rh * 0.6)
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
        return f

    if fmt.ratio < 1.4:
        cols = (1, 2)
    elif fmt.ratio < 2.6:
        cols = (2, 1, 3)
    else:
        cols = (3, 2, 4)
    return first_ok(fmt, [mk(n, k, fmt.ratio < 2.6) for n in counts(ms, (10, 9, 8, 6, 5)) for k in cols])
