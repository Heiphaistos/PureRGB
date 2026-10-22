# Cooler Master : contrôleurs ARGB/RGB (MasterPlus+ / MasterCTRL)

Date : 2026-10-22. **[V]** lu dans la source citée, **[S]** supposé, **[?]** INCONNU.
Écrans (Ion, Core LCD, Atmos, Nex) : `docs/screens-protocols/cooler-master.md`, non repris.

## Source analysée

- **MasterPlus+** (contrôleurs ARGB/RGB, périphériques) et **MasterCTRL** (AIO/écrans) : **non téléchargés** dans cette passe, aucun SHA-256. Une recherche web confirme que MasterPlus+ est le chemin officiel
  (firmware du contrôleur ARGB mis à jour par ce logiciel) mais **aucune spécification publique** du protocole.
- **Sources utilisées (faits seulement)** : OpenRGB (GPL-2.0+) `Controllers/CoolerMasterController/{CMARGBController,CMSmallARGBController,CMARGBGen2A1Controller}` (RGB seul) ;
  plugins SignalRGB `Cooler_Master_ARGB_Controller.js`, `..._A-RGB_LED_Small_Controller.js`, `..._Gen2_LED_Controller_A1.js` (non relus en détail). **Aucun contrôle de vitesse de ventilateur n'est
  documenté pour ces contrôleurs** : ils pilotent les LED, pas les ventilos (les ventilos sont branchés en PWM carte mère / hub séparé).

## Matériel couvert (VID 0x2516)

| PID | Produit | Interface HID | Pilote PureRGB |
|---|---|---|---|
| 1011 | ARGB Controller (4 en-têtes ARGB + 1 RGB 12 V) | interface 0, usage page 0xFF00, usage 1 | aucun |
| 1000 | Small ARGB Controller | idem | aucun |
| 0173 | ARGB Gen 2 LED Controller A1 | interface 1, usage page 0xFF01, usage 1 | aucun (known.rs a 0173) |
| 01C9 | ARGB Gen 2 A1 V2 | idem | aucun |
| 01CB | ARGB Gen 2 A1 Mini | idem | aucun |
| 004F | RGB Controller (12 V, non adressable) | [S] | known.rs le nomme « ARGB Controller » : **à vérifier** (OpenRGB : 004F = RGB, 1011 = ARGB) |

## ARGB Controller gen 1 (1011) [V : CMARGBController.cpp/.h]

- Paquet 65 o (ID `00` + 64). Positions : octet 1 rapport, 2 commande, 3 fonction, 4 zone, 5 mode, 6 index couleur, 7 vitesse, 8 luminosité, 9 R, 10 G, 11 B.
- Ports (octet zone) : ARGB1 `01`, ARGB2 `02`, ARGB3 `04`, ARGB4 `08`, RGB `FE`.
- Firmware : lu dans la **chaîne de série USB** (`A202011171238` = 0023, `A202105291658` = 0028, `A201804091608` = 0000) ; OpenRGB n'accepte que 0023 et 0028.
- **Mode d'un port ARGB** : `80 01 <fonction 01 (normal) ou 02 (passthrough)>`, puis `80 0B 02 <zone> <mode> <index couleur 0x10 fixe / 0x00 aléatoire> <vitesse> <luminosité 0..255> <R G B>`.
  Modes ARGB : spectrum `01`, reload `02`, recoil `03`, breathing `04`, refill `05`, demo `06`, fill-flow `08`, rainbow `09`, static `0A`, off `0B`, direct `FE`, passthrough carte mère `FF`.
  Vitesse `00` (lent) .. `04` (rapide).
