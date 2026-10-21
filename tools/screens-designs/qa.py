"""Controle qualite automatique : geometrie, chevauchement de textes, contraste des palettes."""
import math

MAX_WIDGETS = 200
TOL = 2.0


def _lum(hexc):
    h = hexc.lstrip("#")[:6]
    ch = []
    for i in (0, 2, 4):
        v = int(h[i:i + 2], 16) / 255
        ch.append(v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4)
    return 0.2126 * ch[0] + 0.7152 * ch[1] + 0.0722 * ch[2]


def contrast(a, b):
    la, lb = sorted((_lum(a), _lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def mix(a, b):
    ha, hb = a.lstrip("#")[:6], b.lstrip("#")[:6]
    return "#" + "".join(f"{(int(ha[i:i+2], 16) + int(hb[i:i+2], 16)) // 2:02x}" for i in (0, 2, 4))


def palette_problems(p):
    """Texte >= 4.5 (fond ET milieu de degrade), texte2 >= 3.2, accents >= 3, rampe >= 2.6, piste visible."""
    out = []
    for bg in (p.bg0, p.bg1, mix(p.bg0, p.bg1)):
        if contrast(p.text, bg) < 4.5:
            out.append(f"texte/fond {contrast(p.text, bg):.1f}")
        if contrast(p.text2, bg) < 3.5:
            out.append(f"texte2/fond {contrast(p.text2, bg):.1f}")
        for k, a in enumerate(p.acc):
            if contrast(a, bg) < 3.0:
                out.append(f"accent{k + 1}/fond {contrast(a, bg):.1f}")
        for k, a in enumerate(p.ramp):
            if contrast(a, bg) < 2.1:
                out.append(f"rampe{k + 1}/fond {contrast(a, bg):.1f}")
    if contrast(p.track, p.bg1) < 1.12:
        out.append("piste invisible")
    return sorted(set(out))


def _bbox(w):
    x, y, ww, h = w["x"], w["y"], w["w"], w["h"]
    r = w.get("rotation", 0)
    if not r:
        return x, y, x + ww, y + h
    a = math.radians(r)
    ex = (abs(ww * math.cos(a)) + abs(h * math.sin(a))) / 2
    ey = (abs(ww * math.sin(a)) + abs(h * math.cos(a))) / 2
    cx, cy = x + ww / 2, y + h / 2
    return cx - ex, cy - ey, cx + ex, cy + ey


def _in_circle(px, py, fmt, slack):
    r = fmt.w / 2
    return math.hypot(px - fmt.w / 2, py - fmt.h / 2) <= r + slack


def problems(widgets, fmt):
    out = list(widgets.flags)
    if not widgets:
        return ["design vide"]
    if len(widgets) > MAX_WIDGETS:
        out.append(f"{len(widgets)} widgets")
    ids = [w["id"] for w in widgets]
    if len(set(ids)) != len(ids):
        out.append("ids dupliques")
    for w in widgets:
        x0, y0, x1, y1 = _bbox(w)
        if x0 < -TOL or y0 < -TOL or x1 > fmt.w + TOL or y1 > fmt.h + TOL:
            out.append(f"{w['id']} hors du canevas")
        if fmt.round and w["type"] != "text" and w["type"] != "value":
            if w["type"] == "gauge_arc":
                if math.hypot(x0 + w["w"] / 2 - fmt.w / 2, y0 + w["h"] / 2 - fmt.h / 2) + w["w"] / 2 > fmt.w / 2 + TOL:
                    out.append(f"{w['id']} deborde du cercle")
            else:
                inset_ = min(w["w"], w["h"]) * 0.15 * (1 if w["type"] == "rect" else 0)
                for px, py in ((x0 + inset_, y0 + inset_), (x1 - inset_, y0 + inset_), (x0 + inset_, y1 - inset_), (x1 - inset_, y1 - inset_)):
                    if not _in_circle(px, py, fmt, TOL):
                        out.append(f"{w['id']} deborde du cercle")
                        break
    inks = widgets.inks
    for wid, x0, y0, x1, y1, _s in inks:
        if x0 < -TOL or y0 < -TOL or x1 > fmt.w + TOL or y1 > fmt.h + TOL:
            out.append(f"{wid} texte hors du canevas")
        if fmt.round and not all(_in_circle(px, py, fmt, -6) for px in (x0, x1) for py in (y0, y1)):
            out.append(f"{wid} texte hors du cercle")
    for i, a in enumerate(inks):
        for b in inks[i + 1:]:
            ox, oy = min(a[3], b[3]) - max(a[1], b[1]), min(a[4], b[4]) - max(a[2], b[2])
            if ox > 1.5 and oy > 1.5:
                out.append(f"textes {a[0]}/{b[0]} se chevauchent")
    return sorted(set(out))
