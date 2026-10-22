# ASRock Polychrome RGB (Polychrome Sync) : protocoles RGB des cartes mères ASRock

Date : 2026-10-22. Légende : **[V]** lu dans une source de référence (voir ci-dessous), **[M]** de mémoire, NON vérifié, **INCONNU** non confirmé.
Aucun code ni asset copié : uniquement des faits de protocole.

## Source analysée (honnêteté)

- **Le logiciel officiel (ASRock Polychrome RGB / Polychrome Sync) n'a PAS été analysé** : installeur par carte sur asrock.com, non téléchargé, interdit d'installer
  (il charge un pilote noyau et un service **[M]**). SDK : aucun SDK public identifié.
- Faits tirés de la **lecture du code OpenRGB** (GPL-2.0-or-later, instantané local `D:\Projet\Analyses\rgb\openrgb-src\`, commit non enregistré), construit sur des captures du logiciel officiel.
  Fichiers : `Controllers/ASRockPolychromeUSBController/{ASRockPolychromeUSBController.{h,cpp},ASRockPolychromeUSBControllerDetect.cpp,RGBController_ASRockPolychromeUSB.cpp}`,
  `Controllers/ASRockSMBusController/{ASRockSMBusControllerDetect.cpp,ASRockPolychromeV1SMBusController/*,ASRockPolychromeV2SMBusController/*,ASRockASRRGBSMBusController/*}`, `pci_ids/pci_ids.h`.

## Matériel couvert

| Famille | Identifiant | Interface | Remarque |
|---|---|---|---|
| **Polychrome USB** (cartes récentes, MCU USB) | **26CE:01A2** | HID, rapports de 65 octets (ID 0, puis 64 de données) | [V] |
| ASRock Deskmini ruban ARGB | **26CE:01A6** | idem, même protocole | [V] |
| **Polychrome SMBus V1 / V2 / « ASR RGB »** (cartes plus anciennes) | esclave I2C **0x6A** sur le SMBus carte mère | SMBus | sous-système PCI **0x1849 (ASRock)** [V] |

Version du firmware SMBus : `read_block_data(0x6A, reg 0x00)` doit renvoyer **2 octets** (majeur, mineur). Majeur **1** = ASR RGB LED, **2** = Polychrome V1, **3** = Polychrome V2 [V].
Quels PID USB/quelles cartes ASRock sont « USB » ou « SMBus » : voir cartes avec MCU Nuvoton/ASRock dédiée (INCONNU, pas de liste de cartes dans le code ; USB = le périphérique 26CE:01A2 existe).

## Protocole USB Polychrome (26CE:01A2) [V]

Transport : `hid_write` de **65 octets** (octet[0] = 0 = pas de report ID, donc paquet de 64 octets utiles), puis `hid_read` de 64 octets (accusé de réception ; à lire sinon le contrôleur se bloque).
Octets utiles `u[0..63]` = `buf[1..64]` ; les offsets du code sont ceux de `buf` (dont `buf[0]=0`).

### Commandes (buf[1])

| Cmd | Rôle |
|---|---|
| 0x10 | Régler une zone (mode, couleur, vitesse) ; `buf[3]`=zone ; avec zone=0xFF et `buf[4]`=0xE3/0xE4 : flux par LED |
| 0x11 | Lire la configuration d'une zone (`buf[3]` = zone) ; réponse : `[4]` mode, `[5]` `[6]` `[7]` couleur (R,G,B ou G,R,B selon rgswap), `[8]` vitesse |
| 0x12 | Valider/appliquer (« commit ») |
| 0x14 | Lire une table : `buf[3]` = 0x02 → nombre de LED des 8 zones (`[4..11]`), 0x03 → masque RGSWAP (`[4]`), 0x01 → masque de zones présentes (`[4]`) |
| 0x15 | Écrire une table : `buf[3]` = 0x02 (tailles des zones, 8 octets à partir de `buf[4]`) ou 0x03 (RGSWAP, 1 octet) |
| 0xA4 | Init (déclaré, usage non vu) |

### Zones (index 0..7)

0 RGB header 1 (12 V), 1 RGB header 2 (12 V), 2 ARGB header 1, 3 ARGB header 2, 4 PCH, 5 IO cover, 6 PCB, 7 AUDIO/ARGB header 3 (cartes récentes) [V].
Zone absente : taille 0x1E (`POLYCHROME_USB_ZONE_UNAVAILABLE`) ou bit absent du masque de la table 0x01. Les zones 2, 3 et 7 sont **adressables**, taille réglable (jusqu'à **100 LED**,
`POLYCHROME_USB_ZONE_ADDRESSABLE_MAX`), les autres sont fixes (taille donnée par la table 0x02).

### Ordre des couleurs (RGSWAP)

Le firmware stocke un bit par zone « R/G inversés » (table 0x03, bit n = zone n : bit4 PCH, bit5 IO, bit6 PCB sont à 1 par défaut sur beaucoup de cartes). Octets de couleur : par défaut **G, R, B** ;
si le bit RGSWAP de la zone est mis : **R, G, B**. Ce bit peut être écrit via 0x15/0x03 ; le code le met à 0 pendant le mode direct car il « fait clignoter à chaque mise à jour » [V].

### Mode matériel par zone

`buf[1]=0x10, buf[3]=zone, buf[4]=mode, buf[5..7]=couleur (selon RGSWAP), buf[8]=vitesse, buf[9]=0xFF, buf[16]=allzone(0/1)` (le code écrit `buf[0x10]`, soit l'offset 16) puis lecture de l'accusé [V].
Modes : 0 off, 1 statique, 2 breathing, 3 strobe, 4 spectrum cycle, 5 random, 6 musique, 7 wave, 8 spring, 9 stack, 10 cram, 11 scan, 12 neon, 13 water, 14 rainbow, **15 (0x0F) = direct** (pseudo-mode « Chroma Connect »).
Vitesse : octet 0x00 (le plus rapide) .. 0xFF (le plus lent), défaut 0xE0. Persistance : le firmware conserve l'état ; commit = commande 0x12 (INCONNU : écrit-il en flash ?).

### Mode direct (flux par LED)

Une seule trame « flux » couvre TOUTES les zones, en deux types de paquets [V] :
1. Paquet initial : `buf[1]=0x10, buf[3]=0xFF, buf[4]=0xE3, buf[7]=0xFF`, `buf[64]=0x65` (dernier octet), puis **54 octets de couleur à partir de `buf[9]`** (18 LED).
2. Paquets suivants : `buf[1]=0x10, buf[3]=0xFF, buf[4]=0xE4, buf[64]=0x65`, **57 octets à partir de `buf[5]`** (19 LED).
Le flux est la concaténation des LED des zones actives dans l'ordre RGB1, RGB2, ARGB1, ARGB2, PCH, IO, PCB, AUDIO (3 octets/LED, ordre selon RGSWAP de chaque zone), complétée par des zéros jusqu'à `(Σ leds_max + 8)×3` octets.
Chaque paquet est suivi d'un `hid_read` d'accusé. Cadence : INCONNUE (non imposée dans le code ; à mesurer).

### Taille d'un ruban ARGB

`SetZoneSize` réécrit la table 0x02 entière (8 octets, 0x1E pour une zone absente) via commande 0x15, puis relit la configuration. L'écriture modifie la configuration du firmware (persistance probable → INCONNU : usure de flash).

## Protocole SMBus Polychrome (adresse 0x6A) [V]

Accès par blocs SMBus (`write_block_data(0x6A, registre, n, data)` / `read_block_data`). Pauses de ~1 ms entre écritures.

### V2 (firmware majeur 3)

Registres : 0x00 version (2 octets) ; **0x30 mode** (1 octet) ; **0x31 sélection LED/zone** (1 octet) ; **0x32 nombre de LED supplémentaires** (écriture de 0 octet = « une seule LED ») ; **0x33 configuration LED**
(lecture de **6 octets** : nombre de LED des zones RGB1, RGB2, zone3 audio/PCH, zone4, zone5 IO cover, adressable) ; **0x34 couleur** (3 octets R,G,B) ; **0x35 inversion du flux ARGB (GRB)** (0/1) [V].
Modes : 0x10 off, 0x11 statique, 0x12 breathing, 0x13 strobe, 0x14 spectrum cycle, 0x15 random, 0x17 wave, 0x18 spring, 0x19 stack, 0x1A cram, 0x1B scan, 0x1C neon, 0x1D water, 0x1E rainbow.
Vitesse : écrite (1 octet) dans le registre dont l'adresse est le **mode actif** (ex. `0x12` pour breathing) ; plages : breathing 0x02..0x0A, strobe 0x05..0xA0, spectrum 0x0A..0xA0, wave 0x01..0x06, spring/stack/cram 0x02..0x20.
Séquence de couleur : écrire 0x31 = n° LED/zone, puis 0x34 = R,G,B. Zones : 0 RGB1, 1 RGB2, 2 audio/PCH, 3 audio/PCH, 4 IO cover, 5 adressable (**100 LED max**).
Pas de mode « direct » par LED identifié en SMBus V2 (changement de mode + couleur par zone) ; débit faible.

### V1 (firmware majeur 2)

Registres : 0x30 mode ; 0x31 sélection de zone ; **0x32 « set all »** (1 = appliquer à toutes) ; **0x33 taille des zones** (lecture/écriture de 6 octets) ; **0x35 inversion ARGB**. La couleur+vitesse d'une zone
s'écrit dans le **registre du mode** (0x10..0x20) : bloc de **4 octets R,G,B,vitesse** (ou 3 octets / 1 octet vitesse selon le mode). Modes : 0x10 off, 0x11 statique, 0x12 breathing, 0x13 strobe, 0x14 spectrum,
0x15 random, 0x17 musique, 0x18 wave, 0x19 spring, 0x1A meteor, 0x1B stack, 0x1C cram, 0x1D scan, 0x1E neon, 0x1F water, 0x20 rainbow. Zones : 0 RGB1, 1 RGB2, 2 PCH, 3 IO cover, 4 audio, 5 adressable (100 LED max).

### « ASR RGB LED » (firmware majeur 1)

Registres : 0x30 mode, 0x31 sélection LED ; couleur+vitesse écrites dans le registre du mode en bloc de 4 (R,G,B,vitesse) ou 3 octets. Modes : 0x10 off, 0x11 statique, 0x12 breathing, 0x13 strobe, 0x14 spectrum,
0x15 random, 0x17 musique, 0x18 wave ; vitesse 0x00 (rapide) .. 0x05 (lent), défaut 0x03.

## Connecteurs : 12 V vs ARGB

- **12 V RGB** (RGB header 1/2) : couleur unique par header ; effets matériels (USB : zones 0/1 ; SMBus : zones 0/1) [V].
- **ARGB 5 V** (3 broches) : jusqu'à **100 LED** chacun (limite des deux protocoles), 2 headers (3 sur cartes récentes USB, zone 7) ; en SMBus, un seul header adressable (zone 5). Taille réglable via table 0x02 (USB) ou registre 0x33 (V1)/0x33 lecture (V2).
- Inversion G/R : bit RGSWAP par zone (USB), registre 0x35 pour le flux ARGB (SMBus) [V].

## Accès bas niveau, pilotes, BIOS, ACPI, WMI

- **USB** : HID standard, aucun pilote noyau spécifique. Pas de report ID.
- **SMBus** : bus SMBus du chipset (AMD FCH / Intel I801) → pilote noyau tiers (PawnIO/WinRing0). Le logiciel officiel utilise un pilote ASRock **[M]**, nom INCONNU.
- **BIOS** : option « RGB LED » / « Polychrome Sync » (Advanced > Onboard devices ?) active/désactive l'éclairage au démarrage et en veille **[M]**, valeurs exactes INCONNUES. Mode « off » BIOS peut couper le contrôleur.
- **ACPI/WMI** : aucune dépendance relevée dans le code de référence ; ASRock expose possiblement une classe WMI **[M]**, non analysée.
- DMI : nom de carte pour l'affichage uniquement (USB) ; sous-système PCI du contrôleur SMBus = 0x1849 pour la détection SMBus.

## Ventilateurs / pompes

Hors périmètre : le contrôleur Polychrome ne gère que l'éclairage. Les ventilateurs sont sur le Super I/O (Nuvoton NCT6796/NCT6798) ; non analysé ici.

## INCONNU / non confirmé

- Sémantique exacte de 0x12 (commit), 0xA4 (init) ; persistance des réglages (flash) et usure ; cadence du mode direct.
- Liste exacte des cartes USB et SMBus ; PID USB d'autres cartes.
- Offset exact `buf[0x10]` dans la commande 0x10 (champ `allzone` ; le code écrit à l'offset 16 – différent du reste du paquet).
- Protocole du logiciel officiel (non analysé), SDK, pilote noyau officiel.
- Contrôle de la luminosité globale (non vu).
- Fonctionnement du registre SMBus 0x32 (V2) « LED count ».

## Estimation de difficulté de réimplémentation

- **USB (26CE:01A2) : faible/moyenne** (paquets de 64 octets, accusés à lire, flux direct simple de 3 octets/LED, tables 0x14/0x15 pour la config). Risque : RGSWAP/taille de zones persistantes.
- SMBus V1/V2 : moyenne (pilote noyau + 3 firmwares, pas de mode direct rapide).

## Tests d'octets possibles sans matériel

1. Paquet 0x10 zone : vérifier `buf[1]=0x10`, `[3]=zone`, `[4]=mode`, ordre G,R,B vs R,G,B selon RGSWAP, `[8]=vitesse`, `[9]=0xFF`.
2. Flux direct : N LED → 1 paquet de 18 + ⌈(N-18)/19⌉ paquets de 19 ; `buf[4]` = 0xE3 puis 0xE4 ; `buf[64]` = 0x65 ; zéro-padding à `(Σ leds_max+8)×3`.
3. Lecture de la table 0x14/0x02 : parseur de 8 octets, 0x1E = zone absente.
4. Masque RGSWAP : bits `chnl8<<7 | pcb<<6 | io<<5 | pch<<4 | ahdr1<<3 | ahdr0<<2 | hdr1<<1 | hdr0`.
5. SMBus : encodage mode (0x11 + n), blocs 4 octets (R,G,B,vitesse) et lecture de version (majeur 1/2/3).
