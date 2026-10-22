"""Fusionne docs/led-catalog/images-*.json dans src-tauri/assets/product_catalog.json (idempotent).

Les entrées existantes (même id) gagnent. Le chargement du catalogue côté Rust valide tout (cargo test product_images).
Usage : python tools/led-catalog/merge_images.py
"""
import json
import pathlib

root = pathlib.Path(__file__).resolve().parents[2]
cat_path = root / "src-tauri/assets/product_catalog.json"
cat = json.loads(cat_path.read_text(encoding="utf-8"))
have = {p["id"] for p in cat["products"]}
seen_names = {" ".join(n.lower().split()) for p in cat["products"] for n in p["match"].get("names", [])}
seen_usb = {u for p in cat["products"] for u in p["match"].get("usb", [])}
added = skipped = 0
# Échecs constatés au vrai téléchargement (hôte qui ne résout pas, 403 anti-robot) : écartés tant qu'ils ne sont pas corrigés.
EXCLUDE = {
    "fan-bequiet-light-wings-lx-120", "fan-bequiet-light-loop-360", "fan-arctic-p14-pwm-pst-argb", "fan-arctic-p12-pro-argb",
    "fan-arctic-bionix-p12-argb", "fan-deepcool-fc120", "fan-phanteks-d30-120-drgb", "fan-phanteks-t30-120-drgb",
    "fan-phanteks-m25-gen2-120-drgb", "fan-montech-ax120-pwm", "fan-montech-rx120-pwm", "fan-montech-hyperflow-argb-360",
}
for f in sorted((root / "docs/led-catalog").glob("images-*.json")):
    for p in json.loads(f.read_text(encoding="utf-8")):
        if p["id"] in have or p["id"] in EXCLUDE or not p.get("image"):
            continue
        names = {" ".join(n.lower().split()) for n in p["match"].get("names", [])}
        # Un motif déjà pris par un autre produit donnerait une égalité (aucune photo) : on garde le premier.
        if names & seen_names or set(p["match"].get("usb", [])) & seen_usb:
            skipped += 1
            continue
        seen_names |= names
        seen_usb |= set(p["match"].get("usb", []))
        cat["products"].append({k: p[k] for k in ("id", "kind", "vendor", "name", "match", "image")})
        have.add(p["id"])
        added += 1
cat_path.write_text(json.dumps(cat, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
print(f"{added} produits ajoutés, {skipped} doublons de motif ignorés, {len(cat['products'])} au total")
