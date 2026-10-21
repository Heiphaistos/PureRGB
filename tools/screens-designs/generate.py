"""Generateur de designs d'ecran : archetype x contenu x palette x format -> JSON + generated.rs.

Usage (depuis la racine du depot) :
    python tools/screens-designs/generate.py [--per 1] [--only duo,hud] [--dry] [--report]
"""
import argparse
import importlib
import os
import pkgutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import layouts  # noqa: E402
import qa  # noqa: E402
from content import CONTENT  # noqa: E402
from formats import FORMATS  # noqa: E402
from lib import layout_json  # noqa: E402
from palettes import PALETTES  # noqa: E402

DESIGNS = os.path.abspath(os.path.join(HERE, "..", "..", "src-tauri", "src", "screens", "designs"))
GEN_DIR = os.path.join(DESIGNS, "gen")
CONTENT_FR = {
    "cpu-gpu": "CPU et GPU", "quad": "Vue d'ensemble", "disques": "Disques", "carte-mere": "Carte mère",
    "reseau": "Réseau et E/S", "refroidissement": "Refroidissement", "gpu": "GPU détaillé", "cpu": "CPU détaillé",
    "puissance": "Puissance", "liquide": "Liquide", "ram": "Mémoire",
}


def load_content_extensions():
    """Fichiers `content_ext_*.py` : jeux de contenu supplementaires (CONTENT_EXT, CONTENT_FR_EXT),
    un par contributeur, pour eviter les conflits sur content.py."""
    for info in sorted(pkgutil.iter_modules([HERE]), key=lambda i: i.name):
        if info.name.startswith("content_ext_"):
            m = importlib.import_module(info.name)
            CONTENT.update(getattr(m, "CONTENT_EXT", {}))
            CONTENT_FR.update(getattr(m, "CONTENT_FR_EXT", {}))


load_content_extensions()


def modules():
    out = []
    for info in pkgutil.iter_modules(layouts.__path__):
        m = importlib.import_module(f"layouts.{info.name}")
        if hasattr(m, "build"):
            out.append(m)
    return sorted(out, key=lambda m: m.NAME)


def good_palettes():
    return [p for p in PALETTES.values() if not qa.palette_problems(p)]


def make(mod, content, p, fmt):
    widgets = mod.build(p, fmt, content)
    if widgets is None:
        return None, ["non pertinent"]
    return widgets, qa.problems(widgets, fmt)


def plan(per, fpc, only):
    """[(module, content, fmt, palette, widgets)].

    Par (archetype, contenu) : `fpc` formats valides (depart tournant), `per` palettes chacun
    (palette tournante, repli sur la suivante si le controle qualite echoue)."""
    pals = good_palettes()
    mods = [m for m in modules() if not only or m.NAME in only]
    res, rejected, k = [], [], 0
    for mod in mods:
        pool = [p for p in pals if getattr(mod, "accepts", lambda p: True)(p)]
        fkeys = list(getattr(mod, "FORMATS", None) or FORMATS)
        for content in getattr(mod, "CONTENTS", ["cpu-gpu"]):
            done = 0
            for t in range(len(fkeys)):
                if done >= fpc:
                    break
                fmt = FORMATS[fkeys[(k + t) % len(fkeys)]]
                used = []
                for attempt in range(3):
                    if len(used) >= per:
                        break
                    p = pool[(k * 7 + attempt * 11) % len(pool)]
                    if p.key in used:
                        continue
                    w, probs = make(mod, content, p, fmt)
                    if w is None:
                        break
                    if probs:
                        rejected.append((mod.NAME, content, p.key, fmt.key, probs))
                        continue
                    used.append(p.key)
                    res.append((mod, content, fmt, p, w))
                k += 1
                done += bool(used)
            k += 1
    return res, rejected


def design_id(mod, content, p, fmt):
    arch = mod.NAME if len(getattr(mod, "CONTENTS", [0])) == 1 else f"{mod.NAME}-{content}"
    return f"gen-{arch}-{p.key}-{fmt.key}"


def rust_list(items):
    lines = ["//! GENERE par tools/screens-designs/generate.py : ne pas editer a la main.", "",
             "use super::builtin::Builtin;", "", "pub const LIST: &[Builtin] = &["]
    for did, cat, tags, _ in sorted(items):
        t = ", ".join(f'"{x}"' for x in tags)
        lines.append(f'    Builtin {{ id: "{did}", category: "{cat}", tags: &[{t}], json: include_str!("gen/{did}.json") }},')
    lines.append("];")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per", type=int, default=1, help="palettes par (archetype, contenu, format)")
    ap.add_argument("--fpc", type=int, default=3, help="formats par (archetype, contenu)")
    ap.add_argument("--only", default="", help="archetypes (NAME) separes par des virgules")
    ap.add_argument("--dry", action="store_true", help="n'ecrit rien")
    ap.add_argument("--report", action="store_true", help="liste les rejets QA")
    a = ap.parse_args()
    for p in PALETTES.values():
        pr = qa.palette_problems(p)
        if pr:
            print(f"palette ecartee {p.key} : {', '.join(pr)}")
    res, rejected = plan(a.per, a.fpc, set(filter(None, a.only.split(","))))
    if a.report:
        seen_r = set()
        for r in rejected:
            key = (r[0], r[1], r[3], tuple(r[4]))
            if key not in seen_r:
                seen_r.add(key)
                print("rejet", r[0], r[1], r[3], r[4])
    items, seen = [], set()
    if not a.dry:
        os.makedirs(GEN_DIR, exist_ok=True)
        if not a.only:
            for f in os.listdir(GEN_DIR):
                if f.endswith(".json"):
                    os.remove(os.path.join(GEN_DIR, f))
    for mod, content, fmt, p, w in res:
        did = design_id(mod, content, p, fmt)
        assert did not in seen and len(did) <= 64, did
        seen.add(did)
        title = f"{mod.TITLE} · {CONTENT_FR.get(content, content)}"
        tags = [fmt.label, p.label, *CONTENT[content][0], *p.tags]
        items.append((did, mod.TITLE, tags, None))
        if not a.dry:
            with open(os.path.join(GEN_DIR, did + ".json"), "w", encoding="utf-8", newline="\n") as f:
                f.write(layout_json(f"{title} · {p.label}", fmt, p, w))
    if not a.dry and not a.only:
        with open(os.path.join(DESIGNS, "generated.rs"), "w", encoding="utf-8", newline="\n") as f:
            f.write(rust_list(items))
    print(f"{len(items)} designs, {len({m.NAME for m, *_ in res})} archetypes, {len(rejected)} rejets QA")


if __name__ == "__main__":
    main()
