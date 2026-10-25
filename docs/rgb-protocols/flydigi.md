# Flydigi (Vader 4 Pro et famille) : éclairage

## État : AUCUN protocole d'éclairage public et fiable trouvé. Rien n'est implémenté.

Recherche du 2026-10-25 (web) :
- Patch noyau `xpad` (liste linux-input, octobre 2024) : détection des Vader 3/4 Pro via l'ID Xbox 360 pour les boutons supplémentaires (C, Z, cercle) ; testé Vader 3 Pro et 4 Pro. Aucune trame d'éclairage.
- `ahungry/vader3` (pilote noyau Bluetooth D-Input, Vader 3) : boutons et palettes arrière, pas d'éclairage.
- SDL : prise en charge HIDAPI « améliorée » de la Vader 5 Pro (protocole de commande propre à ce modèle) ; pas la Vader 4 ; non analysée ici.
- `pipe01/flydigictl` (GPL-3.0, Vader 3 Pro) et `BANANASJIM/flydigi-vader5` : liés à d'autres modèles ; licence ou modèle incompatible, non exploités.
- Aucune source ne décrit l'éclairage de la Vader 4.

## Ce que fait PureRGB

Détection par la chaîne produit (voir `manettes.md`) : listée « Manette », vendeur Flydigi, note « éclairage non pris en charge pour l'instant ». Aucune écriture HID vers cette manette.

## Ce qui manque et comment l'obtenir

1. Analyse REA (reverse engineering) du logiciel officiel « Flydigi Space Station » : trouver la collection HID ou l'interface vendeur utilisée, les rapports de sortie d'éclairage (mode, couleur, luminosité), et s'il existe un mode temporaire (sans écriture en mémoire de la manette).
2. Capture USB (`usbcapture.rs`) d'un changement de couleur depuis Space Station, sur le dongle 2,4 GHz et en USB filaire : les deux chemins peuvent différer.
3. Vérifier que la manette expose bien une interface HID vendeur en plus de l'interface XInput (relevé VID/PID/usage page par interface).
Tant que ces trois points ne sont pas établis, aucun octet n'est envoyé : INCONNU.
