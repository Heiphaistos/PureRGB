"""Cinq petites jauges (CPU, GPU, RAM, SSD 1 et 2) en pentagone, inscrites dans le cercle (ecrans ronds)."""
import math

from content import CONTENT
from lib import Canvas
from layouts.uw_common import ring2

NAME, TITLE = "uw_quatuor_rond", "Quatuor rond"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis", "uw-gpu", "uw-cpu", "uw-reseau", "uw-disq", "uw-mix"]
FORMATS = ["rond", "s480", "s320"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:5]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * 0.6
    s36 = math.sin(math.radians(36))
    r = R * s36 / (1 + s36) * 0.98
    a = R - r * 1.0
    for i, m in enumerate(ms):
        ang = math.radians(-90 + 72 * i)
        ring2(c, cx + a * math.cos(ang), cy + a * math.sin(ang), r, m, i, thick=r * 0.12, g="qr")
    return c.finish()
