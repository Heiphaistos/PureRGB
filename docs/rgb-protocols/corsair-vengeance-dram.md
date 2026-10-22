# Corsair Vengeance RGB / Dominator (iCUE) : RAM via SMBus

Voir d'abord `smbus-i2c-acces-et-securite.md` (bus, pilotes, règles d'écriture).

## Source analysée

- Logiciel officiel : **iCUE** (Corsair). **Non téléchargé ni extrait** dans cette passe. Le pilote noyau/le chemin d'accès SMBus d'iCUE sont `INCONNU (non vérifié)`.
- Faits de protocole : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`), fichiers
  `Controllers/CorsairVengeanceController/*` (famille V1) et `Controllers/CorsairDRAMController/*` (famille moderne).
  Deux protocoles totalement différents existent sous le même nom commercial.

## Matériel couvert

### Famille 1 : « Vengeance RGB » d'origine (DDR4, 1 zone)

| Élément | Valeur |
|---|---|
| Mémoire | DDR4 uniquement, JEDEC fabricant **0x021D** (Corsair) |
| Adresses 7 bits | `slot_SPD + 8` soit **0x58..0x5F**, et `slot_SPD - 0x40 + 8` soit **0x18..0x1F** (2 plages testées pour chaque slot SPD occupé) |
| LEDs | 1 couleur par barrette (`led_count = 1`) |

### Famille 2 : Vengeance RGB Pro, Dominator Platinum, Pro SL, RS, DDR5, Titanium, Shugo

| Modèle (nom OpenRGB) | PIDs internes (lus dans le contrôleur) | LEDs | Ordre inversé |
|---|---|---|---|
| Vengeance RGB Pro DDR4 | 0x0100, 0x0101 | 10 | non |
| Dominator Platinum RGB DDR4 | 0x0200, 0x0201 | 12 | oui |
| Vengeance RGB Pro SL DDR4 | 0x0300, 0x0301 | 10 | non |
| Vengeance RGB RS DDR4 | 0x0400, 0x0401 | 6 | non |
| Dominator Platinum RGB DDR5 | 0x0600, 0x0601 | 12 | oui |
| Dominator Titanium RGB DDR5 | 0x0800, 0x0801, 0x0810, 0x0811 | 12 | oui |
| Vengeance RGB DDR5 | 0x0700, 0x0701, 0x0900, 0x0901, 0x0910, 0x0911 | 10 | non |
| Vengeance Shugo Series DDR5 | 0x0A00, 0x0A01, 0x0A10, 0x0A11 | 10 | non |
| Vengeance RGB RS DDR5 | 0x0B00, 0x0B01 | 6 | non |

Ces « PID » ne sont PAS des identifiants USB : ils sont lus dans le bloc d'information du contrôleur (voir ci-dessous). Source : `CorsairDRAMDevices.h/.cpp`.
Adresses balayées : **0x58..0x5F puis 0x18..0x1F** (16 adresses), pause 10 ms entre chaque (`CorsairDRAMControllerDetect.cpp`).

## Détection sûre

### Famille 1 (V1)

1. Pré-requis : module DDR4 de JEDEC 0x021D détecté dans les slots SPD (voir fiche commune).
2. Pour chaque slot, adresse candidate = `adresse_SPD + 8` ; puis `adresse_SPD - 0x40 + 8`.
3. Test d'OpenRGB : `write_quick` (écriture de 0 octet) pour voir si l'adresse répond, puis lire les registres **0xA0..0xAF** : tous doivent valoir **0xBA**. Sinon refus.
4. Recommandation PureRGB : remplacer le `write_quick` par une lecture d'octet (`read_byte_data` sur 0xA0) ; si 0xBA, lire le reste. Aucune écriture avant validation.

### Famille 2 (moderne)

1. Pas de filtre JEDEC dans la source : détection par adresse seule (0x58..0x5F, 0x18..0x1F). **Risque** : lire des registres sur 0x18..0x1F tombe sur les capteurs de température DDR4 : la lecture est sans danger, mais n'y écrire jamais.
2. Test : lire le registre **0x43** : doit valoir 0x1A, 0x1B ou 0x1C ; lire le registre **0x44** : doit valoir 0x01, 0x03 ou 0x04. Toute autre valeur : abandon.
3. Lecture du bloc d'information (32 octets) : écrire `0x00` dans le registre `0x61` (sélection « device info »), écrire `0x00` dans `0x21` (début de transfert), puis lire 32 fois le registre `0x40` ; lire le CRC8 attendu dans `0x42` et le comparer au CRC8 calculé.
   - octets 0-1 : VID little-endian ; octets 2-3 : PID little-endian (table ci-dessus) ; octets 8-11 : version du firmware (majeur = octet 9, mineur = octet 8, build = octets 10-11 LE) ; octet 28 : **version du protocole**.
   - Le mode direct n'est autorisé que si protocole >= 4.

## Protocole : famille 1 (V1)

Registres (adresse 7 bits ci-dessus, écritures `write_byte_data`) :

| Registre | Rôle |
|---|---|
| 0xA4 | durée de fondu (0 = statique) |
| 0xA5 | durée de maintien |
| 0xA6 | mode : 0 = couleur unique, 1 = fondu, 2 = pulsation |
| 0xB0 / 0xB1 / 0xB2 | rouge / vert / bleu |

Séquence de couleur fixe : écrire 0xA4 = 0, puis 0xB0, 0xB1, 0xB2 (R, G, B), puis 0xA6 = 0. Les autres modes (fondu/pulsation) : INCONNU dans le détail (séquence de couleurs non lue ici).

## Protocole : famille 2 (moderne)

### Registres (commande SMBus = adresse de registre)

| Reg | Rôle |
|---|---|
| 0x0B | remise à zéro du tampon (écrire 0x00) |
| 0x20 | écriture d'un octet dans le tampon binaire actif |
| 0x21 | début de transfert binaire (écrire 0x00) |
| 0x23 | bascule Bootloader (0x00) / Normal (0x01) : **ne jamais écrire (risque de brique)** |
| 0x26 | sélection du tampon de configuration à écrire par ID |
| 0x30 | statut : bit 3 (0x08) = occupé ; prêt quand le bit est à 0 |
| 0x31 / 0x32 | tampon couleur directe, blocs 1 et 2 (écriture bloc SMBus) |
| 0x40 | lecture d'un octet du tampon actif |
| 0x41 | occupé si non nul (indication source) |
| 0x42 | CRC8 du tampon actif |
| 0x43 / 0x44 | signature de détection (voir plus haut) |
| 0x61 | sélection du bloc info appareil |
| 0x63 | sélection d'un tampon de configuration à lire |
| 0x82 | écriture/application de la configuration en écrivant l'ID du tampon : 0 = liste de commandes, 1 = configuration d'effet, 2 = données couleur |

**CRC8** : paramètres CRC++ « CRC_8 » = polynôme 0x07, valeur initiale 0x00, XOR final 0x00, sans réflexion (`dependencies/CRCpp/CRC.h`).

### Mode direct (protocole >= 4), le plus utile pour PureRGB

Paquet de `n = 3 x nbLED + 2` octets : octet 0 = **nombre de LEDs** (la source écrit le commentaire « 0x0A ou 0x0C », soit 10 ou 12), octets 1.. = R,G,B par LED
(ordre inversé pour Dominator : LED i reçoit la couleur `nbLED-1-i`), dernier octet = CRC8 de tous les précédents.
Envoi : bloc SMBus (write_block_data) de 32 octets maximum sur le registre `0x31`, le reste sur `0x32`.
Repli si les blocs SMBus échouent (certains pilotes) : écritures en mots, avec des octets de données dans le quartet bas de l'octet de commande, à partir de `0x90` (premier paquet) puis `0xA0` (suivants) ; détail dans `CorsairDRAMController.cpp::SetColorsPerLED` (INCONNU ici pour le plan exact de découpage : relire la source avant d'implémenter).
Cadence : non spécifiée dans la source ; le contrôleur est lent, prévoir 10 à 20 ms entre trames et ne pas dépasser ~30 Hz (recommandation, non mesurée).

### Mode « tampon de couleurs » (protocole < 4)

Données = `nbLED x 4` octets `[R, G, B, 0xFF]` (ordre inversé selon le modèle). Séquence : `0x0B = 0`, `0x21 = 0`, un octet à la fois dans `0x20`, relire `0x42` et comparer au CRC8 local, **seulement si identique** écrire `0x82 = 2`, puis attendre le bit « prêt » du registre `0x30` (jusqu'à 5 essais de 10 ms).

### Effets matériels

Paquet de 20 octets envoyé de la même manière (0x0B, 0x21, 20 x 0x20, vérif CRC, `0x82 = 1`) :

| Octet | Contenu |
|---|---|
| 0 | mode |
| 1 | vitesse (0 lent, 1 moyen, 2 rapide) |
| 2 | 0 = couleurs aléatoires, 1 = couleurs personnalisées |
| 3 | direction (0 haut, 1 bas, 2 gauche, 3 droite) |
| 4-6 | couleur 1 (R,G,B) |
| 7 | luminosité |
| 8-10 | couleur 2 (R,G,B) |
| 11 | luminosité |
| 12-19 | 0 |

Modes : 0x00 Color Shift, 0x01 Color Pulse, 0x03 Rainbow Wave, 0x04 Color Wave, 0x05 Visor, 0x06 Rain, 0x07 Marquee, 0x08 Rainbow, 0x09 Sequential, 0x10 Static.

## Ventilateurs / pompes

Sans objet (RAM). Aucune sonde de température dans ce protocole. (Le capteur de température de la barrette est un composant JEDEC séparé aux adresses 0x18..0x1F ou via le hub DDR5 : ne pas l'écrire.)

## Accès bas niveau

- SMBus via PawnIO (voir fiche commune) ; mutex `Global\Access_SMBUS.HTP.Method`.
- iCUE lui-même : mécanisme d'accès SMBus (pilote noyau Corsair, service, ou autre) `INCONNU (non vérifié)`.
- iCUE et PureRGB ne doivent pas piloter la RAM en même temps (le contrôleur est à état : mode direct vs effet).

## Ce qui est INCONNU

- Fonctionnement des effets V1 au-delà de « couleur unique » (fondu/pulsation).
- Séquence de lecture des couleurs/effets en cours (tampons 0x63).
- Ce qui se passe à la mise sous tension : l'effet matériel actuel est-il persisté ? (le mode direct ne persiste pas selon la logique du code, non vérifié matériellement).
- Comment iCUE détecte et pilote réellement la RAM (aucun installeur extrait).
- Le rôle exact de `0x26`, `0x41`, `0x82` avec ID 0.
- Écriture sécurisée du firmware : **hors périmètre** (ne jamais implémenter 0x23).

## Estimation de difficulté

- V1 : **faible** (4 registres), mais sonde `write_quick` à remplacer.
- Moderne direct : **moyenne** (CRC8 + blocs 32 octets + repli mots). Risque : surtout l'accès SMBus (voir fiche commune).

## Tests d'octets possibles sans matériel

1. CRC8 (poly 0x07, init 0) : vecteur de contrôle classique `"123456789"` -> **0xF4**. À reproduire en test unitaire.
2. Construire le paquet direct pour 10 LEDs tout rouge : `[0x0A, 255,0,0 x10, CRC]` (32 octets au total : un bloc), pour 12 LEDs : 38 octets (bloc 32 + reste 6 sur le registre 0x32).
3. Vérifier le découpage : n=38 -> bloc1 = octets 0..31, bloc2 = octets 32..37.
4. Vérifier l'inversion pour Dominator (LED0 reçoit la couleur 11).
5. Paquet d'effet 20 octets : mode 0x10 (statique), couleur 1 = FF0000, luminosité 0x64 : calculer le CRC8 et comparer avec le comportement attendu du contrôleur (valider sur matériel ensuite).
