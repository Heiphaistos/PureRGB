"""Contenus du lot « Heure, date et infos » (prefixe clk)."""
from content import CPU, CPUT, GPU, GPUT, MOBO, NETD, NETU, RAM, SSD, SSDT, Metric

UPT = Metric("ALLUMÉ", "uptime", "{:.0} s", "999999 s", 0, 86400, "load")

CONTENT_EXT = {
    "clk-systeme": (("heure", "date", "cpu", "gpu", "ram", "ssd"), [CPU, GPU, RAM, SSD]),
    "clk-temps": (("heure", "date", "températures", "cpu", "gpu"), [CPUT, GPUT, SSDT, MOBO]),
    "clk-reseau": (("heure", "date", "réseau", "cpu", "ram"), [NETD, NETU, CPU, RAM]),
    "clk-etat": (("heure", "date", "temps allumé", "cpu", "gpu"), [CPU, GPU, RAM, UPT]),
}
CONTENT_FR_EXT = {
    "clk-systeme": "Heure et charges", "clk-temps": "Heure et températures",
    "clk-reseau": "Heure et réseau", "clk-etat": "Heure et temps allumé",
}
