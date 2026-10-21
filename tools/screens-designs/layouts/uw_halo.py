"""Halo : trois anneaux pleins concentriques, heure et date au centre, trois lectures dessous (ecrans ronds)."""
from content import CONTENT
from lib import Canvas

NAME, TITLE = "uw_halo", "Halo central"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["rond", "s480"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:3]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * 0.6
    th = R * 0.06
    step = th * 1.5
    for i, m in enumerate(ms):
        c.arc(cx, cy, R - i * step, m.src, m.lo, m.hi, th, m.colors(p, i), start=-90, sweep=360)
    inner = R - 2 * step - th
    s = inner * 1.3
    c.value("time", "%H:%M", cx - s / 2, cy - inner * 0.72, s, inner * 0.5, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%d/%m/%Y", cx - s / 2, cy - inner * 0.2, s, inner * 0.16, "88/88/8888", p.text2, align="center", pre="date")
    rh = inner * 0.21
    for i, m in enumerate(ms):
        by = cy + inner * 0.06 + i * rh * 1.18
        c.text(cx - s / 2, by, s * 0.45, rh, m.label, p.acc[i % 3], bold=True, pre="l", group="hl", maxsize=rh)
        c.value(m.src, m.fmt, cx - s / 2 + s * 0.45, by, s * 0.55, rh, m.sample, p.text, bold=True, align="right", pre="v", group="hv", maxsize=rh)
    return c.finish()
