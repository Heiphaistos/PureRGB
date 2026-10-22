# SteelSeries GG / Engine : protocole USB des périphériques

Document d'interopérabilité. Aucun code ni asset SteelSeries copié ; seuls des faits de protocole.

## Source analysée

- **SteelSeries GG / Engine 3 : NON analysé** (installeur non téléchargé : installe services/pilotes ; interdit d'installer par la consigne ; extraction d'un installeur signé non tentée faute de lien stable vérifié).
- Faits tirés de :
  - **OpenRGB** commit `bbc7747`, `Controllers/SteelSeriesController/` : `SteelSeriesDevices.h`, `SteelSeriesControllerDetect.cpp`, `SteelSeriesApexBaseController.h`, `SteelSeriesApexController/`, `SteelSeriesRival3Controller/`, `SteelSeriesAerox3Controller/`, `SteelSeriesMouseController/`.
  - **SteelSeries GameSense SDK** (API publique du logiciel) : `github.com/SteelSeries/gamesense-sdk`, `doc/api/sending-game-events.md` (lu).
  - Le contrôleur Aerox cite `rivalcfg` (`flozz/rivalcfg`, `devices/aerox3.py`) comme référence : non lu.
- Copies : `D:\Projet\Analyses\rgb\_src\` (hors dépôt). Marqueur (O) = lu directement dans OpenRGB.

## Matériel couvert

VID SteelSeries **0x1038**. Interface = numéro d'interface USB à ouvrir ; (IPU) = interface + usage page + usage.

### Claviers
| PID | Modèle | Interface | Famille |
|---|---|---|---|
| 161A | Apex 3 | 3 | Apex3 (zones) |
| 1622 | Apex 3 TKL | 1, UP 0xFFC0, U 1 | Apex3 TKL |
| 161C | Apex 5 | 1 | Apex |
| 1612 / 1618 | Apex 7 / 7 TKL | 1 | Apex |
| 1634 / 1620 | Apex 9 TKL / 9 Mini | 1 | Apex (paquet direct 513) |
| 1610 / 1614 | Apex Pro / Pro TKL | 1 | Apex |
| 1628 | Apex Pro TKL 2023 filaire | 1 | Apex "2023" |
| 1630 / 1632 | Apex Pro TKL 2023 sans fil | 3, UP 0xFFC0, U 1 | Apex 2023 sans fil |
| 1642 | Apex Pro TKL Gen 3 filaire | 1 | Apex Gen3 |
| 1644 / 1646 | Apex Pro TKL Gen 3 sans fil | 3, UP 0xFFC0, U 1 | Gen3 sans fil |
| 1640 | Apex Pro 3 | 1 | Apex |
| 0616 | Apex M750 | 2 | ApexM |
| 1202 / 1206 | Apex (OG) / Apex 350 | 0 | Apex ancien |

### Souris
| PID | Modèle | Interface | Famille |
|---|---|---|---|
| 1702, 170C, 1814, 1816, 1729 | Rival 100 / 100 Dota / 105 / 106 / 110 | 0 | Rival100 |
| 1710, 1714 (Acer), 1394, 1716, 171A, 1392, 1718 | Rival 300 et éditions | 0 | Rival300 |
| 1720, 171E, 1736 | Rival 310 / Howl / PUBG | 0 | Sensei |
| 1832, 1834, 1722 | Sensei Ten / CS:GO Neon Rider / Sensei 310 | 0 | Sensei |
| 1724, 172E | Rival 600 / Dota 2 | 0 | Rival600 |
| 172B, 1726 | Rival 650 / 650 Wireless | 0 | Rival650 |
| 1700, 1730 | Rival 700 / 710 | 0 | Rival700 |
| 1824, 184C | Rival 3 (ancien firmware / actuel) | 3 | Rival3 |
| 1830 | Rival 3 Wireless | 3, UP 0xFFC0, U 1 | Rival3 |
| 183C | Rival 5 | 0, UP 0xFFC0, U 1 | Rival5 |
| 1836 | Aerox 3 | 3, UP 0xFFC0, U 1 | Aerox |
| 1838 / 183A | Aerox 3 Wireless / (filaire) | idem | Aerox |
| 1878 / 187A | Aerox 3 CS2 Dragon Lore sans fil / filaire | idem | Aerox |
| 1890 | Aerox 3 Gen 2 Wireless | idem | Aerox Gen2 |
| 1850 | Aerox 5 filaire | idem | Aerox |
| 1852 / 1854, 185C / 185E, 1860 / 1862 | Aerox 5 Wireless / Destiny 2 / Diablo IV (sans fil / filaire) | idem | Aerox |
| 1858 / 185A | Aerox 9 Wireless / filaire | idem | Aerox |

Remarque (O) : `STEELSERIES_AEROX_5_PID` et `STEELSERIES_AEROX_9_PID` sont définis sur 0x1850 et 0x185A, et 0x185A est aussi le PID "Aerox 9 Wireless Wired" : doublon dans la source, à vérifier sur le matériel avant de s'y fier.

### Tapis et casques
- Tapis QCK Prism Cloth : 150A (Medium), 150D (XL), 151E (XL Destiny), 1514 (XL Neon Rider), 151C (XL Neo Noir), 1516 (3XL), 1518 (4XL), 151A (5XL), 1520 (XL Destiny 2 Lightfall) ; interface 0.
- Casques : Siberia 350 = 1229 (interface 3), Arctis Nova 3 = 12EC (interface 4, UP 0xFFC0, U 1), Arctis 5 = 1250 et 12AA (interface 5).

## Protocole

### Claviers Apex (O, `SteelSeriesApexController.cpp`, `...ApexBaseController.h`)
- Transport : rapports **feature** (`hid_send_feature_report`) pour la couleur ; `hid_write` + lecture pour les requêtes ; paquet "sortie" standard 65 octets (octet 0 = report id 0), entrée 64 octets, timeout 100 ms.
- IDs de paquet : 0x89 profil (octet 2 = n° de profil), 0x90 version firmware, 0x23 luminosité (get 0xA3, plage **0-10**), 0xF5 région, 0xFF série (Gen1).
- **Gen1 (Apex 5/7/Pro anciens firmwares)** : direct = id **0x3A**, retour onboard = id **0x3B**.
- **Gen2 "2023"** : direct filaire id **0x40**, direct sans fil id **0x61**, retour onboard id **0x41**, **init id 0x4B**.
- **Gen3** : détectée par version firmware >= 1.19.7 pour Apex Pro TKL / Apex 9 TKL / Apex 9 Mini ; envoie l'init 0x4B (feature report de 65 octets, octet 1 = 0x4B) et utilise reset 0x41.
- **Trame directe** (feature report de longueur `direct_packet_length` : 643 par défaut, **513** pour la gamme Apex 9, d'après `APEX_PACKET_LENGTH` / `APEX_9_PACKET_LENGTH`) :
  `[0x00][packet_id][nb_touches][ (code_touche, R, G, B) x nb_touches ]` ; octets 3+4i = code touche, 4+4i..6+4i = R, G, B. Nombre maximal = (longueur - 3) / 4. La table `keys[]` (environ 106 codes, non recomptés ; 0x04.. dont 0x31, 0x53-0x63, 0x64, 0x87-0x8B, 0xE0-0xE7, 0xF0, 0xFB) : ce sont des usages HID clavier avec extensions vendeur ; table non transcrite.
- **Retour au mode matériel** : envoyer le paquet de reset (0x3B Gen1 / 0x41 Gen2-3) ; OpenRGB le fait à la fermeture, sinon les profils matériels ne se réappliquent pas.
- Passage mode direct -> onboard sur Gen2/3 : ré-init 0x4B nécessaire en revenant en direct.
- Apex 3 (zones), Apex 3 TKL, Apex M750, Apex 7 TKL "8 zones" et anciens (OG/350) ont chacun un contrôleur dédié (`SteelSeriesApex3Controller`, `...Apex8Zone`, `...ApexTZone`, `...ApexM`, `...OldApex`) : NON lus.

### Souris
- **Rival 3** (O) : paquet `hid_write` ; effet : `00 06 00 <effet>` ; couleur : `00 05 00 <zone> R G B <luminosité>` ; version firmware : `00 10 00` ; enregistrement (famille MouseController) : `00 09`. Tailles de tampon (FW/EFFECT/COLOR/SAVE) définies dans les en-têtes, valeurs non relevées.
- **Aerox 3** (O) : paquet de **65 octets** ; couleur : `00 21 <masque de zone = 1<<zone> ... R G B à l'offset 3 + zone*3>` ; luminosité : `00 23 <valeur>` ; sauvegarde (non confirmée fonctionnelle selon le commentaire) : `00 11 00` ; une requête feature `00 2D` existe en init. Référence citée : rivalcfg `aerox3.py`.
- Autres familles (Rival 100/300/600/650/700, Sensei, Rival 5, Aerox 5/9, wireless) : contrôleurs distincts, NON lus.

