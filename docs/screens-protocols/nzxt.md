# NZXT (CAM) - Kraken Z / 2023 / 2024

Date : 2026-10-21. Etat PureRGB : deja pilote via liquidctl (ne pas refaire). Cette fiche sert au rattachement a l'editeur d'ecrans (phase 2) et aux limites connues.
Clone lu : `liquidctl/liquidctl` @292dc56 (GPL-3.0+), `liquidctl/docs/kraken-x3-z3-guide.md`, `liquidctl/driver/kraken3.py`.

## Logiciel officiel

- **NZXT CAM** (Windows 10/11, pas de Linux/macOS prevu), 4.76.6 au 2026-10-14. Page : <https://nzxt.com/pages/cam> ; installeur annonce `nzxt-app.nzxt.com/NZXT-CAM-Setup.exe` (metadonnee de la page, non telecharge). Pas d'anciennes versions proposees (mise a jour automatique). [recherche web]

## Modeles et VID:PID (VID 1E71) [V : `kraken3.py`]

| Produit | PID | Ecran | Remarque liquidctl |
|---|---|---|---|
| Kraken Z53/Z63/Z73 | 3008 | 320x320 | bulk 512 o |
| Kraken 2023 Elite | 300C | 640x640 | liquidctl l'etiquette "(broken)" : bulk 2 Mo |
| Kraken 2023 (Standard) | 300E | 240x240 | idem, GIF indisponible en firmware 2.x (liquidctl issue 631) |
| Kraken 2024 Elite RGB | 3012 | 640x640 | GIF OK au firmware 1.2.1 selon le guide |
| Kraken 2024 Plus | 3014 | 240x240 | |

## Protocole [V]

Canal controle : HID 64 o (`0x30 0x01` info LCD, `0x30 0x02` luminosite/orientation, `0x70 0x02 0x01 0xB8 <intervalle>` etc.). Donnees image : bulk WinUSB endpoint 0x02 (sous Windows liquidctl passe par `winusbpy`), memoire LCD interne totale `24320` unites, images/GIF redimensionnes. Documente seulement sous forme de code GPL ; la spec ecrite = le guide liquidctl (GPL, doc). PureRGB gere deja ce flux : voir le module existant.

## Autres projets de reference (NZXT)

- `damachine/coolerdash` (MIT) : rendu temps reel via l'API CoolerControl (>= 3.1.0) pour Kraken 2023 / Z63 / Plus 240 ; Elite 2023 partiel a cause des "bucket errors" de liquidctl. <https://github.com/damachine/coolerdash>
- SignalRGB : pas de controle LCD fiable pour Kraken Elite (fils de forum <https://forum.signalrgb.com/t/support-and-or-workaround-to-allow-rgb-control-of-nzxt-kraken-elite-with-lcd-screen/8720>).

## A decouvrir par REA

1. **Kraken 2023 Elite (300C) et GIF firmware 2.x** : cause exacte des "bucket errors" et du mode GIF casse ; rejouer l'envoi de CAM (capture USBPcap sur l'endpoint bulk 0x02).
2. Frequence reelle max d'envoi, formats de file (GIF vs flux d'images), verrous quand CAM tourne.
3. Anneau lumineux Elite (hors perimetre ecran).

Methode : CAM (Electron ; `analyze_javascript_application` sur l'asar) ; chaines `bucket`, `0x300C`, `1E71`, `lcd`, `gif`, `bulk` ; USBPcap filtre `usb.idVendor == 0x1e71`, scenario : envoi GIF puis image fixe puis liquid mode.

## Effort / priorite

Rattachement a l'editeur : effort **faible** (existant). Correctif Elite/GIF : priorite 2. **Priorite 1 pour l'integration** (utilisateurs nombreux).
