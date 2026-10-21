"""Liste dense à barres : une ligne par capteur, plusieurs colonnes selon le format."""
from lib import Canvas, min_text
from layouts.cap_common import ALL, crow, densest, fracs, metrics, table_dims

NAME, TITLE = "cap-liste", "Liste à barres"
CONTENTS = ALL


def build(p, fmt, content):
    ms = metrics(content)
    mt = min_text(fmt)

    def mk(n):
        d = table_dims(fmt, n, 1.9 * mt, 10 * mt)
        if not d:
            return None
        cols, rows = d
        c = Canvas(p, fmt)
        fr = fracs(p, ms[:n])
        z, gx = fmt.zone, fmt.short * 0.05
        cw = (z[2] - gx * (cols - 1)) / cols
        rh = min(z[3] / rows, fmt.short * 0.13)
        top = z[1] + (z[3] - rh * rows) / 2
        for i in range(n):
            col, r = divmod(i, rows)
            with c.m(ms[i]):
                crow(c, (z[0] + col * (cw + gx), top + r * rh, cw, rh * 0.92), ms[i], i, fr)
        return c.finish()
    return densest(mk, fmt, range(len(ms), 6, -1))
