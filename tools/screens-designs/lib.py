"""Helpers de construction : Canvas (widgets), mesure de texte, grilles, ecriture JSON stable."""
import json
import math
import os
from PIL import ImageFont

FONT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "src-tauri", "assets", "fonts")
MIN_TEXT = 15          # taille mini lisible (unites du canevas)
LINE_H = 1.15          # hauteur de boite / taille de police
CAP = 0.72             # hauteur des capitales / taille (encre approximative)

_cache = {}


def _font(name, bold):
    k = (name, bold)
    if k not in _cache:
        w = 700 if bold else 400
        if name == "Rajdhani":
            f = ImageFont.truetype(os.path.join(FONT_DIR, "Rajdhani-Bold.ttf" if bold else "Rajdhani-Medium.ttf"), 100)
        elif name == "Roboto Mono":
            f = ImageFont.truetype(os.path.join(FONT_DIR, "RobotoMono.ttf"), 100)
            f.set_variation_by_axes([w])
        else:
            f = ImageFont.truetype(os.path.join(FONT_DIR, "Inter.ttf"), 100)
            f.set_variation_by_axes([14, w])
        _cache[k] = f
    return _cache[k]


def measure(text, font, bold, size):
    """Largeur du texte a la taille `size` (unites du canevas), avec 3 % de marge."""
    return _font(font, bold).getlength(text) * size / 100.0 * 1.03


def fit_size(text, font, bold, w, h, maxsize=None):
    s = h / LINE_H
    if maxsize:
        s = min(s, maxsize)
    mw = measure(text, font, bold, 100) or 1
    return round(min(s, 100.0 * w / mw), 1)


def n(v):
    v = round(float(v), 1)
    return int(v) if v == int(v) else v


class Widgets(list):
    """Liste de widgets (dicts) + encre des textes pour le controle qualite."""
    inks: list
    flags: list


class Canvas:
    def __init__(self, p, fmt):
        self.p, self.f = p, fmt
        self.out = Widgets()
        self.out.inks, self.out.flags = [], []
        self._n = {}
        self._groups = {}

    # -- ids -------------------------------------------------------------
    def _id(self, pre):
        self._n[pre] = self._n.get(pre, 0) + 1
        return f"{pre}{self._n[pre]}"

    def _add(self, pre, x, y, w, h, **kw):
        d = {"id": self._id(pre), "x": n(x), "y": n(y), "w": n(w), "h": n(h)}
        rot = kw.pop("rotation", 0)
        if rot:
            d["rotation"] = n(rot)
        op = kw.pop("opacity", 1)
        if op != 1:
            d["opacity"] = op
        d.update(kw)
        self.out.append(d)
        return d

    # -- texte -----------------------------------------------------------
    def _style(self, font, size, color, align, bold):
        return {"font": font, "size": n(size), "color": color, "align": align, "bold": bold}

    def _text(self, d, ink_args, group):
        txt, font, bold, align = ink_args
        ink = self._ink_rect(d, txt, font, bold, align, d["style"]["size"], d["id"])
        self.out.inks.append(ink)
        if group:
            self._groups.setdefault(group, []).append((d, ink, ink_args))

    def text(self, x, y, w, h, txt, color=None, font=None, bold=False, align="left", size=None, maxsize=None, pre="t", group=None):
        font = font or self.p.font
        color = color or self.p.text
        s = fit_size(txt, font, bold, w, h, maxsize) if size is None else size
        d = self._add(pre, x, y, w, h, type="text", text=txt, style=self._style(font, s, color, align, bold))
        self._text(d, (txt, font, bold, align), group if size is None else None)
        return s

    def value(self, src, fmt, x, y, w, h, sample, color=None, font=None, bold=False, align="left", size=None, maxsize=None, pre="v", group=None):
        font = font or self.p.font
        color = color or self.p.text
        if font != "Inter" and "°C" in fmt:   # Rajdhani / Roboto Mono : "°C" s'ecarte, on garde le degre seul
            deg = "˚" if font == "Rajdhani" else "°"   # Rajdhani : anneau compact (le degre y a de larges chasses)
            fmt, sample = fmt.replace("°C", deg), sample.replace("°C", deg)
        s = fit_size(sample, font, bold, w, h, maxsize) if size is None else size
        d = self._add(pre, x, y, w, h, type="value", source=src, format=fmt, style=self._style(font, s, color, align, bold))
        self._text(d, (sample, font, bold, align), group if size is None else None)
        return s

    # -- formes ----------------------------------------------------------
    def rect(self, x, y, w, h, fill, radius=0, stroke=None, sw=0, opacity=1, rotation=0, pre="r"):
        return self._add(pre, x, y, w, h, type="rect", fill=fill, stroke=stroke, stroke_width=sw, radius=n(radius), opacity=opacity, rotation=rotation)

    def card(self, x, y, w, h, radius=None, pre="c"):
        r = min(w, h) * 0.14 if radius is None else radius
        return self.rect(x, y, w, h, self.p.card, min(r, 28), self.p.line, 1, pre=pre)

    def arc(self, cx, cy, r, src, lo, hi, thick, colors, start=135, sweep=270, track=None, pre="a"):
        return self._add(pre, cx - r, cy - r, 2 * r, 2 * r, type="gauge_arc", source=src, min=lo, max=hi, start_deg=start,
                         sweep_deg=sweep, thickness=n(thick), track=track or self.p.track, colors=colors)

    def bar(self, x, y, w, h, src, lo, hi, colors, vertical=False, radius=None, track=None, pre="b"):
        r = min(w, h) / 2 if radius is None else radius
        return self._add(pre, x, y, w, h, type="bar", source=src, min=lo, max=hi, vertical=vertical, radius=n(r),
                         track=track or self.p.track, colors=colors)

    def graph(self, x, y, w, h, src, lo, hi, line, fill=None, seconds=60, thick=2, pre="g"):
        return self._add(pre, x, y, w, h, type="graph", source=src, min=lo, max=hi, seconds=seconds, line=line, fill=fill, thickness=thick)

    def clock(self, x, y, w, h, face=None, hands=None, accent=None, pre="k"):
        p = self.p
        return self._add(pre, x, y, w, h, type="analog_clock", face=face or p.card, hands=hands or p.text, accent=accent or p.acc[0])

    def finish(self):
        """Harmonise les tailles de chaque groupe (taille mini du groupe) puis signale les textes trop petits."""
        for ents in self._groups.values():
            s = min(d["style"]["size"] for d, _, _ in ents)
            for d, ink, (txt, font, bold, align) in ents:
                d["style"]["size"] = s
                ink[:] = self._ink_rect(d, txt, font, bold, align, s, ink[0])
        for ink in self.out.inks:
            if ink[5] < MIN_TEXT:
                self.out.flags.append(f"texte trop petit ({ink[5]}) : {ink[0]}")
        return self.out

    @staticmethod
    def _ink_rect(d, txt, font, bold, align, size, wid):
        x, y, w, h = d["x"], d["y"], d["w"], d["h"]
        tw = min(measure(txt, font, bold, size), w)
        x0 = x if align == "left" else x + (w - tw) / 2 if align == "center" else x + w - tw
        cy, ch = y + h / 2, size * CAP / 2
        return [wid, x0, cy - ch, x0 + tw, cy + ch, size]


