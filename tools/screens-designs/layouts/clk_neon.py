"""Heure neon : halo, cadre lumineux, equerres."""
from content import CONTENT
from lib import Canvas
from parts import corners, glow
from layouts.clk_common import cells, date_line

NAME, TITLE = "clk-neon", "Heure néon"
CONTENTS = ["clk-systeme", "clk-temps", "clk-reseau"]


def accepts(p):
    return p.dark


def _halo(c, x, y, w, h, color):
    d = max(1.5, h * 0.012)
    for dx, dy in ((-d, 0), (d, 0), (0, -d), (0, d)):
        c.value("time", "%H:%M", x + dx, y + dy, w, h, "88:88", color, bold=True, align="center", pre="halo")
        c.out[-1]["opacity"] = 0.3
        c.out.inks.pop()   # le halo recouvre volontairement le texte


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    glow(c, x + w * 0.4, y + h * 0.4, min(w, h) * 0.45, p.acc[1])
    if fmt.wide:
        fw, fh = w * (0.56 if not fmt.strip else 0.46), h
        fb = (x, y, fw, fh)
        ty, th = y + fh * 0.1, fh * 0.6
    else:
        fb = (x, y, w, h * 0.62)
        ty, th = y + h * 0.04, h * 0.4
        fw = w
    c.rect(*fb, "#00000000", min(fb[2], fb[3]) * 0.06, stroke=p.acc[0], sw=2, pre="frame")
    corners(c, fb, p.acc[1], min(fb[2], fb[3]) * 0.1, 3)
    _halo(c, x + fw * 0.05, ty, fw * 0.9, th, p.acc[0])
    c.value("time", "%H:%M", x + fw * 0.05, ty, fw * 0.9, th, "88:88", p.text, bold=True, align="center", pre="time")
    date_line(c, (x + fw * 0.05, fb[1] + fb[3] * (0.74 if fmt.wide else 0.62), fw * 0.9, fb[3] * (0.16 if fmt.wide else 0.2)), p.acc[1], short=not fmt.wide)
    if fmt.wide:
        rx = x + fw + g * 2
        cells(c, (rx, y, x + w - rx, h), ms[:4 if fmt.strip else 3], horizontal=fmt.strip, gap=g, bar=False)
    else:
        cells(c, (x, y + h * 0.7, w, h * 0.3), ms[:3], gap=g, bar=False, card=True)
    return c.finish()
