# Roccat Swarm / Turtle Beach Swarm II : protocole USB des périphériques

Document d'interopérabilité. Aucun code ni asset Roccat / Turtle Beach copié ; seuls des faits de protocole.

## Source analysée

- **Roccat Swarm / Turtle Beach Swarm II : NON analysé.** Installeur non récupéré (installe services et pilotes ; interdit d'installer par la consigne). Aucune extraction du logiciel officiel.
- Faits tirés d'**OpenRGB** commit `bbc7747`, `Controllers/RoccatController/` : `RoccatControllerDetect.cpp`, `RoccatVulcanKeyboardController/RoccatVulcanKeyboardController.{h,cpp}`, `RoccatKoneProController/RoccatKoneProController.{h,cpp}` ; `Controllers/TurtleBeachKP7Controller/` (PID seulement). Marqueur (O). Un clone de **libratbag** (`8235b5b`, dossier `data/devices`) existe localement et n'a pas été exploité.
- Copie : `D:\Projet\Analyses\rgb\_src\` (hors dépôt).

## Matériel couvert

VID Roccat **0x1E7D** ; Turtle Beach (après rachat de Roccat) **0x10F5**.

### Claviers (même contrôleur "Vulcan")
| VID:PID | Modèle | Rapport mode / paquet couleur |
|---|---|---|
| 1E7D:307A | Vulcan 100 Aimo | protocole v1 : trame 436 octets |
| 1E7D:3098 | Vulcan 120 Aimo | v1 |
| 1E7D:2FEE | Vulcan TKL | v1 |
| 1E7D:30F7 / 311A | Vulcan Pro / Vulcan TKL Pro | v2 : 384 octets, colonnes de 12 |
| 1E7D:2F4E | Vulcan II | v2 : 396 octets |
| 1E7D:2EE2 | Vulcan II Max | v2 : 567 octets |
| 1E7D:2F09 | Vulcan II Mini | v2 : 345 octets (69 LED, table de renumérotation) |
| 1E7D:314C | Pyro | v2 : 378 octets |
| 1E7D:3124 / 69A0 | Magma / Magma Mini | v2 : 64 octets, colonnes de 5 |
| 10F5:501B | Turtle Beach Vulcan II | v2 (comme Vulcan II) |
| 10F5:5023 | Turtle Beach Vulcan II TKL | v1 (cas par défaut) |
| 10F5:5001 | Turtle Beach Vulcan II TKL Pro | v2 : 320 octets, plans R/G/B séparés |

### Souris, tapis, casque, autres (contrôleurs distincts NON lus, sauf Kone Pro en partie)
| VID:PID | Modèle |
|---|---|
| 1E7D:303E | Horde Aimo (clavier) |
| 1E7D:2DE6 / 2DE1 / 2CA6 | Burst Core / Burst Pro / Burst Pro Air |
| 1E7D:2E27 / 2E2C | Kone Aimo / Kone Aimo 16K |
| 1E7D:2C88 / 2C8E / 2C92 | Kone Pro / Kone Pro Air / Kone Pro Air filaire |
| 1E7D:2C8B | Kone XP |
| 1E7D:2CEE | Kova |
| 1E7D:343A / 343B | Sense Aimo Mid / XXL (tapis) |
| 1E7D:3A34 | Elo 7.1 (casque) |
| 10F5:503A | Turtle Beach Command Series KP7 (pavé) |

Interfaces d'ouverture (O, `RoccatControllerDetect.cpp`), claviers Vulcan : deux collections HID à ouvrir : **ctrl** (rapports feature) et **led** (écriture `hid_write` de 65 octets).
- Pyro, Magma, Magma Mini, Vulcan Pro, Vulcan TKL Pro, Vulcan II / Mini / Max, TB Vulcan II : led = interface 3 UP 0xFF00 ; ctrl = interface 1 UP 0xFF01.
- TB Vulcan II TKL Pro : led = interface 4 UP 0xFF00 ; ctrl = interface 1 UP 0x0001.
- Autres (Vulcan 100/120/TKL, TB Vulcan II TKL) : led = interface 3 UP 0x0001 ; ctrl = interface 1 UP 0x000B (ou 11 décimal, selon la déclaration).
Autres familles : Burst Pro : interface 3 UP 0xFF01 usage 1 ; Kone Pro : interface 3 UP 0xFF01 usage 1 ; Kone Pro Air : interface 2 UP 0xFF00 (filaire : interface 1 UP 0xFF13) ; Kone XP : interface 3 UP 0xFF01 ; Burst Pro Air : interface 0 UP 0x01 usage 2 ; Kone Aimo / Kova / Horde Aimo : interface 0 (ou 1) UP 0x0B ; Sense Aimo : interface 0 UP 0xFF01 ; Elo 7.1 : interface 3 UP 0x0C.

## Protocole

### Claviers Vulcan : (O, `RoccatVulcanKeyboardController.cpp`)
1. **Vérification de disponibilité** : lecture feature report sur ctrl, report id **0x09** (modèles v2 : Pyro, Magma, Vulcan Pro/II, TB II ; longueur 9) ou **0x0F** (v1, longueur 8). L'octet 1 de la réponse doit valoir 1 ("prêt"), sinon avertissement. Octet 6 = 0x06 sur Vulcan II Max JP (disposition JIS).
2. **Activation du mode direct** (feature report sur ctrl) : modèles v2 = `0E 05 <on> 00 00` (5 octets) ; modèles v1 = `15 00 <on>` (3 octets). `<on>` = 1 pour activer, 0 pour rendre la main.
3. **Envoi de la trame couleur** : `hid_write` de paquets de **65 octets** (octet 0 = 0 report id) sur led. En-tête du paquet i : `[1] = 0xA1`, `[2] = n° de paquet` (v2 : i+1 ; v1 : 0x01 seulement sur le premier), puis la longueur totale de trame : 1 octet si <= 255, sinon 2 octets (ordre selon `IsBigEndianDirectMode()`, octets 3-4 du premier paquet). Données utiles : 64 - longueur d'en-tête octets par paquet. Nombre de paquets = ceil(longueur / 64). Après envoi : vider les réponses puis attendre 20 ms.
4. **Disposition des couleurs** :
   - v1 (Vulcan 100/120/TKL, TB II TKL) : trame 436 octets ; par colonne de 12 LED, 3 plans consécutifs (R sur 12, G sur 12, B sur 12) ; `offset = colonne * 3 * 12 + ligne + en-tête`.
   - v2 (Pro, II, II Max, Mini, Pyro, Magma) : même schéma avec colonne = 1 (Vulcan II, Mini, Max, Pyro) ou 12 (Pro) ou 5 (Magma) ; plans R, G, B décalés de `column_length`.
   - TB Vulcan II TKL Pro (v2 spécial) : 320 octets ; les LED sont indexées 0..N-1, plans R, puis G (décalé de N), puis B (décalé de 2N).
5. **Vulcan II Mini** : n'applique une couleur statique 0x11 que si un rapport "keepalive" **0x13** est envoyé en parallèle : `13 7B 1E` suivi d'entrées de 4 octets (30 entrées), avec marqueurs A4, A5, B0, B6 à des positions fixes.
6. **Modes matériels** (feature report sur ctrl, `hid_send_feature_report`) : octet 0 = **0x0D** (v1) ou **0x11** (v2) ; longueur totale du rapport aux octets 1 (si <= 255) ou 1-2 (little-endian, sinon) ; puis, à `offset = longueur d'en-tête + 1` : `[offset+0]=0x00, [+1]=mode, [+2]=vitesse`, luminosité à `[+3]` ou `[+4]` selon le modèle, et blocs avec marqueurs `0x02 .. 0xFB` et `0x03 .. 0x0B 0x01`. **Checksum** : somme 16 bits de tous les octets sauf les deux derniers, écrite en little-endian dans les deux derniers octets. Longueurs : 443 (v1 par défaut), 332 et 284 (certains v2, dont TB TKL Pro = 284) ; tableau complet par modèle non relevé. Modes : **0x01 statique, 0x0A vague, 0x0B direct, 0x02 défaut** (liste complète non relevée).
7. **Ordre des octets de la longueur du paquet couleur** : big-endian si la version firmware lue est >= 1.16 (sauf Vulcan II Mini : toujours little-endian), sinon little-endian (`IsBigEndianDirectMode`).

### Kone Pro (souris) : (O, `RoccatKoneProController.cpp`)
- Mode contrôle : feature `0E 06 01 <direct> 00 FF` (6 octets).
- Direct : feature de 11 octets, report id **0x0D**, octet 1 = **0x0B**, puis 2 LED (zone) en RGB : octets `2+3i`, `3+3i`, `4+3i`.
- Flash/effet : report id **0x06**, 69 octets (`06 45 .. 06 06 1F`, mode aux octets 0x1E, vitesse 0x1F...).

### Autres Roccat (Burst, Kone Aimo/XP/Pro Air, Kova, Sense Aimo, Elo, Horde Aimo) et Turtle Beach KP7
Contrôleurs dédiés dans OpenRGB, **non lus**.

## Cadence
Non mesurée. Une trame Vulcan II = 7 paquets de 65 octets (396 / 64) + attente 20 ms de réponse.

## Ventilateurs / pompes
Sans objet.

## Accès bas niveau
HID uniquement. Roccat Swarm II / Turtle Beach Swarm II (service) tient les interfaces : conflit à prévoir. Pas de pilote vendeur requis en écriture HID. Le "mode direct" doit être explicitement rendu (`<on>` = 0) à la fermeture sinon le clavier garde les couleurs logicielles.

## Ce qui est INCONNU
- Logiciel officiel non analysé (aucune installation/extraction).
- Table des identifiants de modes matériels et du format exact du rapport 0x0D / 0x11 (octets intermédiaires non transcrits).
- Protocoles Burst / Kone Aimo / Kone XP / Kova / Sense / Elo / KP7 / Horde.
- Correspondance LED -> touche (`RoccatVulcanKeyboardLayouts.h`, 81 Ko, non transcrit).
- Cadence maximale.

## Estimation de difficulté
**Moyenne** : trames longues multi-paquets avec en-tête de longueur dépendant du modèle, deux collections HID à ouvrir, plusieurs variantes (v1 / v2 / TB TKL Pro / Mini avec keepalive). Kone Pro : facile.

## Tests d'octets possibles sans matériel
1. Générer pour un Vulcan II (396 octets) les 7 paquets : vérifier `[1]=A1`, `[2]=1..7`, en-tête de longueur 396 en 2 octets, 60 octets de données dans le premier paquet (64 - 4).
2. Vérifier qu'un Vulcan 100 produit 1 seul octet de n° de paquet (0x01 sur le premier uniquement) et l'octet de longueur selon 436 > 255 (2 octets).
3. Calculer le checksum 16 bits d'un rapport de mode et le comparer à la somme des octets.
4. Générer les rapports d'activation : `0E 05 01 00 00` (v2) et `15 00 01` (v1).
