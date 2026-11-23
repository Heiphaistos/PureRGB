"""Régénère src-tauri/assets/gpu_whitelist.tsv depuis docs/rgb-protocols/signalrgb-devices.csv.

Seules les familles dont un pilote natif existe (backends/gpu/*.rs) sont extraites : ajouter une ligne ici ET le pilote.
Usage (depuis la racine du dépôt) : python tools/gen_gpu_whitelist.py
"""
import csv
import re

FAMILIES = {
    'smbus-i2c-nvapi:asus-legacy-gpu': 'aura_gpu',
    'smbus-i2c-nvapi:asus-ampere-lovelace-gpu': 'ene_gpu',
    'smbus-i2c-nvapi:msi-gpu': 'msi_v1',
    'smbus-i2c-nvapi:gigabyte-gen1-gpu': 'gigabyte_gen1',
    'smbus-i2c-nvapi:pny-xlr8-lovelace-gpu': 'pny_lovelace',
}
PCI = re.compile(r'\[PCI subvendor=0x([0-9A-Fa-f]{4}) subdevice=0x([0-9A-Fa-f]{4}) i2c=0x([0-9A-Fa-f]{2})\]')
HEAD = '# famille\tdevice PCI (NVIDIA 10DE)\tsubvendor\tsubdevice\ti2c 7 bits\tnom -- source: docs/rgb-protocols/signalrgb-devices.csv\n'

out = []
for r in csv.DictReader(open('docs/rgb-protocols/signalrgb-devices.csv', encoding='utf-8')):
    family = FAMILIES.get(r['protocol_family'])
    m = PCI.search(r['name'])
    if not family or not m or r['vid'].lower() != '0x10de':
        continue
    name = r['name'][:m.start()].strip().replace('\t', ' ')
    out.append('\t'.join([family, r['pid'][2:].lower().zfill(4), m.group(1).lower(), m.group(2).lower(), m.group(3).lower(), name]))
open('src-tauri/assets/gpu_whitelist.tsv', 'w', encoding='utf-8').write(HEAD + '\n'.join(out) + '\n')
print(len(out), 'cartes')
