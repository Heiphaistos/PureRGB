"""Panneaux par famille : titre, valeur vedette + barre, puis lignes de détail dans une carte."""
from lib import Canvas, min_text
from layouts.cap_common import ALL, GROUPS, boxes, crow, densest, fracs

NAME, TITLE = "cap-panneaux", "Panneaux détaillés"
CONTENTS = ALL
FORMATS = ["s480", "w169", "w1280", "u1920", "r43", "s320"]


def build(p, fmt, content):
    groups = GROUPS[content]
    mt = min_text(fmt)

    def mk(k):
        c = Canvas(p, fmt)
        gap = fmt.short * 0.03
        bxs, _ = boxes(fmt, len(groups), gap)
        for g, ((title, ms), box) in enumerate(zip(groups, bxs)):
            c.card(*box)
            pad = min(box[2], box[3]) * 0.07
            x, y, w, h = box[0] + pad, box[1] + pad, box[2] - 2 * pad, box[3] - 2 * pad
            th = max(1.3 * mt, h * 0.1)
            c.text(x, y, w, th, title, p.acc[g % 3], bold=True, pre="h", group="ph")
            m, bh = ms[0], max(4.0, h * 0.03)
            kk = min(k, len(ms) - 1)
            lh = 1.2 * mt                                    # libellé de la valeur vedette
            avail = h - th * 1.1 - lh - bh - h * 0.1
            vh = min(avail - kk * 1.9 * mt, h * 0.5)
            if vh < max(1.5 * mt, h * 0.12):
                return None
            with c.m(m):
                c.text(x, y + th * 1.1, w, lh, m.label, p.text2, bold=True, pre="l", group="pl")
                c.value(m.src, m.fmt, x, y + th * 1.1 + lh, w, vh, m.sample, p.text, bold=True, pre="v", group="pv")
                c.bar(x, y + th * 1.1 + lh + vh * 1.05, w, bh, m.src, m.lo, m.hi, m.colors(p, g), pre="b")
            top = y + th * 1.1 + lh + vh * 1.05 + bh + h * 0.05
            if kk:
                rh = min((y + h - top) / kk, fmt.short * 0.12)
                fr = fracs(p, ms[1:][:kk])
                for r, mm in enumerate(ms[1:][:kk]):
                    with c.m(mm):
                        crow(c, (x, top + r * rh, w, rh * 0.94), mm, g + r, fr, bar=False)
        return c.finish()
    return densest(mk, fmt, range(max(len(ms) for _, ms in groups) - 1, -1, -1), floor=0)