- Port RGB 12 V : commande `04`, champs décalés de -2 ; modes mirage `01`, flash `02`, breathing `03`, static `05`, off `06`, passthrough `FF`.
- **Nombre de LED** : `80 0D 02 <zone> <nb LED> <1>`.
- **Mode direct** : `80 01 01 02` pour entrer en direct (fonction 1, zone 2), puis paquets `[n°|0x80 si dernier] 00 07 02` ... avec fonction=index du port, zone=nombre de LED, couleurs R,G,B en continu
  sur 60 o par paquet environ (compteur de paquet dans l'octet 1 ; bit 7 = dernier).
- Lecture d'état d'un port : `80 0B 01 <zone>` (ARGB) ou `80 0A 01 00` (RGB), réponse 250 ms.

## Small ARGB Controller (1000) [V : CMSmallARGBController.*]

- Même squelette ; 1 en-tête ; modes : spectrum `01`, reload `02`, recoil `03`, breathing `04`, refill `05`, demo `06`, off `09`, direct `FE`, passthrough `FF`. Octets : mode (5), vitesse (6), luminosité (8), RVB (9..11).
- Mode d'un port : `80 0B 02 01 <mode> <vitesse> ...` ; LED : `80 0D 02 <zone>` avec octets 5 = `0x0F - n` (min 1) et 6 = n [V, formule étrange de la source]. Direct : paquets `[n°|0x80] 00 10 02`.

## ARGB Gen 2 A1 (0173, 01C9, 01CB) [V : en-têtes CMARGBGen2A1Controller.h, corps non relu]

- Paquet 65 o (64 logiques) ; page de commande `80` ; opérations `01` lecture, `02` écriture, `03` réponse ; réponse attendue en 150 ms.
- 3 canaux, 5 sous-ports Gen2 par canal ; jusqu'à 60 LED ciblées ; cadence temps réel 40 ms (écart de paquet 2 ms) ; luminosité 0..255, vitesse 0..4.
- Commandes (octet) : `01` mode temps réel, `03` effet, `04` effet mémorisé, `06` géométrie Gen1, `07` topologie Gen2, `08` couleurs custom, `09` RGB temps réel, `0A` identifier, `0B` génération,
  `0C` banque d'effets, `0D` remap Gen2, `10`/`11` table/mode d'overlay, `B0` sauvegarder, `B1` reset, `B2` rafraîchir, `B4` firmware, `B5` UID (32 o), `B6` quitter le temps réel, `C0` multicouche.
- Modes : direct `FF`, spectrum `00`, static `01`, reload `02`, recoil `03`, breathing `04`, refill `05`, demo `06`, fill-flow `07`, rainbow `08`, off `09`, custom `C0`, multilayer `FE`.
- Séquences d'entrée en temps réel avec pré-roll (2 puis 5 trames) : détail INCONNU ici (lire le .cpp).
- Les ventilateurs Gen2 (SickleFlow Gen2) sont détectés par topologie (`07`) et remappés (`0D`).

## Ventilateurs / pompes

- **Aucune commande de vitesse ou de RPM documentée** pour ces contrôleurs ni pour les AIO MasterLiquid dans les sources disponibles. Les AIO MasterLiquid à écran (Ion, Core LCD) passent par MasterCTRL ;
  PID et protocole inconnus (cf. screens-protocols). Les MasterLiquid sans écran se pilotent en PWM carte mère/ARGB passthrough.

## Accès bas niveau

- HID uniquement ; MasterPlus+ ouvre le contrôleur (conflit à gérer). Firmware ancien : mise à jour via MasterPlus+ requise pour fonctionner avec OpenRGB/SignalRGB selon retours utilisateurs (anecdotique, non officiel).

## INCONNU

- MasterPlus+/MasterCTRL non analysés ; modes propriétaires, stockage de profils, mises à jour de firmware.
- Ventilos/pompes ; séquences exactes du temps réel Gen2 ; PID des AIO MasterLiquid.

## Difficulté de réimplémentation

ARGB gen 1 / Small : **faible-moyenne** ; Gen2 A1 : **moyenne-élevée** (topologie, overlay, temps réel) ; contrôle thermique : non faisable sur les sources actuelles.

## Tests d'octets possibles sans matériel

- Construire `00 80 01 01`, `00 80 0B 02 01 0A 10 02 FF FF 00 00` (port ARGB1, static) ; `00 80 0D 02 01 20 01` (32 LED) ; longueur 65.
- Paquets directs : segmentation en blocs, bit 7 sur le dernier, compteur croissant.
