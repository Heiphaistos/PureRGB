# MSI Center / Mystic Light / Dragon Center : protocoles RGB des cartes mères MSI

Date : 2026-10-22. Légende : **[V]** lu dans une source de référence (voir « Source analysée »), **[M]** de mémoire, NON vérifié, **INCONNU** non confirmé.
Aucun code ni asset copié : uniquement des faits de protocole (VID/PID, octets, offsets, tailles).

## Source analysée (honnêteté)

- **Le logiciel officiel n'a PAS été analysé.** MSI Center (Mystic Light) est livré par Microsoft Store / page support de chaque carte (403 sur
  `msi.com/Landing/...` en téléchargement direct), installe des services + pilotes et dépend d'un compte/paquet par modèle. Règle de la consigne :
  ne pas installer, donc rien à extraire sans l'installeur. Aucun installeur n'a été téléchargé. SDK Mystic Light (`MysticLight_SDK.dll`) : **non obtenu**.
- Les faits ci-dessous proviennent de la **lecture du code source OpenRGB** (GPL-2.0-or-later, instantané local
  `D:\Projet\Analyses\rgb\openrgb-src\`, version/commit non enregistrés), qui est lui-même issu de captures USB de l'application MSI
  (`mystic-why` de A. Garashchenko pour le mode direct 185 octets ; `nagisa/msi-rgb` pour l'accès Super I/O). Ce sont des faits d'interopérabilité ;
  l'implémentation PureRGB doit être réécrite, pas copiée.
- Fichiers lus : `Controllers/MSIMotherboardController/MSIMotherboardCommon.h`, `MSIMotherboardControllerDetect.cpp`,
  `MSIMotherboard185Controller/MSIMotherboard185Controller.{h,cpp}`, `MSIMotherboard761Controller/MSIMotherboard761Controller.cpp`,
  `Controllers/MSIRGBController/MSIRGBController{,Detect}.cpp`, `super_io/super_io.h`.

## Matériel couvert

Trois familles, sélectionnées à l'exécution selon la **longueur du feature report 0x52** que renvoie le contrôleur.

### A. Contrôleur USB HID Mystic Light (carte mère, ITE / Nuvoton MCU intégré)

| VID | PID | Remarque | Interface HID |
|---|---|---|---|
| 0x1462 | 0x1720, 0x7B12, 0x7B16, 0x7B17, 0x7B18, 0x7B50, 0x7B85, 0x7B92, 0x7B93, 0x7C02 | chipsets 300/400/X570 (PID = numéro « MS-xxxx » de la carte) [V] | usage page 0x0001, usage 0x00 |
| 0x1462 | 0x7C34-0x7C37, 0x7C56, 0x7C59, 0x7C60, 0x7C67, 0x7C71, 0x7C73, 0x7C75-0x7C77, 0x7C79-0x7C84, 0x7C86, 0x7C87, 0x7C90-0x7C92, 0x7C94, 0x7C95, 0x7C98 | [V] | idem |
| 0x1462 | 0x7D03, 0x7D04, 0x7D06-0x7D09, 0x7D13-0x7D15, 0x7D17-0x7D20, 0x7D25, 0x7D27-0x7D33, 0x7D36-0x7D38, 0x7D40-0x7D43, 0x7D46, 0x7D50-0x7D54, 0x7D59, 0x7D67, 0x7D69, 0x7D70, 0x7D73-0x7D78, 0x7D86, 0x7D88-0x7D91, 0x7D93, 0x7D96-0x7D99 | Z590/B560/Z690/B660/Z790 [V] | idem |
| 0x1462 | 0x7E01, 0x7E03, 0x7E06, 0x7E07, 0x7E09, 0x7E10, 0x7E20, 0x7E30, 0x7E32, 0x7E34, 0x7E59, 0x7E66, 0x7E70, 0x7E76, 0x7E80, 0x7E81, 0x7E86, 0xB926 | AM5 / LGA1851 (famille 761 octets, voir C) [V] | idem |
| 0x0DB0 | 0x0076 | PID « commun » à un lot de cartes 185 octets ; deux entrées : usage page 0x0001/usage 0x00 et **usage page 0xFF00/usage 0x01** (X870) [V] | voir texte |
| 0x1462 | 0x3EA4, 0x4459, 0x7B10, 0x7B94, 0x7B96, 0x7C42, 0x7C70, 0x7C85, 0x7C88, 0x7C89, 0x7C96, 0x7C99, 0x905D | **désactivés par défaut** dans OpenRGB (`ENABLE_UNTESTED_MYSTIC_LIGHT`) à cause d'un risque de brick passé [V] | idem |

Le PID ne détermine PAS le protocole : c'est la taille du rapport. Règle de détection [V] (`MSIMotherboardControllerDetect.cpp`) :
1. ouvrir l'interface, `GET_FEATURE_REPORT` ID **0x52** avec tampon 200 octets ;
2. longueur 185 (ou 186) → famille **185** ; 162/163 → famille **162** ; 112/113 → famille **112** ;
3. sinon : `GET_FEATURE` ID 0x50 (290 octets, « active la lecture »), puis `SET_FEATURE` ID 0x51 de 761 octets ; si accepté → famille **761**.

PID → config de carte (nombre de LED embarquées, JPIPE1/2, JRGB, zones supportées, mode direct) : tableau `board_config` dans
`MSIMotherboard185Controller.cpp` (≈ 100 lignes). Exemple : 0x7C56 MPG B550 GAMING PLUS = 6 LED embarquées, 0 JPIPE, 1 JRGB, mode LED à LED, firmware ≥ 6 [V].

### B. Super I/O Nuvoton (anciennes cartes 12 V « MSI-RGB », sans MCU USB)

NCT6795D (ID 0xD350) ou NCT6797D (ID 0xD450), masque 0xFFF8, sur le port d'index **0x2E ou 0x4E**. Détection : constructeur DMI « Micro-Star
International Co., Ltd. »/« MSI » ET nom de carte dans une liste (≈ 60 entrées, ex. 7A34, 7B79, 7B89, 7C02, 7C39, 7D95 ; indicateur `invert` par carte) [V].

### C. Famille 761 octets (X870/B850/Z890, 2024+)

Mêmes VID/PID 0x1462:7Exx. Zones : JAF (connecteur ventilo « bizarre »), JARGB1-3, JPIPE1 optionnel. Seules les cartes présentes dans une liste de noms
dans `MSIMotherboard761Controller.cpp` (X870 TOMAHAWK, B850M MORTAR, Z890 GAMING PLUS, etc.) sont acceptées ; sinon exception « board unsupported » [V].

## Protocole : familles 112/162/185 (feature report 0x52)

Tout passe par des **feature reports** HID (pas d'endpoint interrupt), Report ID 0x52, un seul paquet d'état pour toute la carte [V].

Structure d'une zone `ZoneData` (10 octets) : `effect`, `R,G,B` (couleur 1), `speedAndBrightnessFlags`, `R2,G2,B2` (couleur 2), `colorFlags`, `padding`.
- `effect` : 0 off, 1 static, 2 breathing, 3 flashing, 4 double flashing, 5 lightning, 6 marquee, 7 meteor, 8 water drop, 9 MSI rainbow, 10 pop, 11 rap, 12 jazz,
  13 play, 14 movie, 15 color ring, 16 planetary, 17 double meteor, 18 energy, 19 blink, 20 clock, 21 color pulse, 22 color shift, 23 color wave, 24 marquee,
  25 rainbow, 26 rainbow wave, 27 visor, 28 jrainbow, 29-30 rainbow flashing, 31 random, 32 fan control, 34-35 color ring flashing, 36 stack, 37 corsair QUE,
  38 fire, 39 lava, **0x25 (37) = mode « direct par LED »** (`MSI_DIRECT_MODE`) pour les rubans ARGB [V].
- `speedAndBrightnessFlags` : bits 0-1 vitesse (0 lent..2 rapide), bits 2-6 luminosité (0..10 = 0..100 %), bit 7 = synchronisation JRGB [V].
- `colorFlags` : bit0 sync LED embarquées, bit1 sync JRAINBOW1, bit2 sync JRAINBOW2, bit3 sync JCORSAIR, bit4 sync JPIPE1, bit5 sync JPIPE2, bit7 « couleur perso » [V].

Paquet **185** (taille totale 185 octets, report ID inclus ; offsets d'après les commentaires du code) [V] :
`J_RGB_1`@1, `J_PIPE_1`@11, `J_PIPE_2`@21, `J_RAINBOW_1`@31 (11 octets : ZoneData + `cycle_or_led_num`), `J_RAINBOW_2`@42 (11), `J_CORSAIR`@53 (11 : effect, RGB,
fan_flags, corsair_quantity, 4 padding, is_individual), `J_CORSAIR_OUTERLL120`@64, `ON_BOARD_LED`@74, `ON_BOARD_LED_1..9`@84..164 (pas 10), `J_RGB_2`@174,
`save_data`@184 (1 = écrire en flash).
Paquet **162** : J_RGB_1, J_RAINBOW_1, J_CORSAIR_1, J_CORSAIR_OUTERLL120, ON_BOARD_LED 0-10, J_RGB_2, save@161. Paquet **112** : mêmes 4 premiers, ON_BOARD 0-6, save@111 [V].

Procédure « mode matériel » (effets internes) [V] : lire l'état (`GET_FEATURE 0x52`), modifier la zone, puis **envoyer deux rapports consécutifs** (ancien état lu,
puis nouvel état) comme le fait MSI Center ; `save_data` = 1 pour persister dans la flash de la carte. Écrire trop souvent la flash : **risque d'usure/brick** (cf. avertissement OpenRGB).

### Mode direct par LED (185 octets, rubans ARGB)

1. Paquet de commutation (feature report 0x52, 185 octets) mettant `J_RAINBOW_1/2` en effet `0x25` avec `cycle_or_led_num` = 200 (JRAINBOW1) / 240 (JRAINBOW2) et
   `J_CORSAIR.effect = 0x25`, `is_individual` = 240. Valeurs de base : J_RGB `speedFlags=0x08, colorFlags=0x80` ; J_PIPE `0x2A/0x80` ; J_RAINBOW `0x29/0x80` ;
   corsair `fan_flags=0x29, padding[2]=0x82` ; `ON_BOARD.effect = 0x25`, `speedFlags = 0x29 | 0x80`, `colorFlags = 0xBF` (full sync = custom|onboard|pipe1|pipe2|rainbow1|rainbow2|corsair) [V].
2. Données couleur : feature report **ID 0x53**, longueur 5 + 240×3 = **725 octets** : `[0x53, 0x25 (hdr0), hdr1, hdr2, hdr3, (R,G,B)×240]` (hdr0 = 0x25 fixe ; hdr1 défaut 0x06 ; hdr2, hdr3 défaut 0) [V] :
   - rapport « embarqué + sync » : hdr1 = 0x06 par défaut, contient dans l'ordre LED embarquées, JPIPE1, JPIPE2, JRGB, puis 40 LED JRAINBOW1, 40 JRAINBOW2, 120 JCORSAIR (mode sync) ;
   - rapports dédiés (si un ruban dépasse 40 LED, ou 120 pour Corsair → « mode non synchronisé ») : JRAINBOW1 hdr1=4, JRAINBOW2 hdr1=4 + hdr2=1, JCORSAIR hdr1=5 ; pause **13 ms** entre rapports.
3. Limites : 240 LED max par rapport (`NUMOF_PER_LED_MODE_LEDS`) ; JRAINBOW1 200, JRAINBOW2 240, JCORSAIR 240 [V]. Ordre des couleurs : R,G,B.
4. Trois classes de mode direct par carte : `DISABLED`, `ZONE_BASED` (envoie un paquet 185 normal), `PER_LED` (ci-dessus) ; indiqué dans `board_config`.

### Famille 761 (AM5/LGA1851)

- Init : `GET_FEATURE 0x50`(290) puis envoi d'un **tableau de configuration de 290 octets** (ID 0x50 puis 18 lignes de 16 octets `09 FF 00 00 00 FF 00 00 00 FF FF FF FF 03 95/15 1E/78`
  = JARGB1, JARGB2, JARGB3, JAF, JPIPE1..5, JRGB1-2, 6 onboards, « sélection globale », 1 octet de fin) ; dernier octet-ligne `0x95` = zone active, `0x94` = inactive [V].
- Couleurs : feature report **ID 0x51**, 761 octets : `[0x51, 0x09, hdr0, hdr1, 0x00, 0x00, nb_leds, (R,G,B)×240 (colors[720])]`. hdr0 : JAF 0x08, JARGB 0x04, JPIPE 0x06 ; hdr1 : numéro de canal
  (JARGB1=0, JARGB2=1, JARGB3=2, JAF=0, JPIPE1=0) ; `nb_leds` par défaut 240. Redimensionner une zone = renvoyer le tableau de config [V].
- 720 octets de couleur = 240 LED max par rapport.

## Implémentation PureRGB : nombre de LEDs par connecteur ARGB (limite du mode « par zone »)

- Le driver natif (`msi_mystic.rs`) n'implémente que le mode par zone (paquet 185, effet statique) : **une seule couleur par connecteur**. Le rapport 0x53 (LED à LED) n'est PAS implémenté (table `board_config` par PID absente, offsets d'en-tête non validés).
- JPIPE 1/2, JRAINBOW 1/2 et JCORSAIR sont exposés avec un nombre de LEDs réglable par l'utilisateur (1 à 75, limite constructeur du catalogue), enregistré dans les réglages (`zone_sizes`, clé « MSI Mystic Light (carte mère)|JRAINBOW 1 »). Ce réglage ne change PAS les octets envoyés : il dimensionne seulement le tampon d'effet.
- Comme la trame ne porte qu'une couleur par connecteur, la couleur de la 1re LED du tampon est appliquée à tout le connecteur, hub et ventilateurs chaînés compris. Un dégradé ou un arc-en-ciel spatial sur le ruban est donc impossible dans ce mode ; seul un effet qui varie dans le temps s'y voit.
- `cycle_or_led_num` (octet après `ZoneData` des JRAINBOW) n'est jamais écrit : sa sémantique hors mode direct n'est pas confirmée (INCONNU).

## Connecteurs : RGB 12 V vs ARGB 5 V

- **JRGB1/JRGB2** (12 V 4 broches) : une seule couleur par connecteur (une `ZoneData`), 8 bits/canal côté USB ; effets matériels uniquement [V].
- **JRAINBOW1/2** (ARGB 5 V 3 broches) : jusqu'à 200 / 240 LED en mode direct ; `cycle_or_led_num` (défaut 100 dans la structure, 40 en mode sync) = nombre de LED/cycle [V].
- **JCORSAIR** (Corsair iCUE LINK/HD-LL) : 120 LED (sync) ou 240 ; type de ventilo SP/HD/LL (`fan_flags`) [V].
- **JPIPE1/2** : bandeaux de la carte (nombre de LED fixé par la carte, ex. 4/5/6/8/15) [V].
- **JAF/JARGB1-3 (761)** : 240 LED par canal [V].

## Pilote Super I/O (famille B, cartes 12 V sans USB)

Entrée en mode étendu : écriture de **0x87 deux fois** sur le port d'index (0x2E/0x4E) (Linux ; Windows passe par PawnIO) puis lecture de l'ID (reg 0x20/0x21) [V].
Séquence [V] : LDN **0x09**, reg 0x2C : `&= ~0x08`, `|= 0x10` (« requis pour le pulsing ») ; LDN **0x12** (périphérique RGB) :
reg 0xE0 : si `(x & 0xE0) != 0xE0` écrire 0xE0 (activer) ; reg 0xE4 = 0x00 ; reg 0xFF = 0b11100010 (| 0b00011100 si carte « inversée ») ; reg 0xFD = 0x00.
Couleur (4 bits/canal, valeur>>4 répétée dans les deux quartets) : rouge 0xF0-0xF3, vert 0xF4-0xF7, bleu 0xF8-0xFB (8 phases d'une séquence). Seul le statique est implémenté.

## Pilotes noyau, BIOS, ACPI, WMI

- **USB HID (A, C) : aucun pilote noyau spécifique**, HID standard (hidapi suffit). Rapports de type *feature*.
- **Super I/O (B)** : accès aux ports E/S 0x2E/0x4E → pilote noyau tiers (PawnIO, WinRing0 ou équivalent). MSI Center installe ses propres services/pilotes **[M]** : noms
  de pilotes/services du logiciel officiel (INCONNU, non vérifié).
- **BIOS** : l'option « EZ LED Control »/« LED Mode » de certaines cartes éteint ou fige l'éclairage indépendamment du logiciel **[M]** ; effet exact INCONNU.
- **ACPI/WMI** : aucune dépendance relevée dans le code de référence ; le MSI Center officiel utilise un service WMI/SDK propre **[M]**, non analysé.
- **DMI** : le nom de carte (SMBIOS type 2) sert à choisir la configuration (famille B et 761) [V].
- **Mystic Light SDK** (`MysticLight_SDK.dll`, fonctions `MLAPI_*`) **[M]** : API haut niveau côté Windows ; non analysée. À ne pas utiliser pour PureRGB (dépend de l'installation MSI Center).

## INCONNU / non confirmé

- Octets exacts d'en-tête (hdr0..hdr3) du rapport 0x53 : lus dans le code mais non validés sur matériel ici.
- Comportement du rapport 0x52 sur les firmwares < config `minimum_firmware_version` (ex. 7C56 exige APROM ≥ 6) : le code refuse la carte.
- Lecture de version : requête `hid_write` 64 octets `01 B0 CC...` (APROM) et `01 B6 CC...` (LDROM), réponse octet [2] = version (nibble haut = majeur) [V].
- Protocole des cartes de la liste « untested », du connecteur JCORSAIR_OUTERLL120, des modes « music/clock » côté logiciel officiel.
- Pilotes/services du MSI Center, protocole du SDK, ventilateurs/pompes de la carte (la table 0x52 ne contient que `MSI_MODE_FAN_CONTROL = 32`, sémantique INCONNUE).
- Réglage de luminosité globale hors flags.

## Ventilateurs / pompes

Non pilotés par le contrôleur Mystic Light USB : les en-têtes ventilateur sont gérés par le Super I/O / EC (Nuvoton NCT67xx) et l'application MSI Center via d'autres
canaux. Aucun octet documenté ici. Seul l'effet 32 « fan control » et les champs `fan_flags`/`corsair_quantity` (type de ventilateur RGB Corsair) touchent aux ventilateurs, pour l'ÉCLAIRAGE.

## Estimation de difficulté de réimplémentation

- Famille 185/162/112 (HID feature reports) : **faible à moyenne** (structures plates, hidapi, aucun pilote). Risque : écriture flash (`save_data`) ; ne l'utiliser qu'à la demande de l'utilisateur.
- Mode direct LED à LED : moyenne (table de config par carte à constituer, 2 modes sync/non sync).
- Famille 761 : moyenne (config 290 octets + liste blanche de cartes).
- Super I/O : moyenne (pilote noyau tiers requis + liste de cartes + inversion de polarité).

## Tests d'octets possibles sans matériel

1. Calculer les offsets : taille de `FeaturePacket_185` doit valoir 185, 162 → 162, 112 → 112 (assertion de `sizeof` sur structures `repr(C, packed)` Rust).
2. Rapport 0x53 : 5 + 720 = 725 octets ; rapport 0x51 : 7 + 720 = 727 octets (en-tête 7 : id, 0x09, hdr0, hdr1, 0, 0, nb_leds) ; vérifier ces tailles dans un test.
3. Tableau de config 761 : 1 + 18×16 + 1 = 290 octets.
4. Encodage des flags de vitesse/luminosité : `(speed & 3) | (bright_0_10 << 2) | sync<<7`.
5. Quantification Super I/O : `v>>4` puis `v | v<<4`.
6. Test de non-régression de détection : à partir d'une longueur de rapport simulée, choisir la bonne famille.
