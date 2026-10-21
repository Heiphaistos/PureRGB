"""Palettes coherentes. Ajouter une ligne a _ROWS (voir README). Couleurs #rrggbb / #rrggbbaa.

Champs : cle, libelle, sombre?, fond (de, vers, angle), texte, texte2, 3 accents,
piste, carte (translucide), rampe temperature (froid, tiede, chaud), police d'affichage, tags.
"""
from dataclasses import dataclass

from qa import contrast


@dataclass(frozen=True)
class Palette:
    key: str
    label: str
    dark: bool
    bg0: str
    bg1: str
    angle: int
    text: str
    text2: str
    acc: tuple
    track: str
    card: str
    ramp: tuple
    font: str
    tags: tuple = ()

    @property
    def line(self) -> str:   # lisere de carte : texte a ~14 % d'opacite
        return self.text + "24"

    @property
    def temp_ramp(self) -> list:
        return [[0, self.ramp[0]], [0.55, self.ramp[1]], [1, self.ramp[2]]]

    def load_ramp(self, i: int = 0) -> list:
        return [[0, self.acc[i % 3]], [1, self.acc[(i + 1) % 3]]]

    @property
    def background(self) -> dict:
        if self.bg0 == self.bg1:
            return {"kind": "solid", "color": self.bg0}
        return {"kind": "gradient", "from": self.bg0, "to": self.bg1, "angle": self.angle}


