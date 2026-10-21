"""Thèmes vidéo « Forge » : la vidéo intégrée (src-tauri/assets/media/forge-video.mp4) en fond,
avec le socle obligatoire CPU / GPU / RAM / SSD 1 / SSD 2 (masqué si absent) + heure et date.

    python tools/themes/forge_themes.py     -> src-tauri/src/screens/designs/themes/*.json + themes.rs
"""
import hashlib
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
OUT = os.path.join(ROOT, "src-tauri", "src", "screens", "designs", "themes")
MP4 = os.path.join(ROOT, "src-tauri", "assets", "media", "forge-video.mp4")
MEDIA_ID = hashlib.sha256(open(MP4, "rb").read()).hexdigest()[:16]  # même règle que MediaStore::import

TXT, TXT2, LINE = "#ffffff", "#c7d2fe", "#ffffff33"
CARD = "#05080fb3"
RAMP = [[0.0, "#34d399"], [0.65, "#fbbf24"], [1.0, "#f87171"]]


class C:
    def __init__(self):
        self.w, self.n, self.cond = [], 0, None

    def add(self, x, y, w, h, type_, **k):
        self.n += 1
        d = {"id": f"w{self.n}", "x": round(x, 1), "y": round(y, 1), "w": round(w, 1), "h": round(h, 1), "type": type_, **k}
        if self.cond:
            d["visible_if"] = self.cond
        self.w.append(d)

    def text(self, x, y, w, h, t, size, color=TXT2, align="left", font="Rajdhani", bold=True):
        self.add(x, y, w, h, "text", text=t, style={"font": font, "size": size, "color": color, "align": align, "bold": bold})

    def val(self, x, y, w, h, src, fmt, size, color=TXT, align="left", font="Rajdhani", bold=True):
        self.add(x, y, w, h, "value", source=src, format=fmt, style={"font": font, "size": size, "color": color, "align": align, "bold": bold})

    def bar(self, x, y, w, h, src, colors=None):
        self.add(x, y, w, h, "bar", source=src, min=0, max=100, vertical=False, radius=h / 2, track="#ffffff2e", colors=colors or RAMP)

    def rect(self, x, y, w, h, fill=CARD, r=24):
        self.add(x, y, w, h, "rect", fill=fill, stroke="#ffffff2a", stroke_width=2, radius=r)

    def when(self, key):
        self.cond = key
        return self


def layout(name, w, h, c):
    return {"version": 1, "name": name, "width": w, "height": h,
            "background": {"kind": "media", "media_id": MEDIA_ID, "fit": "cover"}, "widgets": c.w}


def metric_row(c, x, y, w, label, temp, load, bar_src, size=30):
    """Ligne : libellé | température | charge, barre dessous."""
    c.text(x, y, w * 0.2, size * 1.2, label, size * 0.85)
    if temp:
        c.val(x + w * 0.2, y - size * 0.1, w * 0.3, size * 1.35, temp, "{:.0}°", size * 1.1, align="right")
    if load:
        c.val(x + w * 0.52, y - size * 0.1, w * 0.3, size * 1.35, load, "{:.0} %", size, TXT2, align="right")
    c.bar(x, y + size * 1.35, w, max(8, size * 0.28), bar_src)


def right_card(name, W, H, size):
    c = C()
    cx, cw = W * 0.585, W * 0.395
    c.rect(cx, H * 0.07, cw, H * 0.86)
    x, w = cx + cw * 0.05, cw * 0.9
    c.val(x, H * 0.10, w * 0.5, H * 0.2, "time", "%H:%M", size * 3, TXT)
    c.val(x + w * 0.42, H * 0.12, w * 0.58, H * 0.14, "date", "%A %d %B %Y", size * 1.05, TXT2, "right", "Inter", False)
    y = H * 0.33
    step = H * 0.125
    metric_row(c, x, y, w, "CPU", "cpu.temp", "cpu.load", "cpu.load", size)
    metric_row(c, x, y + step, w, "GPU", "gpu.temp", "gpu.load", "gpu.load", size)
    metric_row(c, x, y + 2 * step, w, "RAM", None, "ram.used_pct", "ram.used_pct", size)
    metric_row(c, x, y + 3 * step, w, "SSD 1", "ssd.1.temp", "ssd.1.used_pct", "ssd.1.used_pct", size)
    c.when("ssd.2.used_pct")
    metric_row(c, x, y + 4 * step, w, "SSD 2", "ssd.2.temp", "ssd.2.used_pct", "ssd.2.used_pct", size)
    return layout(name, W, H, c)


def bottom_strip(name, W, H, size):
    c = C()
    c.rect(W * 0.02, H * 0.60, W * 0.96, H * 0.36, r=22)
    c.val(W * 0.035, H * 0.04, W * 0.25, H * 0.3, "time", "%H:%M", size * 3.2, TXT)
    c.val(W * 0.74, H * 0.06, W * 0.235, H * 0.2, "date", "%A %d %B %Y", size * 1.0, TXT2, "right", "Inter", False)
    cols = [("CPU", "cpu.temp", "cpu.load", "cpu.load"), ("GPU", "gpu.temp", "gpu.load", "gpu.load"),
            ("RAM", None, "ram.used_pct", "ram.used_pct"), ("SSD 1", "ssd.1.temp", "ssd.1.used_pct", "ssd.1.used_pct"),
            ("SSD 2", "ssd.2.temp", "ssd.2.used_pct", "ssd.2.used_pct")]
    cw = W * 0.92 / 5
    for i, (lab, t, l, b) in enumerate(cols):
        c.when("ssd.2.used_pct" if lab == "SSD 2" else None)
        x = W * 0.04 + i * cw
        c.text(x, H * 0.63, cw * 0.9, size * 1.1, lab, size * 0.8)
        if t:
            c.val(x, H * 0.70, cw * 0.55, size * 1.7, t, "{:.0}°", size * 1.6, TXT)
            c.val(x + cw * 0.5, H * 0.72, cw * 0.4, size * 1.5, l, "{:.0} %", size * 1.1, TXT2, "right")
        else:
            c.val(x, H * 0.70, cw * 0.9, size * 1.7, l, "{:.0} %", size * 1.6, TXT)
        c.bar(x, H * 0.89, cw * 0.88, max(8, size * 0.3), b)
    return layout(name, W, H, c)


