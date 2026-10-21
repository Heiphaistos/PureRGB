"""Contenus « Capteurs détaillés » (préfixe cap) : listes longues de mesures + groupes nommés.

Chaque contenu est défini par ses GROUPES (titre, métriques). Les 7 données obligatoires (CPU temp+charge,
GPU temp+charge, RAM, SSD 1, SSD 2 masqué si absent) figurent dans chaque contenu ; `layouts.cap_common.metrics`
les place en tête de la liste à plat, les archétypes les gardent donc toujours ; les mesures conditionnelles (SSD 3, 4) viennent en dernier.
"""
from dataclasses import replace

from content import (BASE_SOURCES, CFAN, CHIP, CLK, CPU, CPUT, CPW, DC, DD, DRD, DWR, GCLK, GFAN, GPU, GPUH, GPUT, GPW,
                     LIQ, MOBO, NETD, NETU, PUMP, RAM, SSD, SSD2, VRAM, VRM, Metric)

_tmp = lambda s: (s, "{:.0}°C", "100°C")
_pct = lambda s: (s, "{:.0}%", "100%")

TCPU, CCPU = replace(CPUT, label="TEMP. CPU"), replace(CPU, label="CHARGE CPU")
TGPU, CGPU = replace(GPUT, label="TEMP. GPU"), replace(GPU, label="CHARGE GPU")
CHAUD = replace(GPUH, label="P. CHAUD")
SSD1 = replace(SSD, label="SSD 1")


def fan(n):
    return Metric(f"VENT. {n}", f"fan.{n}.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")


def ssd(n):
    return Metric(f"SSD {n}", f"ssd.{n}.used_pct", "{:.0}%", "100%", 0, 100, "load", _tmp(f"ssd.{n}.temp"),
                  None, f"ssd.{n}.used_pct" if n > 1 else None)


def ssdt(n):
    return Metric(f"T° SSD {n}", f"ssd.{n}.temp", "{:.0}°C", "100°C", 20, 90, "temp", _pct(f"ssd.{n}.used_pct"),
                  None, f"ssd.{n}.temp" if n > 1 else None)