### Casques / tapis
Siberia 350, Arctis 5, Arctis Nova 3, QCK : contrôleurs distincts (`SteelSeriesSiberiaController`, `...Arctis5`, `...ArctisNova3`, `...QCKMat`), NON lus.

### GameSense SDK (API publique, côté logiciel)
- Découverte : le fichier `coreProps.json` contient `address` = `"host:port"` du serveur Engine local (ex. `127.0.0.1:51248`). Chemin Windows : `%PROGRAMDATA%/SteelSeries/SteelSeries Engine 3/coreProps.json` ; macOS : `/Library/Application Support/SteelSeries Engine 3/coreProps.json`.
- Endpoints (POST JSON) : `/game_metadata`, `/game_event` (`game`, `event`, `data.value`, `data.frame` optionnel), `/game_heartbeat`, `/multiple_game_events`, `/register_game_event`, `/bind_game_event`.
- Désactivation après **15 s** sans événement (réglable de 1000 à 60000 ms via `deinitialize_timer_length_ms`). Aucune limite de débit documentée (batching conseillé).
- Ce SDK passe par SteelSeries Engine installé et ne parle pas directement au HID : utile seulement comme voie de repli.

## Cadence
Inconnue/non mesurée. Gen1/2/3 : une image = un feature report de 513 ou 643 octets.

