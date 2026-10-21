# ID-COOLING (FX LCD)

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Clones lus : `yesthatsjames/lightgarden` @f0b379f (MIT, `lcd.py` en-tete = protocole), `w1n-ror/fx_lcd_software-for-linux` @91cf8c4 (sans licence).

## Logiciel officiel

- **FX LCD / FX LCD PE Series Software 1.0.4** (Windows 10/11). Page support : <https://www.idcooling.com/Support/file?type=soft> ; fiches produit <https://www.idcooling.com/product/detail?id=566&name=FX360+LCD> et <https://www.idcooling.com/product/detail?id=567&name=FX360+LCD+PE>. [recherche web, non telecharge]

## Modeles

| Produit | Ecran | VID:PID |
|---|---|---|
| FX240 / FX360 LCD | **240x240** (firmware `V26_FX_LCD_1.48_02.0`, 1,48 = pouces) | **2000:3000** ("HOTSPOTEKUSB HID DEMO") [V : deux depots] |
| FX360 LCD PE, FX360 TD, SL/SPACE 2,1" | 480x480 pour la serie SPACE/SL [V : lightgarden le dit "different produit, non supporte"] | [?] |

## Protocole [V, lightgarden `lcd.py` docstring]

- HID, pas de report ID ; chaque ecriture = 1 octet `0x00` hidraw + 1024 o. Commandes : magique `"CRT"` + `00 00` + opcode ASCII de 3 lettres.
- Image : en-tete de 32 o **concatene au JPEG** puis le tout decoupe en paquets de 1024 : `"CRT\0\0" "DRA\0\0" <len BE16 incluant l'en-tete> 0xB1 + 19 octets a 0` puis JPEG baseline. Plafond : BE16 => JPEG < 65 503 o. Pas d'opcode de commit : on **diffuse en boucle** et l'ecran s'efface quand le flux s'arrete. Pause obligatoire ~3 ms entre paquets.
- Identite lisible sans ecriture via feature report `0x01`.
- Famille de puces : "Shenzhen Hotspot", meme OEM que les Mirabox / AJAZZ Stream Dock (observation de l'auteur) [V mais interpretation de l'auteur].

Confiance : **moyenne** (un auteur precis qui a corrige ses hypotheses, recoupe par un second depot).

## A decouvrir par REA

1. Modeles 480x480 (SPACE/SL) : tout.
2. Opcodes autres que `DRA` (luminosite, rotation materielle, video, stockage).
3. Raison du "flux continu sans commit" : y a-t-il un mode d'image persistante ?

Methode : exe "FX LCD Series Software" (typage REA), chaines `CRT`, `DRA`, `2000`, `3000`, `HOTSPOTEKUSB`, `V26_FX`, capture USBPcap `usb.idVendor == 0x2000`.

## Effort / priorite

Effort **faible** (JPEG + en-tete simple), mais ecran 240x240 limite ; priorite **3** (parc modeste, produits budget).
