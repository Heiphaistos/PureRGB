# Gigabyte RGB Fusion 2.0 / Control Center (Aorus) : protocoles RGB des cartes mères Gigabyte

Date : 2026-10-22. Légende : **[V]** lu dans une source de référence (voir ci-dessous), **[M]** de mémoire, NON vérifié, **INCONNU** non confirmé.
Aucun code ni asset copié : uniquement des faits de protocole.

## Source analysée (honnêteté)

- **Le logiciel officiel n'a PAS été analysé** : RGB Fusion 2.0 / Gigabyte Control Center (GCC) est livré par page support par carte (installeur + services + pilotes ITE), interdit d'installation ; aucun installeur téléchargé.
  Le SDK « RGBFusion SDK »/« GvLedLib.dll » **[M]** n'a pas été obtenu.
- Faits tirés de la **lecture du code OpenRGB** (GPL-2.0-or-later, instantané local `D:\Projet\Analyses\rgb\openrgb-src\`, commit non enregistré), construit sur des captures USB de RGB Fusion 2.0 et sur
  la datasheet ITE IT8712F/IT8620. Fichiers : `Controllers/GigabyteRGBFusion2USBController/{GigabyteRGBFusion2USBController.{h,cpp},GigabyteRGBFusion2USBControllerDetect.cpp,GigabyteFusion2USB_Devices.{h,cpp}}`,
  `Controllers/GigabyteRGBFusion2SMBusController/*`, `Controllers/GigabyteRGBFusionController/*`, `Controllers/GigabyteSuperIORGBController/*`, `super_io/super_io.h`, `pci_ids/pci_ids.h`.

## Matériel couvert

| Famille | Identifiant | Interface | Remarque |
|---|---|---|---|
| **RGB Fusion 2 USB (ITE IT8297 / IT5702 / IT5711 / IT8950)** | **048D:8297**, **048D:8950**, **048D:5702**, **048D:5711** | HID, **usage page 0xFF89, usage 0xCC** (rapports *feature* de 64 octets, ID 0xCC) | cartes X470-Z790 [V] |
| Variante Gigabyte | **0414:A100** | idem 0xFF89/0xCC | un seul header ; le code force « tous les LED » (-1) [V] |
| RGB Fusion 2 SMBus (cartes 2018) | I2C **0x68** sur le SMBus carte mère | SMBus | uniquement une liste de cartes (B450 AORUS ELITE/M/PRO, X470 AORUS GAMING 5/7/ULTRA, X399, X299 DESIGNARE, B360/Z370 AORUS) [V] |
| RGB Fusion 1 SMBus | I2C **0x28**, registre 0xF2 doit valoir **0xC4** | SMBus, subsystem PCI **0x1458 (Gigabyte)** | [V] |
| Super I/O ITE (cartes « X570 UD », « Z390 DESIGNARE-CF ») | ITE8620 (ID 0x8620) / ITE8688 (ID 0x8688), index 0x2E/0x4E | E/S ports | [V] |

Identification de la carte (layout des LED, nombre de connecteurs) : le rapport d'info (voir ci-dessous) donne `device_num` (octet [2]) qui indexe `gb_fusion2_device_list` dans `GigabyteFusion2USB_Devices.cpp`
(≈ 180 Ko : zones LED1..LED11, HDR_D_LED1..4, noms et plages min/max de LED par zone). Ce tableau est la « base de cartes » à extraire pour PureRGB ; non recopié ici.

## Protocole USB IT8297/IT57xx [V]

Transport : `hid_send_feature_report` / `hid_get_feature_report`, tampon de **64 octets**, octet[0] = report ID **0xCC**. Aucune temporisation obligatoire sauf :
20 ms après `0x33`, 50 ms après `0x32`, 700 ms entre scan et lecture d'info Gen2 (détection de rubans « digitaux » Gen2).

### Commandes (octet[1], puis 2 paramètres b,c)

| Cmd | Rôle | Paramètres |
|---|---|---|
| 0x60 | Demande du rapport d'info ; puis `GET_FEATURE` ID 0xCC lit la structure ci-dessous | `CC 60 00` |
| 0x61 | Calibration étendue (IT5711 seulement) : `GET_FEATURE` → `cal_strip2..5` à partir de l'offset 4 | `CC 61 00` |
| 0x20..0x27 | En-tête (octet[1]) du paquet d'effet pour la LED n°(cmd-0x20) ; 0x20 avec `zone0=0xFF` (IT5711 : 0x07FF) = toutes ; IT5711 : 0x90..0x92 pour LED 8..10 | paquet d'effet complet (voir ci-dessous) |
| 0x28 | **Appliquer les effets** : forme rapide `CC 28 FF 00` (IT5711 : `CC 28 FF 07`) ; forme longue : `CC 28` + `zone_sel0` (u32 LE) + `zone_sel1` (u32) | |
| 0x31 | `CC 31 <0/1>` : **mode « beat »** (synchronisation audio matérielle) [V] | |
| 0x32 | `CC 32 <masque>` : **désactive les effets internes** des headers ARGB (bit0 = D_LED1, bit1 = D_LED2, 0x08 = D_LED3, 0x10 = D_LED4, 0x20 = ONBOARD1, 0x40 = ONBOARD2 ; le code coupe tous les headers : 0x03 sur IT8297/IT5702, 0x7B sur IT5711, 0x01 sur 0414:A100) → condition du mode direct | |
| 0x33 | **Calibration d'ordre des couleurs** : voir « Calibration » | 64 octets |
| 0x34 | **Nombre de LED des 6 headers** : `CC 34 b2 b3 b4` avec `b2 = (n1<<4)|n0`, `b3 = (n3<<4)|n2`, `b4 = (n5<<4)|n4`, n = énumération 0..4 = 32/64/256/512/1024 LED | |
| 0x47 | `CC 47 <0/1>` : **persistance de l'éclairage** (activer/désactiver) [V] | |
| 0x48 | `CC 48 <0/1>` : **mode « Lamp Array »** (HID Lamp Array) [V] ; effet détaillé INCONNU | |
| 0x5E | `CC 5E 00` : **sauvegarde de l'état d'éclairage en flash** [V] | |
| 0x38.. (`GEN2_LED_BASE_SCAN`) | scan/info des rubans « Gen2 » intelligents (IT5702/IT5711/IT8950 avec `strip_detect==1`, `support_cmd_flag & 1`) | |

### Rapport d'info (`CC 60` puis GET) — structure packée [V]

`report_id`(1), `product`(1), `device_num`(1), `strip_detect`(1), `fw_ver`(u32 : octets 0-2 version, octet 3 = fw_id), `argb01_count`(1), `argb23_count`(1), `argb45_count`(1),
`support_cmd_flag`(1 ; bit0 = Gen2, bit1 = autre), `str_product`[28] (nom ITE, ex. « IT8297BX-GBX570 »), `cal_spare0`, `cal_strip0`, `cal_strip1`, `rgb_cali`, `chip_id`, `cal_spare1` (u32 chacun).
Nombre de LED par header = quartets (bas/haut) des 3 octets argb, énumération 0=32, 1=64, 2=256, 3=512, 4=1024 [V].

### Calibration (ordre des couleurs) [V]

Chaque header ARGB et la zone « carte » (`rgb_cali`) a un u32 `(R_idx<<16)|(G_idx<<8)|B_idx`, valeurs 0..2 = position de l'octet dans le triplet. Ordres : BGR=`00 01 02`, BRG=`01 00 02`, GRB=`02 00 01`,
GBR=`00 02 01`, RGB=`02 01 00`, RBG=`01 02 00`. La commande `0x33` écrit : `d_strip_c0, d_strip_c1, rgb_cali, c_spare0, c_spare1, d_strip_c2..5` (u32 LE à partir de l'offset 2).
L'application doit **lire d'abord** (0x60/0x61) et réécrire telle quelle : un mauvais réglage échange les couleurs de toute la carte.

### Mode direct par LED (connecteurs ARGB) [V]

Paquet feature 64 octets : `CC <header> <offset u16 LE> <count_bytes> <RGB×19>`, soit **19 LED max par paquet** (57 octets), offset exprimé en octets (3 par LED).
- Identifiants de header (octet[1]) : D_LED1 = **0x58**, D_LED2 = **0x59**, D_LED3 = **0x62**, D_LED4 = **0x63**, ONBOARD1 = **0x64**, ONBOARD2 = **0x65** (IT5711).
- Maximum de LED : 1024 par header (DLED) ; IT57xx limité à **256** [V].
- Octets : l'ordre est défini par la calibration du header (positions r,g,b dans le triplet).
- Avant d'envoyer : `CC 32 <bitmask>` pour couper l'effet interne du header ; après : rien à appliquer (la trame est affichée dès réception).

### Effets matériels (RGB 12 V « LED1.. », en-têtes et zones) [V]

Paquet 64 octets (`PktEffect`) : `report_id`(1) `header`(1 = 0x20 + n ou 0x90 + n−8) `zone0`(u32 LE = 1<<n ; tous = 0xFF, IT5711 : 0x07FF) `zone1`(u32) `reserved`(1)
`effect_type`(1) `max_brightness`(1) `min_brightness`(1) `color0`(u32 = **BGR**) `color1`(u32) `period0..3`(u16 LE) `effect_param0..3`(1) + 30 octets de remplissage.
- `effect_type` : 0 none, 1 statique, 2 pulse, 3 blink, 4 color cycle, 6 wave, 8 random, 9-12 wave1-4, 15 double flash.
- Périodes (ms) : pulse 400..1000 pas 100 puis 200 ; flash 700+200·s ; cycle 300+100·s ; wave 2,5·(s+1)²+2,5·(s+1)+25.
- Pour appliquer : `CC 28` (voir tableau).
- LED1..LED8 : zones carte (chipset, audio, PCH, CPU, etc., sémantique par carte dans la base `Devices.cpp`) ; HDR_D_LED1/2 : 5 et 6 ; IT5711 en ajoute (LED9..11, HDR_D_LED3/4).
- Les **12 V RGB** (« LED_C1/LED_C2 ») sont des canaux de la même liste (zones d'effet), pilotés comme les zones carte : couleur unique, effets matériels uniquement ; pas de mode direct.

## Protocole SMBus RGB Fusion 2 (adresse 0x68) [V]

Écritures de blocs de **32 octets** (2 emplacements de 16 octets) ; octet de commande SMBus = `0x20 + 2·(n/2)` pour l'emplacement n (formule telle que lue dans le code, à valider sur matériel) ; 10 emplacements (« slots ») ; octets d'un emplacement : [1] mode, [2] luminosité max (0x64 = 100 %), [3] min,
[4] bleu, [5] vert, [6] rouge, [7] blanc, [8..13] timers LSB/MSB (ms), [14..15] options. Modes : 1 pulse, 2 musique, 3 color cycle, 4 statique, 5 flash/double flash, 9 transition, 10 digital wave,
0x0B..0x0D, 0x0F..0x12 digital A..G. Appliquer = `write_word_data(0x68, 0x17, 0x01FF)`. Lecture de l'état impossible (on peut écraser l'état précédent). Valable seulement pour la liste de cartes
(détection par nom DMI ; sondage par `write_quick` → **ne jamais activer sur cartes USB**) [V].

## Protocole SMBus RGB Fusion 1 (0x28)

Détection : `write_quick(0x28)` OK et registre 0xF2 == 0xC4 ; sous-système PCI 0x1458 ; banque sélectionnée par registre 0xF0 ; canaux 0/1 : mode à 0x03/0x13 (+0x10 en écriture), timers 0x06-0x09 / 0x16-0x19,
couleurs banque 1 : canal 0 R/G/B = 0x00/0x01/0x02, canal 1 = 0x08/0x09/0x0A ; modes 0 statique, 1 breathing, 2 flashing ; vitesses (tables de timers 0x01E0/0x00F0/0x0078 et 0x4000/0x2000/0x1000). Séquence d'init : `write_byte_data(0x28, 0x02, 0x09)` [V]. Couverture des cartes : à vérifier.

## Super I/O ITE8620 / ITE8688 (« Ambient LED » X570 UD, Z390 DESIGNARE) [V]

Entrée en mode étendu : séquence ITE `0x87, 0x01, 0x55, 0x55` (lecture de l'ID 0x20/0x21 : 0x8620 / 0x8688, masque 0xFFF8) ; sélection « chip » : reg 0x07 = 0x00 ; sortie : reg 0x02 = 0x01.
Couleur : rouge 0xB3/0xC3, vert 0xB4/0xC4, bleu 0xB5/0xC5 ; mode 1 (statique/rainbow/breathing/flashing = 0..3) avec registres 0xB0-0xB2 et 0xC0-0xC2 et valeurs fixes (0x0F, 0x00, 0x20, 0x0F, 0x00, 0x10).
Détails de rainbow/breathing/flashing : voir `GigabyteSuperIORGBController.h` (non recopiés).

## Pilotes noyau, BIOS, ACPI, WMI

- **USB (IT8297 & co)** : HID standard, **aucun pilote noyau** requis pour le protocole ci-dessus ; GCC/RGB Fusion installe tout de même `IT*` services et le « GIGABYTE Control Center service » **[M]**.
- **SMBus / Super I/O** : pilote noyau tiers (PawnIO/WinRing0) pour I2C et ports E/S ; les cartes Gigabyte exposent souvent un SMBus « GCC » protégé par le BIOS **[M]** (blocage d'adresses ; non vérifié).
- **BIOS** : options « LED Lighting » / « LEDs in Sleep, Hibernate and Shutdown » (menu Smart Fan/Peripherals) **[M]**, impactent l'état au boot et en veille ; gestion exacte de la persistance dans l'IT8297 INCONNUE.
- **ACPI/WMI** : classe WMI Gigabyte (`GIGABYTE_WMI`/`GigabyteWmi`) **[M]** utilisée par GCC pour ventilateurs/RGB ; non analysée.
- DMI : le nom de carte (SMBIOS type 2) sert à l'affichage (USB) et à la liste blanche (SMBus, Super I/O).

## Ventilateurs / pompes

Non pilotés par l'IT8297 (éclairage uniquement). Les ventilateurs sont sur le Super I/O ITE (IT8688E, IT8792E…) dont les registres PWM/tach standard sont connus mais **hors du périmètre analysé ici** ; Gigabyte
verrouille certains accès via le BIOS. INCONNU.

## INCONNU / non confirmé

- Effet précis de 0x48 (Lamp Array) ; valeurs des octets de remplissage du rapport d'info ; réponses aux commandes autres que 0x60/0x61.
- Interaction exacte entre 0x47 (persistance) et 0x5E (sauvegarde flash) et leur usure de flash : ne les envoyer que sur demande explicite de l'utilisateur.
- Table complète `device_num` → layout (182 Ko dans `Devices.cpp`) : à extraire de façon dérivée.
- Rubans Gen2 (scan 0x38+) : cycle exact de lecture non validé.
- Protocole SDK `GvLedLib.dll`/WMI, services de GCC, accès SMBus bloqué sur certaines cartes.
- Cartes Super I/O ITE autres que X570 UD / Z390 DESIGNARE.

## Estimation de difficulté de réimplémentation

- **USB IT8297/IT57xx : moyenne** (rapports feature 64 octets, mais calibration d'ordre des couleurs à lire/écrire, masque 0x32 et table de cartes à construire). Pas de pilote.
- SMBus Fusion 2 (0x68) : moyenne/haute (pilote I2C + liste blanche + pas de lecture).
- Super I/O : moyenne (accès E/S ports, cartes très limitées).

## Tests d'octets possibles sans matériel

1. Tailles : rapport d'info ≥ 56 octets (4+4+3+1+28+6×4 = 64) ; paquet d'effet 64 octets avec offsets (u32 @2, @6 ; effect_type @11 ; color0 @14 ; period0 @22).
2. Direct : 19 LED/paquet ; 100 LED = 6 paquets, offset en octets (0, 57, 114, …), `count_bytes` ≤ 57.
3. Énumération de LED : nibble bas/haut de `argb01..45` → 6 compteurs ; test aller-retour avec `0x34`.
4. Calibration : encode/décode des 6 ordres, vérifier `RGB = 0x02 0x01 0x00`.
5. Super I/O : séquence d'entrée `87 01 55 55` (suivant le port) ; commande d'effet dans le test de génération de trames.
