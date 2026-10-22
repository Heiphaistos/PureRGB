# Lian Li L-Connect 3 : protocole des hubs ENE (SL/AL/SL-Infinity/V2), TL, Galahad II (Trinity et LCD), Strimer

Date : 2026-10-23. **[V]** = lu dans le logiciel officiel (fichier + classe/offset), **[S]** = supposé, **INCONNU** = non confirmé.
Cette note complète `lianli.md` (qui reposait sur lian-li-linux/OpenRGB/liquidctl) avec une lecture du logiciel officiel.
Aucun code ni asset copié : seuls des faits de protocole (identifiants, octets, plages) sont repris.

## 1. Sources analysées (extraction sans exécution)

| Logiciel | Source | Taille | SHA-256 |
|---|---|---|---|
| L-Connect 3 **v2.1.29** (2024-09-24) | `https://lianli-update-2025.lianli-cn.com/L3_CX/20260924-L-Connect%203-x64-v2.1.29-bc599de7.exe` (lien de la page officielle `lian-li.com/l-connect3/`) | 1 515 066 872 o | `2e6535db640ef3ba3185ca5f81a33316abe97e599cd890e3983974a3774c6138` |
| L-Connect 3 **v1.7.4** (2023-10-31, édition « Unifan Redragon ») | `https://lianli-update.oss-cn-beijing.aliyuncs.com/L3_CX/L-Connect_3_V1.7.4_X64.ZIP` (zip), exe interne 109 611 096 o | zip 109 271 660 o | zip `13214a9cc293c3aa4d9b6dc2245b66e12834ed680ba83770594ade30a68f5a46` ; exe `2171a8b4482816d36ecbbbe6248d73f769d0e6759289c276bd3b8b74f96d29f7` |

- **v1.7.4** : NSIS + `app-64.7z` : application **Electron/Vue** (`app.asar`, node-hid) ; extraction 7z `-tNsis` puis `npx asar extract`. Les trames sont du JavaScript lisible.
  Contient SL, AL, SL-Infinity, SL V2, AL V2, SL Redragon, Strimer Plus, Galahad II Trinity (via `GALAHAD.dll`, non désassemblée). Pas de TL, pas de sans-fil.
- **v2.1.29** : application **.NET/WPF** (réécriture). 7-Zip 26.02 ne sait PAS ouvrir cet installeur NSIS (en-tête NSIS 3 « solide par morceaux » : en-tête
  LZMA puis ~1078 flux LZMA de 2 Mo indépendants, chacun commençant par `5D 33 37 23 00 00`). Contournement : décompression de chaque flux LZMA en Python (`lzma`
  FORMAT_ALONE), recollage, découpe en fichiers `[u64 longueur][octets]` (1063 fichiers, dont les assemblies .NET). Aucun exécutable lancé. Les noms de fichiers sont perdus
  (non reconstruits) ; les assemblies ont été identifiées par contenu. Décompilation `ilspycmd`. Assemblies utiles : « LConnectCore » (couche appareils, 563 Ko, 591 fichiers
  `LConnectCore.Products.*`), « LConnectService » (contrôleurs, 2,5 Mo), plus des outils d'écran/éditeur (hors sujet).
  **Aucune authentification, licence ou DRM n'a été contournée** (l'installeur est public et aucun jeton n'a été lu).
- Le firmware des hubs ENE (SL/AL...) et la DLL native `GALAHAD.dll` (v1.7.4) n'ont pas été analysés (INCONNU pour le détail interne MCU).

## 2. Matériel couvert : identifiants officiels (table `KnownDevices` de LConnectCore, v2.1.29 [V])

Convertis en hexadécimal. VID `0CF2` = ENE ; `0416` = Winbond/Nuvoton ; `04FC` ; `1CBE` ; `1A86` (CH340) ; `43A8` ; `39B0`.

### 2.1 Contrôle RGB + ventilateurs (périmètre de PureRGB)

