"""Contenus du lot « Formats speciaux » (prefixe uw) : 8 mesures chacun pour les bandeaux larges.
Les variantes `uw-l*` portent le tag « fond libre » (centre laisse vide pour une video/GIF de fond)."""
from dataclasses import replace

from content import (CPU, CPUT, GPU, GPUT, GPUH, RAM, SSD, SSDT, DC, DD, MOBO, CHIP, VRM, NETD, NETU, DRD, DWR, LIQ, PUMP,
                     FAN1, CFAN, GFAN, CLK, GCLK, CPW, GPW, VRAM, Metric)

_L = lambda m, label: replace(m, label=label)
CPU, GPU, RAM, SSD, LIQ, NETD, NETU, DRD, DWR = _L(CPU, "CPU"), _L(GPU, "GPU"), RAM, SSD, _L(LIQ, "LIQ."), _L(NETD, "DESC."), _L(NETU, "MONT."), _L(DRD, "LECT."), _L(DWR, "ÉCR.")
CPUT, GPUT, MOBO, CHIP, VRM, SSDT, GPUH = CPUT, GPUT, _L(MOBO, "CARTE"), _L(CHIP, "PUCE"), VRM, SSDT, _L(GPUH, "G. HOT")
CPW, GPW, CLK, GCLK = _L(CPW, "P. CPU"), _L(GPW, "P. GPU"), _L(CLK, "F. CPU"), _L(GCLK, "F. GPU")
CFAN, GFAN, FAN1 = _L(CFAN, "V. CPU"), _L(GFAN, "V. GPU"), _L(FAN1, "VENT. 1")
FAN2 = Metric("VENT. 2", "fan.2.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
FAN3 = Metric("VENT. 3", "fan.3.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
SSD2 = Metric("SSD 2", "ssd.2.temp", "{:.0}°C", "100°C", 20, 90, "temp", ("ssd.2.used_pct", "{:.0}%", "100%"))

_SYS = [CPU, GPU, RAM, SSD, LIQ, NETD, NETU, DRD]
_THERM = [CPUT, GPUT, MOBO, CHIP, VRM, SSDT, LIQ, GPUH]
_STOCK = [DC, DD, SSD, RAM, DRD, DWR, NETD, NETU]
_AIR = [LIQ, PUMP, FAN1, CFAN, GFAN, CPUT, GPUT, FAN2]

CONTENT_EXT = {
    "uw-sys": (("système", "cpu", "gpu", "ram"), _SYS),
    "uw-therm": (("températures", "carte mère", "vrm"), _THERM),
    "uw-stock": (("disques", "réseau", "débit"), _STOCK),
    "uw-air": (("refroidissement", "liquide", "ventilateurs", "aio"), _AIR),
    "uw-puis": (("puissance", "fréquences", "gpu", "cpu"), [CPW, GPW, CLK, GCLK, CPUT, GPUT, VRAM, CFAN]),
    "uw-lsys": (("fond libre", "système", "cpu", "gpu"), _SYS),
    "uw-ltherm": (("fond libre", "températures", "carte mère"), _THERM),
    "uw-lstock": (("fond libre", "disques", "réseau"), _STOCK),
    "uw-lair": (("fond libre", "refroidissement", "aio"), _AIR),
}
CONTENT_FR_EXT = {
    "uw-sys": "Système", "uw-therm": "Températures", "uw-stock": "Stockage et réseau", "uw-air": "Refroidissement",
    "uw-puis": "Puissance et fréquences", "uw-lsys": "Système, fond libre", "uw-ltherm": "Températures, fond libre",
    "uw-lstock": "Stockage, fond libre", "uw-lair": "Refroidissement, fond libre",
}
