# Corsair (iCUE / iCUE LINK)

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Clones lus : `jurkovic-nikola/OpenLinkHub` @c8c1d2a (GPL-3.0+), `zorrobyte/icue-link-lcd-reactor` @11418ae (MIT).

## Logiciel officiel

- **iCUE** (Windows). Telechargements : <https://www.corsair.com/us/en/s/downloads> ; presentation <https://www.corsair.com/us/en/s/icue> ; notes de version <https://www.corsair.com/us/en/explorer/gamer/accessories/icue-release-notes-5697/> (5.52.87 au 2026-10-06). "iCUE Apps" (beta) ne peut pas tourner en meme temps qu'iCUE 5. [recherche web, non telecharge]

## Modeles d'ecran

| Produit | Ecran | Liaison | VID:PID |
|---|---|---|---|
| iCUE LINK TITAN / H-series LCD (capot AIO) | **480x480 rond** | **HID** direct | **1B1C:0C4E** ("iCUE LINK AIO LCD Screen Module") [V : OpenLinkHub `src/devices/lcd/lcd.go`, PID decimal 3150 ; README du reactor] |
| iCUE LINK XD5 LCD (bloc pompe) | 480x480 [S : meme driver] | HID | 1B1C:0C43 (3139) [V] |
| Nautilus LCD cap | 480x480 [S] | HID | 1B1C:0C57 / 0C55 (3159 / 3157) [V] |
| XC7 ELITE LCD (waterblock CPU) | [?] | HID | 1B1C:0C42 (3138) [V : present dans la table, driver `xc7`] |
| iCUE LINK 5" LCD Screen Module | 720x1280 IPS 5" | **DisplayPort / moniteur Windows secondaire** (selon Corsair) | aucun protocole d'ecran a ecrire : fenetre plein ecran sur ce moniteur. Source : <https://www.corsair.com/fr/zh/explorer/diy-builder/cpu-coolers/icue-link-aio-lcd-screen-module/> (specs vues en resultats de recherche) |
| Hydro Elite LCD / Elite LCD XT (H100i/H150i Elite LCD XT) | 2,1" [S] | [?] | **PID non trouve** : OpenLinkHub liste les H100i/H115i/H150i ELITE RGB (PID decimaux 3125-3127, 3136-3137) sans ecran. |

## Protocole HID de l'ecran 480x480 : documente publiquement

Source : README de `icue-link-lcd-reactor`, section "How the screen works" (MIT) [V], recoupe avec OpenLinkHub (GPL-3.0+, `src/devices/lcd/lcd.go` : a lire, pas a copier).

- L'ecran est un peripherique HID separe du System Hub (1B1C:0C3F) : utilisable sans toucher au hub.
- Trame = JPEG 480x480 decoupe en rapports de sortie HID de 1024 o :

```
octets 0-2   02 05 01
octet 3      01 sur le dernier morceau (declenche l'affichage), sinon 00
octet 4      indice du morceau
octet 5      00
octets 6-7   longueur du morceau, little endian (max 1016)
octet 8+     donnees JPEG
```
- Luminosite : feature report `03 0B <0-100> 01` ; rotation : `03 0C <0-3> 01`.

Confiance : **elevee** pour 0C4E (deux projets independants, un teste en continu) ; **moyenne** pour 0C43 / 0C57 / 0C55 (meme driver cote OpenLinkHub, non decrits).

## A decouvrir par REA

1. **Elite LCD / Elite LCD XT** : PID, resolution, protocole. Priorite 2 (modele repandu, rien en OSS).
2. Frequence max, profils en memoire interne, capteurs affiches par le firmware.
3. Coexistence avec le service iCUE (verrouillage HID).

Methode :
- Cible : `iCUE.exe` et ses modules de peripheriques (chemin d'installation [S] `%ProgramFiles%\Corsair\CORSAIR iCUE 5 Software\`, a verifier).
- Chaines : `0C4E`, `1B1C`, `LCD`, `JPEG`, `Elite`, `XT`, `ScreenModule`.
- Capture : USBPcap, filtre `usb.idVendor == 0x1b1c`, scenarios : changer de theme LCD, rotation, luminosite ; Elite LCD XT en priorite.
- 5" LCD : verifier par `Get-PnpDevice` qu'il s'agit d'un moniteur EDID standard.

## Effort / priorite

0C4E : effort **faible** (JPEG decoupe), priorite **1**. Elite LCD XT : priorite 2, effort inconnu. 5" DisplayPort : hors driver, priorite 3.
