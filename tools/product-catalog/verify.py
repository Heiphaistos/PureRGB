#!/usr/bin/env python3
"""Revalide src-tauri/assets/product_catalog.json (schema + images en ligne).

Usage : python tools/product-catalog/verify.py [--offline] [--prune] [catalogue.json]
  --offline : schema seulement (pas de reseau)
  --prune   : retire du fichier les entrees dont l'image echoue (ecrit le JSON)
Code de sortie : 0 si aucune erreur, 1 sinon. Les AVERTISSEMENTS n'echouent pas.
"""
import hashlib, json, re, sys, unicodedata, urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[2]
DEFAULT = ROOT / "src-tauri" / "assets" / "product_catalog.json"
KINDS = {"screen", "light", "fan", "cooler", "aio", "keyboard", "mouse", "ram", "motherboard", "gpu", "other"}
ID_RE = re.compile(r"^[a-z0-9-]+$")
USB_RE = re.compile(r"^[0-9a-f]{4}:[0-9a-f]{4}$")
MAX_BYTES = 4 * 1024 * 1024
UA = {"User-Agent": "Mozilla/5.0 (PureRGB catalog verifier)"}
MAGIC = (b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"GIF8", b"RIFF")  # PNG, JPEG, GIF, WebP


def strip_accents(s):
    return "".join(c for c in unicodedata.normalize("NFD", s) if unicodedata.category(c) != "Mn")


def check_schema(cat):
    errs, warns = [], []
    if cat.get("version") != 1:
        errs.append("version != 1")
    prods = cat.get("products")
    if not isinstance(prods, list) or not prods:
        return ["products absent ou vide"], warns
    ids, usbs = {}, {}
    for i, p in enumerate(prods):
        pid = p.get("id", f"#{i}")
        e = lambda m: errs.append(f"{pid}: {m}")
        if set(p) - {"id", "kind", "vendor", "name", "match", "image"}:
            e(f"champs inconnus {set(p) - {'id','kind','vendor','name','match','image'}}")
        if not isinstance(p.get("id"), str) or not ID_RE.match(p["id"]):
            e("id invalide")
        elif p["id"] in ids:
            e("id duplique")
        ids[p.get("id")] = p
        if p.get("kind") not in KINDS:
            e(f"kind invalide {p.get('kind')!r}")
        for k in ("vendor", "name"):
            if not isinstance(p.get(k), str) or not p[k].strip():
                e(f"{k} vide")
        m = p.get("match") or {}
        names, usb = m.get("names"), m.get("usb", [])
        if not isinstance(names, list) or not names:
            e("match.names vide")
            names = []
        for n in names:
            if not isinstance(n, str) or n != strip_accents(n).lower() or n != n.strip():
                e(f"name non normalise {n!r}")
            elif len(n) < 6 or n == p.get("vendor", "").lower():
                e(f"name trop court/generique {n!r}")
        for u in usb:
            if not USB_RE.match(u):
                e(f"usb mal forme {u!r}")
            elif u in usbs and usbs[u] != pid:
                e(f"usb {u} deja utilise par {usbs[u]}")
            else:
                usbs[u] = pid
        img = p.get("image") or {}
        for k in ("url", "source_page"):
            u = img.get(k, "")
            pu = urlparse(u)
            if pu.scheme != "https" or not pu.netloc:
                e(f"image.{k} non https: {u!r}")
    # noms identiques ou imbriques entre produits differents
    owner = {}
    for p in prods:
        for n in (p.get("match") or {}).get("names", []):
            if n in owner and owner[n] != p["id"]:
                errs.append(f"nom {n!r} partage par {owner[n]} et {p['id']}")
            owner[n] = p["id"]
    for n, a in owner.items():
        for n2, b in owner.items():
            if a != b and n != n2 and n in n2:
                warns.append(f"nom {n!r} ({a}) contenu dans {n2!r} ({b}) : le moteur doit prendre le nom LE PLUS LONG")
    return errs, warns


def fetch(p):
    url = p["image"]["url"]
    try:
        req = urllib.request.Request(url, headers=UA)
        with urllib.request.urlopen(req, timeout=40) as r:
            ct = r.headers.get("Content-Type", "")
            body = r.read(MAX_BYTES + 1)
            if r.status != 200:
                return p["id"], f"HTTP {r.status}", None, 0, ct
            if not ct.lower().startswith("image/"):
                return p["id"], f"Content-Type {ct!r}", None, len(body), ct
            if len(body) > MAX_BYTES:
                return p["id"], "image > 4 Mo", None, len(body), ct
            if not body.startswith(MAGIC):
                return p["id"], "octets non reconnus comme image", None, len(body), ct
            return p["id"], None, hashlib.sha256(body).hexdigest(), len(body), ct
    except Exception as ex:  # reseau : on rapporte, on ne masque rien
        return p["id"], f"{type(ex).__name__}: {ex}", None, 0, ""


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    path = Path(args[0]) if args else DEFAULT
    cat = json.loads(path.read_text(encoding="utf-8"))
    errs, warns = check_schema(cat)
    prods = cat["products"]
    fails = {}
    if "--offline" not in sys.argv:
        with ThreadPoolExecutor(8) as ex:
            res = list(ex.map(fetch, prods))
        hashes = {}
        for pid, err, h, size, ct in res:
            if err:
                fails[pid] = err
                errs.append(f"{pid}: image KO ({err})")
            else:
                hashes.setdefault(h, []).append(pid)
        for h, l in hashes.items():
            if len(l) > 1:
                warns.append(f"images identiques (octets) : {', '.join(l)}")
        sizes = [s for _, e, _, s, _ in res if not e]
        hosts = sorted({urlparse(p["image"]["url"]).netloc for p in prods})
        print(f"images OK : {len(sizes)}/{len(prods)} ; total {sum(sizes)/1e6:.1f} Mo ; max {max(sizes, default=0)/1e6:.2f} Mo")
        print("types :", sorted({ct.split(';')[0] for _, e, _, _, ct in res if not e}))
        print("hotes d'images :", ", ".join(hosts))
    kinds = {}
    for p in prods:
        kinds[p["kind"]] = kinds.get(p["kind"], 0) + 1
    print(f"{len(prods)} produits ; par categorie :", dict(sorted(kinds.items())))
    for w in warns:
        print("AVERT", w)
    for e in errs:
        print("ERREUR", e)
    if fails and "--prune" in sys.argv:
        cat["products"] = [p for p in prods if p["id"] not in fails]
        path.write_text(json.dumps(cat, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"{len(fails)} entree(s) retiree(s) : {', '.join(fails)}")
    print("RESULTAT :", "ECHEC" if errs else "OK")
    sys.exit(1 if errs else 0)


if __name__ == "__main__":
    main()
