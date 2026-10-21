"""LCD 7 segments : fenetres a segments eteints (chiffres fantomes 8) et valeurs allumees, plaque grise."""
import re

from lib import Canvas, fit_size, grid
from layouts.sty_common import ALL8, REQ, fit, ghost, need

NAME, TITLE = "sty-lcd", "LCD sept segments"
CONTENTS = ALL8
MONO = "Roboto Mono"


def _win(c, box, m, i, lab_h, size):
    p = c.p
    x, y, w, h = box
    col = p.acc[i % 3]
    sub = m.sub and (m.src in REQ or w > 4.2 * lab_h * 1.5)
    with c.m(m):
        c.rect(x, y, w, h, p.bg0, min(h * 0.12, 10), stroke=p.text2, sw=2, pre="win")
        c.text(x + w * 0.05, y + h * 0.04, w * (0.45 if sub else 0.9), lab_h, m.label, p.text2, bold=True, pre="l", group="ll", maxsize=lab_h)
        if sub:
            c.value(m.sub[0], m.sub[1], x + w * 0.5, y + h * 0.04, w * 0.45, lab_h, m.sub[2], p.text2, align="right", pre="s", group="ls", maxsize=lab_h)
        vx, vy, vw, vh = x + w * 0.05, y + lab_h + h * 0.06, w * 0.9, h - lab_h - h * 0.12
        sample = re.sub(r"\d", "8", m.sample.replace("°C", "°"))
        ghost(c, vx, vy, vw, vh, sample, col, 0.13, MONO, True, "right", size)
        c.value(m.src, m.fmt, vx, vy, vw, vh, m.sample, col, MONO, True, "right", size=size, pre="v")


def _draw(p, fmt, ms):
    c = Canvas(p, fmt)
    x, y, w, h = fmt.zone
    gap = fmt.short * 0.03
    if not fmt.round:
        c.rect(x - gap * 0.6, y - gap * 0.6, w + gap * 1.2, h + gap * 1.2, p.card, min(fmt.short * 0.06, 28), stroke=p.line, sw=2, pre="plate")
    hh = 0.0
    if not fmt.strip and fmt.short > 240 and not fmt.round:
        hh = h * 0.22
        c.rect(x, y, w, hh, p.bg0, min(hh * 0.1, 10), stroke=p.text2, sw=2, pre="tw")
        s = fit_size("88:88", MONO, True, w * 0.7, hh * 0.8)
        ghost(c, x, y + hh * 0.1, w * 0.62, hh * 0.8, "88:88", p.acc[1], 0.13, MONO, True, "right", s)
        c.value("time", "%H:%M", x, y + hh * 0.1, w * 0.62, hh * 0.8, "88:88", p.acc[1], MONO, True, "right", size=s, pre="tm")
        c.text(x + w * 0.66, y + hh * 0.12, w * 0.3, hh * 0.3, "HEURE", p.text2, bold=True, pre="hl", maxsize=hh * 0.26)
        c.value("date", "%d/%m/%Y", x + w * 0.66, y + hh * 0.46, w * 0.3, hh * 0.38, "88/88/8888", p.text, MONO, True, pre="dt", maxsize=hh * 0.34)
        hh += gap
    cells = grid((x, y + hh, w, h - hh), len(ms), gap, 1.9 if not fmt.strip else 1.2)
    lab = max(cells[0][3] * 0.2, need(fmt) * 0.95)
    lab = min(lab, cells[0][3] * 0.34)
    cw = cells[0][2] * 0.9
    ch = cells[0][3] - lab - cells[0][3] * 0.12
    size = min(fit_size(re.sub(r"\d", "8", m.sample.replace("°C", "°")), MONO, True, cw, ch) for m in ms)
    for i, b in enumerate(cells):
        _win(c, b, ms[i], i, lab, size)
    return c.finish()


def build(p, fmt, content):
    return fit(p, fmt, content, _draw)