D, L = True, False
I, R, M = "Inter", "Rajdhani", "Roboto Mono"
# cle, libelle, sombre, fond0, fond1, angle, texte, texte2, (acc1, acc2, acc3), piste, carte, rampe, police, tags
_ROWS = [
    ("neon", "Néon", D, "#07030f", "#14072e", 90, "#f5f0ff", "#a99bd6", ("#ff2bd6", "#00e5ff", "#b6ff3c"), "#271a4d", "#ffffff12", ("#39ff88", "#ffb020", "#ff2d55"), R, ("néon",)),
    ("ocean", "Océan", D, "#031826", "#07344d", 90, "#eaf8ff", "#8fb9cf", ("#22d3ee", "#38bdf8", "#5eead4"), "#14506f", "#ffffff12", ("#2dd4bf", "#fbbf24", "#f43f5e"), I, ("bleu",)),
    ("lave", "Lave", D, "#140403", "#2e0b06", 90, "#fff1e6", "#d6a58f", ("#ff5a1f", "#ffb020", "#ff2d2d"), "#40160f", "#ffffff10", ("#a3e635", "#fb923c", "#ef4444"), R, ("chaud",)),
    ("foret", "Forêt", D, "#04120a", "#0c2c19", 90, "#eaf7ee", "#92b8a0", ("#34d399", "#a3e635", "#facc15"), "#143d27", "#ffffff12", ("#4ade80", "#facc15", "#f87171"), I, ("vert",)),
    ("aurore", "Aurore", D, "#050a1c", "#0e3340", 45, "#effcff", "#9bbcd0", ("#5eead4", "#a78bfa", "#f472b6"), "#1d4f63", "#ffffff12", ("#5eead4", "#fde68a", "#fb7185"), I, ("dégradé",)),
    ("ambre", "Rétro ambre", D, "#0d0700", "#1c1000", 90, "#ffb000", "#b87a00", ("#ffb000", "#ffd166", "#ff7a00"), "#3a2400", "#ffb00014", ("#ffd166", "#ff9500", "#ff3b00"), M, ("rétro", "terminal")),
    ("crt", "CRT vert", D, "#000a02", "#001a07", 90, "#4dff88", "#27b556", ("#4dff88", "#b6ff6a", "#00ffc3"), "#06361a", "#4dff8814", ("#4dff88", "#d9ff4d", "#ff5a4d"), M, ("rétro", "terminal")),
    ("glace", "Glace", D, "#06121f", "#103049", 90, "#f0f9ff", "#9cc3dc", ("#7dd3fc", "#bae6fd", "#a5b4fc"), "#183a55", "#ffffff14", ("#7dd3fc", "#fde68a", "#fb7185"), I, ("froid",)),
    ("mono", "Sobre mono", D, "#0b0b0c", "#171719", 90, "#f2f2f2", "#8f8f96", ("#ffffff", "#c2c2c9", "#85858d"), "#2a2a2e", "#ffffff0f", ("#d4d4d8", "#fbbf24", "#ef4444"), I, ("sobre",)),
    ("pastel", "Pastel", L, "#fdf2f8", "#eaf0ff", 90, "#3b2f4a", "#6e5f85", ("#e0559d", "#6366f1", "#10a37f"), "#e3daf0", "#ffffffb8", ("#34c9a0", "#f5a524", "#f0506e"), I, ("clair",)),
    ("cyber", "Cyberpunk", D, "#0a0014", "#1d0036", 90, "#fdf6ff", "#c08bff", ("#fcee09", "#00f0ff", "#ff003c"), "#2e0f4d", "#ffffff12", ("#00f0ff", "#fcee09", "#ff003c"), R, ("néon",)),
    ("sunset", "Coucher de soleil", D, "#1b0a33", "#702049", 90, "#fff4ea", "#f3bfae", ("#ffb347", "#ff5e7e", "#ffd166"), "#4a1a4f", "#ffffff14", ("#ffd166", "#ff9a3c", "#ff3d5a"), I, ("chaud", "dégradé")),
    ("sakura", "Sakura", L, "#fff5f7", "#ffe1ea", 90, "#4a2733", "#8a5566", ("#d6336c", "#f06595", "#e8590c"), "#f6d3de", "#ffffffc0", ("#51cf9b", "#ffa94d", "#e03131"), I, ("clair", "rose")),
    ("carbone", "Carbone", D, "#0e0f11", "#1d1f23", 90, "#e8eaed", "#8d929b", ("#ff2d55", "#f59e0b", "#94a3b8"), "#2c2f35", "#ffffff0e", ("#7dd3a8", "#f59e0b", "#ff2d55"), R, ("sobre",)),
    ("minuit", "Minuit", D, "#050816", "#0f1a46", 90, "#e9edff", "#97a3d8", ("#6c8cff", "#a78bfa", "#38bdf8"), "#1b2658", "#ffffff12", ("#4ade80", "#fbbf24", "#f87171"), I, ("bleu",)),
    ("emeraude", "Émeraude", D, "#021a14", "#05382b", 90, "#e8fff6", "#8fc7b0", ("#10e0a0", "#5eead4", "#d9f99d"), "#0c4a39", "#ffffff12", ("#34d399", "#fcd34d", "#fb7185"), I, ("vert",)),
    ("rubis", "Rubis", D, "#17030a", "#40081c", 90, "#fff0f3", "#e0a1b0", ("#ff4d6d", "#ff8fa3", "#ffc2d1"), "#561026", "#ffffff12", ("#86efac", "#fbbf24", "#ff4d6d"), I, ("rouge",)),
    ("saphir", "Saphir", D, "#030b24", "#0a2a78", 135, "#eef3ff", "#9fb3ee", ("#4d8dff", "#79b8ff", "#b8d4ff"), "#1d45a8", "#ffffff14", ("#5eead4", "#fbbf24", "#f87171"), I, ("bleu",)),
    ("or", "Noir et or", D, "#0a0805", "#1a150a", 90, "#f7ecd0", "#a8966a", ("#e5b53c", "#f5d77a", "#b8862a"), "#2f2711", "#e5b53c12", ("#c9d98a", "#e5b53c", "#e0523d"), I, ("luxe",)),
    ("cuivre", "Cuivre", D, "#120904", "#2a1409", 90, "#ffeede", "#c99a7c", ("#e07a3c", "#f2a65a", "#c25b2a"), "#3d2113", "#ffffff10", ("#b5d48a", "#f2a65a", "#e0523d"), R, ("chaud",)),
    ("menthe", "Menthe", L, "#effcf6", "#d9f5ea", 90, "#12382c", "#4e7d6d", ("#0f9d74", "#0ea5b7", "#7c3aed"), "#c7e8da", "#ffffffc0", ("#22b27a", "#f59e0b", "#dc3b4a"), I, ("clair", "vert")),
    ("lavande", "Lavande", L, "#f5f0ff", "#e4dcff", 90, "#2e2250", "#66589a", ("#7c3aed", "#d946ef", "#2563eb"), "#d9cff5", "#ffffffc0", ("#2bb68c", "#f59e0b", "#e0364f"), I, ("clair", "violet")),
    ("papier", "Papier", L, "#faf6ec", "#efe7d3", 90, "#2b2418", "#75684c", ("#b45309", "#0f766e", "#9f1239"), "#e0d6bb", "#ffffffb0", ("#4d9a5c", "#d98a1c", "#c43d2f"), I, ("clair", "sobre")),
    ("ciel", "Ciel", L, "#eaf6ff", "#cfe8ff", 90, "#0d2a45", "#4d7194", ("#0b78d0", "#06a3b8", "#5b5fe0"), "#bddaf2", "#ffffffc0", ("#1fb57a", "#f59e0b", "#e0364f"), I, ("clair", "bleu")),
    ("sable", "Sable", L, "#f7f0e4", "#ecdcc0", 90, "#3a2a14", "#80693f", ("#c2410c", "#a16207", "#0f766e"), "#d6c29a", "#ffffffb0", ("#5d9d4e", "#d88a14", "#c8372d"), I, ("clair", "chaud")),
    ("nordique", "Nordique", D, "#1a202b", "#2b3547", 90, "#eceff4", "#97a3b8", ("#88c0d0", "#81a1c1", "#a3be8c"), "#3b4558", "#ffffff12", ("#a3be8c", "#ebcb8b", "#bf616a"), I, ("sobre", "froid")),
    ("terre", "Terre", D, "#1b1612", "#2e241b", 90, "#f0e4cf", "#a59478", ("#d79921", "#b8bb26", "#fb4934"), "#42352a", "#ffffff10", ("#b8bb26", "#fabd2f", "#fb4934"), M, ("rétro", "sobre")),
    ("dracula", "Violet nuit", D, "#191a2e", "#2a2347", 90, "#f4f1ff", "#a9a3d6", ("#bd93f9", "#ff79c6", "#8be9fd"), "#3a3465", "#ffffff12", ("#50fa7b", "#f1fa8c", "#ff5555"), M, ("sombre",)),
    ("synthwave", "Synthwave", D, "#120428", "#3b0a54", 90, "#fff0ff", "#d09be8", ("#ff3cac", "#2bd1ff", "#ffe66d"), "#4a1670", "#ffffff14", ("#2bd1ff", "#ffe66d", "#ff3cac"), R, ("néon", "rétro")),
    ("tropical", "Tropical", D, "#031c1c", "#0b4a42", 45, "#f2fff8", "#9ccfbf", ("#ff7a59", "#ffd23f", "#2de2c4"), "#145a50", "#ffffff12", ("#2de2c4", "#ffd23f", "#ff5a4d"), I, ("vif",)),
    ("acier", "Acier", D, "#14181d", "#262d36", 135, "#e9eef3", "#93a0ae", ("#7fb3d5", "#b0bec5", "#90caf9"), "#37414d", "#ffffff12", ("#8bd3a6", "#f0c36a", "#e96a6a"), I, ("sobre", "froid")),
    ("plasma", "Plasma", D, "#0c0320", "#3a0f6a", 45, "#f7f0ff", "#c4a8ee", ("#c084fc", "#f0abfc", "#67e8f9"), "#4a1d80", "#ffffff14", ("#67e8f9", "#fde047", "#fb7185"), R, ("dégradé",)),
    ("toxique", "Toxique", D, "#040a02", "#0d1a05", 90, "#efffe0", "#9cc070", ("#b6ff00", "#e6ff4d", "#7dff3c"), "#1f3a0a", "#b6ff0012", ("#b6ff00", "#ffb000", "#ff3b30"), R, ("néon", "vert")),
    ("corail", "Corail", L, "#fff4f0", "#ffe0d6", 90, "#4a2118", "#8e5a4c", ("#e8553a", "#f08a4b", "#2a9d8f"), "#f7d2c6", "#ffffffc4", ("#2fb384", "#f2a03d", "#dc3545"), I, ("clair", "chaud")),
    ("givre", "Givre", L, "#f2f8fc", "#dde9f3", 90, "#14283a", "#587289", ("#2b7fb8", "#5b8def", "#18a999"), "#c2d6e6", "#ffffffc8", ("#21b08a", "#f0a020", "#dc3b4a"), I, ("clair", "froid")),
    ("jungle", "Jungle", D, "#06140a", "#14340f", 90, "#efffe6", "#9fc58a", ("#8bd450", "#f2c94c", "#4ad0a8"), "#204a18", "#ffffff12", ("#8bd450", "#f2c94c", "#f2564a"), I, ("vert",)),
    ("galaxie", "Galaxie", D, "#02010a", "#160a36", 135, "#f1eeff", "#a79fd8", ("#8b7bff", "#ff7ad9", "#6ee7ff"), "#241a52", "#ffffff12", ("#6ee7ff", "#ffe08a", "#ff7a9a"), I, ("sombre",)),
    ("espresso", "Espresso", D, "#140d09", "#2b1c14", 90, "#f6e9dc", "#b79b86", ("#d9a066", "#e8c39e", "#a86b3c"), "#3f2b1f", "#ffffff10", ("#b9c98a", "#e8b04a", "#d9534f"), I, ("chaud", "sobre")),
    ("blueprint", "Plan bleu", D, "#0a2a5c", "#103d82", 90, "#eaf3ff", "#9cc0ee", ("#ffffff", "#7fd0ff", "#ffe27a"), "#1a4a8f", "#ffffff14", ("#9be7b5", "#ffe27a", "#ff9a8a"), M, ("rétro", "technique")),
    ("phosphore", "Phosphore bleu", D, "#00060d", "#001a2e", 90, "#7df0ff", "#3ba6bd", ("#7df0ff", "#4dc3ff", "#b8f4ff"), "#05304a", "#7df0ff12", ("#7df0ff", "#ffd24d", "#ff5a6a"), M, ("rétro", "terminal")),
    ("orange", "Noir et orange", D, "#0b0a09", "#1b1814", 90, "#f6f1ea", "#9a9186", ("#ff7a00", "#ffa640", "#ffd199"), "#322c24", "#ffffff0f", ("#9ad66f", "#ff9f1a", "#ff3b30"), R, ("sobre", "chaud")),
    ("rosegold", "Or rose", D, "#160b0f", "#33181f", 90, "#fdeef0", "#c7969f", ("#e8a0a8", "#f4c2c2", "#d4826f"), "#47232c", "#ffffff10", ("#b9d99a", "#f0b86a", "#e0626f"), I, ("luxe", "rose")),
    ("brume", "Brume claire", L, "#f4f5f7", "#e2e5ea", 90, "#1c2128", "#5f6875", ("#3b6fd4", "#0e9f8a", "#d9480f"), "#d3d7de", "#ffffffc8", ("#23a56f", "#e8930c", "#d6303c"), I, ("clair", "sobre")),
    ("magma", "Magma", D, "#0a0206", "#2a0716", 135, "#fff0ea", "#d49a95", ("#ff3d2e", "#ff9d00", "#ffd60a"), "#431022", "#ffffff10", ("#ffd60a", "#ff7a00", "#ff2d2d"), R, ("chaud",)),
    ("orchidee", "Orchidée", L, "#fbf1fb", "#f0d9f3", 90, "#41204a", "#855a8f", ("#b5179e", "#7209b7", "#e5383b"), "#dcbbe4", "#ffffffc0", ("#2fb384", "#f2a03d", "#dc3545"), I, ("clair", "violet")),
]


def _shade(col, bgs, target, dark):
    """Eclaircit (fond sombre) ou assombrit (fond clair) `col` jusqu'au contraste voulu sur tous les fonds."""
    h = col.lstrip("#")
    rgb = [int(h[i:i + 2], 16) for i in (0, 2, 4)]
    for step in range(21):
        c = "#" + "".join(f"{v:02x}" for v in rgb) + h[6:]
        if min(contrast(c, b) for b in bgs) >= target:
            return c
        rgb = [round(v + (255 - v) * 0.08) if dark else round(v * 0.92) for v in rgb]
    return c


def _make(r):
    p = Palette(*r)
    bgs = (p.bg0, p.bg1)
    f = lambda c, t: _shade(c, bgs, t, p.dark)
    return Palette(p.key, p.label, p.dark, p.bg0, p.bg1, p.angle, f(p.text, 4.5), f(p.text2, 3.6),
                   tuple(f(a, 3.0) for a in p.acc), p.track, p.card, tuple(f(a, 2.2) for a in p.ramp), p.font, p.tags)


PALETTES = {r[0]: _make(r) for r in _ROWS}
