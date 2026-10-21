"""Anneaux concentriques : 3 mesures imbriquees, heure et legende au centre (formats ~carres)."""
from content import CONTENT
from lib import Canvas

NAME, TITLE = "anneaux", "Anneaux concentriques"
CONTENTS = ["quad", "disques", "refroidissement"]
FORMATS = ["s480", "s320", "r43", "rond", "s240"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:3]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * (0.6 if fmt.round else 1)
    th = R * 0.105
    step = th * 1.55
    for i, m in enumerate(ms):
        r = R - i * step
        c.arc(cx, cy, r, m.src, m.lo, m.hi, th, m.colors(p, i), start=120, sweep=300)
    inner = R - 3 * step + th * 0.6
    side = inner * 1.35
    c.value("time", "%H:%M", cx - side / 2, cy - inner * 0.72, side, inner * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
    rh = inner * 0.2
    for i, m in enumerate(ms):
        y = cy - inner * 0.12 + i * rh * 1.15
        lw = side * 0.45
        c.text(cx - side / 2, y, lw, rh, m.label, p.acc[i % 3], bold=True, pre="l", group="al", maxsize=rh)
        c.value(m.src, m.fmt, cx - side / 2 + lw, y, side - lw, rh, m.sample, p.text, bold=True, align="right", pre="v", group="av", maxsize=rh)
    return c.finish()
