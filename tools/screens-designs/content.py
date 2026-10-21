"""Metriques affichables et jeux de contenu. Cles de donnees STABLES (voir README)."""
from dataclasses import dataclass


@dataclass(frozen=True)
class Metric:
    label: str            # libelle francais court
    src: str              # cle de donnee principale
    fmt: str
    sample: str           # texte le plus large attendu (mesure)
    lo: float
    hi: float
    ramp: str = "load"    # "temp" -> rampe vert/orange/rouge de la palette
    sub: tuple = None     # (src, fmt, sample) valeur secondaire
    graph_hi: float = None

    def colors(self, p, i=0):
        return p.temp_ramp if self.ramp == "temp" else p.load_ramp(i)


PCT, TMP = "{:.0}%", "{:.0}°C"
_pct = lambda s: (s, "{:.0}%", "100%")
_tmp = lambda s: (s, "{:.0}°C", "100°C")

CPU = Metric("CPU", "cpu.load", PCT, "100%", 0, 100, "load", _tmp("cpu.temp"))
CPUT = Metric("CPU", "cpu.temp", TMP, "100°C", 20, 100, "temp", _pct("cpu.load"))
GPU = Metric("GPU", "gpu.load", PCT, "100%", 0, 100, "load", _tmp("gpu.temp"))
GPUT = Metric("GPU", "gpu.temp", TMP, "100°C", 20, 100, "temp", _pct("gpu.load"))
GPUH = Metric("POINT CHAUD", "gpu.hotspot", TMP, "100°C", 20, 110, "temp", _tmp("gpu.temp"))
VRAM = Metric("VRAM", "gpu.vram_used_pct", PCT, "100%", 0, 100, "load", ("gpu.vram_used_gb", "{:.1} Go", "99.9 Go"))
RAM = Metric("RAM", "ram.used_pct", PCT, "100%", 0, 100, "load", ("ram.used_gb", "{:.1} Go", "99.9 Go"))
SSD = Metric("SSD", "ssd.1.used_pct", PCT, "100%", 0, 100, "load", _tmp("ssd.1.temp"))
SSDT = Metric("SSD", "ssd.1.temp", TMP, "100°C", 20, 90, "temp", _pct("ssd.1.used_pct"))
DC = Metric("C:", "disk.C.used_pct", PCT, "100%", 0, 100, "load", ("disk.C.free_gb", "{:.0} Go libres", "999 Go libres"))
DD = Metric("D:", "disk.D.used_pct", PCT, "100%", 0, 100, "load", ("disk.D.free_gb", "{:.0} Go libres", "999 Go libres"))
MOBO = Metric("CARTE MÈRE", "mobo.temp", TMP, "100°C", 20, 90, "temp")
CHIP = Metric("CHIPSET", "chipset.temp", TMP, "100°C", 20, 90, "temp")
VRM = Metric("VRM", "vrm.temp", TMP, "100°C", 20, 110, "temp")
NETD = Metric("DESCENTE", "net.down", "{:.1} Mo/s", "99.9 Mo/s", 0, 50, "load")
NETU = Metric("MONTÉE", "net.up", "{:.1} Mo/s", "99.9 Mo/s", 0, 20, "load")
DRD = Metric("LECTURE", "disk.read", "{:.0} Mo/s", "9999 Mo/s", 0, 3000, "load")
DWR = Metric("ÉCRITURE", "disk.write", "{:.0} Mo/s", "9999 Mo/s", 0, 3000, "load")
LIQ = Metric("LIQUIDE", "liquid.temp", "{:.1}°C", "100.0°C", 20, 60, "temp")
PUMP = Metric("POMPE", "pump.rpm", "{:.0} tr/min", "9999 tr/min", 0, 4000, "load")
FAN1 = Metric("VENT. 1", "fan.1.rpm", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
CFAN = Metric("VENTIRAD", "cpu.fan", "{:.0} tr/min", "9999 tr/min", 0, 2500, "load")
GFAN = Metric("VENT. GPU", "gpu.fan", "{:.0} tr/min", "9999 tr/min", 0, 3000, "load")
CLK = Metric("FRÉQ. CPU", "cpu.clock", "{:.0} MHz", "9999 MHz", 0, 6000, "load")
GCLK = Metric("FRÉQ. GPU", "gpu.clock", "{:.0} MHz", "9999 MHz", 0, 3000, "load")
CPW = Metric("PUISS. CPU", "cpu.power", "{:.0} W", "999 W", 0, 250, "load")
GPW = Metric("PUISS. GPU", "gpu.power", "{:.0} W", "999 W", 0, 450, "load")

# nom -> (tags de contenu, metriques). Les archetypes prennent les N premieres.
CONTENT = {
    "cpu-gpu": (("cpu", "gpu"), [CPUT, GPUT, RAM, SSD]),
    "quad": (("cpu", "gpu", "ram", "ssd"), [CPU, GPU, RAM, SSD, LIQ, NETD]),
    "disques": (("disques", "ssd", "ram"), [DC, DD, SSD, RAM, DRD, DWR]),
    "carte-mere": (("carte mère", "chipset", "vrm", "températures"), [CPUT, MOBO, CHIP, VRM, SSDT, GPUT]),
    "reseau": (("réseau", "disque", "débit"), [NETD, NETU, DRD, DWR]),
    "refroidissement": (("liquide", "pompe", "ventilateurs", "aio"), [LIQ, PUMP, FAN1, CFAN, GFAN, CPUT]),
    "gpu": (("gpu", "vram"), [GPUT, GPUH, GPU, VRAM, GCLK, GPW]),
    "cpu": (("cpu", "fréquence", "puissance"), [CPUT, CPU, CLK, CPW, CFAN, RAM]),
    "puissance": (("puissance", "fréquences"), [CPW, GPW, CLK, GCLK, CPUT, GPUT]),
    "liquide": (("liquide", "aio"), [LIQ, PUMP, CPUT, CFAN]),
    "ram": (("ram", "mémoire", "disques"), [RAM, SSD, DC, DD]),
}
