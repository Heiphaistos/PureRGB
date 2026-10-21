"""Héros + mosaïque : grande jauge centrale et mini-cartes (ou lignes si la place manque) des autres capteurs."""
import qa
from lib import Canvas, grid, min_text
from parts import ring, stat_card
from layouts.cap_common import ALL, crow, densest, fracs, metrics

NAME, TITLE = "cap-mosaique", "Héros et mosaïque"
CONTENTS = ALL
FORMATS = ["s480", "s320", "r43", "w169", "w1280", "u1920"]


def build(p, fmt, content):
    ms = metrics(content)
    mt = min_text(fmt)

    def mk(n, lines):
        c = Canvas(p, fmt)
        x, y, w, h = fmt.zone
        gap = fmt.short * 0.03
        if fmt.ratio >= 1.5:
            d = h
            ring(c, x + d / 2, y + d / 2, d / 2 * 0.97, ms[0], 0, sub=True)
            rest = (x + d + gap * 2, y, w - d - gap * 2, h)
        else:
            d = min(w, h * 0.42)
            ring(c, x + w / 2, y + d / 2, d / 2 * 0.97, ms[0], 0)
            rest = (x, y + d + gap, w, h - d - gap)
        if not lines:
            for i, cell in enumerate(grid(rest, n - 1, gap, 1.4)):
                with c.m(ms[i + 1]):
                    stat_card(c, cell, ms[i + 1], i + 1, sub=False)
            return c.finish()
        rx, ry, rw, rh = rest        # lignes compactes en 1 à 3 colonnes
        for cols in (1, 2, 3):
            rows = -(-(n - 1) // cols)
            if rh / rows >= 1.9 * mt:
                break
        else:
            return None
        cw, lh = (rw - gap * (cols - 1)) / cols, min(rh / rows, fmt.short * 0.13)
        fr = fracs(p, ms[1:n])
        for i in range(n - 1):
            col, r = divmod(i, rows)
            with c.m(ms[i + 1]):
                crow(c, (rx + col * (cw + gap), ry + r * lh, cw, lh * 0.92), ms[i + 1], i + 1, fr)
        return c.finish()

    def both(n):
        w = mk(n, False)
        if w is not None and not qa.problems(w, fmt):
            return w
        return mk(n, True) or w
    return densest(both, fmt, range(min(len(ms), 12), 6, -1))
