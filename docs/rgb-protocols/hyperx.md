# HyperX NGENUITY : protocole USB des périphériques (VID 0951 Kingston / 03F0 HP)

Document d'interopérabilité. Aucun code ni asset HyperX copié ; seuls des faits de protocole.

## Source analysée

- **HyperX NGENUITY : NON analysé.** Distribué via le Microsoft Store (v2, UWP) et un installeur HP/HyperX en ligne ; pas d'installeur autonome récupéré, et installation interdite par la consigne. Aucun binaire HyperX examiné.
- Faits tirés d'**OpenRGB** commit `bbc7747`, `Controllers/HyperXKeyboardController/*`, `HyperXMouseController/*`, `HyperXMousematController/*` (fichiers `*Detect*.cpp` et `*Controller.cpp` lus). Marqueur (O). Code copié : aucun.
- Copie de travail : `D:\Projet\Analyses\rgb\_src\` (hors dépôt).

## Matériel couvert

VID **0x0951** (Kingston, anciens) et **0x03F0** (HP, depuis le rachat). Les deux VID coexistent selon la révision ; (HP) = VID 03F0.

### Claviers
| VID:PID | Modèle | Interface / usage page | Famille |
|---|---|---|---|
| 0951:1711 / 03F0:058F | Alloy Elite 2 / (HP) | interface 3 UP 0xFF90 **ou** interface 0 | Elite 2 |
| 0951:16BE | Alloy Elite RGB | interface 2, UP 0xFF01 | Elite |
| 0951:16DC | Alloy FPS RGB | interface 2, UP 0xFF01 | Elite (même contrôleur FPS) |
| 0951:1734 / 03F0:0C8E | Alloy Origins 60 / (HP) | interface 3 ou 0 | Origins 60/65 |
| 03F0:038F | Alloy Origins 65 (HP) | interface 3 ou 0 | Origins 60/65 |
| 0951:16E5 / 03F0:0591 | Alloy Origins / (HP) | interface 3 (HP : UP 0xFF90) ou 0 | Origins |
| 0951:16E6 / 03F0:098F | Alloy Origins Core / (HP) | interface 2 | Origins Core |
| 03F0:08C2 | Eve 1800 (HP) | interface 2 | Eve 1800 |
| 03F0:0CC2 | Origins 2 65 (HP) | interface 3 | Origins 2 |

### Souris
| VID:PID | Modèle | Interface / UP |
|---|---|---|
| 0951:16D3 / 03F0:0490 | Pulsefire Surge / (HP) | interface 1, UP 0xFF01 |
| 0951:16D7 | Pulsefire FPS Pro | interface 1, UP 0xFF01 |
| 0951:16DE / 03F0:0D8F | Pulsefire Core / (HP) | interface 1, UP 0xFF01 |
| 0951:16E1 / 03F0:068E | Pulsefire Dart sans fil | interface 2, UP 0xFF00 |
| 0951:16E2 / 03F0:088E | Pulsefire Dart filaire | interface 1, UP 0xFF13 |
| 0951:16E4 | Pulsefire Raid | interface 1, UP 0xFF01, usage 1 |
| 0951:1727 / 03F0:0F8F | Pulsefire Haste | interface 3 UP 0xFF90 (autres entrées : interface 0 UP 0x0001 / usage 2) |

### Tapis (contrôleur "Mousemat", via wrapper HID)
| VID:PID | Modèle | Interface / UP |
|---|---|---|
| 0951:1705 | Fury Ultra | interface 0 |
| 03F0:0F8D | Pulsefire Mat | interface 1, UP 0xFF90, usage 0xFF00 |
| 0951:1741 | Pulsefire Mat RGB XL (Fury A XL) | interface 1 UP 0xFF90 U 0xFF00, ou interface 0 UP 0x0C U 1 |

### Micros (OpenRGB : contrôleurs dédiés `HyperXMicrophone*`, NON lus)
QuadCast S : 0951:171F ; HP 03F0:0F8B, 068C, 0294, 028C, 048C, 0D8B ; DuoCast 03F0:098C ; QuadCast 2 S 03F0:02B5.

## Protocole

### Alloy Origins / Origins 60-65 / Elite 2 / Mousemat : même schéma "0x81" (O)
- Rapports **feature** de **65 octets** (`hid_send_feature_report`, octet 0 = report id 0x00).
- **Initialisation du mode direct** : octet 0 = 0x00 (report id), octet 1 = 0x04, octet 2 = 0xF2, **octet 9 = 0x09** pour les claviers Origins ; **octet 9 = 0x02** pour le tapis. (Pour Elite 2, un paquet analogue `00 04 F2 ...` sert de sélection de profil.)
- **Trame de couleurs** : jusqu'à **16 LED par paquet**, 4 octets par LED : `0x81, R, G, B` placés aux octets `1 + 4*i` .. `4 + 4*i` ; reste à 0.
- **Origins** : 9 paquets par image (144 emplacements) ; les positions inutilisées (`skip_idx`) sont insérées à noir ; si moins de LED que 9 x 16, les paquets restants sont envoyés avec une LED noire. Chaque image commence par le paquet d'init.
- **Elite 2** : même méthode, 9 paquets au total, les positions ignorées sont remplies de `81 00 00 00`.
- **Mousemat** : init (octet 9 = 0x02) puis 2 paquets de 16 LED (LED 0-15 puis 16-31), soit 32 LED au plus (zones "Underglow" et "LED Strip" dans OpenRGB).

### Alloy Elite / FPS (rapport 0x07, 264 octets) : (O)
- Rapport **feature de 264 octets**, octet 0 = **0x07** (report id), octet 1 = ID de paquet : **0x02** effet de profil, **0x06** couleur de profil, **0x16** direct.
- Sélection de profil : `07 01 <profil> ... [6]=03 [7]=01`.
- Effet : `07 02 <profil> ... [9]=01 [0x0A]=mode [0x0D]=[0x0E]=direction [0x1B..0x22]=mode réactif`, couleurs simples aux octets 0x29 (R), 0x41 (G), 0x59 (B), doubles 0x2A/0x42/0x5A et 0x2B/0x43/0x5B, octets 0x6B..0x70 constants (09 09 05 05 06 05), vitesse octet 0x71. Le mappage de `mode` -> effet n'est pas relevé.
- Le format du paquet direct 0x16 n'a pas été lu : INCONNU.

### Pulsefire Surge (rapport 0x07, 264 octets) : (O)
- Feature de 264 octets, octet 0 = 0x07. IDs : **0x01** config, **0x03** luminosité (octets 2 = profil, 3-5 = 01 01 01, 6 = luminosité), **0x07** sélection de profil (octet 2), **0x14** direct.
- **Direct** : `07 14 .. A0` (octet 3 = 0xA0) puis 32 valeurs R à partir de l'octet 0x08, 32 valeurs G à partir de 0x28, 32 valeurs B à partir de 0x48 (plans séparés), et LED 33 : R/G/B aux octets 0x6C / 0x6D / 0x6E.
- Autres souris (FPS Pro, Core, Dart, Raid, Haste) : contrôleurs distincts NON lus.

## Modes matériels
Profils embarqués sélectionnables (Elite/Surge : sélection de profil 1-3 ; effets du profil modifiables via paquets 0x02/0x06). La liste des effets (statique, respiration, vague, réactif...) et leurs identifiants numériques : INCONNUE.

## Cadence
Non mesurée. Un clavier Origins = 9 + 1 feature reports par image (~10 transferts).

## Ventilateurs / pompes
Sans objet.

## Accès bas niveau
HID uniquement, aucun pilote vendeur. NGENUITY (service) tient les interfaces ; conflit à prévoir (le Microsoft Store de NGENUITY peut être arrêté). Aucun SMBus pour ces périphériques (les mémoires HyperX DRAM relèvent du document RAM, contrôleur `HyperXDRAMController` : hors périmètre).

## Ce qui est INCONNU
- Contenu réel de NGENUITY (non analysé).
- Protocole direct des Alloy Elite/FPS, Eve 1800, Origins Core, Origins 2 65, Pulsefire Core/FPS Pro/Dart/Raid/Haste, micros.
- Identifiants des effets matériels ; mappage LED -> touche (tables `skip_idx`, non transcrites).
- Cadence maximale ; comportement sans fil.

## Estimation de difficulté
**Facile** pour Alloy Origins / Origins 60-65 / Elite 2 / tapis (un seul schéma 0x81 x 16 LED, 65 octets). Moyenne pour la famille 0x07 (264 octets) car peu documentée ici.

## Tests d'octets possibles sans matériel
1. Générer le paquet d'init Origins (65 octets, [1]=04, [2]=F2, [9]=09) et le comparer à la référence.
2. Générer 9 paquets de 16 LED : vérifier octet `1+4i` = 0x81 et couleurs aux `2+4i..4+4i`, max 16 LED par paquet, 144 emplacements au total.
3. Générer le paquet direct Surge (264 octets) : plans R 0x08, G 0x28, B 0x48, LED 33 en 0x6C-0x6E.
