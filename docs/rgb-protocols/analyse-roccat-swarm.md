# Roccat Swarm / Turtle Beach Swarm II : analyse des logiciels officiels + protocoles HID

Document d'interopérabilité (PureRGB, MIT). Aucun code ni asset Roccat / Turtle Beach copié ; uniquement des faits.
Complète `roccat-turtlebeach.md` (qui reposait sur OpenRGB seul). Marqueurs de source :
- **(G)** = lu dans les installeurs officiels extraits sans exécution (`D:\Projet\Analyses\rgb\roccat\`).
- **(O)** = OpenRGB commit `bbc7747`, `Controllers/RoccatController/` (source tierce, NON officielle ; à valider sur matériel).
- `INCONNU` = non confirmé. Rien n'est deviné.

## 1. Sources analysées

| Logiciel | Version | URL officielle | SHA-256 |
|---|---|---|---|
| ROCCAT Swarm (génération 1) | 1.9478.0.0 (« v19478 », fichiers datés 2025-05-13) | `https://acpv.prod.turtlebeach.com/support/software/us/376/ROCCAT-Swarm.zip` -> 302 vers `https://cdn.turtlebeach.com/device/software-standalone/roccat--swarm/ROCCAT_Swarm_1.9478.0.0.zip` (URL relevée dans le manifeste winget `TurtleBeach.ROCCATSwarm`) | zip : `B70F1A26F027DBAE03CE5538074A2AD37B79333CE1309779E7ADCEFA1AEA92EB` ; exe interne `ROCCAT Swarm_v19478.exe` : `A3BB671E9C87B05899EDBB6C1A98CAB66DAA1A7885164F28FA9278A4006C78D8` |
| Turtle Beach Swarm II | 1.0.0.56 (taille 86 596 192 octets, correspond aux « 82,58 Mo » annoncés par la page support citée par un site tiers ; la page support officielle est une SPA illisible hors navigateur) | `https://cdn.turtlebeach.com/device/software-standalone/swarm-ii/Swarm_II_1.0.0.56.zip` (CDN officiel Turtle Beach ; chemin déduit du redirect `acpv.prod.turtlebeach.com/support/software/us/319/Swarm-II.zip` -> `.../Swarm_II_0.0.0.15.zip`, la version la plus récente a été sondée par HEAD : 200) | zip : `A650055291EBA5278ECF2DDB803D390BD0EF8810869E09FCAFC6CBC14572AACA` ; exe interne `Turtle Beach Swarm II Installer Setup_1.0.0.56.exe` : `1725E2E37E6884F50CE30B765C3A90F8D29E75801A7A4BB41EDB505826C2652C` |

Méthode : `7z x` (NSIS) deux niveaux ; `strings` ; aucune exécution. Version 1.9482.0.0 (2026-09-08) annoncée par un site tiers pour Swarm : non téléchargée.

### Contenu réel des installeurs (G) : ce qui manque
- **Swarm 1.9478** (`x2/`) : exécutables Qt5 (`ROCCAT_Swarm.exe` 5 Mo, `ROCCAT_dev_service.exe`, `ROCCAT_Swarm_Monitor.exe`, `uac_agent.exe`), `aimo.dll` et `AimoEffect.dll` (moteur d'effets logiciels), `firmware_upgrade.dll`, `ISPDLL.dll`/`HIDDLL.dll`/`EFORMAT.INI` (programmation ISP Holtek : table `HT66FB540/550/560...`, pour flasher les MCU), `virtual_hid/` (pilote HID virtuel x64/x86), `tool/devcon_x64.exe`. **Aucun module par périphérique** : pas de trame Vulcan/Kone/Burst/Kain dans le paquet. `data/Swarm/version.ini` ne contient que une table `[N] compatibility=M` par identifiant interne de périphérique (N = 3..84, sans lien direct avec les PID).
- **Swarm II 1.0.0.56** (`s2x/`) : Qt6 (`Turtle Beach Swarm II.exe`, `Turtle Beach Device Service.exe`, `Utility.dll`, `Resource.dll`, `LightEffect/{LightEffect,AimoEffect}.dll`, `Plugin/{PluginManager,SMART_KEY,SystemMedia}`), `Data/VirtualHID/{VirtualHID.inf,.sys,.cat}`. `Data/Devices/SWARM_II/version.ini` (version 1.0.0.56 ; 40 identifiants internes `compatibility=1`) et `Data/Devices/QT_LIBRARY/Version.ini` (6.5.3.1) : **les modules périphériques sont téléchargés au premier lancement** (l'exe contient `Start update device json`, `update_device_json`, `Autoupdate.ini` ; endpoint `https://acpr.prod.turtlebeach.com/swarm2/autoupdate/%1` -> répond 500 « Wrong request method » à un GET : requête POST à JSON encodé). 
- **Non fait volontairement** : reproduire cet appel d'API pour télécharger les modules (le protocole d'échange serait à reconstituer ; hors périmètre « installeur officiel » défini). Les trames officielles de Swarm II sont donc `INCONNU` (G).

### Faits confirmés dans les binaires (G)
- VID **0x1E7D** : chaînes `USB\VID_1E7D&PID_%1&MI_00` / `...&MI_01` (`ROCCAT_Swarm_Monitor.exe`, `uac_agent.exe` ; PID substitué à l'exécution, interfaces 0 et 1 surveillées) et `USB\VID_1E7D&PID_39D0&MI_00` codé en dur dans `x2/uac_agent.exe` (modèle `INCONNU`). `Utility.dll` (Swarm II) contient la constante `0x1E7D`.
- Modèles gérés par l'interface de Swarm II (chaînes de `Turtle Beach Swarm II.exe`) : BURST CORE, BURST PRO, BURST PRO AIR, KAIN 100 AIMO, KAIN 120 AIMO, KAIN 200 AIMO, KONE AIMO, KONE AIR, KONE EMP, KONE PRO, KONE PRO AIR, KONE PURE OWL-EYE / SE / SEL / ULTRA / ULTRA iCafe, KONE XP, MAGMA MINI, VULCAN, VULCAN 100 AIMO, VULCAN 80, VULCAN AIMO, VULCAN PRO, VULCAN TKL PRO (+ Vulcan II dans le texte d'aide « Smart Keys »). **Pas de PID** dans ces chaînes.
- Moteur d'effets logiciels (RTTI de `aimo.dll`, Swarm 1) : classes `wave`, `ripple`, `snake`, `breath`, `fade`, `blinking`, `heartbeat`, `fully_lit`, `cycle_*`, `kone_aimo_heartbeat`, `khan_aimo_wave`, `kone_xp_photon`, `kone_xp_air_photon`, `battery_indication`, `multi_effect`. Confirme que les effets AIMO sont calculés **côté PC** (thread par effet) puis envoyés au périphérique ; la prise de contrôle logicielle existe donc pour Vulcan, Kone Aimo, Kone XP, Kone XP Air.
- Services qui tiennent les interfaces HID : `ROCCAT_dev_service.exe` (Swarm 1), `Turtle Beach Device Service.exe` (Swarm II) : conflit probable avec un pilote natif ; arrêter le service avant d'ouvrir les collections. Pilote noyau/HID virtuel : `VirtualHID.sys` (Swarm II `Data/VirtualHID/`), `virtual_hid/` (Swarm 1) : sert à injecter des touches (macros, Smart Keys) ; **non requis** pour l'éclairage (rien ne l'indique côté éclairage dans (G) ni (O)).

## 2. Matériel couvert : PID (O, non confirmés par (G))

VID Roccat **0x1E7D** ; Turtle Beach **0x10F5**. PID tels que dans `RoccatVulcanKeyboardController.h`, `RoccatControllerDetect.cpp`, `TurtleBeachKP7Controller`.

| VID:PID | Modèle | Contrôleur (O) |
|---|---|---|
| 1E7D:307A | Vulcan 100 Aimo | Vulcan v1 |
| 1E7D:3098 | Vulcan 120 Aimo (Vulcan 121 : PID `INCONNU`, pas de constante distincte dans la source) | Vulcan v1 |
| 1E7D:2FEE | Vulcan TKL | Vulcan v1 |
| 1E7D:30F7 / 311A | Vulcan Pro / Vulcan TKL Pro | Vulcan v2 |
| 1E7D:2F4E / 2EE2 / 2F09 | Vulcan II / II Max / II Mini | Vulcan v2 |
| 1E7D:314C | Pyro | Vulcan v2 |
| 1E7D:3124 / 69A0 | Magma / Magma Mini | Vulcan v2 |
| 10F5:501B / 5023 / 5001 | TB Vulcan II / II TKL / II TKL Pro | v2 / **v1** / v2 spécial |
| 1E7D:2DE6 / 2DE1 | Burst Core / Burst Pro | Burst |
| 1E7D:2CA6 | Burst Pro Air | BurstProAir |
| 1E7D:2C88 | Kone Pro | KonePro |
| 1E7D:2C8E / 2C92 | Kone Pro Air (sans fil / filaire) | KoneProAir |
| 1E7D:2E27 / 2E2C | Kone Aimo / Kone Aimo 16K | KoneAimo |
| 1E7D:2C8B | Kone XP | KoneXP |
| 1E7D:2CEE | Kova | Kova |
| 1E7D:343A / 343B | Sense Aimo (tapis) | SenseAimo |
| 1E7D:303E | Horde Aimo | HordeAimo |
| 1E7D:3A34 | Elo 7.1 (casque) | Elo |
| 10F5:503A | Command Series KP7 | `TurtleBeachKP7Controller` (non lu) |
| **Kain 100 / 120 / 200 Aimo** | modèles présents dans Swarm II (G) | **PID INCONNU** ; aucun contrôleur OpenRGB, aucune entrée libratbag/usb.ids locale |

Interfaces à ouvrir (O, `RoccatControllerDetect.cpp`, reprises de `roccat-turtlebeach.md`) : claviers Vulcan = deux collections, **ctrl** (feature) et **led** (écriture 65 octets) ; Pyro/Magma/Vulcan Pro/II/TB II : led = interface 3 UP 0xFF00, ctrl = interface 1 UP 0xFF01 ; TB Vulcan II TKL Pro : led = interface 4 UP 0xFF00, ctrl = interface 1 UP 0x0001 ; Vulcan 100/120/TKL et TB II TKL : led = interface 3 UP 0x0001, ctrl = interface 1 UP 0x000B. Kone Pro / Burst Pro : interface 3 UP 0xFF01 U 1 ; Kone Pro Air : interface 2 UP 0xFF00 (filaire : interface 1 UP 0xFF13) ; Kone XP : interface 3 UP 0xFF01 ; Burst Pro Air : interface 0 UP 0x01 U 2.

## 3. Claviers Vulcan / Pyro / Magma : protocole (O)

Source : `RoccatVulcanKeyboardController.cpp` (lignes 60-440) et `RoccatVulcanKeyboardLayouts.h`.

### 3.1 Prise de contrôle et sortie
1. Prêt ? `hid_get_feature_report(ctrl)` : report id **0x09** (9 octets ; modèles v2 : Pyro, Magma, Vulcan Pro/II/Max/Mini, TB II/TKL Pro) ou **0x0F** (8 octets ; v1 : Vulcan 100/120/TKL, TB II TKL). Octet 1 = 1 si prêt ; **octet 2 = version firmware** (`major = b/100`, `minor = b%100`) ; octet 6 = 0x06 sur Vulcan II Max JP (disposition JIS). Lecture faite deux fois (la première peut être périmée).
2. **Entrée en mode direct** : feature report sur ctrl, v2 = `0E 05 01 00 00` (5 octets), v1 = `15 00 01` (3 octets). **Sortie** : même trame avec `<on>=0`. Sans sortie, le clavier garde les couleurs logicielles. Les mêmes trames avec 0/1 sont utilisées pour revenir au mode matériel.
3. Rien à envoyer pour la restitution du profil autre que ce `0` (pas de seconde trame de reset dans (O)).

### 3.2 Trame couleur (« direct »)
- `hid_write` de paquets de **65 octets** sur la collection led (octet 0 = report id 0). Attente de lecture + ~20 ms après la trame.
- En-tête : `[1]=0xA1` ; `[2]=` numéro de paquet (v2 : 1..N sur chaque paquet ; v1 : `0x01` sur le premier seulement) ; longueur totale de la trame : **1 octet en `[3]` si ≤ 255, sinon 2 octets `[3][4]`** (octet de poids fort d'abord si firmware ≥ 1.16 ; poids faible d'abord sinon ; Vulcan II Mini toujours poids faible d'abord). Décalage des données = 4 (longueur 1 octet) ou 5 (2 octets) dans le paquet de 65, 61 ou 60 octets utiles par paquet ; le code applique le même décalage à tous les paquets.
- Nombre de paquets = `ceil(longueur / 64)`.
- **Longueur de la trame (champ en-tête), par modèle** (O) :

| Modèle | Longueur | LED par colonne | Paquets |
|---|---|---|---|
| Magma, Magma Mini | **64** (corrige `roccat-turtlebeach.md` : « 64 octets, colonnes de 5 ») | 5 | 1 (5 LED = 15 octets utiles) |
| Pyro | 378 | 1 | 6 |
| Vulcan Pro / TKL Pro | 384 | 12 | 6 |
| Vulcan II / TB Vulcan II | 396 | 1 | 7 |
| Vulcan II Max | 567 | 1 | 9 |
| Vulcan II Mini | 345 | 1 | 6 |
| TB Vulcan II TKL Pro | 320 | 1 (plans N) | 5 |
| Vulcan 100/120/TKL, TB II TKL (v1, défaut) | 436 | 12 | 7 |

- **Disposition des couleurs** : pour une LED d'index `i`, `colonne = i / column_length`, `ligne = i % column_length`, `offset = colonne*3*column_length + ligne` (+ décalage d'en-tête) pour **R**, puis `+column_length` pour **G**, `+2*column_length` pour **B** : trois plans R, G, B par colonne. TB Vulcan II TKL Pro : plans globaux R (0..N-1), G (+N), B (+2N), index = valeur de `led.value` maximale.
- **Correspondance LED -> touche** : `RoccatVulcanKeyboardLayouts.h` (1792 lignes, `ROCCAT_VULCAN_120_AIMO_LAYOUT_KEYS_104/105`, `..._II_LAYOUT_KEYS_105`, `..._TKL_...`, `..._II_MINI_...`, `..._II_MAX_...ANSI/JIS`, `ROCCAT_MAGMA_LAYOUT_KEYS[1][5]`) : matrice 6 lignes x 24 colonnes ; **index LED = colonne*6 + ligne** sur la Vulcan 120 Aimo (colonne 0 : Échap=0, `=1, Tab=2, Caps=3, Maj=4, Ctrl=5) ; 104/105 touches pour ANSI/ISO ; Magma = 5 LED en ligne (0..4). Tableaux `led_names` par disposition (UK, US, DE, FR, ...). Non retranscrit (81 Ko). Pas de confirmation par (G).

### 3.3 Rapports de mode matériel (O)
Feature report sur ctrl : octet 0 = **0x0D** (v1) ou **0x11** (v2) ; octets 1(-2) = longueur totale (1 octet si ≤ 255, sinon 2 octets, poids faible d'abord) ; pour la suite : mode, vitesse, luminosité (`[offset+3]` ou `[+4]` selon modèle), blocs avec marqueurs `0x02..0xFB`, `0x03..0x0B 0x01` ; **somme de contrôle** = somme 16 bits des octets sauf les deux derniers, écrite en little-endian dans les deux derniers. Longueurs par modèle : Pyro **365**, Magma/Mini **26**, Vulcan Pro/TKL Pro **371**, Vulcan II / TB II **377**, Vulcan II Max **542**, Vulcan II Mini **332**, TB Vulcan II TKL Pro **284**, défaut v1 **443**. Modes : 0x01 statique, 0x0A vague, 0x0B direct, 0x02 défaut (liste complète : `INCONNU`). Vulcan II Mini : keepalive `13 7B 1E` + 30 entrées de 4 octets pour tenir la couleur statique.

## 4. Souris Roccat (O)

| Modèle | Prise de contrôle | Trame couleur | Remarques |
|---|---|---|---|
| **Kone Pro** (2C88) | feature `0E 06 01 <direct> 00 FF` (6 o.) | feature 11 o. : `0D 0B` puis 2 LED RGB à `2+3i` | effet : report 0x06, 69 octets, mode aux octets 0x1E, vitesse 0x1F... |
| **Kone Pro Air** (2C8E/2C92) | `hid_write` : paramètres actifs lus par requête `00 90 00 04 00 18 25 51 ...` (réponse `[0]=0x90, [2]=0x26`, 42 o.) puis `00 10 50 14 ...` (25 o.) | `hid_write` 16 o. : `[1]=0x10, [2]=0x10` (direct) ou `0x50` (mode), `[3]=0x0B, [4]=0/1, [5]=mode, [6][7]=0x64 luminosité LED1/LED2, [8]=luminosité, [9]=vitesse`, 2 LED RGB à `10+3i` | modes : 0 off, 1 statique, 2 respiration, 5 heartbeat, 6 vague arc-en-ciel, 8 flash, 9 direct, 0x0A batterie ; vitesse 1..11 ; luminosité 0..0x64 ; 2 LED |
| **Burst Core / Pro** (2DE6/2DE1) | feature `0E 06 01 <direct> 00 FF` (6 o.) | feature 11 o. : `0D 0B` + RGB à `2+3i` (1 LED Core, 2 LED Pro) | mode : feature 63 o., report 0x06 : `06 3F .. 06 06 1F`, mode `[30]`, vitesse `[31]`, luminosité `[32]`, CRC16 little-endian en `[61][62]` ; modes : 0 direct, 1 statique, 2 clignotement, 3 respiration, 4 heartbeat, 0x0A vague ; vitesse 1..11, luminosité 0..255 |
| **Burst Pro Air** (2CA6) | pas de trame de prise de contrôle séparée | feature **30** octets, report id **0x06** : `06 01 4C 06 14`, puis par LED `[5+5i]=n°(1..4) [6+5i]=0xFF [7..9+5i]=RGB` (4 LED) | mode : `06 01 4C 06 06 <mode> <vitesse> <luminosité> 0F` ; modes 1 direct, 2 clignotement, 3 respiration, 4 vague ; vitesse 0..11 |
| **Kone Aimo** (2E27/2E2C) | feature `0E 06 01 01 00 FF` (6 o.) | feature **46** octets : `0D 2E` puis couleurs, 11 LED, pas de 4 octets à `2+4i` (index + RGB) ; lecture feature 3 octets avant/après | 11 LED |
| **Kone XP** (2C8B) | feature `0E 06 01 <on> 00 FF` | via profil : lecture/écriture feature report **0x06 de 0xAE octets** (config complète, dpi `[7+2i]`, angle snapping `[0x1B]`, octets de fin 165..171) | profil actif : feature `05` (4 o.), sélection `04 <profil> 80 FF` ; version : feature `09` (9 o.) |
| **Kova, Sense Aimo, Horde Aimo, Elo 7.1** | `INCONNU` (contrôleurs dédiés non relus) | | |
| **Kain 100/120/200 Aimo** | `INCONNU` | | PID inconnus |

## 5. Correspondances demandées : état par point

| Point demandé | Statut |
|---|---|
| PID Vulcan 100/120/121, Pyro, Magma, Burst, Kone Pro Air, Kain | Vulcan 100/120, Pyro, Magma, Burst, Kone Pro Air : **CONFIRMÉ (O)**, pas (G). Vulcan 121 : **INCONNU**. Kain : **INCONNU** |
| En-tête des paquets de couleur | Claviers : **CONFIRMÉ (O)** `A1 <n°> <longueur 1|2 o.>` ; souris : voir §4 |
| Ordre / longueur des champs selon firmware | **CONFIRMÉ (O)** pour l'ordre des octets de longueur (≥ 1.16 : poids fort d'abord) et les longueurs par modèle ; autres champs dépendants du firmware : **INCONNU** |
| LED -> touche | **CONFIRMÉ (O)** (`...Layouts.h`, index = colonne*6 + ligne sur Vulcan 120 Aimo) ; Vulcan Pro/II/Pyro : layouts distincts (non transcrits) |
| Prise de contrôle logicielle et sortie | **CONFIRMÉ (O)** (`0E 05 01 00 00` / `15 00 01`, `0E 06 01 ..` souris) ; (G) ne confirme que l'existence d'un moteur d'effets côté PC |
| Trames officielles (G) | **INCONNU** : modules périphériques absents des installeurs (téléchargés au premier lancement) |

## 6. Cadence, difficulté, tests
- Cadence : `INCONNU` ; (O) attend ~20 ms après chaque trame Vulcan (≈ 50 images/s max). Vulcan II = 7 paquets de 65 octets.
- Difficulté : **Moyenne** claviers (deux collections HID, variantes v1/v2/TB TKL Pro/Mini, longueur selon firmware) ; **Facile** Kone Pro / Burst / Kone Pro Air (trames courtes) ; Kone XP **moyenne** (profil de 174 octets).
- Tests d'octets sans matériel :
  1. Vulcan II : 7 paquets de 65 octets, `[1]=A1`, `[2]=1..7`, longueur 396 en 2 octets, 60 ou 61 octets de données selon décalage.
  2. Magma : 1 paquet, longueur 64, 15 octets de couleur (5 LED).
  3. Checksum 16 bits d'un rapport de mode (somme des octets sauf 2 derniers, LE).
  4. `0E 05 01 00 00` vs `15 00 01` selon `v1/v2`.
  5. Burst Pro Air : 30 octets, `06 01 4C 06 14`, 4 x `(n, FF, R, G, B)`.

## 7. Blocages
- Les deux installeurs sont des « bootstraps » : les modules périphériques (Vulcan, Kone, Burst, Kain...) sont téléchargés après installation ; sans POST authentifié à `acpr.prod.turtlebeach.com/swarm2/autoupdate/` (non reproduit), pas de binaire de protocole à décompiler. Ghidra/REA n'a donc pas été lancé (rien de pertinent à décompiler dans les binaires fournis).
- Swarm 1.9478 : même constat ; le paquet n'inclut que le moteur d'effets, le service et les outils de flash.
- Kain 100/120/200 et Vulcan 121 : PID non trouvés dans (G), (O), ni `usb.ids` (VID 1E7D ne liste que les Kone anciens).
- Version Swarm 1.9482.0.0 / Swarm II plus récent que 1.0.0.56 : non vérifié.
