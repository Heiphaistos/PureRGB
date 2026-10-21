"""Mosaïque de mini-jauges circulaires : valeur dans l'anneau, libellé dessous."""
from lib import Canvas, min_text
from parts import ring
from layouts.cap_common import ALL, densest, metrics

NAME, TITLE = "cap-minijauges", "Mini-jauges multiples"
CONTENTS = ALL


def build(p, fmt, content):
    ms = metrics(content)
    mt = min_text(fmt)

    def cells(n, gap):
        """Grille dont l'anneau est le plus grand ; dernière rangée centrée, jamais une seule orpheline (sauf 1 rangée)."""
        zx, zy, zw, zh = fmt.zone
        best = None
        for cols in range(1, n + 1):
            rows = -(-n // cols)
            if rows > 1 and n % cols == 1:
                continue
            cw, ch = (zw - gap * (cols - 1)) / cols, (zh - gap * (rows - 1)) / rows
            r = min(cw, ch - max(1.3 * mt, ch * 0.16))
            if best is None or r > best[0]:
                best = (r, cols, rows, cw, ch)
        _, cols, rows, cw, ch = best
        out = []
        for i in range(n):
            r, k = divmod(i, cols)
            last = n - (rows - 1) * cols if r == rows - 1 else cols
            off = (cols - last) * (cw + gap) / 2 if r == rows - 1 else 0
            out.append((zx + off + k * (cw + gap), zy + r * (ch + gap), cw, ch))
        return out

    def mk(n):
        c = Canvas(p, fmt)
        for i, (x, y, w, h) in enumerate(cells(n, fmt.short * 0.02)):
            lb = max(1.3 * mt, h * 0.16)
            r = min(w, h - lb) / 2 * 0.96
            with c.m(ms[i]):
                ring(c, x + w / 2, y + (h - lb) / 2, r, ms[i], i, label=False)
                c.text(x, y + h - lb, w, lb, ms[i].label, p.text2, bold=True, align="center", pre="l", group="mj")
        return c.finish()
    return densest(mk, fmt, range(min(len(ms), 12), 6, -1))
