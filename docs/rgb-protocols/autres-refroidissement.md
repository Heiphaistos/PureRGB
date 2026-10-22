# Autres refroidisseurs / boîtiers : Arctic, DeepCool, ID-Cooling, Phanteks, be quiet!, Antec

Date : 2026-10-22. **[V]** lu dans la source, **[S]** supposé, **[?]** INCONNU. Aucun logiciel de ces marques n'a été téléchargé/analysé dans cette passe (aucun SHA-256).

## Arctic : contrôleur A-RGB (port série, non HID) [V : OpenRGB `Controllers/ArcticController`]

- Matériel : pont USB-série **CH340/CH341 `1A86:7523`** (VID/PID génériques du convertisseur, communs à d'autres cartes : détection par sonde, pas par PID).
  Logiciel officiel : aucun logiciel Windows Arctic identifié ; contrôle matériel (boutons) + pilote communautaire.
- Série : **250 000 bauds, 8 bits, sans parité, 2 bits de stop**, DTR levé à l'ouverture et baissé à la fermeture.
- Trame d'envoi : en-tête fixe `01 02 03 FF 05 FF 02 03` (8 o) + octet de commande + charge utile.
  - Identification : commande `5C`, charge `01 FE 01 FE`. Réponse : 15 octets : [0] = `5C`, [1..12] données, [13] = XOR des octets 1..12, [14] = somme (mod 256) des octets 1..12.
  - Couleurs : commande `00`, puis 3 octets par canal (R,G,B), chaque valeur **plafonnée à 254** (valeur `FF` réservée) ; nombre de canaux = nombre de couleurs.
- Remarque : la source calcule la longueur d'écriture avec `sizeof(pointeur)` (bug probable) ; la longueur correcte attendue est `8 + 1 + 3 x canaux` : **à vérifier sur matériel**.
- Ventilos : aucune commande ; ce contrôleur ne pilote que les LED (P12/P14 A-RGB). Difficulté : **faible** (si le port série est détecté).
- Test d'octets : trame d'identification = `01 02 03 FF 05 FF 02 03 5C 01 FE 01 FE` (13 o) ; XOR/ADD de la réponse.

## DeepCool

- Logiciel : DeepCreative (Windows) ; afficheurs numériques VID `3633` / `34D3`, LCD LM-Series `3633:0026` : voir `docs/screens-protocols/deepcool.md`.
- RGB/ventilos : les ventilateurs et les refroidisseurs DeepCool « ARGB » se pilotent par l'en-tête carte mère (5 V, 3 broches) ou boutons de boîtier ; **aucun protocole USB de contrôle de ventilateur
  ou de LED trouvé** dans les sources locales (OpenRGB, liquidctl, SignalRGB n'ont qu'un clavier DeepCool). Les « Digital » n'exposent que de l'affichage (températures/charge) en HID 64 o.
  INCONNU : toute commande de vitesse/LED côté USB (à décider après analyse de DeepCreative).

## ID-Cooling

- Logiciel : FX LCD Series Software (Windows) ; `2000:3000`, voir `docs/screens-protocols/id-cooling.md`. Aucun protocole de ventilateur/LED USB connu (refroidisseurs ARGB 5 V carte mère). INCONNU.

## Phanteks

- Produits : hub **NexLinq** (LINQ6, PWM, JST, alimentation PCIe 6 broches), contrôleurs D-RGB à boutons. Logiciel : application « NexLinq » (mentionnée par les revendeurs) ; aucune documentation de protocole
  ni projet open source trouvé (recherche web). Pas de VID/PID relevé : INCONNU.

## be quiet!

- Light Wings ARGB (3 broches 5 V, hub fourni), Silent Loop / Pure Loop : contrôle par l'en-tête carte mère ou hub à bouton. **Pas de logiciel ni de protocole USB** identifiés. Rien à implémenter nativement hors synchro carte mère.

## Antec

- « ARGB & PWM Fan Controller » (5 ports PWM + 5 ARGB, alimenté en SATA, 2 boutons) : contrôle **matériel uniquement** d'après les fiches revendeurs ; aucune liaison USB. Rien à implémenter.

## Conclusion et priorité

Seul Arctic a un protocole natif documenté ici (série), et il est limité aux LED. Pour DeepCool, ID-Cooling, Phanteks, be quiet!, Antec, il n'y a **pas de protocole USB de ventilateur/LED public** ;
leur contrôle passe par les en-têtes ARGB/PWM de la carte mère (déjà gérés par les backends carte mère de PureRGB). Priorité basse.

Sources web consultées (recherche) : fiches revendeurs Antec ARGB Fan Controller (inside-tech.co.uk), be quiet! Light Wings (overclock3d.net), Phanteks NexLinq Hub (overclockers.co.uk).
