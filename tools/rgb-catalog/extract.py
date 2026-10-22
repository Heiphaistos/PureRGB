"""Régénère src-tauri/assets/rgb_usb_catalog.json (VID/PID de contrôleurs RGB : faits d'identification).

    python -I tools/rgb-catalog/extract.py <dossier OpenRGB cloné> <usb.ids> <sortie.json>

Lit uniquement les identifiants (nom, VID, PID) des enregistrements `REGISTER_*DETECTOR*` ; aucun code n'est repris.
"""
import json
import os
import re
import sys

src, usb_ids, out = sys.argv[1:4]
INT = re.compile(r"0x[0-9A-Fa-f]+|\d+")

defs = {}
for root, _, files in os.walk(src):
    for f in files:
        if f.endswith((".h", ".hpp", ".cpp")):
            t = open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
            for m in re.finditer(r"^\s*#define\s+(\w+)\s+(0x[0-9A-Fa-f]+|\d+)\b", t, re.M):
                defs.setdefault(m.group(1), m.group(2))
            for m in re.finditer(r"(?:uint16_t|int|unsigned short)\s+(\w+)\s*=\s*(0x[0-9A-Fa-f]+)\s*;", t):
                defs.setdefault(m.group(1), m.group(2))


def val(x):
    x = x.strip()
    x = x if INT.fullmatch(x) else defs.get(x, "")
    return int(x, 0) if INT.fullmatch(x) else None


pat = re.compile(r'REGISTER_(?:HID_|HID_WRAPPED_)?DETECTOR(?:_[A-Z]+)?\(\s*"([^"]+)"\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)')
found = {}
for root, _, files in os.walk(os.path.join(src, "Controllers")):
    for f in files:
        if f.endswith(".cpp"):
            for m in pat.finditer(open(os.path.join(root, f), encoding="utf-8", errors="replace").read()):
                v, p = val(m.group(3)), val(m.group(4))
                if v is not None and p is not None:
                    found[(v, p)] = m.group(1)

names = {}
for line in open(usb_ids, encoding="utf-8", errors="replace"):
    m = re.match(r"^([0-9a-fA-F]{4})\s+(\S.*)", line)
    if m:
        names[m.group(1).lower()] = m.group(2).strip()

vendors = {}
for (v, _), n in sorted(found.items()):
    vendors.setdefault(f"{v:04x}", names.get(f"{v:04x}") or n.split()[0])
doc = {"source": "identifiants VID/PID (faits) extraits des détecteurs OpenRGB et de usb.ids", "vendors": vendors,
       "devices": [[f"{v:04x}", f"{p:04x}", n] for (v, p), n in sorted(found.items())]}
open(out, "w", encoding="utf-8", newline="\n").write(json.dumps(doc, ensure_ascii=False, separators=(",", ":")))
print(len(found), "appareils,", len(vendors), "fabricants")