def corners(name, W, H, size):
    c = C()
    cw, ch = W * 0.2, H * 0.4
    c.rect(W * 0.02, H * 0.06, cw, ch)
    c.val(W * 0.03, H * 0.1, cw * 0.9, H * 0.22, "time", "%H:%M", size * 2.6, TXT)
    c.val(W * 0.03, H * 0.33, cw * 0.9, H * 0.1, "date", "%a %d %b %Y", size * 0.85, TXT2, "left", "Inter", False)
    quads = [(W * 0.78, H * 0.06, "CPU", "cpu.temp", "cpu.load"), (W * 0.78, H * 0.54, "GPU", "gpu.temp", "gpu.load")]
    for x, y, lab, t, l in quads:
        c.rect(x, y, cw, ch)
        c.text(x + cw * 0.06, y + ch * 0.06, cw * 0.5, size, lab, size * 0.85)
        c.val(x + cw * 0.06, y + ch * 0.25, cw * 0.55, size * 2, t, "{:.0}°", size * 1.9, TXT)
        c.val(x + cw * 0.5, y + ch * 0.3, cw * 0.44, size * 1.4, l, "{:.0} %", size * 1.1, TXT2, "right")
        c.bar(x + cw * 0.06, y + ch * 0.82, cw * 0.88, max(8, size * 0.28), l)
    c.rect(W * 0.02, H * 0.54, cw, ch)
    rows = [("RAM", None, "ram.used_pct"), ("SSD 1", "ssd.1.temp", "ssd.1.used_pct"), ("SSD 2", "ssd.2.temp", "ssd.2.used_pct")]
    for i, (lab, t, l) in enumerate(rows):
        c.when("ssd.2.used_pct" if lab == "SSD 2" else None)
        yy = H * 0.57 + i * H * 0.12
        c.text(W * 0.032, yy, cw * 0.3, size, lab, size * 0.78)
        if t:
            c.val(W * 0.02 + cw * 0.3, yy - 2, cw * 0.3, size * 1.1, t, "{:.0}°", size * 0.95, TXT, "right")
        c.val(W * 0.02 + cw * 0.58, yy - 2, cw * 0.36, size * 1.1, l, "{:.0} %", size * 0.95, TXT2, "right")
        c.bar(W * 0.032, yy + size * 1.1, cw * 0.88, max(6, size * 0.2), l)
    return layout(name, W, H, c)


THEMES = [
    ("theme-forge-droite-u1920", "Forge · cartes à droite", right_card, 1920, 462, 32, ["u1920", "ultra-large", "fond libre"]),
    ("theme-forge-bandeau-u1920", "Forge · bandeau bas", bottom_strip, 1920, 462, 30, ["u1920", "ultra-large", "bandeau"]),
    ("theme-forge-coins-u1920", "Forge · coins", corners, 1920, 462, 30, ["u1920", "ultra-large", "coins"]),
    ("theme-forge-droite-w1280", "Forge · cartes à droite (1280)", right_card, 1280, 480, 26, ["w1280", "large"]),
    ("theme-forge-bandeau-w1280", "Forge · bandeau bas (1280)", bottom_strip, 1280, 480, 22, ["w1280", "large", "bandeau"]),
    ("theme-forge-carre-s480", "Forge · carré", bottom_strip, 480, 480, 14, ["s480", "carré"]),
]


def main():
    os.makedirs(OUT, exist_ok=True)
    for f in os.listdir(OUT):
        if f.endswith(".json"):
            os.remove(os.path.join(OUT, f))
    items = []
    for did, name, fn, w, h, size, tags in THEMES:
        d = fn(name, w, h, size)
        with open(os.path.join(OUT, f"{did}.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(d, f, ensure_ascii=False, indent=1)
        items.append((did, tags))
    rs = ["//! Thèmes vidéo intégrés (générés par `tools/themes/forge_themes.py`) : fond = vidéo embarquée",
          "//! (`builtin_media`), socle CPU / GPU / RAM / SSD 1-2 + heure et date.", "",
          "use super::builtin::Builtin;", "", "pub const LIST: &[Builtin] = &["]
    for did, tags in items:
        t = ", ".join(f'"{x}"' for x in ["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date"] + tags)
        rs += ["    Builtin {", f'        id: "{did}",', '        category: "Thèmes vidéo",', f"        tags: &[{t}],",
               f'        json: include_str!("themes/{did}.json"),', "    },"]
    rs += ["];", ""]
    with open(os.path.join(OUT, "..", "themes.rs"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(rs))
    print(len(items), "thèmes, media", MEDIA_ID)


if __name__ == "__main__":
    main()
