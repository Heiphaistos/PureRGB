# Cartes graphiques RGB par constructeur : adresses I2C, détection, registres

Prérequis : `gpu-acces-i2c-nvapi-adl.md` (comment atteindre le bus GPU : NVAPI port 1 / ADL ligne 1, liste blanche PCI) et `smbus-i2c-acces-et-securite.md`.

## Source analysée et limites

- Source unique : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`), dossiers `Controllers/<famille>GPUController` + `Controllers/ENESMBusController`.
- Logiciels officiels non extraits (ASUS GPU Tweak III / Armoury Crate, MSI Center + Mystic Light, Gigabyte Control Center / RGB Fusion, EVGA Precision X1, Zotac FireStorm,
  PNY VelocityX, Sapphire TriXX / NITRO Glow, PowerColor DevilZone, ASRock Polychrome, Palit ThunderMaster, Galax Xtreme Tuner, Colorful iGame Center) : mécanisme d'accès `INCONNU (non vérifié)`.
- Dans tous les cas : (1) la carte doit être dans la liste blanche PCI (vendeur, device, subvendor, subdevice) ; (2) toutes les adresses ci-dessous sont des **adresses 7 bits** ; sous NVAPI/ADL l'octet réel est `adresse << 1`.
- Registres notés R,G,B sauf mention. Valeurs en hexadécimal.

## Tableau récapitulatif

| Famille | GPU | Adresse(s) 7 bits | Vérification avant écriture | Accès |
|---|---|---|---|---|
| ASUS Aura GPU (hérité) | NVIDIA GTX 10xx/16xx/RTX 20xx, AMD RX Vega/5000/6000 | 0x29 (cartes les plus anciennes), 0x2A (GTX 1650S/1660, RTX 20xx, AMD...) | registres 0x20,0x21 = `15 89` | NVAPI port 1 / ADL |
| ASUS ENE GPU (Ampere et après) | RTX 30/40/50 ROG Strix / TUF / KO / Astral / Matrix | 0x67 | signature ENE (motif A0..AF) + chaîne de version | NVAPI port 1 |
| MSI Mystic Light GPU v1 | GTX 10xx/16xx, RTX 20xx/30xx, AMD | 0x68 | aucune (PCI seul) | NVAPI / ADL |
| MSI Mystic Light GPU v2 | RTX 40xx Gaming X | 0x68 | aucune (PCI seul) | NVAPI |
| Gigabyte RGB Fusion GPU (v1) | GTX 10xx/16xx, RTX 20xx/30xx | 0x47, 0x48, 0x55, 0x62, 0x63, 0x71 (selon carte) | réponse `AB` puis `10/11/12/14` | NVAPI |
| Gigabyte RGB Fusion 2 GPU | RTX 30/40 AORUS, AMD RX | 0x50, 0x51, 0x55, 0x62..0x66, 0x70..0x73 | réponse bloc `AB xx xx xx` | NVAPI / ADL |
| Gigabyte RGB Fusion 2 Blackwell GPU | RTX 50 + RX 9070 | 0x75 | réponse bloc `01 01/02 01 xx` | NVAPI / ADL |
| EVGA GPUv1 (Pascal) | GTX 10xx | 0x49 | aucune (PCI seul) | NVAPI |
| EVGA GP102 | GTX 1070/1080/1080 Ti FTW2/FTW3 | 0x4A, 0x2A, 0x4F (zones) | registre 0x04 = 0x1F ou 0x91 | NVAPI |
| EVGA GPUv2 (Turing) | RTX 20xx XC/FTW3 | 0x49 | aucune (PCI seul) | NVAPI |
| EVGA GPUv3 (Ampere) | RTX 30xx FTW3 | 0x2D | lecture du firmware (reg 0xB1, 6 octets) | NVAPI |
| Zotac Turing | RTX 20xx | 0x49 | écriture d'activation acceptée | NVAPI |
| Zotac V2 | RTX 30/40 (Twin Edge, AMP...) | 0x49 | lecture brute de bloc réussie + zones > 0 | NVAPI |
| Zotac Blackwell | RTX 50 | 0x4B | lecture du registre 0x10 réussie + version | NVAPI |
| PNY EpicX / XLR8 | RTX 20xx-30xx (ARGB) | 0x60 | aucune (PCI seul) ; registre de détection 0x81 | NVAPI |
| PNY Lovelace | RTX 40xx | 0x60 | aucune (PCI seul) | NVAPI |
| Sapphire NITRO Glow v1 | RX 4xx/5xx | 0x55 | ack `read_byte` | ADL |
| Sapphire NITRO Glow v3 | RX 5000+ (Nitro+) | 0x28 | ack `read_byte` | ADL |
| PowerColor Red Devil v1 / v2 | RX 5000 / 6000 / 7000 | 0x22 | magic `01 05 00` (v1) ou `01 32 00` (v2), reg 0x90 pour v1 | ADL |
| Palit / Gainward v1 | GTX/RTX | 0x08 (Gainward) ou 0x49 | chaîne « PALIT » aux registres 0x07..0x0B, ou write_quick (Gainward) | NVAPI |
| Palit / Gainward v2 | RTX 30/40 | 0x49 | reg 0x00 = 0 et reg 0xE0 < 5 | NVAPI |
| Galax / KFA2 | RTX 30/40 | 0x32 (v1), 0x23 (3080), 0x50/0x51 | valeurs de registres 0x00/0x01 | NVAPI |
| Colorful iGame | RTX 30/40 | 0x61 (la plupart), 0x81 ; Turing 0x50 | handshake paquet `AA EF 81 ...` | NVAPI |
| ASRock Phantom/Taichi/Steel Legend (AMD) | RX 7900 / 9070 / 9060 | 0x36 | commande 0x14 sous-commande 0x01 -> bitmap de canaux | ADL |
| NVIDIA Illumination | RTX FE, quelques PNY/Palit/Gainward/Manli | n/a (NVAPI) | liste blanche | NVAPI Illum |

## ASUS Aura GPU (hérité, GTX 10/RTX 20/Vega...)

- Détection : lire les registres `0x20` (poids fort) et `0x21` (poids faible) : doit donner **0x1589** (`AURA_GPU_MAGIC_VAL`). Sinon refus.
  Pour les cartes AMD : bus nommé « AMD ADL » exigé.
- Registres (`write_byte_data`) : `0x04` rouge, `0x05` vert, `0x06` bleu, `0x07` mode, `0x0C` « sync », `0x0E` application/enregistrement (valeur 0x01).
- Modes OpenRGB : 0 « off » (fictif), 1 statique, 2 respiration, 3 clignotement, 4 cycle de spectre, 5 « direct » (fictif, = écrire statique).
- Sauvegarde : écrire `0x0E = 0x01` fait « Save » : OpenRGB considère par prudence qu'il enregistre en mémoire permanente (« assume the safest option ») sauf Vega 64 Strix inconnu : n'enregistrer que sur demande.
- Cartes : environ 74 entrées (ROG Strix GTX 1050 à RTX 2080 Ti, ROG Poseidon, RX Vega 64 Strix...).

## ASUS ENE GPU (RTX 3000+ ASUS) : voir `gskill-trident-z-ene.md`

- Adresse **0x67**, protocole ENE complet (sélection de registre 16 bits, couleurs R,B,G, table de config à 0x1C00). Environ 173 entrées dans le détecteur ENE (RTX 3050 à 5090, ROG Strix / TUF / KO / Astral / Matrix).
- Chaînes de version GPU reconnues : `AUMA0-E6K5-0107`, `-1107`, `-1110`, `-1111`, `-0008`, `-1113`, `-1114`. Chaîne inconnue = pas de contrôle.
- Détection : motif A0..AF = 0..15, puis lecture des 16 octets de version, rejet si « Micron ».

## MSI Mystic Light GPU

- Adresse **0x68**, détection par PCI seulement (pas de lecture magique) : condition `(NVIDIA et port_id = 1) ou (AMD et bus ADL)`.
- **v1** (`MSIGPUController`) : écritures `write_byte_data` : `0x30/0x31/0x32` = R1/G1/B1, `0x27/0x28/0x29` = R2/G2/B2, `0x2A/0x2B/0x2C` = R3/G3/B3, `0x36` luminosité (0..5, x20 = %), `0x38` vitesse (0..2), `0x22` mode, `0x26` inconnu, `0x3F` = 0x01 enregistrer.
  Modes : 0x01 off, 0x13 statique, 0x08 arc-en-ciel, 0x1A raindrop, 0x07 magic, 0x05 patrolling, 0x06 streaming, 0x15 lightning, 0x1F wave, 0x16 meteor, 0x0D stack, 0x0B rhythm, 0x09 flowing, 0x0F whirling, 0x11 twisting, 0x1D laminating, 0x14 fade-in, 0x04 breathing, 0x02 flashing, 0x03 double flashing.
  Pas de délai imposé dans la source (à ajouter par prudence : 20 ms).
- **v2** (`MSIGPUv2Controller`, RTX 40xx) : 20 ms après **chaque** écriture (`sleep 20 ms`) ; couleurs 1/2/3 écrites en **bloc I2C de 3 octets** aux registres `0x27`, `0x28`, `0x29` dans l'ordre **B, G, R** ; `0x30/0x31/0x32` = couleur 1 en octets séparés ; `0x36` luminosité (1..5), `0x38` vitesse, `0x22` mode, `0x2E` inconnu, `0x46` direction (0 droite, 2 gauche) / 1 = « non-RGB » pour programmer, `0x3F` = **0x00** pour valider (et non 0x01 comme v1). Mode 0x1C « idle » pour la programmation.
- Mystic Light côté carte mère est une autre fiche (`msi-mystic-light.md`, autre agent).

## Gigabyte RGB Fusion (GPU)

### v1
- Détection : `write_byte` (envoi d'octet seul) de `0xAB`, puis trois `0x00` (sept `0x00` à l'adresse 0x62) ; lire 4 octets avec `read_byte` : le 1er = `0xAB`, le 2e ∈ {`0x10`,`0x11`,`0x12`,`0x14`} ; l'adresse **0x48** est acceptée sans vérification (valeur de réponse inconnue).
- Écriture : couleur = suite d'octets seuls `0x40, R, G, B` ; mode = `0x88, mode, vitesse, luminosité` ; enregistrer = `0xAA, 0, 0, 0` ; à l'adresse 0x62 chaque trame est suivie de 4 octets `0x00`.
- Modes : 0x01 statique, 0x02 respiration, 0x04 clignotement, 0x08 double clignotement, 0x10 cycle de couleurs, 0x11 cycle de spectre. Vitesse 0..9 (normal 5), luminosité 0..0x63 (99).
- **Attention** : ce protocole « envoie octet seul » (aucune adresse de registre) : une trame envoyée à un mauvais appareil est interprétée comme données brutes. Liste blanche stricte.

### v2 (AORUS RTX 30/40, RX)
- Trames de 8 octets par écriture I2C brute (`i2c_write_block`, pas d'adresse de registre dans le protocole SMBus) ; détection : envoyer `AB 00 00 00 00 00 00 00` puis lire 4 octets : le premier doit valoir `0xAB` et la réponse ne doit pas être `AB AB AB AB` (écho de bus DDC).
  Réponses vues : `AB 11 52 03`, `AB 10 01 02`, `AB 11 01 00`, `AB 10 52 07`...
- Octets de commande (1er octet du paquet) : `0x88` mode : `[88, mode, vitesse, luminosité, flag, zone+1, 00, 00]` ; `0x40` couleur : `[40, R, G, B, zone+1, 0, 0, 0]` ; `0xB0` couleurs gauche/milieu : `[B0, mode, R1,G1,B1, R2,G2,B2]` ; `0xB1` droite : `[B1, mode, R,G,B]` ; `0xAA` enregistrer : `[AA, 0 x7]`.
  Modes : 1 statique, 2 respiration, 3 cycle de couleurs, 4 clignotement, 5 dégradé, 6 décalage de couleur, 7 vague, 8 double clignotement, 0x0B tricolore. Vitesse 0..5, luminosité 0..99.
- Cartes à plusieurs bus : un seul répond `AB` (voir fiche accès). Une AORUS 4080 expose deux bus à 0x71 dont un gère l'écran LCD : ne pas l'écrire.

### Blackwell (RTX 50, RX 9070 Gaming OC)
- Adresse **0x75** ; trames de **64 octets** (`i2c_write_block`) ; détection : envoyer `10 01 00 ... 00` (64 octets) puis lire 4 octets : doivent être `01`, (`01` ou `02`), `01`, `xx` (valeur vue : `01 01 01 00`, `01 02 01 00` pour RX 9070 XT, `01 01 01 10` pour AORUS 5080 MASTER).
- Type de trame (octet 0) : `0x12` = mode (au plus ~9 mises à jour/s), `0x16` = couleur « intelligente » (mode direct, plus rapide), `0x13` = enregistrer (`13 01 00...`).
  Trame mode/couleur : `[type, 0x01, mode, vitesse, luminosité, R, G, B, 0, zone, nbCouleurs, couleurs R,G,B...]` (les couleurs commencent à l'octet 12, ou 11 sur AORUS 5080 MASTER et 5090D V2 Ice). Zones : 6, ventilateurs AORUS Master : 8 LEDs.
  Modes : 0 direct (type 0x16), 1 statique, 2 respiration, 3 clignotement, 4 double clignotement, 5 cycle, 6 vague, 7 dégradé, 8 décalage, 9 tricolore (Waterforce), 0x0A dazzle, 0x0C claws (AORUS 5080 MASTER). Vitesse 1..6, luminosité 1..10.

## EVGA

- **GPUv1 Pascal** (adresse 0x49) : registres `0x0C` mode (0 off, 1 custom, 2 arc-en-ciel, 5 respiration), `0x09/0x0A/0x0B` R,G,B ; `0x23 = 0xE5` = **enregistrement** (`SaveSettings`) : à n'envoyer que sur demande explicite.
- **GP102 (GTX 1070/1080/1080 Ti FTW)** : trois « zones » = trois adresses : (0x4A : couleurs `09 0A 0B`, acquittement 0x03 / terminé 0x00), (0x2A : `30 31 32`, 0xE9 / 0xE0), (0x4F : `30 31 32`, 0x03 / 0x00). Détection de zone : lire le registre `0x04` = `0x1F` ou `0x91` (3 essais). Commandes sur le registre `0x0E` : `0xE5` début, `0xE9` couleur, `0xE0` fin ; mode sur `0x0C`. L'enregistrement n'a **pas** fonctionné (commentaire de la source).
- **GPUv2 Turing (RTX 20xx)** (0x49) : `0x60` mode (0 off, 1 statique, 2 arc-en-ciel, 3 respiration, 4 pulse ; modes logiciel 0x0F arc-en-ciel...), couleurs A `0x6C..0x6E` + luminosité `0x6F`, couleurs B `0x70..0x72` + `0x73`, durées A->B/B->A `0x62..0x65`, temps allumé A/B `0x66..0x69`, `0x74/0x75` inconnu, `0x61 = 0x01`, `0x6A/0x6B = 0`, `0x19 = 0x11` pour la couleur fixe.
  Séquences d'écriture avec verrou : `0x0E = E5, E9, F5, F9, F0, E0`, `0x08 = 0x01`, `0x1F = 0xE5`, `0x23 = 0xE5` (désactivation/activation des écritures). Valeur de luminosité 0x01..0x64.
- **GPUv3 Ampere (RTX 30xx FTW3)** (0x2D) : accès par **blocs I2C** (`write_i2c_block_data`, `read_i2c_block_data`) ; **initialisation obligatoire** avant toute commande (rejouée avant chaque changement de mode pour survivre à la veille) : bloc `04 C6 EB EA 15` au registre `0xB2` (« Enable ») ;
  enregistrement : `04 9E EB 00 90` au registre `0x90` (envoyé à la fermeture de Precision X1). Firmware : lire 6 octets au registre `0xB1` (majeur = octet 4, mineur = octet 5) -> détection (version vide = refus).
  Mode (registre `0xC0`, paquet de 10 octets `[9, mode zone0, zone1, zone2, zone3, 0xFF x4, sync]`), taille ARGB (octet 8 du même paquet, 1..60 LEDs). Mode par zone : 0 off, 1 statique, 2 respiration, 3 arc-en-ciel, 4 cycle de couleurs, plus vague, étoile, pile...
  Registres de configuration par mode : statique 0xC1+zone, respiration 0xC5, arc-en-ciel 0xC9, cycle 0xCD, arc-en-ciel vague 0xD1, vague 0xD5, étoile 0xD9, pile 0xDD ; nombre de couleurs de cycle `0xE5`, de pile `0xE6`, direction de pile `0xEB`.
  Paquet statique : `[4, luminosité, R, G, B]` (octet 0 = longueur de la charge utile) ; respiration : `[9, lum, R1,G1,B1, R2,G2,B2, vitesse LSB, vitesse MSB]`.

## Zotac

- **Turing (RTX 20xx)** (0x49) : paquet brut de 8 octets sur le registre `0xA0` : `[A0, 00, mode, R, G, B, 00, vitesse]`. Activation/test : `[A0, F1, 01, 0,0,0,0,0]` (`F1 00` désactive). Modes : 0 statique, 1 strobe, 2 wave, 3 respiration, 4 cycle ; vitesse 0 (rapide) .. 9 (lent).
- **V2 (RTX 30/40)** (0x49) : paquet de 21 octets `[A0, on, reset, 0, 0, idle/actif, zone, mode, R1,G1,B1, vitesse, luminosité, direction, 0, synchro, circuit, R2,G2,B2, préréglage]`. Lecture d'une zone : envoyer `[A0, F0, 0,0,0, idle/actif, zone, 0]` puis lire le bloc (octets 0..19). Modes : 0 statique, 1 breath, 2 fade, 3 wink, 4 flash, 5 shine, 6 random, 7 slide, 8 rainbow, 9 marquee, 0x0A drip, 0x0B dance, 0x17 duet, 0x18 path. Synchro : 0 individuel, 1 synchronisé, 2 avec l'externe. Configurations : 0 repos, 1 sous charge.
  Détection : lecture brute d'un bloc réussie, puis nombre de zones obtenu > 0.
- **Blackwell (RTX 50)** (0x4B) : écrire un à un (3 ms entre chaque) les registres `0x20..0x2F` (`0x20` fixe, `0x21` zone, `0x22` mode, `0x23..0x25` couleur 1, `0x26..0x28` couleur 2, `0x29` luminosité, `0x2A` vitesse, `0x2B` direction, `0x2C..0x2F` réservés), puis `0x17 = 0x01` (commit) ; `0x11` = reload. Version du firmware lue au registre `0xF0` (bloc I2C ; une lecture sans registre renvoie un statut). Détection : `read_byte_data(0x10)` réussi.

## PNY

- **Lovelace (RTX 40xx)** (0x60) : un seul registre `0x02`, bloc I2C de 7 octets `[mode, luminosité, vitesse, flag, R, G, B]`. Modes : 0 off, 2 respiration, 3 cycle, 4 vague, 5 flash, 6 statique. Vitesse inversée : cycle `0xB2 - v`, respiration `0x19 - v`, vague `0xBF - v`, flash `0x4D - v`. Mode direct par LED : `[6, 0xFF, led, 0, R, G, B]`.
- **EpicX ARGB (RTX 20/30 XLR8)** (0x60) : blocs I2C de 7 octets identiques, écrits dans le registre de zone : flèche `0x02`, avant `0x04`, logo `0x0F` ; registre de détection `0x81`. Modes ARGB : 0 off, 0x0302 respiration (valeur double), 3 cycle, 4 néon, 5 explosion, 6 supernova, 7 infini, 8 streamer, 9 direct, 0x0A vague.

## Sapphire NITRO Glow

- v1 (0x55) : `0x00` mode (0 bleu Sapphire, 1 arc-en-ciel, 2 température carte, 3 vitesse ventilateur, 4 custom, 5 off), `0x01` luminosité, `0x03/0x04/0x05` R,G,B. Détection : ack de `read_byte`.
- v3 (0x28) : `0x10` mode (0 arc-en-ciel, 1 runway, 2 cycle de couleurs, 3 serial, 4 bleu Sapphire, 5 visualisation audio, 6 custom, 7 off, 0xFF contrôle externe), `0x0F` contrôle externe, `0x3E` luminosité, `0x11` vitesse runway, `0x12` répétitions, `0x13` vitesse cycle, `0x15` vitesse arc-en-ciel, `0x16` vitesse serial, `0x29` volume, `0x1A/0x1B/0x1C` R,G,B.
- Détection : seulement l'ack d'un `read_byte` (pas de signature) : la liste blanche PCI (sous-vendeur 0x1DA2 ou 0x174B, 33 entrées) est le garde-fou.

## PowerColor Red Devil

- Adresse 0x22 ; détection par lecture de bloc I2C de 3 octets : v1 au registre `0x90` attend `01 05 00` ; v2 (autre registre) attend `01 32 00`.
- v1 : `0x01` mode (0 off, 1 statique, 2 respiration, 3 néon, 4 blink...), `0x02..0x0D` LED 1..12, `0x0E` toutes, `0x0F` couleur de mode, `0x12` synchro carte mère (coupe le contrôleur pour l'ARGB). Le détail de l'encodage couleur et v2 : `Controllers/PowerColorGPUController/*` (INCONNU ici).

## Palit / Gainward

- v1 (0x49 ou 0x08) : chaîne `P A L I T` aux registres `0x07..0x0B` ; si absente, adresse 0x08 sondée par `write_quick` (Gainward) ; registre rouge `0x03` (suite R,G,B ; détail non relu).
- v2 (0x49) : doit lire `0x00` au registre 0 et `< 5` au registre `0xE0`. Registres : `0x60` contrôle (direct), `0x62..0x65` vitesse respiration (octets bas/haut, couleur 1 et 2), `0x6C..0x6E` R,G,B, `0x6F` luminosité, `0x70..0x72` couleur 2, `0xE0` mode, `0xE1` direction.

## Galax / KFA2, Colorful, ASRock

- **Galax** : v1 à 0x32 (registre 0 ∈ {0x26,0x27} et registre 1 ∈ {0x10,0x20}), variante RTX 3080 à 0x23 (registre 0 ∈ {0x27,0x30}) ; v2 autre protocole (0x50/0x51) : registres détaillés `INCONNU` ici (voir `GalaxGPUv1Controller`, `GalaxGPUv2Controller`).
- **Colorful iGame** : poignée de main : écrire 6 octets `AA EF 81 02 1C 02` (écriture brute) puis lire un bloc (jusqu'à 64 octets) : les 3 premiers doivent être `AA EF 81`. Adresses 0x61 (40 modèles), 0x81, Turing 0x50. Registres/trames d'effet : `INCONNU` ici.
- **ASRock (AMD RX 7900 / 9070 / 9060)** : adresse **0x36** ; protocole par commandes/sous-commandes en blocs (`WriteBlock(cmd, sous-cmd, données)` puis lecture de bloc après 20 ms) ; détection : commande `0x14`, sous-commande `0x01` -> la réponse (octets 4-5, little-endian) est un **bitmap de canaux actifs** ; aucun canal = refus. Reste du protocole : `INCONNU` ici.

## Ventilateurs / pompes

Certaines cartes (Gigabyte Waterforce, MSI Sea Hawk, EVGA Hybrid) ont un contrôle de ventilateur/pompe séparé non couvert par ces registres : `INCONNU`.

## Ce qui est INCONNU

- Le chemin d'accès réel de chaque logiciel officiel (aucun installeur extrait).
- Les registres complets pour PowerColor v2, Galax v2, Colorful, ASRock GPU, Palit v1 couleurs, Sapphire (effets ou séquences d'application).
- La persistance des réglages après coupure pour chaque famille (les octets « Save » sont marqués ci-dessus).
- La liste des cartes de chaque famille doit être reprise des tables PCI d'OpenRGB (`Controllers/<famille>/*Detect.cpp`) : plus de 600 entrées au total, non recopiées ici.
- Les cartes Intel Arc : aucun contrôleur RGB documenté.

## Estimation de difficulté

- Faciles (registres à octets, peu de risques) : ASUS Aura GPU hérité, MSI v1/v2, Sapphire v1/v3, EVGA v1, PNY Lovelace, Zotac Blackwell.
- Moyennes : Gigabyte v1/v2/Blackwell (trames brutes), Zotac V2, EVGA v2/v3 (séquences de verrou/init), ENE GPU.
- Difficiles / non documentées : ASRock, Colorful, Galax v2, PowerColor v2.

## Tests d'octets possibles sans matériel

1. ASUS : magic `0x15 0x89` -> mot 0x1589.
2. MSI v2 : couleur (R=1,G=2,B=3) -> bloc I2C de 3 octets `03 02 01` au registre 0x27.
3. Gigabyte v2 : paquet d'écriture de couleur `[40, R, G, B, zone+1, 0, 0, 0]` (8 octets) ; paquet Blackwell : 64 octets, 1er `0x12` ou `0x16`.
4. Zotac V2 : construire le paquet de 21 octets ; Zotac Turing activation `A0 F1 01 00 00 00 00 00`.
5. EVGA v3 : paquet statique `04 lum R G B` ; mode `09 mode0..3 FF FF FF FF sync` (10 octets) ; initialisation `04 C6 EB EA 15`.
6. PNY Lovelace : vitesse inversée (`0xB2 - v`) pour v = 0..0xB2 ; paquet 7 octets.
7. Rejeter toute réponse de détection Gigabyte valant `AB AB AB AB` (écho).
