"""Heure neon : halo, cadre lumineux, equerres, date complete et socle."""
from parts import corners, glow
from layouts.clk_common import CONTENTS, attempt, date_line, panel

NAME, TITLE = "clk-neon", "Heure néon"


def accepts(p):
    return p.dark


def _halo(c, x, y, w, h, color):
    d = max(1.5, h * 0.012)
    for dx, dy in ((-d, 0), (d, 0), (0, -d), (0, d)):
        c.value("time", "%H:%M", x + dx, y + dy, w, h, "88:88", color, bold=True, align="center", pre="halo")
        c.out[-1]["opacity"] = 0.3
        c.out.inks.pop()   # le halo recouvre volontairement le texte


def _draw(c, ms, mode):
    p, fmt = c.p, c.f
    x, y, w, h = fmt.zone
    g = fmt.short * 0.03
    glow(c, x + w * 0.3, y + h * 0.3, min(w, h) * 0.45, p.acc[1])
    if fmt.wide:
        fw = w * (0.46 if fmt.strip else 0.5)
        fb = (x, y, fw, h)
        ty, th = y + h * 0.08, h * 0.55
    else:
        fw = w
        fb = (x, y, w, h * 0.3)
        ty, th = y + h * 0.02, h * 0.2
    c.rect(*fb, "#00000000", min(fb[2], fb[3]) * 0.06, stroke=p.acc[0], sw=2, pre="frame")
    corners(c, fb, p.acc[1], min(fb[2], fb[3]) * 0.1, 3)
    _halo(c, x + fw * 0.05, ty, fw * 0.9, th, p.acc[0])
    c.value("time", "%H:%M", x + fw * 0.05, ty, fw * 0.9, th, "88:88", p.text, bold=True, align="center", pre="time")
    date_line(c, (x + fw * 0.05, fb[1] + fb[3] * (0.72 if fmt.wide else 0.7), fw * 0.9, fb[3] * (0.16 if fmt.wide else 0.22)), p.acc[1], short=fw < 600)
    if fmt.wide:
        rx = x + fw + g * 2
        panel(c, (rx, y, x + w - rx, h), ms, mode, gap=g, bar=False)
    else:
        panel(c, (x, y + h * 0.34, w, h * 0.66), ms, mode, gap=g, bar=False)


def build(p, fmt, content):
    return attempt(_draw, p, fmt, content)
