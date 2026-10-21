"""Horloge numerique + date, avec 3 capteurs en pied (ou en colonne sur les ecrans larges)."""
from content import CONTENT
from lib import Canvas, split
from parts import glow

NAME, TITLE = "horloge", "Horloge et capteurs"
CONTENTS = ["cpu-gpu", "carte-mere", "quad"]


def build(p, fmt, content):
    c = Canvas(p, fmt)
    ms = CONTENT[content][1][:3]
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    if p.dark:
        glow(c, x + w * (0.3 if fmt.wide else 0.5), y + h * (0.5 if fmt.wide else 0.3), min(w, h) * 0.42, p.acc[1])
    if fmt.wide:   # heure a gauche, capteurs en colonne a droite
        tw = w * 0.52
        c.value("time", "%H:%M", x, y + h * 0.08, tw, h * 0.62, "88:88", p.text, bold=True, align="center", pre="time")
        c.value("date", "%A %d %B", x, y + h * 0.74, tw, h * 0.18, "mercredi 30 septembre", p.text2, align="center", pre="date")
        c.rect(x + tw + gap, y + h * 0.1, 2, h * 0.8, p.line, 1, pre="sep")
        for i, (bx, by, bw, bh) in enumerate(split((x + tw + gap * 3, y, w - tw - gap * 3, h), 3, gap, horizontal=False)):
            m = ms[i]
            c.text(bx, by, bw * 0.45, bh, m.label, p.acc[i % 3], bold=True, pre="l", group="hl", maxsize=bh * 0.5)
            c.value(m.src, m.fmt, bx + bw * 0.45, by, bw * 0.55, bh, m.sample, p.text, bold=True, align="right", pre="v", group="hv", maxsize=bh * 0.7)
        return c.finish()
    c.value("time", "%H:%M", x, y, w, h * 0.46, "88:88", p.text, bold=True, align="center", pre="time")
    c.value("date", "%A %d %B", x, y + h * 0.48, w, h * 0.1, "mercredi 30 septembre", p.text2, align="center", pre="date")
    for i, (bx, by, bw, bh) in enumerate(split((x, y + h * 0.68, w, h * 0.32), 3, gap)):
        m = ms[i]
        c.card(bx, by, bw, bh)
        c.text(bx + 2, by + bh * 0.08, bw - 4, bh * 0.3, m.label, p.acc[i % 3], bold=True, align="center", pre="l", group="hl", maxsize=bh * 0.28)
        c.value(m.src, m.fmt, bx + 2, by + bh * 0.42, bw - 4, bh * 0.5, m.sample, p.text, bold=True, align="center", pre="v", group="hv")
    return c.finish()
