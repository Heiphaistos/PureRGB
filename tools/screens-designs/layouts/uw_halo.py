"""Halo : trois anneaux pleins concentriques (CPU, GPU, RAM), heure et date au centre, cinq lectures du socle dessous (ecrans ronds)."""
import math

from content import CONTENT
from lib import Canvas
from layouts.uw_common import line

NAME, TITLE = "uw_halo", "Halo central"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["rond", "s480", "s320"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * 0.6
    th = R * 0.05
    step = th * 1.4
    for i, m in enumerate(ms[:3]):
        c.arc(cx, cy, R - i * step, m.src, m.lo, m.hi, th, m.colors(p, i), start=-90, sweep=360)
    inner = R - 2 * step - th
    s = inner * 1.62
    c.value("time", "%H:%M", cx - s * 0.4, cy - inner * 0.9, s * 0.8, inner * 0.4, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%d/%m/%Y", cx - s * 0.35, cy - inner * 0.5, s * 0.7, inner * 0.17, "88/88/8888", p.text2, align="center", pre="date")
    rh = inner * 0.19
    for i, m in enumerate(ms[:5]):
        ry = cy - inner * 0.33 + i * rh * 1.06
        dy = max(abs(ry - cy), abs(ry + rh - cy))
        w = min(s, 2 * math.sqrt(max(inner * inner - dy * dy, 1)) * 0.97)
        line(c, (cx - w / 2, ry, w, rh), m, i, g="hl")
    return c.finish()
