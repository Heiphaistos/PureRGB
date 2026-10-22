# RAM RGB « autres » : XPG/ADATA, TeamGroup T-Force, Patriot Viper, Crucial Ballistix, Gigabyte RGB Fusion 2 DRAM (SMBus)

Voir d'abord `smbus-i2c-acces-et-securite.md` (bus, pilotes, règles d'écriture). Pour les puces ENE (T-Force Delta, Geil, une partie des XPG selon modèle) voir `gskill-trident-z-ene.md`.

## Source analysée

- Logiciels officiels : XPG Prime/Spectrix (ADATA), T-Force Blade / Cardea (TeamGroup), Patriot Viper RGB, Crucial Ballistix MOD Utility, Gigabyte RGB Fusion. **Aucun n'a été téléchargé ni extrait** dans cette passe ; leurs pilotes d'accès SMBus sont `INCONNU (non vérifié)`.
- Faits de protocole : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`), dossiers `TForceXtreemController`, `PatriotViperController`, `PatriotViperSteelController`,
  `CrucialController`, `GigabyteRGBFusion2DRAMController`, `ENESMBusController`.

## Matériel couvert

| Gamme | Fabricant SPD (JEDEC) | Mémoire | Adresse(s) 7 bits | Détection dans OpenRGB | LEDs |
|---|---|---|---|---|---|
| XPG / ADATA DDR4 / DDR5 | ADATA 0x044A | DDR4/5 | `INCONNU` | aucun contrôleur dédié trouvé dans cette source (peut-être ENE : non prouvé) | `INCONNU` |
| XPG Spectrix S40G (SSD NVMe RGB) | - | - | 0x67 (ENE, par commandes NVMe/SCSI, pas SMBus) | recherche du disque `SCSI#Disk&Ven_NVMe&Prod_XPG_SPECTRIX_S40#` | via config ENE |
| T-Force Delta RGB (TeamGroup) | TeamGroup 0x046E | DDR4 | ENE, 0x77 puis remappage | chaîne de version `DIMM_LED-0103` | via config ENE |
| T-Force Xtreem ARGB | TeamGroup 0x046E | DDR4 | 0x77 puis remappage vers `0x70..0x76, 0x78, 0x39..0x3D` | test de signature complet | 15 par barrette |
| Patriot Viper RGB (Viper 4 Blackout RGB...) | Patriot 0x8501 | DDR4 | **0x77** (fixe) | SPD « MICSYS_D » + ack 0x77 | 5 par barrette |
| Patriot Viper Steel RGB | JEDEC 0xFF7E (SPD) | DDR4 | **0x77** | SPD « PDA1 » + ack 0x77 | 5 |
| Crucial Ballistix RGB | (Micron) | DDR4 | **0x27** puis remappage vers `0x39..0x3C, 0x20..0x23` | motif A0..AF + chaîne « Micron » | 8 |
| Gigabyte RGB Fusion 2 DRAM (Aorus) | - | DDR4 | **0x67** | magic 0x3282 / 0x0800 | 6 (masque 0x3F) |
| Mushkin 0x8313, Thermaltake 0x8A41 | JEDEC connus | - | - | aucun contrôleur trouvé | `INCONNU` |

## T-Force Xtreem (TeamGroup)

- **Détection sûre** (`TForceXtreemControllerDetect.cpp`) : pour chaque slot SPD de JEDEC TeamGroup, refaire sur 0x77 le test complet : ack (`read_byte` ou `read_byte_data(0)`) puis les registres `0x90..0xA0` (17 octets) doivent valoir `reg - 0x80` (0x10, 0x11, ... 0x20) : **signature différente de l'ENE (0xA0..0xAF = 0..15)**, donc deux familles voisines sur la même adresse 0x77 ne se confondent pas.
  Libre ensuite : sonder l'adresse suivante de la liste (`write_quick` dans OpenRGB, à remplacer par une lecture), puis remapper par registres `0xE0F8` (index de slot) et `0xE0F9` (adresse << 1) à l'adresse 0x77.
- **Registres** : même protocole de pointeur que l'ENE (sélection de registre 16 bits en mot inversé sur la commande 0x00, lecture sur 0x81, écriture d'octet sur 0x01, bloc de 3 octets sur 0x03).
  `0xE020` direct, `0xE021` mode, `0xE022` vitesse (0 rapide..4 lent), `0xE023` direction, `0xE02F` apply (0x01), `0xE0F8` slot, `0xE0F9` adresse, `0xE100` couleurs directes (45 octets = 15 LEDs), `0xE300` couleurs d'effet.
