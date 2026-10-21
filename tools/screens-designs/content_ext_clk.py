"""Contenus du lot « Heure et calendrier » (prefixe clk) : BASE (CPU, GPU, RAM, SSD 1-2) + extras."""
from content import (CFAN, CLK, CPW, DC, DD, DRD, DWR, FAN1, GCLK, GFAN, GPUH, GPW, LIQ, MOBO, NETD, NETU, PUMP,
                     CHIP, VRAM, VRM, Metric)

UPT = Metric("ALLUMÉ", "uptime", "{:.0} s", "999999 s", 0, 86400, "load")

CONTENT_EXT = {
    "clk-systeme": (("heure", "date", "disques", "vram"), [DC, DD, VRAM, NETD]),
    "clk-temps": (("heure", "date", "températures", "carte mère"), [MOBO, CHIP, VRM, GPUH]),
    "clk-reseau": (("heure", "date", "réseau", "débit"), [NETD, NETU, DRD, DWR]),
    "clk-etat": (("heure", "date", "temps allumé", "fréquences"), [UPT, CLK, GCLK]),
    "clk-air": (("heure", "date", "liquide", "ventilateurs"), [LIQ, PUMP, CFAN, GFAN]),
    "clk-watts": (("heure", "date", "puissance", "fréquences"), [CPW, GPW, CLK, GCLK]),
    "clk-disques": (("heure", "date", "disques", "débit"), [DC, DD, DRD, DWR]),
    "clk-gpu": (("heure", "date", "gpu", "vram"), [GPUH, VRAM, GCLK, GPW]),
    "clk-cpu": (("heure", "date", "cpu", "fréquence"), [CLK, CPW, CFAN, VRM]),
    "clk-ventil": (("heure", "date", "ventilateurs", "pompe"), [FAN1, CFAN, GFAN, PUMP]),
}
CONTENT_FR_EXT = {
    "clk-systeme": "Heure, date et système", "clk-temps": "Heure, date et températures",
    "clk-reseau": "Heure, date et réseau", "clk-etat": "Heure, date et temps allumé",
    "clk-air": "Heure, date et refroidissement", "clk-watts": "Heure, date et puissance",
    "clk-disques": "Heure, date et disques", "clk-gpu": "Heure, date et GPU",
    "clk-cpu": "Heure, date et CPU", "clk-ventil": "Heure, date et ventilateurs",
}
