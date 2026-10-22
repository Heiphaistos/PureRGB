"""Génère src/data/fanCatalog.ts depuis docs/led-catalog/fans.json (références avec un nombre de LEDs sourcé).

Usage : python tools/led-catalog/gen_fans.py
"""
import json
import pathlib

root = pathlib.Path(__file__).resolve().parents[2]
rows = json.loads((root / "docs/led-catalog/fans.json").read_text(encoding="utf-8"))
keep = [r for r in rows if r.get("led_count") and r["led_count"] > 0 and r.get("confidence") in ("official", "retailer")]
keep.sort(key=lambda r: (r["brand"].lower(), r["model"].lower()))


def q(s: str) -> str:
    return json.dumps(s, ensure_ascii=False)


lines = [
    "// GÉNÉRÉ par tools/led-catalog/gen_fans.py depuis docs/led-catalog/fans.json — ne pas éditer à la main.",
    "// Nombre de LEDs adressables PAR UNITÉ, relevé sur la documentation du constructeur (source_url dans le JSON).",
    'import type { FanPreset } from "./fanPresets.ts";',
    "",
    "export const FAN_CATALOG: FanPreset[] = [",
]
for r in keep:
    lines.append(f'  {{ brand: {q(r["brand"])}, model: {q(r["model"])}, ledsPerUnit: {r["led_count"]}, source: {q(r["source_url"])} }},')
lines += ["];", ""]
(root / "src/data/fanCatalog.ts").write_text("\n".join(lines), encoding="utf-8")
print(f"{len(keep)} références écrites")