- Ordre des LEDs : l'index logique est converti par `LED_OFFSET(x) = (x impair) ? 14 - (x>>1) : (x>>1)` (les LEDs sont intercalées dans les 15 positions).
- Couleurs : trois octets par LED dans l'ordre **R, B, G** (confirmé dans `TForceXtreemController.cpp::SetAllColorsDirect`), envoyés par blocs de 3 octets à `0xE100 + 3 x LED_OFFSET(i)`.
- Ordre des tests : ne remapper que si le test de signature réussit.

## Patriot Viper RGB et Viper Steel

- **Adresse** : 0x77 pour toutes les barrettes ; **pas de remappage**.
- **Détection** : `write_quick(0x77)` doit réussir, puis le SPD de la barrette (octets « fabricant » à l'offset 0x161..0x168) doit contenir la chaîne ASCII :
  Viper RGB : `4D 49 43 53 59 53 5F 44` = `"MICSYS_D"` ; Viper Steel : `50 44 41 31 00 00 00 00` = `"PDA1"` + 4 zéros. Le masque des slots valides (4 slots) vient de ces correspondances.
  **Risque** : un ack sur 0x77 ne prouve rien, le seul garde-fou fiable est la chaîne SPD (ne rien écrire sans elle).
- **Écriture d'un registre** (particulière) : `ViperRegisterWrite(reg, v0, v1, v2)` = deux écritures SMBus : `write_byte_data(adresse, commande = reg, donnée = v0)` puis `write_byte_data(adresse, commande = v2, donnée = v1)`. Le troisième octet voyage dans l'octet de commande : ce ne sont donc pas de vraies adresses de registre. Ne jamais envoyer ce schéma à une adresse non identifiée.
- **Viper RGB** (registres) : `0x01` statique, `0x03` mode, `0x30..0x34` couleurs directes LED0..4 (R,B,G), `0x35` apply (`01 00 00`), `0x3B..0x3F` couleurs d'effet LED0..4, `0xFF` start-frame (`FF FF FF`).
  Mode direct : `START`, `STATIC(04 00 00)`, `MODE(DIRECT=07, 00, 00)` ; chaque LED : `START`, `STATIC`, `couleur directe`, `APPLY(01)`.
  Modes : 00 sombre, 01 respiration, 02 viper, 03 heartbeat, 04 marquee, 05 raindrop, 06 aurora, 07 direct, 08 néon (cycle de couleurs). Vitesses : 0xC8 lent .. 0x14 rapide (0xFF .. 0x00 pour la respiration).
  Les effets matériels exigent un **thread de maintien** (keepalive) qui renvoie `MODE(mode, step, speed)` puis `MODE(0xAA, 0, sous-étape)` en boucle : le contrôleur n'anime pas seul.
- **Viper Steel** : registres de couleur directe `0x17..0x1B` (LED0..4, ordre R,B,G), écrits avec le même schéma ; pas de start/apply dans la source (écriture directe). Modes d'effet : `INCONNU` dans cette source.

## Crucial Ballistix RGB (contrôleur ENE-like Micron)

- **Adresses** : sonde `0x27`, puis remappage vers `0x39, 0x3A, 0x3B, 0x3C, 0x20, 0x21, 0x22, 0x23` (pour 4 slots au plus, 1 ms entre sondes).
- **Remappage** : écrire à `0x27` les registres 16 bits `0x82EE = slot`, `0x82EF = adresse << 1`, `0x82F0 = 0xF0` (même protocole de pointeur que ENE).
- **Test de signature** : motif `A0..AF` = `0..15`, puis la chaîne `"Micron"` à `0x1025..` ou `0x1030..` ; sinon refus (si la chaîne est absente c'est un ENE d'un autre fabricant).
  **Même risque que l'ENE** : le remappage ne vérifie que l'ack de 0x27 dans le code d'OpenRGB. PureRGB doit tester la signature d'abord.
- **Registres** : `0x1000` version ; mode `0x820F` ; luminosité (`0x82EE=0xFF`, `0x82EF=lum`, `0x82F0=0x83`) ; vitesse/mode (`0x82EE=0x00`, `0x82EF=vitesse`, `0x82F0=0x84`) ;
  couleurs directes **planaires** par bloc de 8 octets (une LED = un octet par plan) : rouge `0x8300`, vert `0x8340`, bleu `0x8380` ; couleur d'effet d'une LED : `0x82E9 = (1 << led)`, `0x82EA..0x82EC = 0`, `0x82ED/0x82EE/0x82EF = R/G/B`, `0x82F0 = 0x01`.
  Modes (octet de mode) : 0x1F shift, 0x2F gradient shift, 0x3F fill, 0x4F stack, 0x5F double stack, 0x6F respiration, 0x7F motion point, 0x8F inside out, 0x9F color step, 0xAF water wave, 0xBF clignotement, 0xCF statique.
  LEDs : 8 par barrette.

## Gigabyte RGB Fusion 2 DRAM (Aorus RGB)

- **Adresse** : **0x67** (une seule), bus SMBus RAM.
- **Détection** (`GigabyteRGBFusion2DRAMControllerDetect.cpp`) : `write_quick(0x67)` ; si ack : `write_byte_data(0x67, 0xE1, 0x01)` (écriture vers un appareil encore non identifié : **à éviter**), puis `read_word_data(0xED)` doit valoir **0x3282** et `read_word_data(0xEB)` doit valoir **0x0800**.
  PureRGB : tenter d'abord les lectures de `0xED`/`0xEB` (sans écrire `0xE1`), n'écrire 0xE1 que si les valeurs magiques sont déjà présentes (le contrôleur peut ne répondre qu'après `0xE1`, `INCONNU`).
- **Écriture** : bloc `write_block_data(0x67, 0x20, 15 ou 32 octets)` ; apply : `write_byte_data(0x67, 0x28, 0x0F)`. Indices dans le bloc : `0x00` masque d'activation des LED (0x3F), `0x09` mode, `0x0A` luminosité, `0x0C` bleu, `0x0D` vert, `0x0E` rouge.
  Modes : 0 off, 1 statique, 2 pulsation, 3 flash, 0xFF « direct » = factice, implémenté en pulsation par LED.

## Ventilateurs / pompes

Sans objet.

## Accès bas niveau

Même pilote et même mutex que les autres fiches. Tous ces contrôleurs sont fragiles (délais de 1 à 10 ms entre accès, relancer en cas d'échec). Les logiciels officiels : mécanisme `INCONNU (non vérifié)`.

## Ce qui est INCONNU

- Contrôleur et protocole des XPG DDR4/DDR5 (ADATA 0x044A) ; des TeamGroup T-Force Delta DDR5 / Xtreem DDR5 ; des Patriot Viper DDR5 ; des Crucial Pro RGB DDR5 ; des Mushkin/Thermaltake RAM.
- Ordre et nombre exact de LEDs pour chaque modèle (le nombre vient du contrôleur ou d'une constante locale).
- Modes d'effet matériels Viper Steel ; persistance des réglages à la coupure ; durée de vie des écritures flash.
- Comportement du remappage ENE-like au redémarrage à chaud.

## Estimation de difficulté

- Patriot Viper : **moyenne** (schéma d'écriture atypique + thread de maintien pour les effets).
- Crucial / TeamGroup Xtreem : **moyenne** (protocole pointeur 16 bits + remappage à sécuriser).
- Gigabyte DRAM : **faible** (bloc + apply), détection à durcir.
- XPG : non estimable (rien de documenté).

## Tests d'octets possibles sans matériel

1. `"MICSYS_D"` = `4D 49 43 53 59 53 5F 44` ; `"PDA1"` = `50 44 41 31 00 00 00 00` : tests unitaires de reconnaissance SPD.
2. Viper : couleur R=1,G=2,B=3 pour LED0 en direct -> écritures (cmd 0x30,data 1) puis (cmd 2... voir schéma : `v0=R=1, v1=B=3, v2=G=2`) soit (cmd 0x30, data 0x01), (cmd 0x02, data 0x03). Vérifier le schéma à deux écritures sans bus réel.
3. Crucial : mot de sélection du registre `0x82F0` -> `0xF082` ; remappage de la cible 0x39 -> octet `0x72`.
4. Xtreem : `LED_OFFSET` pour x = 0..14 doit produire une permutation de 0..14 (test exhaustif).
5. Gigabyte DRAM : paquet de 15 octets avec masque `0x3F` en tête, apply `0x28 = 0x0F`.
