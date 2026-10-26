# Groupe C : constructeurs PC/portables, manettes, webcams/capture, audio, refroidissement, contrôleurs génériques, alimentations

Date : 2026-10-26. **[V]** lu dans une source citée, **[S]** supposé / une seule source indirecte, **INCONNU** non établi.
Catalogue : `src-tauri/assets/rgb_usb_catalog_extra_c.json` (53 appareils, 10 fabricants). Aucun PID inventé : chaque ligne vient d'une des sources ci-dessous.

## Sources (faits seulement, aucun code copié)

- `usb.ids` (linux-usb.org, daté 2026-06-26) : VID/PID et noms. Copie locale `D:\Projet\Analyses\rgb\usb.ids`.
- OpenRGB `Controllers/*/…Detect.cpp` (GPL-2.0+, copie locale `D:\Projet\Analyses\rgb\openrgb-src`) : tous les détecteurs HID des marques du groupe étaient déjà dans le catalogue principal (Alienware/Dell G, HP Omen 30L, HyperX, Legion/ITE, Cougar, Creative G6, Edifier Halo, GameSir Nova 2 Lite, Trust, DeepRGB/Airgoo/YICO).
- SignalRGB `signalrgb-devices.csv` (déjà dans ce dossier) : PID absents du catalogue (Alienware, HyperX, Aqua Computer, Arctic, Creative Katana, SRGBmods, Thermaltake, Acer P09-600).
- liquidctl (`D:\Projet\Analyses\rgb\cooling\liquidctl`) : Aquacomputer (f00a, f00d, f00e, f010, f011), NZXT E-series (7793).
- Linux `drivers/input/joystick/xpad.c` (master, 2026-10-26) : VID 20D6 = PowerA ; 11C9 = Nacon ; 146B = BigBen ; 24C6 = PowerA/Hori/Razer (OEM).
- litra-rs (`src/lib.rs`, licence MIT, 2026-10-26) : protocole Logitech Litra.

**Logiciels officiels : aucun installé, aucun téléchargé, aucune décompilation REA/Ghidra dans cette passe.** Les sources publiques ci-dessus ont suffi pour ce qui est établi ; pour le reste
(AWCC, Omen Gaming Hub, Legion Toolkit, Predator/Nitro Sense : logiciels de portable qui touchent BIOS/EC/ventilateurs, interdits d'installation ici ; JBL QuantumENGINE, Creative App, Elgato
Control Center/Camera Hub, AVerMedia RECentral, Govee/Hue/Nanoleaf : non analysés) le protocole reste **INCONNU**. Aucun pilote ni service ajouté, rien à désinstaller, aucun processus restant.

## Résultats par marque

