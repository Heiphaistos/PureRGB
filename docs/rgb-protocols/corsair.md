# Corsair iCUE : hubs, ventilateurs, AIO (RGB + vitesse)

Date : 2026-10-22. Légende : **[V]** lu dans la source citée, **[S]** supposé/déduit, **[?]** INCONNU.
Périmètre : refroidissement/boîtier. Déjà natif dans PureRGB (expérimental) : Lighting Node Pro/Core (0C0B/0C1A), non redocumenté ici.
Les écrans LCD Corsair sont dans `docs/screens-protocols/corsair.md` (hors sujet ici).

## Source analysée

- **Logiciel officiel iCUE (5.x)** : NON téléchargé ni décompilé dans cette passe (installeur de plusieurs centaines de Mo, service + pilotes
  noyau ; non ouvert faute de temps, voir « Inconnu »). Aucun SHA-256 d'installeur à citer.
- **Sources de protocole utilisées (faits seulement, rien copié)** :
  - `liquidctl` cloné dans `D:\Projet\Analyses\rgb\cooling\liquidctl` (GPL-3.0+) : `driver/commander_core.py`, `commander_pro.py`,
    `hydro_platinum.py`, `asetek.py` (liste de PID seulement).
  - OpenRGB (GPL-2.0+) `D:\Projet\Analyses\rgb\openrgb-src\Controllers\CorsairCommanderCoreController` et `CorsairICueLinkController`
    (RGB seulement, pas de ventilateurs).
  - OpenLinkHub (GPL-3.0+) `D:\Projet\Analyses\rgb\cooling\OpenLinkHub\src\devices\lsh\lsh.go` (vitesse/RPM/température iCUE LINK).
- Aucune de ces sources n'est une spécification Corsair : confiance **moyenne-élevée** (projets actifs, testés sur matériel par leurs auteurs).

## Matériel couvert (VID 0x1B1C)

| PID | Produit | Type | Interface HID | Pilote PureRGB |
|---|---|---|---|---|
| 0C10 | Commander Pro | hub 6 ventilos + 4 sondes + 2 canaux RGB | HID 64 o, ID 0 | aucun (à faire) |
| 0C0B / 0C1A | Lighting Node Pro / Core | RGB seul | HID | natif (expérimental) |
| 1D00 | Obsidian 1000D (hub intégré) | même protocole que Commander Pro [V liquidctl] | HID | aucun |
| 0C1C | Commander Core | hub 6 ventilos + pompe AIO + 6 ports RGB | HID, usage page 0xFF42, usage 0x01 [V OpenRGB] | aucun |
| 0C32 | Commander ST (« Core 2 ») | idem Core, paquets 65 o (OpenRGB) | idem | aucun |
| 0C1D, 0C3C, 0C3D, 0C3E | variantes Commander Core (OpenRGB les range dans le même pilote) | [S] mêmes commandes | idem | aucun |
| 0C2A | Commander Core XT | hub 6 ventilos, sans pompe ; port RGB externe | idem, paquets 385 o côté OpenRGB | aucun |
| 0C3F | iCUE LINK System Hub | chaîne iCUE LINK (ventilos QX/LX/RX, AIO, pompes, sondes) | HID 513 o écriture / 512 o lecture, usage page 0xFF42 | aucun |
| 0C18, 0C19, 0C17 | Hydro H100i Platinum / Platinum SE / H115i Platinum | AIO USB | HID 64 o | liquidctl (sidecar) |
| 0C29, 0C20, 0C21, 0C22 | H60i / H100i / H115i / H150i Pro XT | AIO USB, sans LED ventilos | HID 64 o | liquidctl |
| 0C35, 0C36, 0C37, 0C40, 0C41 | iCUE H100i/H115i/H150i Elite RGB (et White) | AIO USB | HID 64 o | liquidctl |
| 0C02, 0C03, 0C07, 0C08, 0C09, 0C0A | Hydro H80i GT / H100i GTX / H110i GTX / H80i v2 / H100i v2 / H115i | AIO Asetek | HID, protocole Asetek | liquidctl |

Écarts avec `src-tauri/src/backends/hid/known.rs` : `0C12` (« Hydro H150i Pro ») absent de liquidctl et d'OpenRGB, non confirmé ;
`0C21` est « H115i Pro XT » chez liquidctl (et non « Platinum / Pro XT ») ; `0C35` est « H100i Elite RGB » (et non « Elite Capellix » générique) : **à corriger/valider**.

