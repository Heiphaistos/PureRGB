"""Quatre petites jauges en 2x2 tangentes, inscrites dans le cercle (ecrans ronds)."""
from content import CONTENT
from lib import Canvas
from parts import ring

NAME, TITLE = "uw_quatuor_rond", "Quatuor rond"
CONTENTS = ["uw-sys", "uw-therm", "uw-stock", "uw-air", "uw-puis"]
FORMATS = ["rond", "s480"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:4]
    cx, cy = fmt.w / 2, fmt.h / 2
    R = fmt.short / 2 - fmt.margin * 0.6
    r = R / (1 + 2 ** 0.5) * 0.97
    a = r * 1.03
    for i, m in enumerate(ms):
        ring(c, cx + (a if i % 2 else -a), cy + (a if i // 2 else -a), r, m, i, thick=r * 0.12)
    return c.finish()
