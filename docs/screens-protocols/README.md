# Protocoles des mini-ecrans LCD : etat de l'art par constructeur (phase 2)

Recherche du 2026-10-21, branche `screens/research`. Documentation uniquement. Contexte : `docs/superpowers/specs/2026-10-21-screens-design.md`.
Legende dans chaque fichier : **[V]** verifie dans la source citee (depot clone ou page lue), **[S]** suppose/deduit, **[?]** inconnu. Rien n'est teste sur materiel par nous. Hors perimetre de cette recherche (autres agents) : Thermalright/TRCC et Turing/XuanFang serie. NZXT Kraken est deja pilote (liquidctl).

## Tableau recapitulatif priorise

Effort : F = faible (1-2 jours, trame simple), M = moyen, E = eleve (inconnu ou chiffrement). "Doc." = protocole documente publiquement (O = oui par un projet open source ou une doc ecrite, P = partiel, N = non). Priorite 1 = a implementer en premier.

| Prio | Constructeur (fichier) | Modeles / ecran | VID:PID principaux | Doc. | Effort | Logiciel a analyser avec REA |
|---|---|---|---|---|---|---|
| 1 | **Lian Li** (`lian-li.md`) | Galahad II LCD/Vision, HydroShift LCD (480x480 HID) ; HydroShift II, TL/SL Flex, Lancool 207, Universal Screen 8,8", Vision 9,2" (famille TURZX 1CBE) | 0416:7391/7395/7398/7399/739A ; 1CBE:A021/A034/A065/A088/A092/A018/A019/A068 | O (MIT) | M | L-Connect 3 (+ `LIANLI_display_driver.dll`) |
| 1 | **TURZX/Turing USB 1CBE generique** (`generiques-usb.md`) | 2,8" a 12,3" (480x480 a 720x1920) ; recoupe Lian Li | 1CBE:0028/0046/0050/0080/0088/0092/0123 | O (GPL ref., a decrire) | M | USBMonitor / logiciel Turing (autre agent) |
| 1 | **Corsair** (`corsair.md`) | iCUE LINK AIO LCD 480x480 rond ; XD5 / Nautilus (S) | 1B1C:0C4E (+0C43, 0C57, 0C55, 0C42) | O (MIT + GPL) | F | iCUE (surtout pour Elite LCD XT) |
| 1 | **DeepCool LM** (`deepcool.md`) | LM240/LM360 320x240 RGB565 | 3633:0026 | O (MIT) | F | DeepCreative |
| 2 | **Thermaltake** (`thermaltake.md`) | TH420 V2 Ultra EX 480x480 rond ; AIO LCD ; RC Pro 480x128 | 264A:233C / 2328 / 232A | P (2 depots recents) | F-M | TT RGB PLUS 2.0 |
| 2 | **ASUS ROG** (`asus.md`) | Ryujin III 360/Extreme/White/EVA 320x240 | 0B05:1AA2 / 1BCB / 1ADA / 1ADE | O (MIT, doc + captures) | M | Armoury Crate (AacAIOFanHal) |
| 2 | **NZXT** (`nzxt.md`) | Kraken Z / 2023 / 2024 (240-640) | 1E71:3008/300C/300E/3012/3014 | O (GPL, existant) | F (rattachement) | NZXT CAM (Elite 300C, GIF fw 2.x) |
| 2 | **MSI** (`msi.md`) | K360 320x240 ; K/V2, D, P13 2,1-2,4" | 0DB0:B130 (+CA00/CA02) ; autres inconnus | P (K360 seul) | E | MSI Center (EZ Display) |
| 2 | **Cooler Master** (`cooler-master.md`) | 360 Ion 2,1", Core LCD 6" 960x480, Stage LCD | inconnus (VID 2516) | N | E | MasterCTRL + CMAgent.exe |
| 2 | **HYTE** (`hyte.md`) | Y70 Touch Infinite 14,5" 688x2560 (moniteur DP) ; THICC Q60 ? | pas de VID:PID d'ecran necessaire | N/A (moniteur) | F (hors driver) | HYTE Nexus (Q60) |
| 3 | **Gigabyte AORUS** (`gigabyte.md`) | X II 240/360 LCD rond ; X gen.1 texte | 1044:7A4D (gen.1) ; X II inconnu | N (gen.1 P) | E | Gigabyte Control Center |
| 3 | **ID-COOLING** (`id-cooling.md`) | FX240/360 LCD 240x240 | 2000:3000 | P (2 depots) | F | FX LCD Series Software |
| 3 | **DeepCool Mystique** (`deepcool.md`) | 2,8" 640x480 | PID 9 sous VID 3633 (decimal, a confirmer) | N | E | DeepCreative |
| 3 | **Autres** (`autres-constructeurs.md`) | be quiet! Light Loop IO LCD, Phanteks Glacier One LCD 6", EK Nucleus Vision, ASRock, Antec, Montech, Zalman, Jonsbo (345F:9132), Arctic/Fractal (aucun ecran trouve) | inconnus | N | E | IO Center, etc. |
| 3 | AX206 / DPF (`generiques-usb.md`) | 320x240 a 800x480 | 1908:0102 (cas observe) | O (ancien) | M | aucun (tools open source) |

