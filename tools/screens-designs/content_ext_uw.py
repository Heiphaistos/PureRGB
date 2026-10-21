"""Contenus du lot « Formats speciaux » (prefixe uw) : BASE (CPU, GPU, RAM, SSD 1-2) + extras (ajoutes par with_base).
Dix jeux d'extras ; chacun existe en `uw-<cle>` et en `uw-l<cle>` (tag « fond libre » : centre laisse vide pour une video/GIF)."""
from dataclasses import replace

from content import (CPUT, GPUT, GPUH, SSDT, MOBO, CHIP, VRM, NETD, NETU, DRD, DWR, LIQ, PUMP,
                     FAN1, CFAN, GFAN, CLK, GCLK, CPW, GPW, VRAM, Metric)

_L = lambda m, label: replace(m, label=label)
_DISK = lambda l: Metric(f"{l}:", f"disk.{l}.used_pct", "{:.0}%", "100%", 0, 100, "load", (f"disk.{l}.free_gb", "{:.0} Go", "999 Go"))
DC, DD = _DISK("C"), _DISK("D")
LIQ, NETD, NETU, DRD, DWR = _L(LIQ, "LIQ."), _L(NETD, "DESC."), _L(NETU, "MONT."), _L(DRD, "LECT."), _L(DWR, "ÉCR.")
MOBO, CHIP, GPUH = _L(MOBO, "CARTE"), _L(CHIP, "PUCE"), _L(GPUH, "G. HOT")
CPW, GPW, CLK, GCLK = _L(CPW, "P. CPU"), _L(GPW, "P. GPU"), _L(CLK, "F. CPU"), _L(GCLK, "F. GPU")
CFAN, GFAN = _L(CFAN, "V. CPU"), _L(GFAN, "V. GPU")
FAN2 = Metric("VENT. 2", "fan.2.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
FAN3 = Metric("VENT. 3", "fan.3.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
SSD1T = _L(SSDT, "T° SSD 1")

_SETS = {   # cle : (tags, francais, extras)
    "sys": (("système", "cpu", "gpu", "ram"), "Système", [LIQ, NETD, NETU, DRD, DWR, DC, DD]),
    "therm": (("températures", "carte mère", "vrm"), "Températures", [MOBO, CHIP, VRM, LIQ, GPUH, SSD1T]),
    "stock": (("disques", "réseau", "débit"), "Stockage et réseau", [DC, DD, DRD, DWR, NETD, NETU, SSD1T]),
    "air": (("refroidissement", "liquide", "ventilateurs", "aio"), "Refroidissement", [LIQ, PUMP, FAN1, CFAN, GFAN, FAN2, FAN3]),
    "puis": (("puissance", "fréquences", "gpu", "cpu"), "Puissance et fréquences", [CPW, GPW, CLK, GCLK, VRAM, CFAN, GFAN]),
    "gpu": (("gpu", "vram", "puissance"), "GPU détaillé", [GPUH, VRAM, GCLK, GPW, GFAN, LIQ, NETD]),
    "cpu": (("cpu", "fréquence", "puissance"), "CPU détaillé", [CLK, CPW, CFAN, MOBO, VRM, CHIP, LIQ]),
    "reseau": (("réseau", "débit", "disque"), "Réseau et E/S", [NETD, NETU, DRD, DWR, DC, DD, LIQ]),
    "disq": (("disques", "ssd", "températures"), "Disques", [DC, DD, SSD1T, DRD, DWR, MOBO]),
    "mix": (("système", "températures", "ventilateurs"), "Vue d'ensemble", [LIQ, CLK, GCLK, NETD, FAN1, DC, MOBO]),
}

CONTENT_EXT, CONTENT_FR_EXT = {}, {}
for _k, (_tags, _fr, _ms) in _SETS.items():
    CONTENT_EXT[f"uw-{_k}"] = (_tags, _ms)
    CONTENT_EXT[f"uw-l{_k}"] = (("fond libre", *_tags), _ms)
    CONTENT_FR_EXT[f"uw-{_k}"] = _fr
    CONTENT_FR_EXT[f"uw-l{_k}"] = f"{_fr}, fond libre"