| VID:PID | Type officiel | Usage page / usage exigé | Remarque |
|---|---|---|---|
| 0CF2:A100 | **SLFan** (Uni Hub SL) | aucun filtre | v1.7.4 accepte aussi **0CF2:7750** (SL d'origine, `app.js` module `7c25`/classe SL, liste `[41216, 30544]`) |
| 0CF2:A101 | **ALFan** (Uni Hub AL) | aucun filtre | |
| 0CF2:A102 | **SLInfinity** | aucun filtre | |
| 0CF2:A103 | **SLV2Fan** | aucun filtre | |
| 0CF2:A104 | **ALV2Fan** | aucun filtre | |
| 0CF2:A105 | **SLV2AFan** (SL V2 « A », firmware 0.5) | aucun filtre | même pilote que SLV2 (`SLV2FanDevice`), firmware MinorID 199 |
| 0CF2:A106 | **SLRedragon** | aucun filtre | pilote = SL (`SLRedragonDevice : SLFanDevice`), firmware MinorID 200 |
| 0CF2:A200 | **StrimerPlus** | aucun filtre | Strimer Plus / L Connect filaire |
| 0416:7372 | **TLFan** (hub Uni Fan TL) | usage page `FF1B` (65307) obligatoire | `ControllerFinder.cs` |
| 0416:7371 | **Galahad2TrinityPerformance** | usage page `FF1B` ou usage `0x91` | pompe 4200 tr/min max |
| 0416:7373 | **Galahad2TrinityRegular** | usage page `FF1B` ou usage `0x91` | pompe 3200 tr/min max |
| 0416:7391 | **Galahad2LCD** | aucun | Galahad II LCD/Vision, protocole sec. 6 |
| 0416:7395 | **Galahad2LCDV2** | aucun | idem |
| 0416:7398, 7399, 739A | HydroShiftLCD, HydroShiftLCDRGB, HydroShiftLCDTL | aucun | non détaillés ici |
| 04FC:7393 | TLLCD | aucun | écran TL (hors RGB) |
| 0416:8040 / 8041 | WirelessControllerTransmitter / Receiver (V1) | | sans fil, non détaillé |
| 1A86:E304 / E305 | Transmitter / Receiver V2 | | sans fil, non détaillé |
| 0416:8050 | UniversalScreen8p8InchLighting | | anneau LED de l'écran |
| 0416:8051, 8052 | HydroShiftIIOLEDCurveLighting / Lighting2 | | LED OLED Curve |
| 0416:3F01 | TLControllerUpgradeToolUSBDeviceID | | mode mise à jour firmware du hub TL |

### 2.2 Autres produits listés par le logiciel (identifiants seulement, hors périmètre RGB filaire)

1CBE:0005 SLV3LCD ; 1CBE:0006 TLV2LCD ; 1CBE:A021 / 1A86:AD20 HydroShiftII LCD Circle (+ mode affichage) ; 1CBE:A034 / 1A86:AD22 HydroShiftII LCD Square ;
1CBE:A065 / 1A86:AD11 Lancool 207 Digital ; 1CBE:A088 / 1A86:AD21 Universal Screen 8.8" ; 1CBE:A092 / 1A86:AD26 VM 9.2" ; 1CBE:A068 / 1A86:AD23 HydroShiftII OLED Curve ;
1CBE:A018 TL FLEX LCD ; 1CBE:A019 SL-INF FLEX LCD ; 43A8:0105 P28 V2 ; 43A8:0101 TL FLEX ; 39B0:0101 / 0104 Edge Hub Advanced (avec / sans température).
Source : `LConnectCore.Devices/KnownDevices.cs`. Protocoles non lus dans cette passe : **INCONNU**.

## 3. Famille ENE 6K77 : SL, AL, SL-Infinity, SL V2 / V2A / Redragon, AL V2 (PID A100 a A106)

Source : `LConnectCore.Products.Ene6K77Fan/{SLFanDevice,ALFanDevice,SLInfinityDevice,SLV2FanDevice,ALV2FanDevice,Constants,*FirmwareVersion}.cs` et
`LConnectService.Products.Ene6K77Fan/*Controller.cs` (v2.1.29). **Confirme intégralement** les commandes de `lianli.md` ; précisions et corrections en 3.8.

### 3.1 Transport [V]

- ID de rapport unique **0xE0**. Quatre types d'échange :
  - **feature report** (`HidD_SetFeature`) : `[E0][cmd]...` pour toutes les commandes de réglage ;
  - **output report** (`WriteFile`) : uniquement les données couleur `[E0][0x30|port][R B G ...]` ;
  - lecture : **get input report** (ID E0, tampon de 65 o ; les données utiles commencent à l'octet 1) après une commande « lecture » en feature report.
- Délai d'écriture après chaque rapport : **20 ms** (`writeDelayTime`), temporisation de lecture 10 ms, d'écriture 1000 ms.
- v1.7.4 (Electron) envoyait tout via `node-hid` `device.write(tableau)` (output report) et lisait avec `getInputReport(0xE0, 65)` : les deux versions du logiciel officiel
  utilisent donc E0 ; la distinction feature/output de la v2 est la plus précise.
- Longueur réelle des rapports HID (descripteur) : **INCONNU** (le logiciel envoie des tableaux de la taille exacte : 3 à 8 o en feature, 2+3N en output).

### 3.2 Identification du matériel (firmware) [V]

Feature `E0 50 01` puis lecture de 5 octets : `customer, project, major, minor, fine`.

| Modèle | customer | project | major | minor (décimal / hex) |
|---|---|---|---|---|
| SL | E0 | 50 | 0x64 | 194 (C2) |
| AL | E0 | 50 | 0x80 | 195 (C3) |
| SL-Infinity | E0 | 50 | 0x80 | 196 (C4) |
| SL V2 | E0 | 50 | 0x64 | 197 (C5) ; **199 (C7)** = V2A (A105), version forcée selon `fine` |
| AL V2 | E0 | 50 | 0x80 | 198 (C6) |
| SL Redragon | E0 | 50 | 0x64 | 200 (C8) |

Version (octet `fine`) : SL `(fine>>4)*10+(fine&15)` puis major=/10 ; AL `fine<8 -> 1.0` sinon `(fine>>4)*10+(fine&15)+2` ; AL V2/SL V2/SL-Inf : `(fine>>4)*10+(fine&15)`.
(v1.7.4 : AL RPM lisible seulement si firmware >= 1.4.)
Note : v1.7.4 testait seulement `80 C3` pour AL, SL-Inf et AL V2 (anciens firmware) ; la v2 distingue C3/C4/C6.

### 3.3 Nombre de ventilateurs : pas de détection matérielle [V]

- **Aucune commande ne détecte le nombre de ventilateurs.** L'utilisateur le règle dans l'interface (« Set Quantity of Fan ») ; défaut **3 par groupe**.
  Maximum : SL / AL / SL-Inf / SL Redragon **4** ; SL V2 / AL V2 **6** (`Constants.cs`). Le contrôleur n'accepte `0..max` ; une présence se déduit d'un RPM non nul (v1.7.4 : `if (currentRpm == 0) continue`).
- Déclaration au hub (feature) : **SL, SL V2, A105, Redragon** : `E0 10 <sub> <(groupe<<4)|qty> 00 00` avec sub = **0x32 (50)** pour SL/Redragon et **0x60 (96)** pour SL V2 ;
  **AL** : `E0 10 40 <groupe+1> <qty> 00` ; **SL-Inf, AL V2** : `E0 10 60 <groupe+1> <qty> 00`.
  (Correction de `lianli.md` : SL V2 utilise bien la forme `(groupe<<4)|qty`, SL-Inf et AL V2 la forme `groupe+1, qty`.)
- 4 groupes (« ports » physiques 0 a 3) par hub.

### 3.4 LED par ventilateur et trames de couleur [V]

| Modèle | LED / ventilateur | Ports d'effet (octet bas de `E0 1p` et `E0 3p`) | Palette (modes animés) | Couleurs statiques par LED (sortie `E0 3p`) |
|---|---|---|---|---|
| SL, SL Redragon | **16** | `p` = groupe (0..3) | 16 couleurs = 4 groupes de 4 couleurs (la même palette de 4 répétée) | **64** couleurs = 4 ventilateurs x 16 (une couleur par ventilateur répliquée 16 fois) |
| SL V2 / A105 | **16**, jusqu'à 6 ventilateurs | `p` = groupe (0..3) | 24 couleurs | **96** couleurs = 6 x 16 |
| AL | **8 intérieures + 12 extérieures** (extérieur = 4 coins x 3) | intérieur `p = 2*groupe`, extérieur `p = 2*groupe+1` (8 ports) | 16 couleurs (4x4) ; mode « colorful » 48 | intérieur **32** (4 x 8) ; extérieur **48** (4 x 12) |
| SL-Infinity | **8 + 12** | idem AL (8 ports) | 16 couleurs | intérieur 32 ; extérieur 48 |
| AL V2 | **8 + 12**, jusqu'à 6 ventilateurs | idem AL | 36 ou 72 couleurs (6 x 6, 6 x 12) | intérieur **48** (6 x 8) ; extérieur **72** (6 x 12) |

- **Ordre des octets d'une LED : R, B, G** (rouge, bleu, vert) : `SLFanDevice.SetColorSetting` écrit `color.R, color.B, color.G`. Confirmé aussi dans v1.7.4 (`0f7a`, fonction `l(e)` renvoie `[r, b, g]`).
- Trame : output report `E0 | (0x30|port) | N x 3 octets`. Les LED d'un ventilateur sont contiguës, les ventilateurs se suivent dans l'ordre de la chaîne. Les ventilateurs absents sont remplis de noir.
- Les modes animés (arc-en-ciel, etc.) se pilotent par une **palette** (couleurs ci-dessus) + une commande d'effet ; seuls `StaticColor` (mode 1) et les variantes « Static » utilisent la trame **par LED**.
  Le logiciel ne propose pas de rendu temps réel LED par LED : aucune cadence d'animation côté hôte. Contrôle fin par LED en mode statique : **CONFIRME** (une trame = une image figée).
- **Limite de luminosité totale (v1.7.4, `0f7a`)** : la somme R+G+B d'une couleur est ramenée à **459** (SL/AL V1) avec mise à l'échelle proportionnelle (`459/somme`), ou **390** pour la seconde fonction ; v2 : non retrouvée (INCONNU si conservée).
- Trame d'**effet** (feature) : `E0 | (0x10|port) | mode | vitesse | direction | luminosité`.
  - Vitesse : lent `02`, `01`, normal `00`, rapide `FF`, très rapide `FE`. Direction : droite `00`, gauche `01`.
  - Luminosité : 100 % `00`, 75 % `01`, 50 % `02`, 25 % `03`, minimum `04` (le contrôleur v2 remplace `04` par `08`), **éteint `08`**.
- Valider les changements : **commit** feature `E0 60 <hi> <lo>` avec valeur **1** (SL, AL, SL-Inf, AL V2) ou **4** (SL V2), appelé après les trames de couleur/effet de tous les groupes.
  Lecture : `E0 55 05` (SL, SL-Inf, AL, AL V2) ou `E0 50 05` (SL V2) puis lire 2 octets.

### 3.5 Octets de mode d'effet (tables officielles v2) [V]

Les modes « _Merge » enchaînent les 4 groupes en une seule bande (voir 3.6). Les noms sont ceux de `LightingMode.cs`.

- **SL / Redragon** (un seul jeu) : StaticColor 01, Breathing 02, RainbowMorph 04, Rainbow 05, Staggered 18 (0x18=24), Tide 1A (26), Runway 1C (28), Mixing 1E (30), Stack 20 (32), StackMulti 21 (33),
  Neon 22 (34), ColorCycle 23 (35), Meteor 24 (36). Fusion : Runway_Merge = 28, Meteor_Merge = 36.
- **SL V2** : mêmes valeurs + Voice 38 (0x26), Groove 39, Render 40, Tunnel 41 ; fusions : Meteor 42, Runway 43, Tide 44, Mixing 45, StackMulti 46.
- **AL** (anneau tout) : StaticColor 1, Breathing 2, Rainbow 40, RainbowMorph 53, Taichi 44, ColorCycle 43, Runway 26, Meteor 25, Warning 45, Voice 46, SpanningTeacups 56, Tornado 54, Mixing 47, Stack 48,
  Staggered 55, Tide 49, Scan 50, Contest 51 ; variantes séparées intérieur / extérieur (Rainbow 5, Breathing 2, ColorCycle 24, Runway 26, MopUp 27, Meteor 25, MeteorRainbow 8, PacMan 39 intérieur seul, etc.) ;
  fusions Scan 50, Contest 51.
- **SL-Infinity** (tout) : StaticColor 1, Breathing 2, BreathingRainbow 6, Meteor 25, Runway 26, Disco 35, Rainbow 50, Warning 54, Voice 55, Mixing 56, Stack 57, Tide 58, Scan 59, Door 60, ElectricCurrent 65,
  HeartBeat 66, HeartBeatRunway 67, MopUp 68, RainbowMorph 63 ; fusions 70 a 78 (Runway 70, MopUp 71, Mixing 72, Stack 73, Tide 74, Scan 75, Door 76, HeartBeatRunway 77, ElectricCurrent 78).
- **AL V2** (tout) : StaticColor 1, Breathing 2, RainbowMorph 4, Meteor 25, Runway 26, Rainbow 43, ColorCycle 46, Taichi 47, Warning 48, Voice 49, Mixing 50, Tide 51, Scan 52, Contest 53, ColorfulCity 56, Render 57, Twinkle 58,
  Wave 59, Spring 60, TailChasing 61, MopUp 62, Tornado 63, Staggered 64, SpanningTeacups 65, ElectricCurrent 66, Stack 67 ; fusions 68 a 79.
- Les tables intérieur / extérieur complètes (AL, SL-Inf, AL V2) sont dans `LightingModeValueLookup` de chaque contrôleur : non recopiées ici (reprendre l'énumération si un mode précis est voulu).

### 3.6 Fusion (« merge ») des groupes [V]

- SL / Redragon : début feature `E0 10 33 00 01 02 03 08`, fin `E0 10 34 00 00 00`.
- SL V2, AL V2, SL-Inf : ordre `E0 10 63 <g0> <g1> <g2> <g3> 08` (permutation des groupes).
- AL : `E0 10 43 <1|0> 00 00`.
- En mode fusion, seul le port 0 reçoit la configuration ; les ports 1..7 reçoivent un effet vide (Rainbow, luminosité 08) pour SL-Inf (`setMergeLighting`).

### 3.7 Vitesse et RPM [V]

- **Lecture RPM** : feature `E0 50 00`, puis get input report ; octets 1.. :
  - SL, AL, SL-Inf, Redragon : **4 x u16 big-endian** aux octets 1..8 (groupe 0 = octets 1-2) ;
  - SL V2, AL V2 : 4 x u16 BE aux octets **2..9** (un octet de décalage).
  Le logiciel interroge toutes les **1000 ms**.
- **Consigne** : feature `E0 <0x20|groupe> 00 <duty>`. Le logiciel la renvoie toutes les 1000 ms (sinon la valeur ne persiste pas, sauf synchro carte mère).
  - **L'octet `duty` est un POURCENTAGE 0..100 de la vitesse maximale** pour les deux logiciels officiels (v1.7.4 : SL, AL, Redragon envoient la valeur de la courbe telle quelle ;
    SL-Inf, SL V2, AL V2 envoient `floor(100 * rpm / maxRpm)`).
  - v2 : SL / AL : courbe par défaut `(temp CPU -> %) 0:10, 25:20, 40:50, 55:62, 70:75, 90:100` (le duty est donc directement le pourcentage).
    SL V2 / AL V2 : `duty = (rpm == 0 ? 1 : max(10, round(rpm / maxRpm * 100)))` ; SL-Inf identique. **Minimum de fonctionnement : 10 %** ; **`1` = arrêt** (le logiciel envoie 1 quand la cible est 0).
  - Vitesse max de référence : SL-Inf 2100 (120 mm) / 1600 (140 mm) ; SL V2 / AL V2 2000 (120 mm) / 1600 (140 mm) ; v1.7.4 : 120 mm 2000, 140 mm 1800, SL-Inf 120 mm 2100, 140 mm 1600.
    Le matériel ne renvoie **pas** la taille : choix utilisateur.
  - **Tranche la divergence de `lianli.md`** : le logiciel officiel envoie le pourcentage (formule lian-li-linux), pas l'octet calculé de liquidctl. Ce sont donc des
    pourcentages 0..100 ; liquidctl compense probablement un firmware plus ancien. Test matériel recommandé (RPM pour 25/50/75/100) pour confirmer.
- **Synchro vitesse carte mère (PWM)** : feature `E0 10 <sub> <b>` avec sub = **0x31** (SL/Redragon), **0x42** (AL), **0x62** (SL-Inf, SL V2, AL V2), et pour chaque groupe g :
  `b = (1 << (g+4)) | (sync << g)` (donc `0x11<<g` activé, `0x10<<g` désactivé, un envoi par groupe).
- **Synchro éclairage ARGB carte mère** : feature `E0 10 <sub> <on>` avec sub = **0x30** (SL), **0x41** (AL), **0x61** (SL-Inf, SL V2, AL V2).
- **Repérer un groupe (ping/locate)** : SL / Redragon `E0 (10|g) 11 <vitesse FF> 00 <luminosité 02>` ; AL `E0 (10|2g) 34 ...` ; SL-Inf `E0 (10|2g) 3E ...` ; AL V2 `E0 (10|2g) 36 ...`
  (ou `E0 10 50 <g>` en mode fusion) ; SL V2 `E0 10 2F <g>` en fusion sinon `E0 (10|g) 11 ...`.

### 3.8 Écarts avec `lianli.md`

- `E0 55 05` / `E0 50 05` (frame) et le sous-code de ping n'y figuraient pas ; maintenant documentés.
- Le sous-code de déclaration du nombre de ventilateurs et sa forme sont maintenant exacts par modèle.
- L'octet de consigne est un pourcentage (voir 3.7).
- Valeurs de luminosité : `04` est remplacé par `08` dans la v2 (« Lowest -> Off »).
- SL V2A (A105) et Redragon (A106) sont des variantes de SL V2 / SL ; le pilote SL V2 gère C5 et C7.

## 4. Strimer Plus (0CF2:A200) [V : StrimerPlusDevice.cs, Constants.cs]

- 2 canaux, **12 ports**, 6 ports par canal, jusqu'à **27 LED par port**. Canal 2 : nombre de ports lu par feature `E0 50 01` puis octet 0 = 4 ou 6 (sinon -1 = invalide).
- Mêmes enveloppes que SL : effet `E0 (10|port) mode vitesse direction luminosité` ; couleurs output `E0 (30|port) R B G ...` ; délai **40 ms**.
- Activation d'effet : feature `E0 (20|port) 00 00` (port < 12), ou `E0 2C <mask hi> <mask lo>` pour plusieurs ports (>= 12).
- Firmware : `E0 50 00` puis 5 octets. Trame : `E0 55 05` (lecture), `E0 60 <hi> <lo>` (validation, envoyée seulement si hi = 0 et lo dans 0x0F..0x3C ou 0x50..0xEB).
- Modes : « Complete » (un effet sur un canal entier) et « Individual » (par port) : voir `LightingMode.cs` pour les noms ; octets de mode : **INCONNU** (non extraits).

## 5. Hub TL (0416:7372) [V : LConnectCore.Products.TLFan/*, LConnectService.Products.TLFan/TLFanController.cs]

### 5.1 Transport et détection

- HID, **rapport de sortie 64 o, ID 1** : `[01][cmd][00][pkt_hi][pkt_lo][len]` + 58 o max de données ; la réponse a la même enveloppe (lecture 64 o, délai 100 ms).
  Les longues données sont découpées en morceaux de 58 o (compteur de paquet).
- Interface : usage page **0xFF1B** (la v2 filtre dessus).
- **4 ports**, jusqu'à 16 ventilateurs par port ; `MaxLEDFanCount = 4` ventilateurs LED ; `MaxLCDFanCount = 3` pour le port LCD ; 31 groupes d'éclairage maximum.
- **Détection automatique des ventilateurs** : commande `A1` (handshake) ; la réponse est une liste de blocs de 3 octets (un par ventilateur possible) :
  `[bit7 détecté | bit6 en mise à jour | bits5-4 port | bits3-0 index][RPM hi][RPM lo]`.
  Le nombre de ventilateurs d'un port = `max(index détecté)+1`. Le RPM de chaque ventilateur vient de cette seule réponse (à interroger périodiquement).
- **Identification du modèle de ventilateur** : commande `A6` (info produit) par ventilateur (`data = [esclave=1][(port<<4)|index]`), deux réponses : texte ASCII à 6 champs séparés par `,`
  (chip, boot, type, ModelA, **ModelB**, version) et la date. `ModelB` : préfixe `TL` = LED, `TLC` = LCD, `TLF` = LED « reverse », `TLFC` = LCD « reverse » ; deux chiffres `12` = 120 mm, `14` = 140 mm.
  Firmware du contrôleur : même commande avec esclave = 0.

### 5.2 Commandes (octet `cmd`) [V : LEDCommands.cs]

| cmd | Nom | Données |
|---|---|---|
| A1 | handshake | aucune |
| A2 | liste des modes | (non utilisée ici) |
| A3 | éclairage d'UN ventilateur | 20 o : `[(port<<4)|sync][(port<<4)|fan][mode][luminosité][vitesse][4 x RVB = 12 o][direction][désactivé][nb couleurs]`, soit octets 0 et 1, 2, 3, 4, 5-16, 17, 18, 19 |
| A6 | info produit / firmware | `[esclave][(port<<4)|fan]` |
| A7 | test mode LED | |
| AA | **consigne vitesse d'UN ventilateur** | `[(port<<4)|fan][pwm]` |
| AD | groupement de ventilateurs | `[n° groupe][nb][1 octet par ventilateur : bit7 côté haut, bit6 côté bas, bits5-4 port, bits3-0 index]` |
| AE | direction d'un ventilateur | `[(port<<4)|fan][bit1 échange haut/bas | bit0 gauche/droite]` |
| AF | direction d'un port | `[(port<<4)][1|0]` |
| B0 | éclairage d'un GROUPE | 20 o : `[00][n° groupe][mode][luminosité][vitesse][4 x RVB][direction][désactivé][nb couleurs]` (mêmes positions que A3) |
| B1 | synchro vitesse carte mère | `[sync<<7 | port<<4 | fan]` |
| B2 | commutation LCD | |
| B3 | test éclairage | `[type 1 ventilo / 2 groupe / 3 port][numéro][1 si bas]` |
| B4 | faire clignoter un port | `[port]` |

- **Ordre des couleurs : R, G, B** (différent des hubs ENE).
- **Mode** = énumération `% 1000` : Rainbow 1, RainbowMorph 2, StaticColor 3, Breathing 4, Runway 5, Meteor 6, ColorCycle 7, Staggered 8, Tide 9, Mixing 10, Voice 11, Door 12, Render 13, Ripple 14, Reflect 15,
  TailChasing 16, Paint 17, PingPong 18, Stack 19, CoverCycle 20, Wave 21, Racing 22, Lottery 23, Intertwine 24, MeteorShower 25, Collide 26, ElectricCurrent 27, Kaleidoscope 28.
  Familles : base (0xxx), `_Merge` (1xxx, sur un groupe fusionné), `_Side` (2xxx, face latérale seulement), `_Side_Merge` (3xxx). Seul l'octet `% 1000` est envoyé : la face (haut/bas) s'exprime par le groupe.
- Luminosité et vitesse : octets 0..4 (défauts 2 ; **luminosité 0 = ventilateur éteint** (champ « désactivé » mis à 1) ; luminosité 4 = pleine au démarrage). Sémantique exacte des échelons de vitesse : INCONNU.
- Direction : `0` droite/horaire, `1` gauche/anti-horaire, `2` haut, `3` bas, `4` écarté, `5` rassemblé.
- **LED par ventilateur / contrôle par LED : INCONNU** : le protocole n'expose que mode + palette de 4 couleurs par ventilateur ou groupe (pas de trame LED par LED trouvée).

### 5.3 Vitesse [V]

- Consigne `AA [(port<<4)|fan][pwm]` par ventilateur. **`pwm` = 12..100 (pourcentage) ; `1` = arrêt** (valeur envoyée quand la courbe vaut 0) ; ventilateur absent mais attendu : 1.
  Renvoi toutes les 1000 ms (seulement si la valeur change).
- Le logiciel convertit un RPM voulu en pwm par une table de calibration : ventilateur LED 120 mm, 19 points `rpm -> %` : 200 ->10, 360 ->15, 530 ->20, 680 ->25, 830 ->30, 980 ->35, 1120 ->40, 1250 ->45,
  1400 ->50, 1530 ->55, 1660 ->60, 1800 ->65, 1920 ->70, 2030 ->75, 2160 ->80, 2290 ->85, 2390 ->90, 2490 ->95, 2600 ->100.
  Limites RPM : LED 120 mm 200..2600 ; LED 140 mm 200..1800 ; LCD 120 mm 350..1900 ; LCD 140 mm 400..1600 (`Constants.cs`).
- Synchro carte mère : `B1`.

## 6. Galahad II Trinity (0416:7371 / 7373) et Galahad II LCD (0416:7391 / 7395)

### 6.1 Trinity : même enveloppe 64 o ID 01 que le TL [V : Galahad2TrinityDevice.cs, Constants.cs]

| cmd | Nom | Données |
|---|---|---|
| 81 | handshake | réponse 4 o : `[RPM ventilos u16 BE][RPM pompe u16 BE]` |
| 86 | firmware | deux réponses ASCII (version CSV + date) |
| 83 | éclairage pompe | 19 o : `[portée 0 int / 1 ext / 2 tout][mode%1000][luminosité][vitesse][4 x RVB][direction][désactivé][source ARGB 0 MCU, 1 carte mère, 2 application]` |
| 85 | éclairage ventilos | 20 o : `[mode][luminosité][vitesse][4 x RVB][direction][désactivé][source][synchro pompe][nombre de LED]` (24 par défaut, 8..50) |
| 8A | consigne pompe | `[source 0 MCU / 1 carte mère][pwm 0..100]` |
| 8B | consigne ventilos | `[sync 0/1][pwm 0..100]` (la valeur est limitée à 100) |

- **R, G, B** (pas R B G). Echelons de luminosité/vitesse du logiciel : 0..100 par pas de 25 (octet envoyé : 0..4 probable, défaut 2) : mapping exact **INCONNU**.
- **Pompe** : pwm 30 % = 2200 tr/min, 100 % = 4200 (Performance) ou 3200 (Regular) tr/min ; interpolation linéaire entre ces points. Plage de sécurité : ne pas descendre sous 2200 tr/min (pwm 30).
  Ventilateurs du radiateur : pwm min 10 %. Courbe par défaut ventilos : temp 0:10, 25:20, 40:50, 55:62, 70:75, 90:100, 100:100.
  Défaut pompe Performance : fixe 3200 tr/min ; Regular : 2200 (<=30 C) a 3200 (>=70 C).
- Modes ventilos : Rainbow 1, RainbowMorph 2, StaticColor 3, Breathing 4, Runway 5, Meteor 6, Vortex 7, CrossingOver 8, TaiChi 9, ColorfulStarryNight 10, StaticStarryNight 11, Voice 12, BigBang 13, Pump 14,
  ColorsMorph 15, Bounce 16. Pompe : intérieur 1-6, 9-12, 14 ; extérieur (1001..) + Bounce 16 ; tout (2001.. + Vortex 7, CrossingOver 8, BigBang 13, ColorsMorph 15).
- Pas de capteur de température de liquide sur Trinity (aucune lecture dans le handshake).

### 6.2 Galahad II LCD / Vision (7391, 7395) [V : LConnectCore.Products.Galahad2Vision/*]

- Trois enveloppes : **A** (ID 01, 64 o, identique à Trinity), **B** (ID 02, sortie **1024 o**, entrée 512 o) et **C** (ID 03, 512 o) :
  B = `[02][cmd][taille totale u32 BE][n° paquet u24 BE][longueur u16 BE][charge <= 1013 o]` ; C identique mais 512 o (charge 501). Commandes B/C découpées en morceaux.
- Commandes A (comme Trinity, étendues) : 81 handshake (**7 o** : RPM ventilos u16, RPM pompe u16, température valide 1 o, entier 1 o, fraction 1 o : **température du liquide disponible**), 82/84 lecture état LED pompe/ventilos,
  83 éclairage pompe (portée, mode 1..16 : Rainbow, RainbowMorph, Static, Breathing, Runway, Meteor, TickerTape, Fluctuation, Transmit, ColorfulStarryNight, StaticStarryNight, Voice, BigBang, Burst, ColorsMorph, Bounce),
  85 éclairage ventilos (mode 1..6), 86 firmware, 88/89 numéro de série, 8A/8B pwm pompe/ventilos (0..100, plage 0..100), 8E reset.
- Commandes B : 0A activer firmware, 0B envoyer firmware, **0C contrôle LCD** (8 o : mode 0 UI locale / 1 application / 2 H.264 local / 3 AVI local / 4 réglage / 5 test, luminosité 0..100,
  rotation 0/90/180/270, test, couleur de test RVB, fps vidéo), **0D envoi d'une image H.264** (flux continu), 0E JPEG, 0F envoi JPEG, 12/13/16 AVI / H.264 / animation de démarrage,
  **14 couleurs LED** (`[1 = ventilos | 0 = écran]` suivi de RVB ; pour l'écran 24 octets de réglage avant), 15 info H.264, 17 écran disponible.
- Ecran : **480 x 480, 24 images/s**, 12 LED d'anneau d'écran. Firmware >= 1.5 : trames H.264 envoyées en mode « 512 o » (enveloppe C).
- Format exact du flux H.264 / JPEG et des octets d'en-tête image : INCONNU (hors RGB, non lu). Le contrôle RGB seul (ventilos et pompe) suit la table ci-dessus.

## 7. Ce qui reste INCONNU

- Longueurs des rapports HID (descripteurs) ; usage page / usage des hubs ENE et Strimer (la v2 ne les filtre pas).
- Contenu de `GALAHAD.dll` (v1.7.4) et du firmware des MCU ; tables intérieur/extérieur complètes d'AL, SL-Inf et AL V2 (non recopiées).
- Sémantique des échelons vitesse/luminosité du TL et Trinity (octets 0..4) ; nombre de LED par ventilateur TL / Trinity / HydroShift ; trame par LED pour le TL.
- Strimer : octets de mode exacts ; sans fil (dongles 8040/8041, E304/E305) ; écrans autres que Galahad II LCD ; Edge Hub, P28 V2, TL FLEX.
- Limite de somme RVB de la v2 (v1.7.4 : 459 / 390).

## 8. Difficulté de réimplémentation

- Hubs ENE (SL, AL, SL-Inf, V2, Redragon) : **faible** (une enveloppe, quelques sous-commandes, tables de modes disponibles) ; difficulté principale : variantes par modèle (3.3, 3.7) et
  tests matériels du pourcentage de consigne.
- Strimer : **moyenne-faible**. TL et Galahad Trinity : **moyenne** (enveloppe 64 o, handshake, fusions, direction) ; Galahad II LCD : **moyenne-élevée** (H.264/JPEG, enveloppes B/C).

## 9. Tests d'octets possibles sans matériel

- ENE SL : feature `E0 20 00 32` (groupe 0 à 50 %) ; `E0 21 00 0A` (groupe 1 à 10 %) ; `E0 50 01` ; couleur statique d'un groupe : `E0 30` + 64 x (R,B,G) ; effet `E0 10 01 00 00 00` (static, normal, droite, 100 %).
- SL V2 : taille de sortie couleur = 2 + 96 x 3 = 290 o ; SL : 2 + 64 x 3 = 194 o ; AL intérieur : 2 + 32 x 3 = 98 o, extérieur : 2 + 48 x 3 = 146 o ; AL V2 : 2+48x3 et 2+72x3.
- Déclaration ventilos : SL groupe 2, 3 ventilos : `E0 10 32 23 00 00` ; AL groupe 0, 4 ventilos : `E0 10 40 01 04 00` ; SL-Inf groupe 1, 3 ventilos : `E0 10 60 02 03 00`.
- Synchro vitesse carte mère SL, groupe 1 on : `E0 10 31 22 00 00`.
- TL : handshake `01 A1 00 00 00 00` (64 o) ; consigne ventilo port 1 index 2 à 40 % : `01 AA 00 00 00 02 12 28` ; décoder un handshake fabriqué (3 o par ventilateur) ; ordre RGB du TL et du Trinity : R,G,B.
- Trinity : `01 8B 00 00 00 02 00 4B` (ventilos 75 %) ; décoder un handshake 4 o `0A F0 0C 80` (ventilos 2800 tr/min, pompe 3200 tr/min).
- Conversion rpm -> pwm TL : vérifier la table 19 points (interpolation) ; pompe Trinity : pwm = 30 + (rpm - 2200) * 70 / (max - 2200).