| Marque | VID | Établi | Catalogue / code |
|---|---|---|---|
| Alienware / Dell G | 187C | Dell G Series 0550/0551, AW410K/510K, moniteurs 100E/1013 déjà là [V OpenRGB]. Ajout : 0511 (AlienFX Mobile), 0513 (Aurora R4) [usb.ids], 0530 (15 R3), moniteurs 1008, 100B, 1010, 1014 [S SignalRGB]. Protocole portable AWCC : **INCONNU** | 7 PID |
| HP Omen / HyperX | 03F0, 0951, 103C | Omen 30L (103C:84FD) déjà là ; clavier 4 zones Omen par WMI (OpenRGB), pas USB. Ajout : OMEN Mindframe 03F0:2841 [usb.ids] ; HyperX Pulsefire Haste (Wireless 028E/048E, 2 : 0B97/0D97/0F98, 2 Mini : 0BA0/0DA0), Cloud Flight 0951:16C4/1723, QuadCast S 0951:171D, Pulsefire FPS Pro 0951:1729 [S]. Pilote natif HyperX existant (`hyperx.rs`) | 12 PID |
| Acer Predator / Nitro | 0502, 04F2 | Clavier Nitro (HID ENEC) déjà dans le catalogue via OpenRGB. `usb.ids` n'a aucun produit gaming Acer (0502 = anciens PDA). Ajout : Cooler Master OEM « Acer P09-600 RGB controller » 2516:010B [S]. Protocole Predator/Nitro Sense : **INCONNU** | 1 PID |
| Lenovo Legion | 17EF, 048D | Legion K500/M300/K510 et claviers de portable 4 zones (ITE 048D) déjà là [V OpenRGB]. Rien d'ajouté. Legion Toolkit/Vantage non analysés | 0 |
| Razer Blade | – | couvert côté Razer | – |
| 8BitDo | 2DC8 | VID [usb.ids]. Aucun éclairage piloté par PC trouvé dans les sources locales ; non classé NO_RGB faute de preuve par produit | vendeur |
| PowerA | 20D6, 24C6 | VID [xpad.c]. Manettes Xbox filaires, éclairage non piloté par PC dans les sources : **INCONNU** | 2 vendeurs |
| PDP | 0E6F | VID déjà là (« Logic3 »). Afterglow Prismatic (0139, 02B3, 02B8 [xpad.c]) : LED gérées par le protocole Xbox One, aucun pilote PC. Non ajoutés (ne sont pas des contrôleurs RGB PC) | 0 |
| Hori / Nacon / BigBen / Thrustmaster | 0F0D, 11C9, 146B, 044F | VID [usb.ids, xpad.c]. Pas d'éclairage RGB piloté par PC trouvé (Thrustmaster Sol : détecteur OpenRGB sans VID/PID fixe, **INCONNU** côté USB) | 4 vendeurs |
| Turtle Beach, Flydigi, GameSir | 10F5, 3537 | déjà documentés / dans le catalogue (GameSir Nova 2 Lite 100F/1098) | 0 |
| Elgato | 0FD9 | Stream Deck déjà là. Key Light et Light Strip = **réseau** (OpenRGB `ElgatoKeyLight`) : liste sans pilote. Caméras/capture : PID absents des sources, **INCONNU** | 0 |
| Logitech Litra | 046D | **[V litra-rs]** voir ci-dessous. Glow C900, Beam C901/B901 : lumière blanche pilotée (marche, luminosité, température) ; Beam LX C903 : blanc + face arrière couleur | 4 PID + pilote |
| AverMedia | 07CA | Live Gamer Ultra Pro-RGB D553 [usb.ids] ; protocole d'éclairage **INCONNU** | vendeur + 1 PID |
| Creative | 041E | Sound BlasterX G6 déjà là ; AE-5 = carte PCI (détecteur OpenRGB sans USB) ; Katana 3247 [S] | 1 PID |
| JBL Quantum | – | VID/PID non vérifiables dans les sources locales : **INCONNU**, rien ajouté (pas de PID inventé) | 0 |
| Edifier | 2D99 | Halo Pixelbar A106 déjà là [V OpenRGB] | 0 |
| Arctic | 1A86, 3904 | A-RGB par série CH340 déjà documenté (`autres-refroidissement.md`). Ajout : « Arctic Fan Controller » 3904:F001 [S SignalRGB] | vendeur + 1 PID |
| Noctua | – | Aucun périphérique USB dans usb.ids/OpenRGB/SignalRGB/liquidctl : rien à reconnaître, NO_RGB impossible (pas de VID:PID) | – |
| Scythe, ID-Cooling (hors écrans), Jonsbo, Montech, Raijintek | – | Ventilateurs/AIO ARGB 5 V sur en-tête carte mère (SignalRGB « composants » sans VID/PID, ex. Montech AX120/AX140, ID-Cooling TF120/Zoomflow X). Aucun contrôleur USB : rien à ajouter | – |
| Cougar | (voir OpenRGB) | Revenger ST et 700K EVO déjà là [V OpenRGB] | 0 |
| Aqua Computer (refroidissement) | 0C70 | **[V liquidctl]** Farbwerk F00A, Quadro F00D, D5 Next F00E, Farbwerk 360 F010, Octo F011 ; [S SignalRGB] Farbwerk Nano F00F, High Flow Next F012. Protocole HID lisible dans liquidctl, **non implémenté** | vendeur + 7 PID |
| Cartes RGB génériques | 16D0, 1A86 | SRGBmods 113B/1176/1201 [S]. WS2812 USB = séries (1A86:7523 déjà). LEDENET, Govee, Hue, Nanoleaf, WLED = **réseau** : liste sans pilote, aucune trame envoyée | 3 PID |
| Alimentations | 7793, 264A | NZXT E500/E650/E850 (7793:5911/5912/2500) → **NO_RGB** [V liquidctl `nzxt_epsu.py` : set_color = NotSupportedByDevice]. Thermaltake « DPSG PSU » 264A:2329 (12 LED) [S]. Seasonic, be quiet! Dark Power Pro, MSI MPG : aucune interface USB identifiée, rien à reconnaître | 1 PID + 3 NO_RGB |
| Thermaltake (accessoires) | 264A | LedBox/Pacific, Floe Riing 1FA4, Level 20, Argent [S] ajoutés au catalogue (protocole Riing déjà dans `thermaltake.md`) | 19 PID (dont 2329) |

