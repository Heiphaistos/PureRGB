# Kingston FURY CTRL : Beast / Renegade RGB (DDR4, DDR5) et HyperX Fury/Predator RGB (DDR4) via SMBus

Voir d'abord `smbus-i2c-acces-et-securite.md` (bus, pilotes, règles d'écriture).

## Source analysée

- Logiciel officiel : **Kingston FURY CTRL** (ex HyperX NGENUITY pour la RAM). **Non téléchargé ni extrait** dans cette passe ; pilote/méthode d'accès `INCONNU (non vérifié)`.
- Faits de protocole : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`) :
  `Controllers/KingstonFuryDRAMController/*` (FURY CTRL, DDR4 et DDR5) et `Controllers/HyperXDRAMController/*` (HyperX Fury/Predator RGB DDR4, protocole plus ancien).
  Plusieurs commentaires de la source indiquent des valeurs « prises du logiciel Kingston » (vitesses, décalages de pluie/feux d'artifice), donc observées sur FURY CTRL.

## Matériel couvert

| Famille | Mémoire | JEDEC fabricant | Adresses 7 bits | LEDs / barrette |
|---|---|---|---|---|
| FURY Beast / Beast White DDR4 | DDR4 | Kingston 0x0117 ou 0x300F | **0x58 + index_slot** (0x58..0x5F) | 10 |
| FURY Beast / Renegade / Beast RGB White / Beast2 DDR5 | DDR5 | Kingston 0x0117, 0x300F ou 0x3011 | **0x60 + index_slot** (0x60..0x67) | 12 |
| HyperX Fury RGB / Predator RGB DDR4 | DDR4 | Kingston 0x0117 | une adresse commune **0x27** pour toutes les barrettes | 5 (voir ci-dessous) |

`index_slot` = numéro du slot SPD (adresse SPD - 0x50), 0 à 7. Un seul contrôleur logique regroupe toutes les barrettes FURY détectées sur le bus.

## Détection sûre (FURY CTRL)

Pré-conditions strictes : bus SMBus de la liste blanche, DIMM de JEDEC Kingston dans le slot, type mémoire correspondant (DDR4 ou DDR5).

### DDR5 : reconnaissance par le SPD (sans toucher au contrôleur RGB)

Les octets « données fabricant » du SPD DDR5 commencent à l'offset `0x22B` ; la famille RGB est identifiée par deux octets de ce bloc, aux index `0x18` (variante) et `0x19` (modèle), c.-à-d. SPD `0x243` et `0x244` :

| Variante (octet 0x243) | Modèles acceptés (octet 0x244) |
|---|---|
| 0x00 = Renegade | 0x11, 0x13, 0x15 |
| 0x01 = Beast | 0x10, 0x13, 0x15 |
| 0x02 = Renegade White ou 0x03 = Beast White | 0x12, 0x13, 0x15 |

Les variantes « Limited Edition » (0x14) et « v2 » (0x20) utilisent d'autres API d'effets et ne sont **pas** gérées. Si le SPD identifie la barrette, la signature « FURY » n'est pas relue.

### Signature dans l'espace du contrôleur (obligatoire pour DDR4, repli DDR5)

Séquence faite sur chaque adresse candidate `base + index_slot` (DDR4 base 0x58, DDR5 base 0x60) :
1. **Début de transaction** : `write_byte_data(adresse, reg 0x08, 0x53)`. Si cette écriture échoue -> barrette absente (erreur), 3 essais avec pause de 10 ms.
2. Lire en mots (`read_word_data`) les registres 1, 2, 3, 4 : l'**octet de poids fort** de chaque mot doit valoir `'F'`, `'U'`, `'R'`, `'Y'` (0x46, 0x55, 0x52, 0x59) ; un mot invalide (<0 ou 0xFFFF) est retenté 3 fois.
3. Lire en mot le registre `0x06` (modèle) : octet de poids fort = **0x21** (Beast White DDR4), **0x23** (Beast DDR4) ; DDR5 : 0x10, 0x11, 0x12, 0x15.
4. **Fin de transaction** : `write_byte_data(adresse, reg 0x08, 0x44)`.
Délai de 10 ms entre chaque accès.

**Sécurité** : l'écriture `0x08 = 0x53` est la seule « écriture de sonde » ; elle n'est tolérable que parce qu'elle est précédée du filtre JEDEC + type mémoire (+ variante SPD en DDR5). Ne jamais l'envoyer sur une adresse sans ce filtre. Terminer systématiquement par `0x08 = 0x44`.

## Protocole FURY CTRL (DDR4 / DDR5)

Adresses : `base + index_slot` pour chaque barrette ; toutes les écritures sont `write_byte_data(adresse, registre, valeur)` (5 essais, attente 3 x tentative x 10 ms en cas d'échec) ; les lectures renvoient la valeur dans l'octet de poids fort d'un `read_word_data`.
Pause de 10 ms après une série d'écritures. Cache de registres dans OpenRGB : n'écrit pas si la valeur est inchangée.

### Registres

| Reg | Rôle |
|---|---|
| 0x06 | modèle (lecture) |
| 0x08 | APPLY : 0x53 = début de transaction, 0x44 = fin / application |
| 0x09 | mode matériel |
| 0x0B | index physique du slot (0..3 ; au-delà de 4 barrettes les 4 premières et 4 dernières ont une numérotation séparée) |
| 0x0C | direction : 1 bas->haut, 2 haut->bas |
| 0x0D | délai |
| 0x0E | vitesse |
| 0x12-0x15 | mode « Dynamic » : attentes/fondus A et B |
| 0x16-0x1D | mode « Breath » : temps entre niveaux, maintien, luminosités max (100), moyenne (64), min (0) |
| 0x20 | luminosité globale |
| 0x23 / 0x24 / 0x25 | couleur de fond R / G / B |
| 0x26 | longueur d'effet |
| 0x27 | nombre de slots (écrire min(n, 4)) |
| 0x30 | nombre de couleurs du mode (max 10) |
| 0x31.. | couleurs du mode, trois registres par couleur : R = 0x31 + 3i, G = 0x32 + 3i, B = 0x33 + 3i |
| 0x50.. | **couleurs par LED (mode direct)** : R = 0x50 + 3i, G = 0x51 + 3i, B = 0x52 + 3i, pour i = 0..9 (DDR4) ou 0..11 (DDR5) |

Ordre des couleurs : R, G, B (pas de permutation). Luminosité en pourcentage (0-100, défaut 80 dans OpenRGB).

### Modes (valeur du registre 0x09)

0x00 Static ; 0x01 Rainbow ; 0x101 (même valeur 0x01 + variante) Spectrum ; 0x02 Rhythm ; 0x03 Breath ; 0x04 Dynamic ; 0x05 Slide, Slither, Teleport, Wind (variantes distinguées côté logiciel) ;
0x06 Comet, Rain, Firework ; 0x07 Voltage ; 0x08 Countdown ; 0x09 Flame ; 0x0A Twilight ; 0x0B Fury ; **0x10 Direct** ; 0x11 Prism ; 0x13 Breath-Direct.
Les « variantes » (0x101, 0x105, 0x205, 0x305, 0x106, 0x206) ne sont que des étiquettes OpenRGB : seul l'octet bas est écrit dans 0x09, la différence se fait via d'autres registres (direction, longueur, délai ; détail dans `RGBController_KingstonFuryDRAM.cpp::DeviceUpdateLEDs`).

### Séquence de mise à jour complète

1. `0x08 = 0x53` sur toutes les barrettes, pause 10 ms (début).
2. `0x09 = mode`.
3. Paramètres du mode (direction 0x0C, délai 0x0D, vitesse 0x0E, longueur 0x26, ...).
4. Couleurs : mode direct -> `0x50..` par LED ; mode à couleurs -> `0x30` puis `0x31..`.
5. `0x20 = luminosité`, `0x27 = nombre de slots (<=4)`.
6. `0x08 = 0x44` sur toutes les barrettes, pause 10 ms (application).
Pour le mode direct, il suffit de répéter 4 -> 6 avec `0x09 = 0x10`.

## Protocole HyperX Fury/Predator RGB DDR4 (ancien)

- Détection : un seul `read_byte` sur **0x27** (aucune signature !). Accepté uniquement avec un module DDR4 de JEDEC Kingston 0x0117 dans les slots SPD ; Fury si l'octet fabricant SPD n°6 (offset 0x161 + 6) vaut 1, sinon Predator.
  **Risque** : un simple ack de 0x27 ne prouve pas un contrôleur HyperX. PureRGB doit exiger en plus une lecture cohérente d'un registre du contrôleur avant d'écrire (INCONNU : valeur de contrôle non documentée par la source).
- Registres (`write_byte_data`) : couleur LED j du slot s : R = `0x11 + 0x30*s + 3*j`, G = +1, B = +2 ; luminosité LED j = `0x21 + 0x30*s + 3*j` (0-100).
  Effets : `0xE5` mode indépendant (0x21 = direct), `0xE3`/`0xE4` mode couleurs aléatoires/personnalisées, couleur d'effet `0xEC/0xED/0xEE`, luminosité d'effet `0xDD`,
  temps : timer `0xD1/0xD2`, durée allumé `0xD3/0xD4`, changement `0xD5/0xD6`, fondu entrant `0xD7/0xD8`, sortant `0xD9/0xDA`, éteint `0xDB/0xDC`, délai `0xEA/0xEB` (MSB puis LSB).
  Application : `0xE1 = 0x02`, puis `0xE1 = 0x03` (et `0xE1 = 0x01` avant la configuration d'un effet).
  Modes : 1 = cycle (0x04) / arc-en-ciel (0x05) ; 2 = rebond 0x02, respiration 0x03, clignotement 0x06, battement 0x07, comète 0x08, statique 0x09 ; 3 = direct 0x21.
- Nombre de LEDs par barrette : 5 (registres LED0..LED4 dans les définitions) ; exact pour toutes les gammes : `INCONNU`.

## Ventilateurs / pompes

Sans objet.

## Accès bas niveau

PawnIO + mutex global (voir fiche commune). FURY CTRL : pilote d'accès `INCONNU (non vérifié)`. Le DDR5 FURY nécessite que le hub SPD (0x50..0x57) soit lisible en mode SMBus (page virtuelle), ce qui est vrai avec PawnIO i801/PIIX4.
Attention : une écriture sur 0x60..0x67 sur une plateforme DDR5 inconnue tombe dans une plage utilisée par d'autres composants (PMIC 0x48-0x4F, TS 0x30-0x37 ne sont pas là, mais la plage reste partagée avec le bus carte mère) : respecter le filtre JEDEC + SPD.

## Ce qui est INCONNU

- Signification exacte de chaque variante de mode à partir de 0x05 (ce sont des combinaisons de registres).
- Modèles « Limited Edition » et « v2 » DDR5 : protocole d'effets différent, non documenté dans cette source.
- Registres non listés dans le tableau (par ex. 0x10, 0x11, 0x1E, 0x1F, 0x21, 0x22, 0x28..0x2F, 0x34..0x4F) : rôle inconnu, **ne pas écrire**.
- Fonctionnement interne de FURY CTRL (installeur non extrait).
- Durée de vie / persistance : le contrôleur retient-il le dernier mode après coupure ? `INCONNU`.

## Estimation de difficulté

**Faible à moyenne** : registres simples sans CRC, mode direct en 3 registres par LED. L'essentiel du travail est la détection sûre (SPD DDR5 + signature).

## Tests d'octets possibles sans matériel

1. Table SPD DDR5 : (variante 0x01, modèle 0x10) -> accepté ; (0x00, 0x10) -> refusé ; (0x14, 0x13) -> refusé.
2. Signature : mots lus `0x46xx 0x55xx 0x52xx 0x59xx` -> accepté, modèle `0x23xx` ou `0x21xx` en DDR4.
3. Adresses : DDR4 slots {0,1} -> 0x58, 0x59 ; DDR5 slots {0,1,2,3} -> 0x60..0x63.
4. Registres couleur LED 3 (DDR4) : R = 0x59, G = 0x5A, B = 0x5B ; LED 11 (DDR5) : R = 0x71, G = 0x72, B = 0x73.
5. Mode direct : séquence d'écriture théorique pour 2 barrettes DDR5 en rouge : `0x08=0x53 (x2)`, `0x09=0x10 (x2)`, `0x50..0x73 = FF,00,00,... (x2)`, `0x20=100`, `0x27=2`, `0x08=0x44 (x2)`.
