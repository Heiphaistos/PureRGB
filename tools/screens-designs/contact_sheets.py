"""Planches-contact des PNG rendus par le test Rust (revue visuelle).

    python tools/screens-designs/contact_sheets.py [--src DIR] [--out DIR] [--only duo,hud] [--tile 230]

Une planche par archetype (gen-<archetype>-...) ; les designs sont etiquetes par id.
"""
import argparse
import os

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_SRC = os.path.join(HERE, "..", "..", "src-tauri", "target", "screens-preview", "designs")
NAMES = sorted((f[:-3] for f in os.listdir(os.path.join(HERE, "layouts")) if f.endswith(".py") and f != "__init__.py"), key=len, reverse=True)


def arch_of(stem):
    rest = stem[4:]
    return next((n for n in NAMES if rest.startswith(n + "-")), "?")


def sheet(files, out, tile, width=1700):
    font = ImageFont.load_default()
    items = []
    for f in files:
        im = Image.open(f).convert("RGB")
        k = tile / im.height
        w = min(int(im.width * k), width // 2 - 10)
        k = min(k, w / im.width)
        items.append((os.path.basename(f)[4:-4], im.resize((max(1, int(im.width * k)), max(1, int(im.height * k))), Image.LANCZOS)))
    rows, cur, x = [], [], 0
    for it in items:
        if x + it[1].width > width and cur:
            rows.append(cur)
            cur, x = [], 0
        cur.append(it)
        x += it[1].width + 10
    rows.append(cur)
    H = sum(max(i[1].height for i in r) + 16 for r in rows)
    canvas = Image.new("RGB", (width, H), (60, 60, 64))
    d = ImageDraw.Draw(canvas)
    y = 0
    for r in rows:
        x = 0
        for name, im in r:
            canvas.paste(im, (x, y + 14))
            d.text((x + 2, y + 1), name[:int((im.width + 20) / 6)], fill=(235, 235, 235), font=font)
            x += im.width + 10
        y += max(i[1].height for i in r) + 16
    canvas.save(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=DEFAULT_SRC)
    ap.add_argument("--out", default=os.path.join(HERE, "..", "..", "src-tauri", "target", "screens-preview", "sheets"))
    ap.add_argument("--only", default="")
    ap.add_argument("--tile", type=int, default=230)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    only = set(filter(None, a.only.split(",")))
    groups = {}
    for f in sorted(os.listdir(a.src)):
        if f.startswith("gen-") and f.endswith(".png"):
            groups.setdefault(arch_of(f[:-4]), []).append(os.path.join(a.src, f))
    for arch, files in groups.items():
        if only and arch not in only:
            continue
        for part in range(0, len(files), 16):
            out = os.path.join(a.out, f"{arch}-{part // 16 + 1}.png")
            sheet(files[part:part + 16], out, a.tile)
            print(out)


if __name__ == "__main__":
    main()
