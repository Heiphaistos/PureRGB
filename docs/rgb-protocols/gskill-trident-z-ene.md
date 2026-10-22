# G.Skill Trident Z RGB (Trident Z Lighting Control) et contrôleur ENE SMBus (ASUS Aura DRAM, T-Force Delta, Geil...)

Voir d'abord `smbus-i2c-acces-et-securite.md` (bus, pilotes, règles d'écriture).

## Source analysée

- Logiciel officiel : **G.Skill Trident Z Lighting Control**. **Non téléchargé ni extrait** dans cette passe ; ses pilotes/méthode d'accès SMBus sont `INCONNU (non vérifié)`.
- Faits de protocole : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`), `Controllers/ENESMBusController/*`.
  C'est la puce **ENE** (la même famille que l'Aura de la carte mère ASUS) qui pilote les LED des barrettes G.Skill Trident Z RGB (commentaire de la source :
  « DIMM_LED-0102, première génération de contrôleur DRAM, Trident Z RGB »). Le même protocole sert aussi à des cartes mères ASUS, des GPU ASUS et un SSD XPG.

## Matériel couvert (identifié par la chaîne de version lue dans la puce)

Pas de VID/PID USB : l'identification se fait par la chaîne ASCII de 16 octets lue aux registres `0x1000..0x100F` (`ENE_REG_DEVICE_NAME`).

| Chaîne de version | Rôle (commentaire source) | Registres couleur directe / effet | Table de canaux |
|---|---|---|---|
| `DIMM_LED-0102` | DRAM génération 1 (Trident Z RGB) | 0x8000 / 0x8010 (V1) | offset 0x13 |
| `DIMM_LED-0103` | DRAM génération 1 (T-Force Delta RGB) | V1 | 0x13 |
| `AUDA0-E6K5-0101` | DRAM génération 2 (Geil Super Luce) | 0x8100 / 0x8160 (V2) | 0x13 ; mode 14 supporté si un canal vaut 0x0E |
| `LED-0116` | carte mère génération 1 | V1 | 0x13 |
| `AUMA0-E6K5-0104/0105/0106` | carte mère génération 2 | V2 | 0x1B |
| `AUMA0-E8K4-0101` | carte mère génération 1 (E8K4) | V1 | 0x13 |
| `AUMA0-E6K5-0107`, `-1107`, `-1110`, `-1111`, `-0008`, `-1113`, `-1114` | GPU ASUS (3000, 4000, 5000, ROG Matrix...) | V2 | 0x1B (0x13 pour -0008) |
| `ROG STRIX ARION` | boîtier SSD externe ASUS (accès SCSI, pas SMBus) | V2 | table fixe |

**Toute autre chaîne de version -> 0 LED -> le contrôleur n'est pas enregistré** (protection voulue : « si la lecture échoue, ne pas écrire »). PureRGB doit faire de même.
G.Skill Trident Z5 RGB / Royal et XPG DDR4/DDR5 (ADATA 0x044A) : la chaîne de version réelle n'apparaît pas dans cette source -> `INCONNU` (non attribué à une chaîne ci-dessus sans preuve).

## Adresses I2C (7 bits)

- **Adresse de départ de toutes les barrettes ENE : 0x77** (avant remappage).
- **Après remappage** (une adresse par slot) : liste ordonnée `0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x4F, 0x66, 0x67, 0x39, 0x3A, 0x3B, 0x3C, 0x3D` (`ene_ram_addresses`).
- Cartes mères ASUS Aura : `0x40, 0x4E, 0x4F`. GPU ASUS (ENE) : 0x67 (voir `gpu-controleurs-par-constructeur.md`). SSD XPG Spectrix S40G : 0x67 via NVMe (SCSI).
- Toute adresse est le décalage du bus **SMBus principal (chipset)**, pas le GPU.

## Détection sûre

### Test de signature d'un contrôleur ENE à une adresse (`TestForENESMBusController`)

1. Lecture `read_byte` ; si échec, `read_byte_data(0x00)`. Pas de réponse -> pas de contrôleur.
2. **Motif de signature** : lire les registres directs `0xA0..0xAF` (16 octets, accès SMBus direct, pas via le pointeur ENE) : la valeur lue doit être `reg - 0xA0` (donc 00, 01, 02 ... 0F). Sinon -> pas un ENE.
3. Lire la chaîne `0x1030..` (`ENE_REG_MICRON_CHECK`) ; si elle vaut `"Micron"` -> c'est un contrôleur **Crucial/Micron** (autre famille, voir `ram-xpg-teamgroup-patriot-crucial.md`) -> le tester ailleurs, pas comme ENE.
4. Lire la chaîne de version (16 octets à `0x1000`) puis la table de configuration de 64 octets à `0x1C00`.
   Le nombre de LEDs est `config_table[0x02]` (certaines révisions : `config_table[0x03]`). Chaîne inconnue -> 0 LED -> refus.

### Remappage des adresses de 0x77 vers les adresses libres (étape dangereuse)

Code OpenRGB (DRAM, bus `IF_DRAM_SMBUS` uniquement) : pour `slot = 0..7` :
1. `read_byte(0x77)` : si échec -> plus aucune barrette ENE à remapper, on arrête.
2. Choisir la prochaine adresse de la liste ci-dessus pour laquelle `read_byte` **échoue** (adresse libre).
3. Écrire (via le protocole ENE ci-dessous) à l'adresse 0x77 : registre `0x80F8` = numéro de slot ; registre `0x80F9` = `adresse_choisie << 1` (octet à 8 bits).
La barrette quitte alors 0x77 et répond à sa nouvelle adresse ; la suivante apparaît à 0x77.

**RISQUE** : dans la source, l'étape 1 est un simple « ack » sur 0x77, **sans test de signature** avant d'écrire `0x80F8/0x80F9`. Si un autre composant (hub, capteur) répond à 0x77, on lui écrit à l'aveugle.
**Obligation PureRGB** : exécuter le test de signature (étapes 1 à 3 ci-dessus) sur 0x77 AVANT toute écriture de remappage (c'est ce que fait TForce Xtreem, voir `ram-xpg-teamgroup-patriot-crucial.md`), et seulement si au moins une barrette SPD de JEDEC attendu (G.Skill 0x044C, TeamGroup 0x046E, ADATA 0x044A...) est détectée.
Nombre de remappages = nombre de barrettes ENE présentes, jamais plus de 8.
Le remappage n'est pas confirmé persistant (suppose volatil, comme au cold boot les barrettes reviennent à 0x77) : `INCONNU`, à vérifier sur matériel.

## Protocole d'accès aux registres ENE (SMBus)

Les registres ENE font 16 bits ; on y accède par un pointeur :
- **Sélection** : `write_word_data(adresse, commande 0x00, mot = registre avec octets inversés)` c.-à-d. `((reg << 8) & 0xFF00) | (reg >> 8)`.
- **Lecture** : après la sélection, `read_byte_data(adresse, 0x81)`.
- **Écriture d'un octet** : après la sélection, `write_byte_data(adresse, 0x01, valeur)`.
- **Écriture par bloc** : après la sélection, `write_block_data(adresse, 0x03, n, données)` avec n <= 3 (`GetMaxBlock = 3`) ; si le bloc échoue, repli en `write_byte_data(0x01, ...)` octet par octet.
  Le pointeur s'auto-incrémente (permet d'écrire 3 octets = 1 LED à la suite).

### Registres

| Registre | Nom | Rôle |
|---|---|---|
| 0x1000 | DEVICE_NAME | chaîne de version (16 octets) |
| 0x1030 | MICRON_CHECK | chaîne « Micron » si Crucial |
| 0x1C00 | CONFIG_TABLE | 64 octets : LED count (octet 2 ou 3), canaux (octet 0x13 V1 ou 0x1B V2 + zone) |
| 0x8000 | COLORS_DIRECT (V1) | 15 octets = 5 LEDs |
| 0x8010 | COLORS_EFFECT (V1) | 15 octets |
| 0x8020 | DIRECT | 1 = mode direct, 0 = effets matériels |
| 0x8021 | MODE | numéro de mode |
| 0x8022 | SPEED | 0 le plus rapide .. 4 le plus lent |
| 0x8023 | DIRECTION | 0 avant, 1 arrière |
| 0x80A0 | APPLY | 0x01 = appliquer ; **0xAA = enregistrer en mémoire non volatile** |
| 0x80F8 | SLOT_INDEX | (RAM) numéro de slot pour le remappage |
| 0x80F9 | I2C_ADDRESS | (RAM) nouvelle adresse << 1 |
| 0x8100 | COLORS_DIRECT_V2 | 30 octets = 10 LEDs |
| 0x8160 | COLORS_EFFECT_V2 | 30 octets |

**Ordre des octets couleur : R, B, G** (et non R, G, B) : la source envoie `{rouge, bleu, vert}` pour chaque LED. Ne pas le « corriger ».
Canaux de la table (valeurs de l'octet de canal) : 0x05/0x0E/0x8A = DRAM, 0x82/0x83 = centre, 0x84 audio, 0x85 I/O arrière, 0x86/0x87/0x91 en-têtes RGB, 0x88 backplate, 0x8B PCIe.

### Modes matériels (registre 0x8021)

0 off, 1 statique, 2 respiration, 3 clignotement, 4 cycle spectre, 5 arc-en-ciel, 6 cycle respiration, 7 chase fade, 8 chase fade arc-en-ciel, 9 chase,
10 chase arc-en-ciel, 11 vague, 12 chase pulse arc-en-ciel, 13 scintillement aléatoire, 14 double fondu (seulement si `supports_mode_14`, DRAM `AUDA0-E6K5-0101`).

### Séquences

- **Couleur directe** : `0x8020 = 1`, `0x80A0 = 1`, puis écrire les couleurs à `0x8000` (V1) ou `0x8100` (V2) par blocs de 3 octets (1 LED par bloc), sans `APPLY` après chaque LED (la source écrit puis laisse le contrôleur rafraîchir).
- **Effet matériel** : `0x8020 = 0`, écrire les couleurs d'effet à `0x8010`/`0x8160` puis `APPLY = 1`, puis `MODE`, `SPEED`, `DIRECTION`, `APPLY = 1`.
- **Sauvegarde** : `0x80A0 = 0xAA` uniquement sur demande explicite (usure de la mémoire du contrôleur).
- Cadence : non spécifiée ; chaque octet = 2 transactions SMBus (pointeur + donnée) ; prévoir quelques dizaines de ms par barrette en mise à jour complète.

## Ventilateurs / pompes

Sans objet (le contrôleur ENE carte mère ne pilote pas de ventilateurs dans ces registres).

## Accès bas niveau

PawnIO + `Global\Access_SMBUS.HTP.Method` (voir fiche commune). Trident Z Lighting Control / Aura : pilote `INCONNU (non vérifié)`.
Conflit connu : si Armoury Crate / Aura / un autre outil tient le contrôleur, il re-remappe ou revient en effet matériel.

## Ce qui est INCONNU

- Chaînes de version des Trident Z5 RGB, Trident Z Royal, XPG Spectrix D41/D50/D35G (ADATA 0x044A) : non attribuées à l'ENE sur preuve dans cette source.
- Persistance réelle du remappage, effets de `0x80A0 = 0xAA` (durée de vie), temporisations minimales.
- Mapping physique des LEDs pour chaque barrette (nombre de LED lu en config table, ordre haut/bas).
- Fonctionnement de Trident Z Lighting Control (non extrait).

## Estimation de difficulté

**Moyenne** : protocole simple et très documenté, mais l'étape de remappage touche le bus partagé : tout l'effort de réimplémentation doit aller dans la détection sûre (signature d'abord, ensuite seulement écriture).

## Tests d'octets possibles sans matériel

1. Mot de sélection de registre : `reg = 0x8021` -> mot SMBus `0x2180` ; `reg = 0x80F9` -> `0xF980` ; `reg = 0x1000` -> `0x0010`.
2. Paquet couleur LED (rouge pur) : octets envoyés `FF 00 00` ; (bleu pur) `00 FF 00` car l'ordre est R,B,G ; (vert pur) `00 00 FF`.
3. Adresse écrite en `0x80F9` : pour la cible 0x70 -> `0xE0` ; 0x4F -> `0x9E` ; 0x39 -> `0x72`.
4. Rejeter une chaîne de version inconnue (liste ci-dessus) -> 0 LED.
5. Signature : un faux bus qui répond `reg - 0xA0` sur 0xA0..0xAF doit passer ; tout autre motif doit refuser avant toute écriture.