# -- grilles ----------------------------------------------------------------
def grid(zone, count, gap, target=1.25):
    """Cellules (x, y, w, h) d'une grille cols x rows dont l'aspect de cellule se rapproche de `target`."""
    zx, zy, zw, zh = zone
    best = None
    for cols in range(1, count + 1):
        rows = math.ceil(count / cols)
        cw, ch = (zw - gap * (cols - 1)) / cols, (zh - gap * (rows - 1)) / rows
        if cw <= 0 or ch <= 0:
            continue
        score = abs(math.log((cw / ch) / target)) + (0.15 if cols * rows != count else 0)
        if best is None or score < best[0]:
            best = (score, cols, rows, cw, ch)
    _, cols, rows, cw, ch = best
    cells = []
    for i in range(count):
        r, c = divmod(i, cols)
        # derniere ligne incomplete : cellules centrees
        in_row = cols if r < rows - 1 or count % cols == 0 else count % cols
        off = (cols - in_row) * (cw + gap) / 2 if r == rows - 1 else 0
        cells.append((zx + off + c * (cw + gap), zy + r * (ch + gap), cw, ch))
    return cells


def split(zone, count, gap, horizontal=True):
    """Decoupe la zone en `count` bandes egales (colonnes si horizontal)."""
    zx, zy, zw, zh = zone
    if horizontal:
        cw = (zw - gap * (count - 1)) / count
        return [(zx + i * (cw + gap), zy, cw, zh) for i in range(count)]
    ch = (zh - gap * (count - 1)) / count
    return [(zx, zy + i * (ch + gap), zw, ch) for i in range(count)]


def inset(box, m):
    x, y, w, h = box
    return (x + m, y + m, w - 2 * m, h - 2 * m)


# -- sortie -----------------------------------------------------------------
def layout_json(name, fmt, palette, widgets):
    """JSON compact et stable : un widget par ligne."""
    dump = lambda o: json.dumps(o, ensure_ascii=False, separators=(",", ":"))
    head = {"version": 1, "name": name, "width": fmt.w, "height": fmt.h, "background": palette.background}
    s = dump(head)[:-1] + ',"widgets":[\n'
    s += ",\n".join(dump(w) for w in widgets)
    return s + "\n]}\n"
