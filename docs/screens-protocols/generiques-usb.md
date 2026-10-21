# Ecrans USB generiques (AliExpress / "USB-C 3,5 / 5 / 8,8 pouces")

Date : 2026-10-21. **[V]** verifie dans une source, **[S]** suppose.
Les familles Turing / XuanFang / UsbPCMonitor (serie) et Thermalright sont traitees par d'autres agents ; ce fichier ne fait que situer les autres familles et les recoupements avec les constructeurs d'AIO.

## Famille TURZX / Turing USB (VID 1CBE) : le point de convergence

- `mathoudebine/turing-smart-screen-python` @2b33ab4 (GPL-3.0, 2300+ etoiles) [V `library/lcd/lcd_comm_turing_usb.py`] : VID **1CBE**, PID : 0028 (2,8" rond 480x480), 0046 (4,6" 320x960), 0050 (5,2" 720x1280), 0080 (8,0" 800x1280), 0088 (8,8" 480x1920), 0092 (9,2" 462x1920), 0123 (12,3" 720x1920). Protocole : DES-CBC (cle `slv3tuzx`), JPEG (101), PNG (102), H264 par blocs. Liste officielle des revisions materielles : wiki/README du projet (revisions A/B/C/D/flagship + UART et USB).
- **Meme famille que Lian Li** (HydroShift II, Lancool 207, Universal Screen, Vision 9.2, TL/SL Flex LCD : PID 1CBE:A0xx) selon `sgtaziz/lian-li-linux` @f55f3e5 (MIT) : voir `lian-li.md`. Implication : un driver "TURZX USB" unique + une table de PID couvre ecrans generiques ET une bonne partie de Lian Li.
- AIDA64 gere "Turing (TURZX)" 2,1" 480x480, 3,5" 480x320, 5" 800x480, 7" 1024x600, 8,8" 1920x480 depuis fev. 2025 (forum : <https://forums.aida64.com/topic/16594-new-lcd-device-support-turing-turzx/>).

## Series CH340/CDC (UART) : identification a titre informatif [V : turing-smart-screen-python]

| Famille | VID:PID / identifiant | Remarque |
|---|---|---|
| Rev A / B (Turing 3,5") | 1A86:5722 ; serie USB35INCHIPSV2 | UART |
| Rev C | 1A86:CA21, 0525:A4A7, 1D6B:0121 / 0106 | UART |
| Rev D | 454D:4E41 | UART |
| WeAct Studio Display FS V1 (0,96" 80x160 / 3,5" 320x480) | 1A86:FE0C, serie commence par "AB" (rev A) ou "AD" (rev B) | CDC/UART ; 115200 bauds (0,96") et 1 152 000 (3,5") selon `colmarius/display-fs` (MIT) ; protocole communautaire (decrit via analyse Wireshark, non officiel) |

Ceux-ci sont deja du ressort de l'agent Turing/XuanFang (serie).

## AX206 / DPF "cadres photo hackes"

- Puce AppoTech **AX206**, USB bulk, sans pilote, **1908:0102** observe sur un cadre (forum OpenWrt, <https://forum.archive.openwrt.org/viewtopic.php?id=34133>) [V pour ce cas, pas pour tous]. Outils : `dpf-ax` / `dpflib` (SourceForge), `lcd4linux` avec pilote AX206 (fork <https://github.com/ukoda/lcd4linux-ax206>). Resolutions courantes : 320x240, 480x272, 800x480 [S, a verifier par modele]. AIDA64 le gere via un module "AX206 LCD" dedie (extensions `.axlcd`).
- Produit vieillissant, mais protocole bien connu (blocs RGB565 en bulk). Priorite basse sauf demande utilisateur.

## Jonsbo 960x376

Voir `autres-constructeurs.md` : 345F:9132 ; probablement un adaptateur video USB (pilote "MSUSBDisplay") et non un LCD a protocole proprietaire [S].

## "Cloudfree", Weact autres, ecrans "SmartMonitor HID"

- "Cloudfree" : aucun projet pertinent trouve par `gh search repos` ; ne rien affirmer.
- `Agentry433/turing-smart-screen-python-HIDdev2` (fork GPL-3.0, octobre 2026) : annonce un support "HID SmartMonitor" en plus de Turing/XuanFang ; non lu en detail [?].

## Effort / priorite

Un driver TURZX USB (1CBE) a une couverture maximale (generiques + Lian Li) : priorite **1** si l'agent Turing ne le couvre pas deja (l'etat actuel de la spec ne mentionne que la serie). AX206 : priorite 3.
