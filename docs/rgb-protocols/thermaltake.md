# Thermaltake TT RGB Plus / Riing Plus / Quad / Trio / Toughliquid

Date : 2026-10-22. **[V]** lu dans la source citée, **[S]** supposé, **[?]** INCONNU.
Écrans (Toughliquid Ultra, TH420) : `docs/screens-protocols/thermaltake.md`, non repris.

## Source analysée

- **Logiciel officiel** : TT RGB PLUS (1.0 / 2.0 / 3.0), page `https://thermaltake.com/pages/downloads` : **non téléchargé** (non trouvé en lien direct stable dans cette passe, aucun
  SHA-256). Aucun octet n'a été relevé dans le logiciel officiel.
- **Sources utilisées (faits seulement)** : OpenRGB (GPL-2.0+) `Controllers/ThermaltakeRiingController/*` (Riing Plus, Quad, Trio) ; plugin SignalRGB
  `Plugins/Thermal Take/ThermalTake_Riing_Plus.js` (lu en entier : **seule source des commandes ventilateur**, RGB + vitesse + RPM) ; SignalRGB composants `Components/ThermalTake`.
  Confiance : **élevée** pour la famille Riing (deux implémentations indépendantes), **moyenne** pour la vitesse (un seul plugin).

## Matériel couvert (VID 0x264A)

| PID | Produit | Type | Interface | Pilote PureRGB |
|---|---|---|---|---|
| 1FA4, 1FA5 .. 1FB5 (17 PID) | Riing Plus / Floe Riing / TT RGB PLUS contrôleur | 5 ports, ventilos + RGB | HID 65 o (ID 0 + 64) | aucun (known.rs a 1FA5) |
| 2260 .. 2270, 232B | Riing Quad | 5 ports, 4 anneaux / ventilo | HID, paquet 193 o | aucun (known.rs a 2260) |
| 2135 .. 213F, 2141, 2142 | Riing Trio | 5 ports | HID, paquet 65 o | aucun |
| Poseidon Z RGB (clavier) | hors périmètre | | | |
| autres PID (Toughliquid RGB non-Ultra, Floe RC, Pacific) | **[?]** non couverts par ces sources | | | |

Les listes de PID viennent de l'enregistrement OpenRGB (`ThermaltakeRiingControllerDetect.cpp`) : chaque PID de la plage est une unité du même contrôleur (jusqu'à 16 contrôleurs dans la plage 1FA5-1FB5).

## Protocole Riing Plus (1FA4..1FB5) [V : OpenRGB + SignalRGB]

Tous les paquets : ID rapport `00` suivi de 64 octets (65 en tout), réponse lue juste après (timeout 100 ms OpenRGB).

| Action | Octets (après l'ID `00`) |
|---|---|
| Init | `FE 33` |
| Firmware | `33 50` ; version = octets 2.3.4 de la réponse |
| **RGB d'un port** | `32 52 <port 1..5> <mode + (vitesse & 3)> <couleurs G,R,B ...>` |
| **Vitesse ventilo** | `32 51 <port 1..5> 01 <pourcentage 0..100>` [V SignalRGB] |
| **Lecture ventilo** | `33 51 <port 1..5>` ; RPM = (réponse[7] << 8) + réponse[6] (indexation SignalRGB : à vérifier avec ou sans l'octet ID) |

- Couleur : ordre **G,R,B** par LED. Mode direct par LED : octet mode `0x18` (SignalRGB envoie `0x18` + 54 LED max/port en canal « Inline ») ; le plugin envoie aussi 40 LED en pulsation.
- Modes matériels (octet « mode ») : flow `00`, spectrum `04`, ripple `08`, blink `0C`, pulse `10`, wave `14`, per-LED `18`, full `19`. Vitesse (2 bits bas) : extrême `0`, rapide `1`, normal `2`, lent `3`.
- 5 ports ; 54 LED max annoncés par port ; limite totale du plugin 270.
- **Détection de ventilos** : SignalRGB envoie d'abord 75 % à chaque port (« burst »), lit le RPM, et ne crée le contrôle que si RPM > 0. Relecture toutes les 3 s.
- Plage de consigne : 0..100 % ; plancher matériel [?] (le plugin n'en impose pas).

## Riing Quad (2260..2270, 232B) [V : OpenRGB]

- Paquet de **193 octets** (`THERMALTAKE_QUAD_PACKET_SIZE`) ; octet 1 commande, 2 fonction, 3 zone (port), 4 mode, 5+ données ; commande RGB `32 52` ; mode **direct = `0x24`** ; vitesse 2 bits bas (0 extrême .. 3 lent).
- Init `FE 33` ; keepalive ~3 s [V `THERMALTAKE_QUAD_KEEPALIVE`] ; 5 canaux ; couleurs envoyées pour tout le canal en une trame (4 anneaux/ventilo : layout non détaillé ici).
- Ventilateurs Quad : la source OpenRGB n'en parle pas ; la vitesse/RPM par `32 51` / `33 51` est **probablement identique** [S] ; à valider.

## Riing Trio (2135..2142) [V : OpenRGB]

- Paquet de 65 octets ; envoi par morceaux (octet 5 = `03`, mode `0x24`, chunk id) ; 5 canaux ; init `FE 33`. Vitesse/RPM : même remarque [S].

## Toughliquid / Floe (AIO)

- Toughliquid (non Ultra) et Floe DX/RC utilisent le contrôleur Riing/TT RGB Plus ou un en-tête ARGB 5 V de carte mère : aucun PID propre trouvé. Toughliquid Ultra : LCD (voir screens-protocols). **[?]** : pompe, RPM pompe.

## Accès bas niveau

- HID uniquement (interface 0, `endpoint.interface == -1 || 0` selon SignalRGB). TT RGB PLUS doit être fermé.

## INCONNU

- Logiciel TT RGB PLUS non analysé : modes matériels exacts du Quad/Trio, profils, courbes en température, contrôle de la pompe Toughliquid.
- Décalage d'octets de la lecture RPM ; plancher de vitesse ; commandes pour Floe/Toughliquid ; contenu exact des 193 octets du Quad (zones/LED par ventilo).

## Difficulté de réimplémentation

Riing Plus : **faible** (4 commandes courtes). Quad/Trio : **faible-moyenne** (LED, paquets plus longs). Toughliquid : **inconnue**.

## Tests d'octets possibles sans matériel

- `00 FE 33`, `00 33 50`, `00 32 51 01 01 4B` (port 1, 75 %), `00 33 51 01`, `00 32 52 01 18 <G R B x n>` ; longueurs 65 (Plus/Trio) / 193 (Quad).
