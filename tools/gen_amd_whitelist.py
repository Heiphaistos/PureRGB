"""Extrait les listes blanches PCI des cartes AMD (bus ADL, amd_whitelist.tsv) et NVIDIA (openrgb_nvidia.tsv, en complement du catalogue SignalRGB) depuis les enregistrements de detection d'OpenRGB (faits : identifiants PCI et adresses).

Usage : python tools/gen_amd_whitelist.py <dossier openrgb-src> -> ecrit src-tauri/assets/amd_whitelist.tsv et openrgb_nvidia.tsv
Seuls les identifiants PCI (vendeur 1002, device, sous-vendeur, sous-device) et l'adresse I2C sont repris, jamais de code.
Les cartes annoncees avec deux adresses/familles differentes sont ecartees (ambiguite = jamais d'ecriture).
"""
import collections
import pathlib
import re
import sys

src = pathlib.Path(sys.argv[1])
NUM = re.compile(r'0[xX][0-9A-Fa-f]+|[0-9]+')
DEFINE = re.compile(r'#define\s+(\w+)\s+(' + NUM.pattern + r')')
ENUM = re.compile(r'^\s*(\w+)\s*=\s*(' + NUM.pattern + r')\s*,?', re.M)
REG = re.compile(r'REGISTER_I2C_PCI_DETECTOR\(\s*"([^"]*)"\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*(\w+)\s*\)')


def num(s):
    return int(s, 16) if s.lower().startswith('0x') else int(s, 10)


files = [f for f in src.glob('Controllers/**/*') if f.suffix in ('.h', '.cpp')]
texts = {f: f.read_text(encoding='utf-8', errors='ignore') for f in files}
texts[src / 'pci_ids/pci_ids.h'] = (src / 'pci_ids/pci_ids.h').read_text(encoding='utf-8', errors='ignore')
defs = {}
for txt in texts.values():
    for rx in (DEFINE, ENUM):
        for m in rx.finditer(txt):
            defs.setdefault(m.group(1), num(m.group(2)))


def resolve(tok):
    if tok in defs:
        return defs[tok]
    return num(tok) if NUM.fullmatch(tok) else None


rows = []
unknown = collections.Counter()
for f in files:
    if 'Detect' not in f.name or f.suffix != '.cpp':
        continue
    for m in REG.finditer(texts[f]):
        name, func, ven, dev, sven, sdev, addr = m.groups()
        toks = (ven, dev, sven, sdev, addr)
        vals = [resolve(t) for t in toks]
        if vals[0] not in (0x1002, 0x10DE):
            continue
        if None in vals:
            unknown[(func, toks[vals.index(None)])] += 1
            continue
        rows.append((func, vals[1], vals[2], vals[3], vals[4], name, vals[0]))

by_key = collections.defaultdict(set)
for r in rows:
    by_key[(r[6],) + r[1:4]].add((r[0], r[4]))
ambiguous = {k for k, v in by_key.items() if len(v) > 1}
for vendor, fname, label in ((0x1002, 'amd_whitelist.tsv', 'AMD 1002'), (0x10DE, 'openrgb_nvidia.tsv', 'NVIDIA 10DE')):
    seen, out = set(), []
    for r in rows:
        k = (r[6],) + r[1:4]
        if r[6] != vendor or k in ambiguous or k in seen:
            continue
        seen.add(k)
        out.append('\t'.join([r[0], f'{r[1]:04x}', f'{r[2]:04x}', f'{r[3]:04x}', f'{r[4]:02x}', r[5].replace('\t', ' ')]))
    head = f'# detecteur OpenRGB\tdevice PCI ({label})\tsubvendor\tsubdevice\ti2c 7 bits\tnom -- faits extraits des detecteurs OpenRGB\n'
    pathlib.Path('src-tauri/assets/' + fname).write_text(head + '\n'.join(out) + '\n', encoding='utf-8')
    print(fname, len(out), 'cartes retenues')
print(len(rows), 'lignes,', len(ambiguous), 'ambigues ecartees ; constantes inconnues :', dict(unknown))
