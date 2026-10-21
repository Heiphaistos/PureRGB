"""Orbites : trois arcs ouverts vers le bas, heure et date au centre, trois lectures dans l'ouverture (ecrans ronds)."""
from content import CONTENT
from lib import Canvas, split
from parts import mini_col

NAME, TITLE = "uw_orbites", "Orbites rondes"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["rond", "s480"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:3]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * 0.6
    th = R * 0.075
    step = th * 1.45
    for i, m in enumerate(ms):
        c.arc(cx, cy, R - i * step, m.src, m.lo, m.hi, th, m.colors(p, i), start=135, sweep=270)
    inner = R - 2 * step - th
    s = inner * 1.3
    c.value("time", "%H:%M", cx - s / 2, cy - inner * 0.62, s, inner * 0.62, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%d/%m/%Y", cx - s * 0.35, cy + inner * 0.04, s * 0.7, inner * 0.2, "88/88/8888", p.text2, align="center", pre="date")
    for i, box in enumerate(split((cx - R * 0.5, cy + R * 0.5, R, R * 0.38), 3, 4)):
        mini_col(c, box, ms[i], i)
    return c.finish()