Reference transversale : `projets-reference.md` (liquidctl, CoolerControl, OpenLinkHub, lian-li-linux, OpenRGB, SignalRGB, AIDA64, HWiNFO, lcd4linux...).

## Les 5 constructeurs les plus rentables a implementer ensuite

1. **Lian Li + famille TURZX 1CBE** : le plus gros parc d'AIO/boitiers a ecran, protocole HID (Galahad II/HydroShift) documente et protocole 1CBE chiffre DES recoupe par 3 projets independants dont un MIT ; un seul driver "TURZX USB" ouvre aussi tous les ecrans generiques USB-C. Meilleur rapport couverture/effort.
2. **Corsair iCUE LINK (1B1C:0C4E)** : trame JPEG decoupee en rapports de 1024 o, 4 champs d'en-tete, luminosite/rotation par feature reports : le plus simple a implementer, grosse base installee.
3. **DeepCool LM240/LM360 (3633:0026)** : paquets `0xAA` + CRC-16/XMODEM, RGB565 320x240 en un write bulk, 5 paquets constructeur disponibles comme controle croise.
4. **ASUS ROG Ryujin III** : seul cas avec une spec ecrite avec captures (`protocol.md`, MIT) ; manque le flux temps reel, mais l'affichage capteurs et les GIF stockes sont deja valides.
5. **Thermaltake (TH420 V2 Ultra EX / AIO LCD)** : JPEG sur HID sans chiffrement, ecran rond 480x480 courant ; deux depots recents mais a recouper par capture.

(NZXT est a rattacher a l'editeur mais deja fait. HYTE Y70 = simple fenetre moniteur, utile mais hors driver.)

## Procedure commune REA (par constructeur)

L'outil REA n'etait plus connecte a la fin de cette session ; la methode ci-dessous est celle a executer quand il sera disponible. Les chemins d'installation sont des suppositions a verifier sur place.

1. **Preparer une VM Windows** (checkpoint) avec le logiciel officiel (liens dans chaque fichier). Brancher le materiel (passthrough USB) ou, a defaut, un PC de test. Ne jamais envoyer de mise a jour de firmware depuis la VM.
2. **Typer le binaire** : `binary_overview`, `inspect_pe_resources` ; Electron => extraire l'asar (`extract_artifact`) puis `analyze_javascript_application` ; .NET => `inspect_managed_artifact` / `compare_managed_members` ; natif => `list_strings`, `search_strings`.
3. **Chaines a chercher** : le VID:PID (en hexa et en decimal), le nom du produit, `hid`, `libusb`, `WinUSB`, `LCD`, `JPEG`, `RGB565`, `GIF`, `H264`, `crc`, `DES`, `brightness`, `rotate`.
4. **Capture USB** : USBPcap sur le bus du peripherique ; filtrer par `usb.idVendor` ; un scenario = une action (image fixe, luminosite, rotation, GIF, video, theme capteurs) ; `inspect_pcap_capture` / `inspect_packet_fields` pour les champs.
5. **Rejouer** les trames sur le materiel (tests d'octets ecrits a la main d'apres la capture, conformement a `CLAUDE.md`), marquer `ScreenInfo.verified = false` tant que Momo n'a pas valide.
6. **Licence** : ne copier aucun code GPL (liquidctl, OpenLinkHub, turing-smart-screen-python, deepcool-digital-linux, ttlcd) ; MIT/Apache avec attribution OK ; PureRGB reimplemente d'apres la documentation du protocole.

## Mises en garde sur la qualite des sources

- La plupart des depots recents (octobre 2026) sont des projets d'un seul auteur, parfois assistes par IA (le README de `lian-li-linux` le dit), et peu recoupes : recouper par capture avant de livrer.
- Les pages produit et sites constructeurs n'ont pas pu etre telecharges : les URLs de logiciels viennent de resultats de recherche, non d'un telechargement.
- Les VID:PID notes "decimal" proviennent du code source tel quel et ont ete convertis ; tout est liste avec sa source dans chaque fichier.
