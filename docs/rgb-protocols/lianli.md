# Lian Li L-Connect 3 : Uni Fan SL/AL/SL-Infinity/SL V2, TL, Galahad II Trinity, Strimer (LED, vitesse, synchro)

Date : 2026-10-22. **[V]** lu dans la source citée, **[S]** supposé, **[?]** INCONNU.
Les écrans (Galahad II LCD, HydroShift, Lancool 207, Universal Screen...) sont dans `docs/screens-protocols/lian-li.md` et `IMPL-lianli.md` : non repris ici.

## Source analysée

- **L-Connect 3 v2.1.29** (page `https://lian-li.com/l-connect3/`), installeur `20260924-L-Connect 3-x64-v2.1.29-bc599de7.exe` téléchargé depuis le miroir CDN du site officiel :
  1 515 064 950 o utiles (~1445 Mo), NSIS (« NullsoftInst »), SHA-256 `2e6535db640ef3ba3185ca5f81a33316abe97e599cd890e3983974a3774c6138`.
  **Extraction impossible avec 7-Zip 26.02** (flags NSIS non reconnus : il ne voit qu'un blob `[0]` de 1,5 Go ; `-tNsis` : « Is not archive »). Installeur supprimé ; **aucun octet
  de protocole n'a été tiré du logiciel officiel**. Prochaine tentative : version portable/un autre extracteur NSIS, ou installation en VM.
- **Sources utilisées (faits seulement, rien copié)** :
  - `sgtaziz/lian-li-linux` (MIT) cloné dans `D:\Projet\Analyses\rgb\cooling\lian-li-linux` : `crates/lianli-devices/src/{ene6k77,tl_fan,galahad2_trinity.rs,strimer_plus.rs,wireless}`, `lianli-shared/src/device_id.rs`.
    Le projet dit avoir utilisé l'IA pour analyser captures et logiciel constructeur : confiance **moyenne-élevée**, à recouper.
  - OpenRGB (GPL-2.0+) `Controllers/LianLiController` (RGB seul) ; liquidctl `driver/lianli_uni.py` (GPL-3.0+, vitesse des hubs).
  - Écart notable entre deux sources, signalé plus bas (consigne de vitesse des hubs ENE).

## Matériel couvert

| VID:PID | Produit | Famille | Interface HID | Pilote PureRGB |
|---|---|---|---|---|
| 0CF2:7750 | Uni Hub (SL d'origine) | ENE 6K77 (liquidctl : « SL ») | HID, usage page 0xFF72, usage 0xA1, feature+output ID 0xE0 | aucun |
| 0CF2:A100 | Uni Hub SL | ENE 6K77 | idem | aucun (known.rs OK) |
| 0CF2:A101 | Uni Hub **AL** | ENE 6K77 | idem | aucun |
| 0CF2:A102 | Uni Hub SL Infinity | ENE 6K77 | idem | aucun |
| 0CF2:A103 | Uni Hub SL V2 | ENE 6K77 | idem | aucun |
| 0CF2:A104 | Uni Hub AL V2 | ENE 6K77 | idem | aucun |
| 0CF2:A105 | Uni Hub SL V2 v0.5 / « V2A » | ENE 6K77 | idem | aucun |
| 0CF2:A106 | SL « Redragon » | ENE 6K77 | idem | aucun |
| 0CF2:A200 | Strimer L Connect / Strimer Plus filaire | ENE (voir ci-dessous) | idem | aucun |
| 0416:7372 | Hub UNI FAN TL | Nuvoton, rapports 64 o ID 01 | usage page 0xFF1B = interface de contrôle | aucun |
| 04FC:7393 | hub TL (variante) | [S] même famille | [?] | aucun |
| 0416:7371 / 7373 | Galahad II Trinity Performance / Regular | AIO pompe + ventilos, sans LCD | usage page 0xFF1B | aucun |
| 0416:8050 | anneau LED Universal Screen | HID RGB | [?] | aucun |
| 0416:8051 | HydroShift II OLED Curve (MCU LED : pompe, liquide) | HID | [?] | aucun |
| 0416:8040 / 8041 ; 1A86:E304 / E305 | dongles TX/RX sans fil (SL V3, TL V2, CL/RL...) V1 / V2 | RF | USB, voir plus bas | aucun |
| 0416:7391, 7395, 7398, 7399, 739A | Galahad II LCD/Vision, HydroShift (LCD+pompe) | HID rapports A/B/C | voir screens-protocols | écran seulement |

**Écart `known.rs`** : il donne `0CF2:A100` = « Uni Hub **AL** » ; OpenRGB, liquidctl et lian-li-linux disent A100 = **SL**, A101 = **AL**. À corriger.

## Famille ENE 6K77 (hubs filaires SL / AL / SL-Infinity / SL V2 / AL V2) [V : ene6k77/*, lianli_uni.py, OpenRGB]

- Un seul ID de rapport : **0xE0**. Commandes en **feature report** (`[E0][cmd]...`), données couleur en **output report** `[E0][30|port][...]`, états lus par **get input report** ID E0.
- 4 groupes (« ports ») indépendants ; 4 ventilos max/groupe (SL, AL, SL-Inf), **6** pour SL V2 / AL V2 / V2A.
- Délai d'1 commande ~20 ms (Strimer : 40 ms).

### Init / lecture firmware
- `E0 50 01` puis lecture 5 octets : customer, project, major, minor, fine. Attendu : customer `E0`, project `50`, (major, minor) : SL `64 C2`, SL Redragon `64 C8`, AL `80 C3`,
  SL-Inf `80 C4`, SL V2 `64 C5` ou `64 C7`, AL V2 `80 C6`.
- Déclarer le nombre de ventilos par groupe (important pour le RPM et les effets) : feature `E0 10 <sub> <groupe+1|(groupe<<4)|qty> ...` ; sous-commande selon modèle :
  `40` (AL), `60` (V2/SL-Inf), `32` (SL), forme `E0 10 40 <groupe+1> <qty> 00` (AL) ou `E0 10 60/32 <(groupe<<4)|qty> 00 00`. Par défaut 3 ventilos/groupe.

### Vitesse et RPM
- **RPM** : feature `E0 50 00` puis lecture : SL/AL/SL-Inf : 8 octets, **4 x u16 BE** (RPM par groupe, octets 0..7) ; SL V2/AL V2 : 9 octets, 4 x u16 BE à partir de l'octet 1. Le RPM est la somme/moyenne du groupe [S].
- **Consigne** : feature `E0 <0x20|groupe> 00 <valeur>` (4 groupes).
  - lian-li-linux envoie un **pourcentage 0..100** (conversion `duty*100/255`).
  - liquidctl envoie `224, groupe+31, 0, octet` avec un **octet calculé** : SL/AL `(800+11p)/19` (p=0 -> 40) ; SL-Inf `(250+17,5p)/20` (p=0 -> 10) ; V2 `(200+19p)/21` (p=0 -> 7), après avoir
    mis le canal en manuel : feature `E0 10 <49|66|98> <0x10<<(canal-1)>` (SL=49/0x31, AL=66/0x42, autres=98/0x62) ; mode auto (synchro carte mère PWM) = `0x11<<(canal-1)`.
  - **Les deux sources divergent** : INCONNU laquelle est juste par modèle ; un test matériel avec lecture RPM tranchera (mesurer RPM pour 25/50/75/100).
- **Synchro PWM carte mère** : mode « auto » ci-dessus (liquidctl l'active à l'init pour tous les canaux).

### RGB
- **Couleurs** : output report `E0 <30|port>` + triplets **R,B,G** (rouge, bleu, vert dans cet ordre) pour toute la chaîne du groupe.
  - SL / SL V2 : 16 LED/ventilo, palette 4 couleurs ; AL / AL V2 / SL-Inf : double anneau, ports 2/groupe (`port = groupe*2` intérieur 8 LED/ventilo, `groupe*2+1` extérieur 12 LED/ventilo) ; palette 4 (6 pour AL V2).
  - Statique/respiration = 1 couleur par LED répliquée ; autres modes = palette.
- **Effet** : feature `E0 <10|port> <mode> <vitesse> <direction> <luminosité>`.
  - Vitesse : lente `02`, `01`, normale `00`, rapide `FF`, très rapide `FE`. Direction : `00` G->D, `01` D->G.
  - Luminosité : 100 % `00`, 75 % `01`, 50 % `02`, 25 % `03`, 0 ("minimum") `04`, éteint `08`.
  - Modes (SL, SL V2) : statique `01`, respiration `02`, rainbow morph `04`, rainbow `05`, staggered `18`, tide `1A`, runway `1C`, mixing `1E`, stack `20`, stack multi `21`, neon `22`,
    color cycle `23`, meteor `24`, voice `26`, groove `27`, render `28`, tunnel `29`. AL / SL-Inf : variantes intérieur/extérieur (tables `map_mode_inner/outer`, non recopiées ; voir OpenRGB).
- **Commit** : feature `E0 60 <hi> <lo>` avec valeur 4 pour SL V2/V2A, 1 pour les autres modèles.
- **Fusion** (merge de groupes) : SL : `E0 10 33 00 01 02 03 08` (début), `E0 10 34 00 00 00` (fin) ; SL V2/AL V2/SL-Inf : `E0 10 63 <ordre0..3> 08` ; AL : `E0 10 43 <on> 00 00`.
- Contrôle par LED : chaque couleur est réellement envoyée à la LED (pas de gamma matériel).

### Strimer L Connect / Plus (0CF2:A200) [V : strimer_plus.rs, OpenRGB]
- 12 ports sur 2 canaux, 27 LED max/port ; canal 2 : 4 ou 6 ports détectés (requête de nombre de ports). Même schéma feature/output (`E0 ...`), délai 40 ms ; des modes « complets » couvrent un canal entier
  (n'activer que le port de base). Palette 6 couleurs arc-en-ciel injectée pour rainbow/twinkle/bullet-stack.
- Détails des octets : INCONNU dans cette note (lire `strimer_plus.rs` MIT).

## Famille TL (hub UNI FAN TL 0416:7372) [V : tl_fan/controller.rs]

- Rapports de sortie 64 o, ID `01`, en-tête 6 o : `[01][cmd][00][pkt_hi][pkt_lo][len]` + 58 o max ; chaque commande attend une réponse synchrone (lecture 100 ms ; 3 s à l'init).
- 4 ports, plusieurs ventilos/port ; adresse ventilo = `(port<<4)|index`.
- Commandes : `A1` handshake (réponse : 3 octets/ventilo = `[détecté<<7|upgrading<<6|port<<4|index][RPM u16 BE]`), `A6` info produit (firmware), `AA <adresse> <%>` **consigne 0-100 %**,
  `B1` synchro RPM carte mère (octet `sync<<7|adresse`), `AD` groupes, `A3` lumière d'un ventilo, `B0` lumière d'un groupe, `AE`/`AF` direction, `B3` test lumière, `B4` blink port.
- RPM : uniquement via la réponse du handshake (A1) ; interroger périodiquement.
- `B0` (20 o) : `[00][groupe][mode][luminosité 0..4][vitesse 0..4][4 x RVB (12 o)][direction 0..5][désactivé][nb couleurs]`. Le code source contient une 2e variante sans réponse avec décalage d'un octet : INCONNU laquelle est juste.

## Galahad II Trinity (0416:7371 / 7373) [V : galahad2_trinity.rs]

- Même enveloppe 64 o ID `01` ; pas de capteur de liquide. `81` handshake (RPM pompe + ventilos), `86` firmware, `8A` consigne pompe (0-100), `8B` consigne ventilos (0-100), `83` LED pompe, `85` LED ventilos radiateur.
- `83` (19 o) : portée (0 intérieur, 1 extérieur, 2 tout), mode, luminosité, vitesse, 4 x RVB, direction, désactivé, source ARGB (0 MCU, 1 carte mère). `85` (20 o) : mode, luminosité, vitesse, 4 x RVB, direction, désactivé,
  source ARGB, synchro pompe, nombre de LED (24 par défaut).
- Pompe : plage 2200-4200 tr/min (Performance), 2200-3200 (Regular).

## Sans fil (SL V3, TL V2, SL-Inf sans fil, AIO sans fil) via dongles TX/RX [V partiel : wireless/*]

- Dongle TX 0416:8040 / RX 8041 (V1) ; 1A86:E304 / E305 (V2, puce CH340). Les ventilos sont adressés par adresse MAC de 6 octets ; trames RF de 240 o (4 morceaux de 60 o) ; 14 emplacements RX max.
- Commandes USB : `10` envoyer RF, `11` lire MAC, `15` reset du compagnon. Commandes RF : `10` PWM (4 valeurs/appareil, renvoi toutes les 1 s sinon retour à la vitesse matériel), `12` sélection groupe,
  `14` synchro horloge, `16` reboot LCD, `19` AIO sans fil, `20` RGB (animations compressées, 12 288 o max, morceaux de 220 o), `21` paramètres AIO (32 o), `22` image, `23` fermer Wi-Fi 217, `27` synchro lumière carte mère.
- Contrôle de vitesse minimale et filtre PWM 153-155 appliqués côté hôte ; synchro PWM carte mère matérielle sur SL V3.
- Détail des octets RF, appairage (bind) : INCONNU dans cette note (source MIT `wireless/{bind,discovery,transport,rgb}.rs`, ~5000 lignes).

## Accès bas niveau

- Hubs ENE/TL/Trinity : HID natif (hidapi). Choix de l'interface via usage page (0xFF72 ENE, 0xFF1B TL/Trinity). L-Connect 3 les ouvre en continu : prévoir « Fermez L-Connect 3 ».
- Sans fil : USB brut (WinUSB/libusb) vers les dongles.

## INCONNU

- Logiciel officiel non extrait (voir Source) : modes propriétaires, courbes de température sur Trinity, formats de profils.
- Correspondance exacte pourcentage / octet de consigne sur hubs ENE (deux sources divergentes) ; plages minimales de rotation ; modes AL/SL-Inf complets ; Strimer détaillé ; protocole RF complet.

## Difficulté de réimplémentation

Hubs ENE (vitesse + RPM + effets) : **faible-moyenne** ; TL / Trinity : **moyenne** (enveloppe 64 o, handshake) ; Strimer : **moyenne** ; sans fil : **élevée** (RF, appairage, états).

## Tests d'octets possibles sans matériel

- Feature `E0 20 00 32` (groupe 0, 50 %) ; `E0 50 00` ; `E0 50 01` ; `E0 60 00 01` ; vérifier la longueur de rapport.
- Couleur d'un groupe : `E0 30` + 16 x (R,B,G) x n ventilos ; vérifier l'ordre R,B,G.
- TL : `01 AA 00 00 00 02 <adresse> <pct>` ; décoder une réponse `A1` fabriquée (3 octets/ventilo).
- Comparer les deux formules de consigne (liquidctl vs pourcentage) pour p = 0, 25, 50, 75, 100.
