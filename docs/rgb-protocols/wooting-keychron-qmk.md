# Wooting, Keychron, QMK / VIA / VialRGB : protocoles de claviers ouverts

Document d'interopérabilité. Aucun code ni asset copié ; seuls des faits de protocole.

## Source analysée

- **Aucun logiciel officiel analysé** (Wooting "Wootility", Keychron Launcher, VIA, Vial : applications web/Electron non téléchargées dans cette session). Les faits viennent du code d'**OpenRGB** commit `bbc7747` lu localement : `Controllers/WootingKeyboardController/` (V1/V2/V3), `Controllers/KeychronKeyboardController/`, `Controllers/QMKController/` (`QMKCommon.h`, `QMKViaCommands.h`, `QMKOpenRGBController/`, `QMKVialRGBController/`). Marqueur (O).
- QMK / VIA / Vial sont des projets ouverts : leur documentation publique officielle existe (docs.qmk.fm, usevia.app, get.vial.today) mais **n'a pas été relue** ici ; seules les constantes présentes dans OpenRGB sont citées.
- Copie : `D:\Projet\Analyses\rgb\_src\` (hors dépôt).

## Matériel couvert

### Wooting (O, `WootingKeyboardControllerDetect.cpp`)
VID anciens **0x03EB** (Atmel), nouveaux **0x31E3**. Le PID est suivi d'un suffixe de mode d'entrée : OpenRGB enregistre `PID`, `PID|0x0000` (XInput), `PID|0x0001` (DInput) et `PID|0x0002` (mode "none") ; la base du PID s'obtient par masque `0xFFF0`.
| VID:PID de base | Modèle | Type |
|---|---|---|
| 03EB:FF01 | Wooting One (legacy) | TKL, protocole V1 |
| 03EB:FF02 | Wooting Two (legacy) | pleine taille, V1 |
| 31E3:1100 | Wooting One | TKL |
| 31E3:1200 | Wooting Two | pleine taille |
| 31E3:1210 | Wooting Two Lekker Edition | pleine taille |
| 31E3:1220 | Wooting Two HE | pleine taille |
| 31E3:1230 | Wooting Two HE (ARM) | pleine taille |
| 31E3:1300 / 1310 / 1320 / 1340 | Wooting 60HE / 60HE ARM / 60HE+ / 60HEv2 | 60 % |
| 31E3:1400 | Wooting 80HE | 80 % |
| 31E3:1510 | Wooting UwU RGB | pavé 3 touches |

Collection HID à ouvrir : usage page **0x1337** (famille V2 / V1 ancien) ou **0xFF55** (famille V3).

### Keychron (O)
Seul modèle propriétaire géré : **05AC:024F** (Keychron K3 V2 Optical RGB, "Gaming Keyboard 1"), VID 0x05AC (Apple, hérité du firmware), interface 0, usage page 0x0001, usage 0x06. Les Keychron récents (Q, V, K Pro avec QMK / VIA) relèvent de la section QMK ci-dessous.

### QMK / VIA / VialRGB (O)
Pas de VID/PID fixe : détection dynamique par l'usage page **0xFF60** et l'usage **0x61** (HID brut QMK). Raspberry Pi 500+ et YMDK 84 sont enregistrés explicitement en VialRGB.

## Protocole

### Wooting V2 (famille Two / HE, 6 x 21 LED) : (O)
- `hid_write` d'un rapport de **257 octets**. Octet 0 = 0x00 (report id), octets 1-3 = **D0 DA 0B** (mots magiques 0xD0, 0xDA puis commande "raw colors" = 11), données à partir de l'octet 4.
- **Couleur par LED : RGB565 sur 2 octets, little-endian** (R5 G6 B5), index `4 + 2*i` ; ordre des LED = matrice 6 lignes x 21 colonnes (126 LED pleine taille ; 17 colonnes One, 14 colonnes 60 %, 7 colonnes UwU).
- Une trame = un rapport. Pas d'init relevée.

### Wooting V3 (80HE et suivants) : (O)
- `hid_write` d'un rapport de **2046 octets** : octet 0 = **0x04** (report id), octets 1-3 = **D1 DA 0B**, données (RGB565 LE) à partir de l'octet 4, même disposition que V2.

### Wooting V1 (One / Two legacy, VID 03EB) : (O)
- Rapports de **129 octets** (`hid_write`) : octet 0 = 0, octets 1-3 = **D0 DA 0B**, octet 4 = n° d'esclave, octet 5 = adresse de registre de départ, octets 6..101 = 96 octets de PWM, octets 127-128 = **CRC-16/CCITT** (polynôme 0x1021, init 0, MSB d'abord) des 127 premiers octets (little-endian).
- Image = 4 paquets (5 pour la pleine taille) : (esclave 0, reg 0), (0, 96), (1, 0), (1, 96), (2, 0). Les 96 octets = trois plans de 16 : R à `idx`, G à `idx + 0x10`, B à `idx + 0x20`, avec `idx = pwm_mem_map[led % 24]` (table non transcrite). Ce sont des pages de registres de contrôleurs LED I2C (esclaves 0-2) : la structure physique est exposée par le HID.
- Commandes annexes déclarées : 30 couleur unique, 31 reset unique, 32 reset tout, 33 init couleur (taille 8). Leur format n'a pas été lu : INCONNU.
- Le SDK RGB officiel de Wooting existe (open source) mais son dépôt exact n'a pas été retrouvé dans cette session (`github.com/Wootingco/wooting-rgb-sdk` : 404) ; ne pas s'y référer.

### Keychron K3 V2 Optical RGB (O, `KeychronKeyboardController.h/.cpp`)
- Paquets de 64 octets, report id 0x00, **octet 0 des données = 0x04** (`PACKET_HEADER`), octet 1 = commande.
- Commandes : 0x02 fin de communication, 0x05 infos de base, 0x10/0x11 lire/écrire la zone des définitions de touches, 0x12 lire la zone des effets LED, **0x13 écrire la zone d'effets LED spéciaux** (octet 8 = 0x12 : nombre de paquets), 0x14/0x15 macros, 0x16/0x17 mode jeu, **0x18 / 0x19 activer / désactiver la personnalisation**, **0xF0 démarrer un effet LED**, 0xF1-0xF3 synchro LED, 0xAB début de paquet aléatoire.
- Page d'effet de 16 octets : `[0]=mode, [1..3]=R,G,B, [8]=couleur aléatoire, [9]=luminosité, [10]=vitesse, [11]=direction, [14]=0xAA, [15]=0x55` (codes de contrôle). Modes : 0x00 personnalisé, 0x01 statique, 0x02 touche allumée à l'appui, 0x03 touche éteinte à l'appui, 0x04 étincelle, etc.
- Tampon de couleurs personnalisées : **576 octets** (`COLOR_BUF_SIZE`), entrée de 4 octets par LED `[0x80, R, G, B]` ; envoyé en 64 octets x 9 paquets.

### QMK avec le protocole "OpenRGB" (firmware QMK modifié) : (O, `QMKOpenRGBController.h`)
- Rapport de **65 octets** (octet 0 = 0, octet 1 = commande), timeout lecture 50 ms.
- Commandes : **1** GET_PROTOCOL_VERSION, 2 GET_QMK_VERSION, 3 GET_DEVICE_INFO, 4 GET_MODE_INFO, 5 GET_LED_INFO, 6 GET_ENABLED_MODES, **7 SET_MODE**, **8 DIRECT_MODE_SET_SINGLE_LED**, **9 DIRECT_MODE_SET_LEDS**.
- SET_MODE : `[1]=7, [2]=hue (0-255), [3]=saturation, [4]=value, [5]=mode, [6]=vitesse, [7]=sauvegarder`. Vitesse : 0x00 la plus lente, 0x7F normale, 0xFF la plus rapide.
- Statuts de réponse : 25 échec, 50 succès, 100 fin de message.
- Modes : 1 = OpenRGB direct, 2 solide, 3 alphas mods, 4/5 dégradés haut-bas / gauche-droite, 6 respiration, 7-12 bandes (sat/val, pinwheel, spiral), 13-17 cycles, 18 chevron arc-en-ciel, 19 cycle pinwheel, 20 spiral, 21-23 balises / pinwheels arc-en-ciel, 24-25 gouttes (raindrops / jellybean), 26-28 teintes (respiration, pendule, vague), 29 carte de chaleur de frappe, 30 pluie numérique, 31-38 variantes réactives (simple, normal, large, multiwide, croix, multicroix, nexus, multinexus), 39-42 splash (splash, multisplash, solid splash, solid multisplash), 43-45 pixel (rain, flow, fractal). Les identifiants sont séquentiels dans l'énumération OpenRGB ; la numérotation exacte côté firmware dépend de la version de protocole (`Rev9`, `RevB`, `RevD`, `RevE` existent) : à vérifier par `GET_ENABLED_MODES`.
- Format des commandes DIRECT_MODE_* (octets de LED / couleurs) : INCONNU (non lu).

### QMK avec VIA (clavier standard, `QMKViaCommands.h`)
- Commandes VIA : 0x01 GET_PROTOCOL_VERSION, 0x02/0x03 GET/SET_KEYBOARD_VALUE, 0x04/0x05 keymap dynamique get/set, 0x06 reset keymap, **0x07 CUSTOM_SET_VALUE, 0x08 CUSTOM_GET_VALUE, 0x09 CUSTOM_SAVE**, 0x0A reset EEPROM.
- Canaux : 0 custom, 1 backlight, 2 rgblight, 3 **rgb_matrix**, 4 audio, 5 led_matrix.
- Valeurs rgb_matrix : 1 luminosité, 2 effet, 3 vitesse, 4 couleur (HS). rgblight : 1 luminosité, 2 effet, 3 vitesse, 4 couleur. led_matrix : 1 luminosité, 2 effet, 3 vitesse. Le rapport VIA fait 32 octets (`MSG_LEN` 32 dans le contrôleur VialRGB).
- Limite : VIA ne fournit pas de mode "par touche direct" : seulement effets + couleur HSV globale.

### VialRGB (Vial) : (O, `QMKVialRGBController.h`)
- Préfixe de commande Vial **0xFE** ; sous-commandes VialRGB : **0x40 GET_INFO**, 0x41 GET_MODE / SET_MODE, 0x42 GET_SUPPORTED / **DIRECT_FASTSET**, 0x43 GET_NUMBER_LEDS, 0x44 GET_LED_INFO. Effets : OFF, DIRECT (mode par LED), SOLID_COLOR, ALPHAS_MODS, dégradés, BREATHING, bandes, cycles, ...
- Format de DIRECT_FASTSET (taille de lot, HSV vs RGB) : INCONNU.

## Cadence
Non mesurée. Wooting V2/V3 : un `hid_write` par image (257 / 2046 octets). Keychron : 9 paquets de 64 octets par image. QMK OpenRGB : 1 commande SET_LEDS par lot.

## Ventilateurs / pompes
Sans objet.

## Accès bas niveau
HID uniquement. Wooting : le logiciel Wootility peut garder les interfaces ; les claviers apparaissent avec plusieurs collections (config 0x1337 / 0xFF55). Keychron / QMK : collection brute 0xFF60/0x61. Aucun pilote vendeur. Sans fil Keychron / Bluetooth : non couvert.

## Ce qui est INCONNU
- Applications Wootility / Keychron Launcher / VIA / Vial : non analysées.
- Format DIRECT_MODE_* QMK-OpenRGB et DIRECT_FASTSET Vial ; disposition précise des LED par modèle.
- Commandes Wooting 30-33 (format) ; tables `pwm_mem_map` et `rgb_led_index` (V1) non transcrites.
- Le protocole Keychron "Launcher" web (WebHID) des modèles récents.
- Cadence maximale ; Wooting analogique (lecture de profondeur) hors périmètre RGB.

## Estimation de difficulté
- **Wooting V2 / V3 : très facile** (un rapport RGB565 avec 3 octets magiques).
- Wooting V1 : moyenne (CRC-16, tables PWM).
- QMK (OpenRGB ou VialRGB) : moyenne (dépend du firmware, négociation de version).
- Keychron K3 V2 : moyenne (protocole de zone mémoire par paquets de 64).

## Tests d'octets possibles sans matériel
1. Générer un rapport Wooting V2 : longueur 257, `[1..3] = D0 DA 0B`, une LED rouge pure -> RGB565 0xF800 -> octets `00 F8` à l'index 4.
2. Générer un rapport Wooting V3 : longueur 2046, `[0]=04`, `[1..3]=D1 DA 0B`.
3. CRC-16/CCITT (poly 0x1021, init 0, sans réflexion) du début d'un rapport V1 : tester sur le vecteur classique `"123456789"` -> 0x31C3.
4. Générer `GET_PROTOCOL_VERSION` QMK-OpenRGB : 65 octets, `[0]=0`, `[1]=1`.
