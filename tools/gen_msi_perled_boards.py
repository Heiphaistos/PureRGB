"""Extrait la table des cartes MSI Mystic Light 185 octets en mode LED par LED depuis les faits d'OpenRGB (MSIMotherboard185Controller.cpp) :
identifiant USB, nombre de LEDs embarquees / JPIPE1 / JPIPE2 / JRGB et connecteurs presents. Usage : python tools/gen_msi_perled_boards.py <openrgb-src>.
Ecrit src-tauri/assets/msi_perled_boards.tsv : pid, onboard, pipe1, pipe2, jrgbs, connecteurs (liste separee par des virgules). Jamais de code repris.
"""
import pathlib
import re
import sys

src = pathlib.Path(sys.argv[1]) / 'Controllers/MSIMotherboardController/MSIMotherboard185Controller/MSIMotherboard185Controller.cpp'
t = src.read_text(encoding='utf-8', errors='ignore')
sets = {}
for m in re.finditer(r'const std::vector<MSI_ZONE> (zones_set\d+)\s*=\s*\{(.*?)\};', t, re.S):
    sets[m.group(1)] = [z.strip().replace('MSI_ZONE_', '') for z in m.group(2).split(',') if z.strip()]
rows = []
for m in re.finditer(r'\{\s*(0x[0-9A-Fa-f]{4})\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*&(zones_set\d+)\s*,\s*MSIMotherboard185Controller::(DIRECT_MODE_\w+)', t):
    pid, onb, p1, p2, jr, zs, mode = m.groups()
    if mode != 'DIRECT_MODE_PER_LED':
        continue
    rows.append((int(pid, 16), int(onb), int(p1), int(p2), int(jr), ','.join(sets[zs])))
head = '# pid\tonboard\tpipe1\tpipe2\tjrgbs\tconnecteurs -- faits extraits de MSIMotherboard185Controller (OpenRGB), mode LED par LED\n'
out = pathlib.Path('src-tauri/assets/msi_perled_boards.tsv')
out.write_text(head + '\n'.join(f'{p:04x}\t{a}\t{b}\t{c}\t{d}\t{z}' for p, a, b, c, d, z in sorted(set(rows))) + '\n', encoding='utf-8')
print(len(set(rows)), 'cartes LED par LED ;', len(sets), 'ensembles de zones')
