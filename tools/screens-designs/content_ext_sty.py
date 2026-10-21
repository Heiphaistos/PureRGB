"""Jeux de contenu du lot « Styles et ambiances » (prefixe sty)."""
from content import (CHIP, CFAN, CLK, CPU, CPUT, CPW, DC, DD, DRD, DWR, FAN1, GFAN, GPU, GPUH, GPUT, GPW, LIQ,
                     MOBO, NETD, NETU, PUMP, RAM, SSD, SSDT, VRM)

from dataclasses import replace

MOBO = replace(MOBO, label="C. MÈRE")
GPUH = replace(GPUH, label="P. CHAUD")
CPW = replace(CPW, label="W CPU")
GPW = replace(GPW, label="W GPU")
CLK = replace(CLK, label="CPU MHz")
CFAN = replace(CFAN, label="VENTIRAD")

CONTENT_EXT = {
    "sty-pc": (("cpu", "gpu", "ram", "ssd", "vue d'ensemble"), [CPU, GPU, RAM, SSD, CPUT, GPUT, LIQ, NETD]),
    "sty-therm": (("températures", "carte mère", "vrm", "chipset"), [CPUT, GPUT, MOBO, VRM, SSDT, LIQ, CHIP, GPUH]),
    "sty-io": (("disques", "réseau", "débit", "ram"), [DC, DD, DRD, DWR, NETD, NETU, RAM, SSD]),
    "sty-air": (("ventilateurs", "pompe", "liquide", "refroidissement"), [CFAN, GFAN, PUMP, FAN1, LIQ, CPUT, GPUT, CPW]),
    "sty-puiss": (("puissance", "fréquences", "cpu", "gpu"), [CPW, GPW, CLK, CPUT, GPUT, RAM, SSD, NETD]),
}
CONTENT_FR_EXT = {
    "sty-pc": "Vue d'ensemble", "sty-therm": "Températures", "sty-io": "Disques et réseau",
    "sty-air": "Ventilation", "sty-puiss": "Puissance",
}
