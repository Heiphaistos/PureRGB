"""Contenus « Capteurs détaillés » (préfixe cap) : listes longues de mesures + groupes nommés."""
from dataclasses import replace

from content import (CFAN, CHIP, CLK, CPU, CPUT, CPW, DC, DD, DRD, DWR, GPU, GPUH, GPUT, GPW, LIQ, MOBO,
                     NETD, NETU, PUMP, RAM, VRAM, VRM, Metric)

_tmp = lambda s: (s, "{:.0}°C", "100°C")
_pct = lambda s: (s, "{:.0}%", "100%")

TCPU, CCPU = replace(CPUT, label="TEMP. CPU"), replace(CPU, label="CHARGE CPU")
TGPU, CGPU = replace(GPUT, label="TEMP. GPU"), replace(GPU, label="CHARGE GPU")
CHAUD = replace(GPUH, label="P. CHAUD")


def fan(n):
    return Metric(f"VENT. {n}", f"fan.{n}.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")


def ssd(n):
    return Metric(f"SSD {n}", f"ssd.{n}.used_pct", "{:.0}%", "100%", 0, 100, "load", _tmp(f"ssd.{n}.temp"))


def ssdt(n):
    return Metric(f"T° SSD {n}", f"ssd.{n}.temp", "{:.0}°C", "100°C", 20, 90, "temp", _pct(f"ssd.{n}.used_pct"))


COMPLET = [TCPU, CCPU, CLK, CPW, CFAN,
           TGPU, CHAUD, CGPU, VRAM, GPW,
           RAM, MOBO, CHIP, VRM, LIQ,
           DC, DD, ssdt(1), ssdt(2), DRD]
SYSTEME = [CCPU, CGPU, RAM, VRAM, DC, DD, NETD, NETU, DRD, DWR, TCPU, TGPU]
THERMIQUE = [TCPU, TGPU, CHAUD, MOBO, CHIP, VRM, ssdt(1), ssdt(2), ssdt(3), LIQ]
STOCKAGE = [DC, DD, ssd(1), ssd(2), ssd(3), ssd(4), ssdt(1), ssdt(2), ssdt(3), ssdt(4), DRD, DWR]
REFROID = [fan(1), fan(2), fan(3), fan(4), fan(5), fan(6), LIQ, PUMP, CFAN, TCPU, TGPU, MOBO]

CONTENT_EXT = {
    "cap-complet": (("capteurs", "cpu", "gpu", "ram", "disques", "carte mère", "tableau de bord"), COMPLET),
    "cap-systeme": (("capteurs", "charge", "disques", "réseau", "tableau de bord"), SYSTEME),
    "cap-thermique": (("capteurs", "températures", "carte mère", "ssd", "liquide"), THERMIQUE),
    "cap-stockage": (("capteurs", "disques", "ssd", "débit"), STOCKAGE),
    "cap-refroid": (("capteurs", "ventilateurs", "liquide", "pompe", "aio"), REFROID),
}
CONTENT_FR_EXT = {
    "cap-complet": "Tableau de bord complet", "cap-systeme": "Système complet", "cap-thermique": "Toutes les températures",
    "cap-stockage": "Stockage détaillé", "cap-refroid": "Ventilation détaillée",
}
# groupes nommés (titre, début, fin) pour les archétypes en colonnes / panneaux
GROUPS = {
    "cap-complet": [("CPU", 0, 5), ("GPU", 5, 10), ("SYSTÈME", 10, 15), ("DISQUES", 15, 20)],
    "cap-stockage": [("CAPACITÉ", 0, 6), ("TEMPÉRATURES", 6, 10), ("DÉBIT", 10, 12)],
    "cap-refroid": [("VENTILATEURS", 0, 6), ("LIQUIDE", 6, 9), ("TEMPÉRATURES", 9, 12)],
}
