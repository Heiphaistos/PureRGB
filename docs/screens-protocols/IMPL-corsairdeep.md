# Implémentation Corsair iCUE LINK LCD et DeepCool LM

Date : 2026-10-21, branche `screens/corsairdeep`. Sources : `corsair.md`, `deepcool.md`. Dépôts lus dans un dossier temporaire hors dépôt (reactor MIT, deepcool-lcd MIT, opencreative MIT/Apache) ; OpenLinkHub (GPL) cité par la fiche, non copié. Tout est livré `verified: false`, `resolution_confirmed: false`.

## Corsair : `drivers/corsair_lcd.rs` (`CorsairProvider`)

Implémenté [V, README icue-link-lcd-reactor + `lcd.py`] :
- 1B1C:0C4E, 480x480, JPEG (qualité 85) découpé en rapports de 1024 octets : `02 05 01 <dernier> <indice> 00 <len LE16>` + 1016 octets de données, remplissage de zéros.
- Luminosité : feature report `03 0B <0-100> 01` (via `ScreenDriver::set_brightness`).
- Rotation matérielle : feature report `03 0C <0-3> 01` (`CorsairScreen::set_rotation`, hors trait ; sens des quarts de tour INCONNU).
- Octet 0 (`02`) = identifiant de rapport inclus dans les 1024 octets : le pilote écrit directement via hidapi (le `HidTransport` commun ajoute un `00` devant, il ne convient pas). Cette lecture vient de `lcd.py` (écriture hidraw Linux) ; non confirmée sur le descripteur de rapport Windows.
- Détection par énumération hidapi sans ouverture ; ouverture paresseuse au premier envoi, réouverture après erreur. Erreurs françaises (« Fermez iCUE… »).

Derrière le drapeau `CorsairProvider { experimental: true }` [S] : 0C43 (XD5 LCD), 0C57 et 0C55 (Nautilus LCD), même protocole supposé, 480x480 supposé.

Non implémenté (`INCONNU`) : 0C42 (XC7 Elite, résolution inconnue), Elite LCD / XT (PID inconnu), 5" LCD (moniteur DisplayPort, pas de pilote), indices de morceau au-delà de 255 (refusé), lecture de réponses (aucune documentée), choix de la collection HID quand l'appareil en expose plusieurs (on prend la première énumérée).

## DeepCool : `drivers/deepcool_lm.rs` (`DeepcoolLmProvider`)

Implémenté [V, 5 paquets capturés + 2 docs MIT] :
- 3633:0026, interface bulk via `NusbTransport` (OUT 0x01, IN 0x81), WinUSB requis (message Zadig du transport commun, complété par « Fermez DeepCreative »).
- Paquet `AA | len LE16 | payload | CRC-16/XMODEM LE16`.
- Init `09` (puis attente 500 ms, envoyée au premier envoi, pas à la détection), luminosité `06 03 <niveau LE16>` (bornée 10 à 100), bascule zen `03 00 00 00` (`DeepcoolScreen::toggle_screen`), en-tête de trame `00 01 <153600 LE32> 2c 01`, puis 153 600 octets RGB565 little-endian en une écriture bulk.
- Vecteurs de test : les 4 paquets capturés reproduits dans la doc (init, luminosité 97, zen, en-tête). Le 5e (luminosité 29) n'est pas reproduit dans les docs : ses octets CRC (E6 0B) est calculé à part, noté comme tel dans le test.

Non implémenté / `INCONNU` : signification de `2c 01` (= 300 = 153600/512 selon OpenCreative, déduit), réponses sur l'endpoint IN (aucune lecture faite, donc aucune validation de réponse possible), luminosité sous 10, rotation, état courant de l'écran (zen est une bascule), différences LM240/LM360 (un seul modèle « LM240 / LM360 »), Mystique 240/360 (PID 9, 640x480, protocole inconnu), afficheurs numériques AK/LD/LS/CH (hors périmètre).

## À valider sur matériel

1. Corsair : le descripteur HID Windows accepte-t-il un rapport de 1024 octets avec ID 0x02 via `hidapi::write` ? Quelle collection HID reçoit les écritures ? iCUE ouvert bloque-t-il l'écriture ? L'image est-elle affichée (orientation, rognage du cercle) ? Cadence maximale.
2. Corsair expérimentaux (0C43/0C57/0C55) : protocole et résolution.
3. DeepCool : l'écran affiche-t-il la trame après un seul envoi ? Faut-il renvoyer des trames en continu pour qu'il reste allumé ? Le sleep de 500 ms est-il suffisant ? Une réponse IN apparaît-elle après `09` et bloque-t-elle les écritures si elle n'est pas lue ? LM240 identique ?
4. Orientation/rotation logicielle : le moteur applique `ScreenInfo.rotation` (0 ici) ; le pilote ne pivote rien.

## Modification hors fichiers pilotes

`screens/driver.rs` : ajout de deux variantes à `Protocol` (`CorsairLcd`, `DeepcoolLm`), sans autre changement. Le front et `drivers/mod.rs` (deux `pub mod`) à fusionner avec les autres branches.