## Logitech Litra : protocole [V litra-rs, MIT, une implémentation]

- Interface HID : page d'usage **0xFF43** ; PID : Glow C900, Beam C901 et B901, Beam LX C903.
- Rapports 20 octets, `11 FF <idx> <cmd> …` ; idx `04` (Glow/Beam) ou `06` (Beam LX) pour la lumière blanche : marche `1C`, luminosité `4C` (u16 grand-boutiste), température `9C` (u16, 2700 à 6500 K). Implémentés (voir `litra.rs`) : la couleur choisie dans PureRGB est convertie (noir = éteint, luminosité = max(R,V,B) sur la plage Glow 20..250 / Beam 30..400 lm, température = 2700 K + 3800 K x B/(R+B), arrondie à 100 K).
- **Face arrière couleur (Beam LX seulement)** : zone 1 à 7 : `11 FF 0C 1B <zone> R G B FF 00 00 00 FF 00 00 00 FF 00 00 00` puis validation `11 FF 0C 7B 00 00 01 00 00`. La source ramène chaque composante à 1 minimum (« l'appareil se comporte mal à 0 »). Luminosité arrière `11 FF 0A 2B 00 <%>`, marche arrière `11 FF 0A 4B <0|1>` : non utilisées.
- Pilote : `backends/hid/native/litra.rs`, famille `litra`, **expérimental** (non vérifié sur matériel), tests MockHid (3). Aucune écriture persistante connue, non testé avec l'appareil : le rapport de validation de 9 octets de la source est complété à 20 par PureRGB (équivalent du remplissage de hidapi, à confirmer).

## Non fait / reste

Analyse des logiciels officiels (voir plus haut), protocoles Aquacomputer (liquidctl les décrit), Thermaltake DPSG, AverMedia, Elgato réseau, JBL : à traiter si Momo le demande. Aucun pilote écrit sans protocole [V].

## Suite (2026-10-26)

- **Litra** : marche, luminosité (lm) et température (K) exposées par `DeviceSetting` (`white_on`, `white_lumens`, `white_kelvin`), en plus de la correspondance couleur. Tests MockHid.
- **Aqua Computer** : liquidctl (`aquacomputer.py`) lit les capteurs et écrit la vitesse des ventilateurs en réécrivant tout le rapport de configuration 0x03 (lecture-modification-écriture + CRC-16 USB), donc probablement persistant : **refusé** (pas de profil, pas de ventilateur). `set_color` y est « not yet reverse engineered » : éclairage **INCONNU** ; il faudrait décompiler aquasuite.
- Non traités à ce jour : Creative, JBL, manettes, HyperX/Omen périphériques, Arctic, Elgato (aucun logiciel officiel téléchargé ni décompilé).