## Ventilateurs / pompes
Sans objet.

## Accès bas niveau
HID uniquement, aucun pilote vendeur nécessaire en écriture. Les interfaces 0xFFC0 (Aerox, Rival 3 Wireless, Apex sans fil) sont des collections vendeur : sous Windows elles peuvent être tenues par SteelSeries GG (service `SteelSeriesGG`/Engine) : prévoir le conflit.

## Ce qui est INCONNU
- Contenu réel de GG / Engine (non analysé).
- Protocole des Rival 100/300/600/650/700, Sensei, Rival 5, Aerox 5/9, Apex 3 / M750 / OG, casques, QCK : non lus.
- Table complète des codes touches Apex (non transcrite) ; mappage LED -> touche par modèle.
- Cadence maximale, comportement des variantes sans fil.
- Valeur exacte des tailles de tampon des souris Rival 3.

## Estimation de difficulté
**Facile pour Apex Pro / Apex 7 / 9 (une seule trame feature)** ; moyenne pour la détection Gen1/2/3 (version firmware) ; facile pour Rival 3 / Aerox 3. Beaucoup de familles à couvrir une par une.

## Tests d'octets possibles sans matériel
1. Construire la trame directe Apex 9 (513 octets) avec 3 touches : octet 1 = 0x40 (ou 0x3A), octet 2 = 3, octets 3-14 = (code, R, G, B) x 3, reste 0 ; nombre maximal = (513 - 3) / 4 = 127.
2. Construire `00 21 <1<<zone> ...` pour Aerox 3 et vérifier l'offset 3 + zone*3.
3. Construire l'init Gen3 : feature report de 65 octets, octet 1 = 0x4B.
4. Parser une version "x.y.z" et vérifier la règle >= 1.19.7.
