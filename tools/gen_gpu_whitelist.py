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
    'smbus-i2c-nvapi:msi-gpu-lovelace': 'msi_v2',
    'smbus-i2c-nvapi:msi-gpu-blackwell': 'msi_v2',
    'smbus-i2c-nvapi:gigabyte-gen1-gpu': 'gigabyte_gen1',
    'smbus-i2c-nvapi:gigabyte-gen2-gpu': 'gigabyte_gen2',
    'smbus-i2c-nvapi:gigabyte-gen3-gpu': 'gigabyte_gen3',
    'smbus-i2c-nvapi:pny-xlr8-lovelace-gpu': 'pny_lovelace',
    'smbus-i2c-nvapi:pny-blackwell-gpu': 'pny_epicx',
    'smbus-i2c-nvapi:pny-turing-ampere-gpu-controller': 'palit_v2',
    'smbus-i2c-nvapi:colorful-ampere-gpu': 'colorful',
    'smbus-i2c-nvapi:evga-turing-gpu': 'evga_turing',
    'smbus-i2c-nvapi:evga-ampere-gpu': 'evga_ampere',
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
# Une carte annoncée par deux protocoles/adresses différents est ambiguë : écartée (jamais d'écriture au hasard).
by_key = {}
for line in out:
    f = line.split('	')
    by_key.setdefault(tuple(f[1:4]), set()).add((f[0], f[4]))
ambiguous = {k for k, v in by_key.items() if len(v) > 1}
out = [line for line in out if tuple(line.split('	')[1:4]) not in ambiguous]
print(len(ambiguous), 'cartes ambiguës écartées')
open('src-tauri/assets/gpu_whitelist.tsv', 'w', encoding='utf-8').write(HEAD + '\n'.join(out) + '\n')
print(len(out), 'cartes')