## Famille A : Commander Core / Core XT / ST (0C1C, 0C32, 0C2A)

### Trames (communes avec iCUE LINK) [V : commander_core.py + CorsairCommanderCoreController.cpp]

- Taille de rapport : liquidctl 96 o utiles + ID 0 (97 o) ; OpenRGB 97 o (Core), 65 o (Core 2/ST), 385 o (XT), **1025 o pour un 0C1C en firmware majeur 1**
  (détecté via la réponse à `02 13`).
- Écriture : `[00][08][cmd0][cmd1][données…]` (le premier `00` est l'ID de rapport). Lecture : relire jusqu'à un rapport dont l'octet 0 vaut `00` ;
  l'octet 1 de la réponse est l'écho de `cmd0` (liquidctl l'impose).

| Action | Octets (après `08`) |
|---|---|
| Réveil (mode logiciel) | `01 03 00 02` |
| Veille (retour au mode matériel) | `01 03 00 01` |
| Version firmware | `02 13` ; réponse : octets 3,4,5 = majeur.mineur.patch |
| Ouvrir un endpoint | `0D 00` + `<mode>` |
| Fermer l'endpoint | `05 01 00` |
| Lire (3 paquets) | `08 00 01`, `08 00 02`, `08 00 03` ; type de données dans les octets 3..4 de la 1re réponse |
| Écrire, 1er paquet | `06 00` + `[len LE16 = taille + len(type)][00 00][type 2 o][données]` |
| Écrire, suivants | `07 00` + données |

Mode ouvert = objet lu/écrit [V] :

| Mode | Type | Contenu |
|---|---|---|
| `17` | `06 00` | RPM : octet 0 = N, puis N x u16 LE |
| `21` | `10 00` | températures : octet 0 = N ; N x (statut `00` = connecté, u16 LE = °C x 10) |
| `1A` | `09 00` | ports ventilo connectés (octet par port, `07` = connecté) |
| `20` | `0F 00` | nombre de LED : octet 0 = N ; N x (u16 LE état, `2` = connecté ; u16 LE nb LED) |
| `60 6D` | `03 00` | **mode de vitesse** par canal : `00` fixe %, `02` courbe |
| `61 6D` | `04 00` | **consigne fixe** : octet 0 = N ; N x u16 LE pourcentage 0-100 |
| `62 6D` | `05 00` | **courbe** par canal : octet capteur, octet nb points, points (u16 LE température x10, u16 LE %) ; 2 à 7 points |

- Écriture sûre (liquidctl) : lire l'objet, modifier le canal, réécrire l'ensemble (sinon les autres canaux sont cassés).
- Canaux de vitesse : indice 0 = pompe si AIO, puis ventilos 1..6 ; Core XT (sans pompe) : indice 0 = ventilo 1.
- Liquidctl étiquette 0C1C/0C2A/0C32 « broken » (issues #520, #583, #598, #623, #705) : fiabilité de l'écriture des vitesses à valider sur matériel.

### RGB (Commander Core) [V : OpenRGB]

- Passer en mode ventilateur « 6 QL » (34 LED/port) : endpoint `1E 00`, type `0D 00`, 15 octets :
  `07 | 01 08 (Core AIO) ou 00 00 (XT sans port externe) / 01 01 (XT port externe) | (01 06) x 6`, puis réveil `01 03 00 02`.
- Nombre de LED d'un port : endpoint `1D 00`, type `0C 00`, octets `07 <n>`.
- **Couleurs directes** : endpoint `22 00`, type `12 00`, 3 octets/LED (R,G,B) ; 699 octets (Core/ST) ou 1224 (XT) ; le premier appareil (pompe/AIO)
  occupe son nombre réel de LED, les suivants un **pas fixe de 34 LED** (padding de 3 x (34 - n) octets).
- Cadence mini ~33 ms ; **sans trame ~10 s le firmware retombe en mode matériel** : renvoyer la dernière trame ou `02 13` (keepalive).
- Fin de session : `01 03 00 01`.

## Famille B : iCUE LINK System Hub (0C3F)

- Rapport 513 o écriture (ID `00`, octet 2 = `01`, commande dès l'octet 3) / 512 o lecture ; en-têtes 3 o (lecture) et 4 o (écriture) [V OpenRGB `CorsairICueLinkProtocol.h`].
- Commandes : réveil `01 03 00 02`, veille `01 03 00 01`, firmware `02 13` (octets 4,5 puis u16 LE 6..7), ouvrir endpoint `0D 01`, fermer `05 01 01`,
  lire `08 01`, écrire `06 01` ; pour les couleurs : ouvrir `0D 00`, écrire `06 00` puis suites `07 00` par morceaux de 508 o.
- Endpoints (octet « mode ») : `36` liste des appareils (type `21 00`), `21` températures (`10 00`), `17` vitesses (`25 00`), `18` réglage de vitesse (`07 00`),
  `22` couleurs (`12 00`), `20` nombre de LED, `1D`/`1E` codes, `28`/`29` volts/ampères PSU, `40` modèles [V OpenLinkHub].
- **Lecture des appareils** : réponse de `36` : octet 6 = nombre de canaux, puis blocs de 8 octets + identifiant (longueur à l'octet 7 du bloc) ;
  octet 2 = type, octet 3 = modèle. Table OpenRGB : (05,02) 5000T 160 LED ; (05,01) 9000D 22 ; (11,xx) TITAN 20 ; (07,xx) H100i/H115i/H150i/H170i 20 ;
  (06,00) cooler pump LCD 24 ; (02,00) LX RGB 18 ; (01,00) QX RGB 34 ; (0F,00) RX RGB 8 ; (03,00) RX RGB MAX 8 ; (09,00) XC7 Elite 24 ;
  (0C,00) XD5 Elite 22 ; (0E,00) XD5 Elite LCD 22 (interne, ignorer) ; (19,00) XD6 Elite 22 ; (0D,00) XG7 RGB 16 ; (04,00)/(13,00) RX/RX MAX 0 LED ; (0A,00) XG3 0 LED.
- **Lecture vitesses** (OpenLinkHub) : octet 6 = nombre de capteurs ; à partir de l'octet 7 triplets (statut `00` OK, u16 LE rpm). Températures `21` : triplets (statut, u16 LE °C x 10).
- **Consigne de vitesse** : écrire sur `18`, type `07 00`, données `[N]` puis N x `[canal, 00, pourcent, 00]` (4 octets/canal). Réponse octet 3 non nul = échec
  (OpenLinkHub réessaie jusqu'à 20 fois, 100 ms). Minimums appliqués par OpenLinkHub : 20 % ventilos, 50 % pompe.
- **Couleurs** : endpoint `22`, type `12 00`, RVB packé pour toute la chaîne (somme des LED des appareils, ordre des canaux).

## Famille C : Commander Pro / Lighting Node (0C10, 0C0B, 0C1A, 1D00)

Rapports de 64 o (ID `00` + 64 o ; réponse lue sur 16 o) [V : commander_pro.py].

| Commande (octet 1) | Rôle |
|---|---|
| `02` / `06` | firmware (octets 1..3) / bootloader (octets 1..2) |
| `10` | config sondes de température (octets 1..4 = connecté) |
| `11 <n>` | température sonde n (0-3) : u16 BE / 100 °C |
| `12` | tensions |
| `20` | modes des 6 ventilos : `00` absent, `01` DC, `02` PWM |
| `28 02 <ventilo> <mode>` | fixer le mode DC/PWM d'un ventilo |
| `21 <n>` | **RPM** du ventilo n (0-5) : u16 BE aux octets 1..2 |
| `23 <n> <pct>` | **consigne fixe** 0-100 % |
| `25 …` | courbe (26 o) : octet 0 ventilo, 1 sonde, 2..13 = 6 températures °C x100 BE, 14..25 = 6 RPM BE (courbe en **RPM**, pas en %) ; point critique 60 °C, RPM max de référence 5000 (« je ne sais pas si c'est bon », dit la source) |
| `37 <canal>`, `34 <canal>`, `38 <canal> 01` | reset canal, début d'effet, canal en mode logiciel |
| `35 <canal> <début> <nb> <mode> <vitesse> <dir> <aléa> FF <couleurs>` | effet matériel (jusqu'à 3 couleurs) |
| `33 FF` | valider |

- Modes matériels : rainbow `00`, color_shift `01`, color_pulse `02`, color_wave `03`, fixed `04`, visor `06`, marquee `07`, blink `08`, sequential `09`, rainbow2 `0A`.
  Vitesse `00` rapide / `01` moyen / `02` lent ; direction `01` avant / `00` arrière.
- LED : 2 canaux (Commander Pro, Node Pro), 1 (Node Core) ; 204 LED max par canal ; jusqu'à 8 effets cumulés.
- Le mode direct par pixel du Commander Pro n'est pas décrit par liquidctl : [?] (un plugin SignalRGB `Corsair_Lighting_Commander_Pro.js` existe, non relu).

## Famille D : Hydro Platinum / Pro XT / Elite RGB (0C17..0C41)

[V : hydro_platinum.py] Rapport 64 o : `[00][3F][seq<<3 | feature][cmd][données…][CRC-8 des octets 2..62]` ; la réponse a un CRC-8 vérifiable (CRC nul).

- Cooling : feature `0` (ou `3` pour le 3e ventilo), cmd `14`. Statut : feature `0`, cmd `FF`.
- Statut (réponse) : liquide = octet 8 + octet 7/255 °C ; ventilo 1 : % (octet/255) à l'octet 14, rpm u16 LE à 15 ; ventilo 2 : 21/22 ; ventilo 3 : 42/43 ; pompe : 28/29.
- Charge « set cooling » (60 o) : préfixe `00 FF 05 FF FF FF FF FF` ; longueur de profil 7 à l'offset `0x1D-3` ; par ventilo : mode (`00` courbe, `01` courbe + sonde externe,
  `02` % fixe, `04` RPM fixe) aux offsets `0x0B-3` / `0x11-3`, consigne 0-255 à +5, courbe 7 points (temp, duty 0-255) aux `0x1E-3` / `0x2C-3` ; mode pompe `00` silence,
  `01` équilibré, `02` extrême à `0x17-3` (`FF` sur le 2e paquet des modèles 3 ventilos).
- Éclairage : 16 LED sur la tête + LED ventilos (4/ventilo Platinum, 16 Platinum SE, 0 sinon) ; couleurs en **B,G,R**, 20 LED par paquet (cmd `100`/`101`/`110`).
  Avant le premier envoi : 3 paquets d'activation (features `001`, `010`, `011`) dont le contenu est une constante « magique » non expliquée.
- Ré-initialiser à chaque démarrage et reprise de veille.

## Accès bas niveau

- Tout est USB HID (pas de SMBus). Le service iCUE détient les HID : détecter et demander de le fermer ; rendre la main (`01 03 00 01`) en quittant.

## INCONNU

- Logiciel iCUE non analysé : modes matériels du Commander Core, correspondance LED de chaque ventilo (QL/LL/SP), écriture de courbes iCUE LINK, mises à jour firmware.
- Taille de rapport réelle sous Windows (96 vs 97/1025/385) ; sémantique des PID 0C1D/0C3C/0C3D/0C3E ; contenu des paquets d'activation Hydro Platinum ; protocole Asetek.
- iCUE LINK : « time warp » (cmd `37`, type `35 00`), LED power (`15 01`) : constantes vues, non décrites.

## Difficulté de réimplémentation

Commander Pro **faible** ; Commander Core **moyenne** (endpoints, lecture/écriture sûre, keepalive) ; iCUE LINK **moyenne-élevée** (énumération de chaîne, tables) ; Hydro Platinum **moyenne** (CRC-8, offsets).

## Tests d'octets possibles sans matériel

- Construire `[00 08 02 13]`, `[00 08 01 03 00 02]`, `[00 08 0D 00 61 6D]` ; contrôler les longueurs (97 / 65 / 385 / 1025 / 513).
- Premier paquet d'écriture : longueur LE16 = len(données) + len(type) ; champ `00 00` ; type en tête.
- Commander Pro : courbe 26 o (températures x100 BE, RPM BE) ; consigne `23 n pct`.
- Hydro : CRC-8 poly standard sur octets 2..62 ; trame reçue + CRC doit redonner 0.
- iCUE LINK : `[N]` suivi de `[canal 00 pct 00]` x N, taille `4N+1`.
