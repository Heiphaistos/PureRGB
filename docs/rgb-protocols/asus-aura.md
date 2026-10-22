# ASUS Armoury Crate / Aura Sync / Aura Creator : protocoles RGB des cartes mères ASUS

Date : 2026-10-22. Légende : **[V]** lu dans une source de référence (voir ci-dessous), **[M]** de mémoire, NON vérifié, **INCONNU** non confirmé.
Aucun code ni asset copié : uniquement des faits de protocole.

## Source analysée (honnêteté)

- **Le logiciel officiel n'a PAS été analysé** (Armoury Crate = gros installeur + services + pilotes ASUS ; interdit d'installer, aucun installeur téléchargé).
- **Aura SDK public** : la page `https://www.asus.com/campaign/aura/us/SDK.php` (HTTP 200) pointe vers `https://www.asus.com/campaign/aura/download/AuraSDK.ZIP`
  qui répond **HTTP 404** (2026-10-22). SDK **non obtenu**. Il s'agit d'un SDK COM Windows (haut niveau), donc de toute façon peu utile pour un pilote natif.
- Les faits viennent de la **lecture du code OpenRGB** (GPL-2.0-or-later, instantané local `D:\Projet\Analyses\rgb\openrgb-src\`, commit non enregistré), issu de captures USB / SMBus
  d'Aura Sync. Fichiers : `Controllers/AsusAuraUSBController/AsusAuraUSBController/{AsusAuraUSBController,AsusAuraMainboardController,AsusAuraAddressableController}.{h,cpp}`,
  `RGBController_AsusAuraUSB.cpp`, `AsusAuraUSBControllerDetect.cpp`, `Controllers/ENESMBusController/{ENESMBusController.h,.cpp,ENESMBusControllerDetect.cpp}`,
  `ENESMBusInterface/ENESMBusInterface_i2c_smbus.cpp`, `pci_ids/pci_ids.h`.

## Matériel couvert

Deux générations pour les cartes mères : **USB (X570+ / Intel 400+)** et **SMBus ENE (cartes plus anciennes)**.

| VID:PID | Modèle / type | Interface HID | Remarque |
|---|---|---|---|
| 0B05:18F3 | Aura carte mère USB (gen 1) | défaut (rapports de 65 octets, ID 0xEC) | [V] |
| 0B05:1939 | Aura carte mère USB | idem | [V] |
| 0B05:19AF | Aura carte mère USB | **usage page 0xFF72, usage 0x00A1** | [V] |
| 0B05:1AA6 | Aura carte mère USB | idem 0xEC | [V] |
| 0B05:1BED | Aura carte mère USB (récent) | idem | [V] |
| 0B05:1867, 0B05:1872, 0B05:18A3, 0B05:18A5 | Aura « Addressable » : boîtier/contrôleur ARGB externe (sans LED fixes) | 0xEC | [V] |
| 0B05:1B29 | ROG Strix LC III (pompe AIO) : **même protocole que les cartes mères USB** | 0xEC | [V] |

Autres 0B05:xxxx du même détecteur (claviers, souris, moniteurs, ROG Ally 1ABE/1B4C, Strix LC 879E, Ryuo 1887, Ryujin 18AE, terminal 1889) : hors périmètre carte mère.

Détecteur SMBus : voir « Accès bas niveau » (ENE, pas de VID/PID USB).

## Protocole USB (cartes mères et contrôleurs « Addressable ») [V]

Transport : `hid_write` / `hid_read` de **65 octets** (octet 0 = report ID **0xEC** pour toutes les commandes), sur l'endpoint HID (rapports « output », pas feature).
Cadence : aucune temporisation imposée dans le code ; envoi à chaque mise à jour.

### Initialisation

1. Firmware : `EC 82 00...` → réponse : octet[1] == 0x02, chaîne ASCII de 16 octets en [2..17].
2. Table de configuration : `EC B0 00...` → réponse : octet[1] == 0x30, **60 octets de table dans [4..63]** (`config_table[0..59]`).
   Si octet[1] ≠ 0x30 → périphérique rejeté.
3. Carte mère (`AuraMainboardController`) : lit `config_table[0x1B]` = nombre total de LED fixes (carte), `config_table[0x1D]` = nombre de connecteurs 12 V RGB,
   `config_table[0x02]` = nombre de connecteurs ARGB. Si `leds_totales < headers_12V` alors headers_12V := 0. Crée : un canal « carte » (effet n°0, canal direct **0x04**, FIXED) puis un canal par
   connecteur ARGB (effet n°i+1, canal direct i, ADDRESSABLE, 1 LED par défaut, redimensionnable par l'utilisateur jusqu'à **500**, `AURA_ADDRESSABLE_MAX_LEDS`) [V].
4. Mode « Gen1 » : paquet `EC 52 53 00 01` (commande personnalisée) envoyé après l'init [V]. Sa sémantique exacte est INCONNUE (probablement bascule vers le mode de contrôle logiciel).

### Mode direct (couleurs temps réel)

Paquet `EC 40 <cmd> <offset> <n> <RGB × n>` [V] :
- octet[1] = **0x40** ;
- octet[2] = `cmd` = numéro de canal direct (0x04 carte, 0..N-1 ARGB) | **0x80** sur le DERNIER paquet d'une mise à jour (« apply ») | **0x10** si l'offset LED dépasse 255 (débordement 8 bits) ;
- octet[3] = offset LED de départ (modulo 256) ;
- octet[4] = nombre de LED dans ce paquet, **20 max** (`LEDS_PER_PACKET = 0x14`) ;
- octets[5..] = R,G,B par LED (3 octets). Ordre des canaux : R,G,B.
Un ruban de 500 LED = 25 paquets ; le dernier porte le bit 0x80.
Avant d'envoyer des couleurs directes, il faut avoir sélectionné le mode « direct » (0xFF) par la commande d'effet ci-dessous.

### Effets matériels

- Sélection d'effet (cartes mères) : `EC 35 <canal_effet> 00 <shutdown> <mode>` où `<shutdown>` = 1 pour écrire l'effet d'**extinction** (appliqué quand le PC s'éteint), 0 sinon [V].
- Couleur de l'effet : `EC 36 <mask_hi> <mask_lo> <shutdown> <RGB × n à partir de l'offset 5+3*start_led>` avec un **masque 16 bits** de LED (bit = LED du canal, `((1<<count)-1) << start_led`) [V].
  La couleur est écrite pour chaque LED du canal ; `start_led` = somme des LED des canaux précédents.
- **Commit** (mémoriser en flash) : `EC 3F 55` [V]. Les modes autres que direct sont marqués « sauvegarde manuelle » : changer l'effet puis envoyer `EC 3F 55`.
- Contrôleur « Addressable » (0x18xx) : effet `EC 3B <canal> 00 <mode> <R> <G> <B>` [V] ; direct identique (`EC 40`, canal 0x01 pour l'effet).
- Modes (octet `mode`) : 0 off, 1 static, 2 breathing, 3 flashing, 4 spectrum cycle, 5 rainbow, 6 spectrum breathing, 7 chase fade, 8 spectrum chase fade, 9 chase, 10 spectrum chase,
  11 spectrum wave, 12 chase rainbow pulse, 13 random flicker, 14 music, 16 gentle transition, 17 wave propagation, 18 wave propagation pause, 19 red pulse, **0xFF direct** [V].
  Vitesse/direction/luminosité par canal : INCONNUS dans ce contrôleur (le code carte mère n'envoie que mode+couleur).

### Connecteurs

- **RGB 12 V** (`num_rgb_headers`, 4 broches) : comptés dans les LED « carte » (zone fixe, `leds_count` = `config_table[0x1B]`), pas de séquence d'adressage séparée [V].
- **ARGB 5 V** (`config_table[0x02]` connecteurs) : 1 canal chacun, nombre de LED à régler par l'utilisateur (aucun réglage matériel n'est écrit : le contrôleur envoie simplement N LED) [V].
  Le nombre de LED stocké en flash du contrôleur (côté Armoury Crate) : INCONNU.

## Protocole SMBus ENE (cartes mères Aura anciennes, Intel 100-300, AMD 300/400, X299…) [V]

Registre-fichier 16 bits sur un bus SMBus/I2C :
- **Adresses esclave candidates carte mère : 0x40, 0x4E, 0x4F** (DRAM : 0x70-0x77, 0x4F, 0x66, 0x67, 0x39-0x3D ; GPU ASUS : 0x67 pour les cartes listées).
- Accès registre : `write_word_data(addr, 0x00, swap16(reg))` (octet de poids fort envoyé en premier) puis lecture `read_byte_data(addr, 0x81)` ou écriture `write_byte_data(addr, 0x01, val)`.
  Écriture en bloc : `write_block_data(addr, 0x03, n, data)`, repli sur `write_byte_data(addr, 0x01, ...)` octet par octet si le bloc échoue.
- Détection : lecture des registres 0xA0..0xAF qui doivent renvoyer 0x00..0x0F ; lecture de « Micron » à 0x1030 (si présent → c'est de la DRAM Micron, ignorer).
  Vérification subsystem PCI du bus SMBus = **0x1043 (ASUS)** ou 0 ou 0xFFFF.
- Bus SMBus visés (PCI) : AMD FCH 1022:790B, Intel ICH10 8086:3A30, Sunrise Point-H 8086:A123, Skylake-X PCU 8086:2085, etc. (macro `IF_MOBO_SMBUS`, ≥ 9 entrées Intel) [V].
- Registres : nom du firmware 16 octets @0x1000 (ex. « LED-0116 », « AUMA0-E6K5-0104/0105/0106 », « AUMA0-E8K4-0101 ») ; table de config 64 octets @0x1C00 ;
  couleurs directes @0x8000 (15 octets) ou **0x8100 (30 octets, v2)** ; couleurs d'effet @0x8010 / 0x8160 ; **direct** @0x8020 (1 = actif) ; **mode** @0x8021 ; **vitesse** @0x8022 ;
  **direction** @0x8023 ; **apply** @0x80A0 (1 = appliquer, **0xAA = sauvegarder en flash**) ; index d'emplacement RAM @0x80F8 ; adresse I2C RAM @0x80F9.
- **Ordre des octets couleur écrits : R, B, G** (`SetAllColorsDirect/Effect` échangent G et B) [V].
- Modes ENE : 0 off, 1 static, 2 breathing, 3 flashing, 4 spectrum cycle, 5 rainbow, 6 rainbow breathing, 7 chase fade, 8 chase fade rainbow, 9 chase, 10 chase rainbow, 11 wave,
  12 chase rainbow pulse, 13 random flicker, 14 double fade (certains firmwares). Vitesse : 0x00 (max) .. 0x04 (min). Direction 0/1.
- Zones (octet de canal dans la table de config @0x1C00 + offset 0x13 (v1) ou 0x1B (v2), une entrée par zone) : 0x82 centre (début), 0x83 centre, 0x84 audio, 0x85 back I/O, 0x86 header RGB,
  0x87 header RGB 2, 0x88 backplate, 0x8A DRAM, 0x8B PCIe, 0x91 header RGB 3 ; nombre de LED par zone = `table[0x03 + zone]`, total = `table[0x02]`. 8 zones max (`ENE_NUM_ZONES`).
- Cartes avec **header ARGB** en SMBus : seul le 0x91/0x86 type « header » est exposé ; le nombre de LED vient de la table. La séquence d'adressage des connecteurs ARGB v2 (30 octets directs → 10 LED par écriture) : INCONNUE au-delà.
- Variantes firmware (affectent offsets canal/LED count) : LED-0116 (v1), AUMA0-E6K5-0104/0105/0106 (v2), AUMA0-E8K4-0101 (v1), AUMA0-E6K5-0107/1107/1110/1111/1113/1114/0008 (GPU-like, v2) [V].

## Accès bas niveau, pilotes, BIOS, ACPI, WMI

- **USB HID** : aucun pilote noyau spécifique ; le périphérique est HID standard (interface sur usage page vendeur 0xFF72 pour 19AF) [V].
- **SMBus** : accès au contrôleur SMBus du chipset (PCI AMD FCH 1022:790B ; Intel 8086:3A30 / A123 / 2085 et autres de la macro `IF_MOBO_SMBUS`) via un pilote noyau (PawnIO, WinRing0 ou équivalent). L'application officielle utilise son pilote ASUS
  (nom `AsIO3.sys` / service « Asusgsvc », « AsusCertService ») **[M]** : non vérifié, non nécessaire.
- Sur les cartes USB, une routine ACPI/BIOS peut aussi piloter les LED (« Aura Lighting Effect »/« LED in sleep, off »). Les réglages BIOS « RGB LED lighting when system is in Sleep/Hibernate/Soft-off »
  décident si l'effet d'extinction (`EC 35 ... shutdown=1`) est appliqué **[M]**. Aucune dépendance WMI relevée dans le code de référence ; Armoury Crate utilise ACPI/WMI (`ASUSATK`, `ATKACPI`) pour
  d'autres fonctions (ventilateurs/mode) **[M]** : non analysé.
- DMI : le nom de carte (SMBIOS type 2) donne le nom du périphérique ; **pas** de liste blanche de cartes pour l'USB (la table de config 0xB0 décrit le matériel) [V].

## Ventilateurs / pompes

- Aura USB : aucune commande ventilateur dans ce contrôleur [V]. La pompe ROG Strix LC III (0B05:1B29) réutilise seulement la partie éclairage ; contrôle pompe/ventilos de l'AIO : INCONNU.
- Ventilateurs de la carte : gérés par le Super I/O (Nuvoton NCT67xx / ITE) et l'EC ASUS, via Armoury Crate / Fan Xpert ; non couvert ici.

## INCONNU / non confirmé

- Sémantique exacte de `EC 52 53 00 01` (SetGen1) et de tout octet de réponse au-delà de [4..63].
- Luminosité/vitesse/direction des effets matériels sur les cartes USB (aucune commande identifiée dans le code lu).
- Format de la table de config 0xB0 hors octets 0x02, 0x1B, 0x1D.
- Persistance du nombre de LED ARGB dans le contrôleur.
- Aura SDK (COM) : non obtenu (404).
- Signature de la pompe LC III pour son propre écran/ventilos.

## Estimation de difficulté de réimplémentation

- **USB (0B05:xxxx)** : **faible** (paquets de 65 octets, direct = 20 LED/paquet, table de config auto-descriptive, aucun pilote). Le plus simple de ce dossier avec ASRock USB.
- **SMBus ENE** : moyenne/haute (pilote noyau de bus SMBus, détection prudente car risque de corruption de SPD/PMIC DRAM si mal adressé ; variantes firmware).

## Tests d'octets possibles sans matériel

1. Construire les paquets : `EC 82`, `EC B0`, `EC 3F 55`, `EC 35 ch 00 sd mode`, `EC 36 mask_hi mask_lo sd ...`, `EC 40 cmd off n ...` et comparer longueurs (65) et champs.
2. Direct : 45 LED → paquets de 20+20+5, le 3e avec cmd | 0x80 ; 300 LED → bit 0x10 après le 256e.
3. Masque d'effet : `((1<<n)-1) << start` pour n ≤ 16 (attention : champ 16 bits → plafond 16 LED par canal pour l'effet couleur).
4. Parseur de table de config : fournir un tableau de 60 octets synthétique (0x02=2, 0x1B=8, 0x1D=2) et vérifier la création de 1+2 canaux.
5. ENE : permutation 16 bits du registre (`0x8020` → octets `20 80`) et ordre R,B,G.
