"""Jeux de contenu du lot « Styles et ambiances » (prefixe sty). BASE (CPU, GPU, RAM, SSD 1-2) vient en tete (generate.py)."""
from dataclasses import replace

from content import (CHIP, CFAN, CLK, CPW, DC, DD, DRD, DWR, FAN1, GCLK, GFAN, GPUH, GPW, LIQ, MOBO, NETD, NETU,
                     PUMP, VRAM, VRM)

MOBO = replace(MOBO, label="C. MÈRE")
GPUH = replace(GPUH, label="P. CHAUD")
CPW = replace(CPW, label="W CPU")
GPW = replace(GPW, label="W GPU")
CLK = replace(CLK, label="CPU MHz")
GCLK = replace(GCLK, label="GPU MHz")
CFAN = replace(CFAN, label="VENTIRAD")

CONTENT_EXT = {
    "sty-pc": (("cpu", "gpu", "ram", "ssd", "vue d'ensemble"), [LIQ, NETD, NETU, DC, DD, CPW, GPW]),
    "sty-therm": (("températures", "carte mère", "vrm", "chipset"), [MOBO, VRM, LIQ, CHIP, GPUH]),
    "sty-io": (("disques", "réseau", "débit", "ram"), [DC, DD, DRD, DWR, NETD, NETU]),
    "sty-air": (("ventilateurs", "pompe", "liquide", "refroidissement"), [CFAN, GFAN, PUMP, FAN1, LIQ, CPW]),
    "sty-puiss": (("puissance", "fréquences", "cpu", "gpu"), [CPW, GPW, CLK, GCLK, NETD]),
    "sty-gpu": (("gpu", "vram", "fréquences"), [GPUH, VRAM, GCLK, GPW, GFAN]),
    "sty-cpu": (("cpu", "fréquence", "carte mère"), [CLK, CPW, CFAN, MOBO, VRM, LIQ]),
    "sty-mem": (("ram", "mémoire", "disques"), [DC, DD, DRD, DWR, VRAM, NETD]),
}
CONTENT_FR_EXT = {
    "sty-pc": "Vue d'ensemble", "sty-therm": "Températures", "sty-io": "Disques et réseau",
    "sty-air": "Ventilation", "sty-puiss": "Puissance", "sty-gpu": "GPU détaillé", "sty-cpu": "CPU détaillé",
    "sty-mem": "Mémoire et disques",
}