_CPU4, _GPU4 = [TCPU, CCPU], [TGPU, CGPU]
GROUPS = {
    "cap-complet": [("CPU", [TCPU, CCPU, CLK, CPW, CFAN]), ("GPU", [TGPU, CGPU, CHAUD, VRAM, GPW]),
                    ("SYSTÈME", [RAM, MOBO, CHIP, VRM, LIQ]),
                    ("DISQUES", [SSD1, SSD2, ssdt(1), ssdt(2), DC, DD])],
    "cap-systeme": [("PROCESSEURS", _CPU4 + _GPU4), ("MÉMOIRE", [RAM, VRAM, SSD1, SSD2]),
                    ("DISQUES", [DC, DD, DRD, DWR]), ("RÉSEAU", [NETD, NETU])],
    "cap-thermique": [("PROCESSEURS", _CPU4 + _GPU4 + [CHAUD]), ("CARTE MÈRE", [MOBO, CHIP, VRM, LIQ]),
                      ("STOCKAGE", [RAM, SSD1, SSD2, ssdt(1), ssdt(2), ssdt(3)])],
    "cap-stockage": [("PROCESSEURS", _CPU4 + _GPU4 + [RAM]), ("CAPACITÉ", [SSD1, SSD2, ssd(3), ssd(4), DC, DD]),
                     ("TEMPÉRATURES", [ssdt(1), ssdt(2), ssdt(3), ssdt(4)]), ("DÉBIT", [DRD, DWR])],
    "cap-refroid": [("PROCESSEURS", _CPU4 + _GPU4), ("MÉMOIRE", [RAM, SSD1, SSD2]),
                    ("VENTILATEURS", [fan(1), fan(2), fan(3), fan(4), fan(5), fan(6)]),
                    ("LIQUIDE", [LIQ, PUMP, CFAN, MOBO])],
    "cap-reseau": [("PROCESSEURS", _CPU4 + _GPU4), ("RÉSEAU", [NETD, NETU]), ("DISQUES", [SSD1, SSD2, DRD, DWR]),
                   ("MÉMOIRE", [RAM, DC, DD])],
    "cap-gamer": [("CPU", [TCPU, CCPU, CLK, CPW]), ("GPU", [TGPU, CGPU, GCLK, GPW, VRAM]),
                  ("MÉMOIRE", [RAM, SSD1, SSD2, DC])],
    "cap-puissance": [("CPU", [TCPU, CCPU, CLK, CPW]), ("GPU", [TGPU, CGPU, GCLK, GPW]),
                      ("SYSTÈME", [RAM, SSD1, SSD2]), ("VENTILATION", [CFAN, GFAN, fan(1), PUMP])],
    "cap-serveur": [("PROCESSEUR", [TCPU, CCPU, CPW, CFAN]), ("GRAPHIQUE", _GPU4 + [VRAM]),
                    ("STOCKAGE", [SSD1, SSD2, ssdt(1), ssdt(2), DRD, DWR]), ("MÉMOIRE", [RAM, DC, DD])],
    "cap-overclock": [("CPU", [TCPU, CCPU, CLK, CPW, CFAN]), ("GPU", [TGPU, CGPU, CHAUD, GCLK, GPW, GFAN]),
                      ("CARTE MÈRE", [MOBO, VRM, CHIP]), ("MÉMOIRE", [RAM, SSD1, SSD2, LIQ])],
    "cap-silence": [("TEMPÉRATURES", [TCPU, TGPU, MOBO, LIQ]), ("CHARGES", [CCPU, CGPU, RAM]),
                    ("VENTILATEURS", [CFAN, GFAN, fan(1), fan(2)]), ("DISQUES", [SSD1, SSD2, ssdt(1)])],
    "cap-gpu": [("GPU", [TGPU, CGPU, CHAUD]), ("MÉMOIRE GPU", [VRAM, GCLK, GPW, GFAN]),
                ("CPU", [TCPU, CCPU, CLK]), ("SYSTÈME", [RAM, SSD1, SSD2])],
    "cap-cpu": [("CPU", [TCPU, CCPU, CLK, CPW, CFAN]), ("GPU", _GPU4), ("SYSTÈME", [RAM, MOBO, CHIP, VRM]),
                ("DISQUES", [SSD1, SSD2, ssdt(1), ssdt(2)])],
    "cap-bureau": [("CHARGES", [CCPU, CGPU, RAM]), ("TEMPÉRATURES", [TCPU, TGPU, MOBO]),
                   ("DISQUES", [SSD1, SSD2, DC, DD]), ("RÉSEAU", [NETD, NETU])],
    "cap-nas": [("STOCKAGE", [SSD1, SSD2, ssd(3), ssd(4), DRD, DWR]), ("RÉSEAU", [NETD, NETU, DC, DD]),
                ("PROCESSEURS", _CPU4 + _GPU4), ("MÉMOIRE", [RAM, CFAN])],
    "cap-carte": [("CARTE MÈRE", [MOBO, CHIP, VRM]), ("CPU", [TCPU, CCPU, CPW]), ("GPU", [TGPU, CGPU, GPW]),
                  ("MÉMOIRE", [RAM, SSD1, SSD2])],
}
GROUPS = {k: [(t, sorted(ms, key=lambda m: (m.src not in BASE_SOURCES, m.cond is not None and m.src not in BASE_SOURCES))) for t, ms in g] for k, g in GROUPS.items()}
_TAGS = {
    "cap-complet": ("cpu", "gpu", "ram", "disques", "carte mère", "tableau de bord"),
    "cap-systeme": ("charge", "disques", "réseau", "tableau de bord"),
    "cap-thermique": ("températures", "carte mère", "ssd", "liquide"),
    "cap-stockage": ("disques", "ssd", "débit"),
    "cap-refroid": ("ventilateurs", "liquide", "pompe", "aio"),
    "cap-reseau": ("réseau", "débit", "disques"),
    "cap-gamer": ("jeu", "fréquences", "puissance", "gpu"),
    "cap-puissance": ("puissance", "fréquences", "ventilateurs"),
    "cap-serveur": ("serveur", "stockage", "mémoire"),
    "cap-overclock": ("overclocking", "fréquences", "températures"),
    "cap-silence": ("silence", "ventilateurs", "températures"),
    "cap-gpu": ("gpu", "vram", "fréquences"), "cap-cpu": ("cpu", "fréquences", "disques"),
    "cap-bureau": ("bureautique", "charge", "réseau"), "cap-nas": ("nas", "stockage", "réseau"),
    "cap-carte": ("carte mère", "puissance", "températures"),
}
CONTENT_FR_EXT = {
    "cap-complet": "Tableau de bord complet", "cap-systeme": "Système complet", "cap-thermique": "Toutes les températures",
    "cap-stockage": "Stockage détaillé", "cap-refroid": "Ventilation détaillée", "cap-reseau": "Réseau et disques",
    "cap-gamer": "Spécial jeu", "cap-puissance": "Puissance et fréquences", "cap-serveur": "Serveur et stockage",
    "cap-overclock": "Overclocking", "cap-silence": "Silence et ventilation", "cap-gpu": "GPU en détail",
    "cap-cpu": "CPU en détail", "cap-bureau": "Bureautique", "cap-nas": "NAS et réseau", "cap-carte": "Carte mère et puissance",
}


def flat(key):
    """Métriques à plat : les 7 obligatoires d'abord, puis le reste dans l'ordre des groupes."""
    ms = [m for _, g in GROUPS[key] for m in g]
    extra = [m for m in ms if m.src not in BASE_SOURCES]   # métriques conditionnelles (SSD 3, 4...) en dernier : pas de trou
    return [m for m in ms if m.src in BASE_SOURCES] + sorted(extra, key=lambda m: m.cond is not None)


CONTENT_EXT = {k: (("capteurs",) + t, flat(k)) for k, t in _TAGS.items()}
assert all({m.src for m in flat(k)} >= set(BASE_SOURCES) for k in GROUPS)
