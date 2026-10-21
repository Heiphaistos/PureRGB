"""Formats d'ecran : dimensions de conception + zone sure."""
from dataclasses import dataclass


@dataclass(frozen=True)
class Fmt:
    key: str          # slug d'id : [a-z0-9]
    label: str        # libelle francais (tag)
    w: int
    h: int
    round: bool = False

    @property
    def ratio(self) -> float:
        return self.w / self.h

    @property
    def short(self) -> int:
        return min(self.w, self.h)

    @property
    def margin(self) -> float:
        return round(self.short * 0.08, 1)

    @property
    def zone(self) -> tuple:
        """Zone sure (x, y, w, h) : marge de 8 % du petit cote ; rond = carre inscrit."""
        if self.round:
            s = self.short * 0.66
            return ((self.w - s) / 2, (self.h - s) / 2, s, s)
        m = self.margin
        return (m, m, self.w - 2 * m, self.h - 2 * m)

    @property
    def wide(self) -> bool:
        return self.ratio >= 1.7

    @property
    def strip(self) -> bool:
        return self.ratio >= 2.5


FORMATS = {f.key: f for f in [
    Fmt("s480", "carré 480", 480, 480),
    Fmt("s320", "carré 320", 320, 320),
    Fmt("s240", "petit 240", 240, 240),
    Fmt("r43", "4:3 320×240", 320, 240),
    Fmt("w169", "16:9 854×480", 854, 480),
    Fmt("w1280", "large 1280×480", 1280, 480),
    Fmt("u1920", "ultra-large 1920×462", 1920, 462),
    Fmt("rb640", "ruban 640×172", 640, 172),
    Fmt("rond", "rond 480", 480, 480, True),
]}
