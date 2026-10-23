# NZXT CAM 4.76.6 : analyse du logiciel officiel (interopérabilité PureRGB)

Date : 2026-10-23. **CONFIRMÉ** = lu dans le logiciel officiel (fichier + repère). **RECOUPÉ** = vu dans CAM ET identique à une source indépendante (liquidctl). **INCONNU** = non confirmé, jamais deviné.
Complète `nzxt.md` (même dossier), qui s'appuyait sur liquidctl/SignalRGB faute d'avoir pu ouvrir CAM. Aucun code ni asset copié ; installeur jamais exécuté.

## 1. Source analysée

| Élément | Valeur |
|---|---|
| Logiciel | NZXT CAM 4.76.6 (publié 2026-10-14 ; version précédente 4.76.5 du 2026-07-07) |
| Flux de mise à jour public (sans authentification) | `https://nzxt-app.nzxt.com/latest.yml` (electron-updater) |
| Installeur complet | `https://nzxt-app.nzxt.com/NZXT%20CAM%20Setup%204.76.6.exe` (le « bootstrapper » `NZXT-CAM-Setup.exe` de la note précédente ne contient pas la charge utile ; ce fichier-ci oui) |
| Taille | 133 424 080 o |
| SHA-256 | `4a9f27d0155f99d659b3bd45f1ab949c443e0249afe314f6876105fc6b55aa10` |
| SHA-512 (base64) | `/4JnLUZJi7yiYr4tJclcujuaftkr05NLoEw+Uv2h3V8pCLuhiJXRTSEwvhR4zUZTOLePArwNUnZYKR1hoQde8w==` : **identique** à celui de `latest.yml` (intégrité vérifiée) |
| Format | NSIS (7-Zip) contenant `$PLUGINSDIR/app-64.7z` : application Electron |
| Extraction | `7z x` puis `7z x app-64.7z` puis `npx asar extract resources/app.asar` ; aucun exécutable lancé |
| Parties utiles | (a) `app.asar` (SHA-256 `aff5b2ec…9d95`) : JS/TS renderer + main, **sans** protocole HID ; (b) `resources/app.asar.unpacked/node_modules/@nzxt/cam-core/dist/index.node` (46 Mo, DLL Rust napi-rs, SHA-256 `5d3ed9f8…6077`, cam-core 71.9.3) : **c'est elle qui parle au matériel** ; (c) `dist/` : `service.exe`, `cam_helper.exe`, `firmware-update.exe`, pilotes Asetek (`SiUSBXp`), `cam_driver_mb.sys` (carte mère), `SeaSonicsESeries64.dll`, firmwares de cartes de capture |
| Outils | 7z, asar, Ghidra 12.1.4 headless (import + analyse partielle de `index.node`), `strings`, lecture des bundles JS minifiés |
| Limite | Le code matériel est du Rust compilé sans symboles, très abstrait (async, machines à états). Les **constructeurs de trames d'écriture** (couleur, vitesse) n'ont pas pu être isolés dans le temps imparti : voir §8. |

## 2. Architecture du pilote (CONFIRMÉ)

Chemins de sources Rust présents dans les messages de panique de `index.node` (chaînes ASCII, adresses virtuelles `index.node` image base `0x180000000`).

| Module Rust (chemin dans `index.node`) | Rôle déduit | Preuve |
|---|---|---|
| `nzxt-device\src\device\usb\v1\smartdevice.rs` | **Smart Device V1** (rapport entrant d'ID `0x04`, ≥ 20 o) | VA `0x18200b9c0` ; parseur `0x1813db6d0` |
| `…\usb\v1\kraken_x.rs`, `kraken_m22.rs` | Kraken X (gén. 3, Asetek/HID) et M22 | VA `0x1820071c8`, `0x182007288` |
| `…\usb\v1\gridv3.rs`, `lumi_h7.rs`, `n7_z370.rs` | Grid V3, Lumi H7, cartes mères N7 Z370 | chaînes |
| `…\usb\v2.rs`, `v2\generic.rs`, `v2\lighting_modes.rs` | **Protocole « v2 »** : Fan Controller, LED Strip Controller (modes Static / Direct), Device ID, LCD | VA `0x18200bea0` etc. |
| `…\usb\v2_with_ack\{generic_with_ack,lcd,win32}.rs` | variante « v2 avec accusé » : messages `Ack`/`NAck`/`KeepAlive` (champs `acknowledged_report_id`, `acknowledged_subreport_id`, `counter`) | chaînes |
| `…\serial\{gridplus,hueplus}.rs`, `device\asetek\{kraken,siusbxp}.rs` | HUE+, Grid+ (série) ; Kraken Asetek via `SiUSBXp64.dll` | chaînes |
| `crates\device-classes\v2-class\…` | classe générique : capacités `lighting_v2`, `fan_cooling`, `pump_cooler`, `lcd`, `rpm`, `firmware_update` | chaînes |
| `crates\protocols\v2-protocol\…`, `hid-write-read-protocol` | encodage/décodage des messages v2 | chaînes |
| `crates\device-classes\mobo-driver-class` + `cam_driver_mb.sys` | accès carte mère (SuperIO/LPC : `CreateLpcMutex`, `QueryRpmData`, `SetDutyCycles`, `ApplyRamLightingMode`) | chaînes |

Types d'appareil énumérés dans `index.node` (énumération `DeviceType`, chaîne contiguë « GridV3 KrakenM22 KrakenX LumiH7 N7Z370 SmartDevice Hue2 Hue2Ambient N7Z390 N7Z490 N7Z590 N7B550 N5Z690 N7Z690 N7B650E N7Z790 SmartDevice2 SmartDevice3 VertagearChair KrakenX3 RGBFanController RGBFanControllerV2 RGBController H1Refresh KrakenZ KrakenElite KrakenBase » ; plus HuePlus, GridPlusRev1/RevA/Rev2, E500/E650/E850 (alimentations), Keyboard/Mouse…). **Smart Device 3 et RGB & Fan Controller V2 existent** dans CAM (absents de liquidctl).

Conséquence pour PureRGB : les appareils « v2 » (Smart Device V2/3, RGB & Fan Controller, Hue 2, Kraken 2023/Elite/Base) partagent un seul protocole à messages typés (`Device ID`, `Fan Controller *`, `LED Strip Controller *`) ; une couche commune est justifiée.

## 3. Identifiants USB (VID 0x1E71)

Source : descripteurs USB 18 o (`12 01 00 02 … 40 71 1e <PID LE>`) présents dans `index.node` (tables de détection / fixtures). Recherche exhaustive de la séquence `71 1e` suivie d'un PID 0x1000..0x3FFF ; deux copies du tableau (adresses `0x182161d23`…`0x1826…`).

| PID | Présence dans CAM 4.76.6 | Produit (nom d'après CAM/liquidctl) |
|---|---|---|
| 2001, 2002 | CONFIRMÉ | HUE 2, HUE 2 Ambient |
| 2005 | CONFIRMÉ, **nouveau** (absent de `nzxt.md`) | INCONNU (modèle non nommé par CAM) |
| 2006, 200D, 200F | CONFIRMÉ | Smart Device V2 |
| 2007, 2014 | CONFIRMÉ | Kraken X53/X63/X73 |
| 2009 | non retrouvé par cette recherche (la détection RGB & Fan peut passer par une autre table) | RGB & Fan Controller |
| 200C | CONFIRMÉ, **nouveau** | INCONNU |
| 200E, 2010 | CONFIRMÉ | RGB & Fan Controller |
| 2011, 2019, 201F, 2020 | CONFIRMÉ | RGB & Fan Controller (3+6 canaux) |
| 2012, 2021 | CONFIRMÉ | 2023 RGB Controller |
| 2016 | CONFIRMÉ, **nouveau** | INCONNU |
| 201B | CONFIRMÉ, **nouveau** | INCONNU |
| 2022, 2023 | CONFIRMÉ, **nouveaux** | INCONNU (famille « 2023 RGB Controller / RGB & Fan V2 » probable) |
| 2029 | CONFIRMÉ, **nouveau** | INCONNU |
| 300C | CONFIRMÉ | Kraken 2023 **Elite** (cohérent avec liquidctl) |
| 300E | CONFIRMÉ | Kraken 2023 (standard) |
| 3012 | CONFIRMÉ | Kraken 2024 Elite RGB |
| 3014 | CONFIRMÉ | Kraken 2024 Plus |
| 3016 | CONFIRMÉ, **nouveau** | INCONNU (Kraken récent) |
| 3008 (Kraken Z53/Z63/Z73), 2015 (H1 V2), 1711/1714/1715/170E | non retrouvés dans ce tableau (détection séparée : séries/HID anciens) | INCONNU pour CAM ; voir `nzxt.md` |

Interfaces HID / usage page : INCONNU (non lu). Le descripteur seul ne les donne pas.

## 4. Configuration, accessoires, LEDs

### 4.1 Modèle de canal LED « v2 » (CONFIRMÉ, `456.bundle.js`, module lighting V2)
- Types de canal : `NzxtAccessories`, `AddressableRgb5V`, `Rgb12V`. Le `Rgb12V` est un canal non adressable (luminosité seulement, pas de nombre de LEDs).
- Propriétés de canal : `channelId`, `channelType`, `deviceUniqueId`, `productType` (`Fan`, `LightStrip`, `PowerSupply`, `CableCombs`, `AIO`, `CaseLight`, `Ambient`, `Other`), `isConfigured`, `ledCount`, `brightness`, `syncToDeviceUniqueId`.
- `requires_user_defined_accessories` (descripteur de canal Rust) : pour les canaux adressables 5 V génériques, **c'est l'utilisateur qui déclare le nombre de LEDs** ; valeur par défaut de la bande : **10 LEDs** si le canal exige une déclaration sans valeur fournie (expression `overrideStripLEDCount … : 10` dans le bundle).
- Pour `NzxtAccessories`, la liste des accessoires et leur `numberOfLeds` **sont renvoyés par le contrôleur** (la distribution des LEDs est lue sur l'appareil via le message « LED Strip Controller Strip Layout Response » et son événement). Le JS n'a aucune table identifiant → nombre de LEDs pour les ventilateurs.
- Capacité maximale affichée par canal dans les jeux de test : `maxLeds: 96`. Plafond du tampon couleurs : constante interne (message `pushLedRgb maximum number of colors reached`), valeur INCONNUE.
- Segments d'accessoire (jeux de test, ordre d'adressage) : `Track` puis `Hub` puis `Line`. Exemple : accessoire 64 LEDs = Track 0-39 / Hub 40-55 / Line 56-63 ; 96 LEDs = Track 0-55 / Hub 56-79 / Line 80-95 ; 32 LEDs = Track 0-23 / Hub 24-31.

### 4.2 Accessoires et identifiants (CONFIRMÉ en partie)
Types connus de CAM (énumération `AccessoryType`, `456.bundle.js`) : Aer, AerV2_120/140, F120/F140 RGB, F120/F140 RGB Duo, F120/F140 RGB Hub (et Hub Case), UnderglowV2_200/300, Aurora Performance 120/140/240/280/360, Shoreline 120…420 (+ reverse, + EV2), Unibody RGB 240/280/360/420 (+Entry/CV), Coral Ultra RGB 120…360, Kraken X3 Logo, Kraken Elite V2 Ring, N9 X870E Cover…

Seuls les cas suivants ont **identifiant numérique + nombre de LEDs** dans les données de test (donc à traiter comme exemples, pas comme table officielle) :

| accessoryId | Nom | LEDs |
|---|---|---|
| 28 | UnibodyRGB280mm | 16 |
| 30 | KrakenEliteV2Ring | 24 |
| 33 | AuroraPerformance140mm | 16 |
| 35 | AuroraPerformance280mm | 24 |
| 36 | AuroraPerformance360mm | 32 |
| 45 | CoralUltraRGB140mm | 32 |
| 47 | CoralUltraRGB280mm | 64 |
| 48 | CoralUltraRGB360mm | 96 |
| 49 | N9X870ECover | 17 |

Table complète id → nombre de LEDs : INCONNU (la source de vérité est dans le firmware du contrôleur qui renvoie `numberOfLeds`). Le nombre de LEDs par ventilateur historique (Aer RGB 2 : 8 ; bande HUE+ : 10) vient de liquidctl, pas de CAM : voir `nzxt.md`.

### 4.3 Détection du nombre de canaux (CONFIRMÉ)
Le message Fan Controller renvoie `number_of_fan_channels` ; CAM refuse un appareil dépassant la limite connue (`device has N fan channels which is more than the supported number of M`, valeur M INCONNUE). Les messages `Device ID` / `Device ID V2` identifient le type d'appareil (enum `LEDController`, `FanController`, `CoolerController`, `LCDController`). Lecture de la configuration : INCONNU au niveau des octets.

### 4.4 Types de ventilateur et états (CONFIRMÉ)
- Type de ventilateur : `Unknown`, `Integrated`, `Pins3` (DC), `Pins4` (PWM) ; états de canal : `Available`, `NotConnected`, `OCPTriggered` (protection contre les surintensités déclenchée).
- Champs lus par canal ventilateur : `fan_type`, `fan_rpm`, `fan_set_speed`, `fan_actual_speed`, `voltage`, `current`, `wattage`, `noise_measurement`, `min_active_percentage` (fraction ; mise à `0.2` dans les données de test), `led_channel_idx`, `led_accessory`, `led_accessory_count`. Le rapport est découpé en deux moitiés (« FirstHalf » / « SecondHalf ») pour les contrôleurs à beaucoup de canaux.

## 5. Ventilateurs et pompes : plages de sécurité

### 5.1 Bornes appliquées par l'interface CAM (CONFIRMÉ, `456.bundle.js`, table `qi`, chaîne `"KrakenX-fan":[25,…`)
Le premier élément est le minimum implicite / valeur plancher, le second le prédicat de validation d'un pourcentage.

| Appareil / canal | Plancher | Validation du pourcentage |
|---|---|---|
| Kraken X (gén. 3, X42…X72) ventilo | 25 | ≥ 25 |
| Kraken X (gén. 3) pompe | 60 | ≥ 60 |
| `Kraken` générique (M22 etc.) ventilo / pompe | 25 / 60 | ≥ 25 / ≥ 60 |
| Kraken X3, Z3, Elite, Base, Plus V3 : ventilo | 0 | ≥ 20 |
| Kraken X3, Z3, Elite, Base, Plus V3 : pompe | 20 | ≥ 20 |
| Smart Device (V1/V2/3), RGB & Fan Controller (V1/V2), N7 cartes mères, série inconnue | 0 | 20 ≤ x ≤ 100 |
| Grid+ Rev1 / RevA | 20 | 20 ≤ x ≤ 100 |
| H1 Refresh : ventilos 1 et 2 | 0 | 20 ≤ x ≤ 100 |
| H1 Refresh : pompe | lecture seule (`READ_ONLY`) | non réglable |

Donc : **aucune consigne en dessous de 20 % sur tous les appareils, sauf Kraken gén. 3 (ventilo ≥ 25 %, pompe ≥ 60 %)**. Ces bornes sont celles du logiciel ; la borne du firmware peut être plus basse (INCONNU).

### 5.2 Courbes
- Profils : `Silent`, `Performance`, `Fixed`. Courbe = liste de points `{temperature, fanPercentage}`.
- Courbes d'usine « Silent » : (0 °C, 25 %), (35, 25), (50, 55), (55, 75), (60, 100). « Performance » : (0, 50), (35, 50), (60, 100) (données de test des cartes N7/Smart Device ; valeurs d'exemple CONFIRMÉES comme jeux de données de CAM, pas forcément pour chaque appareil).
- Kraken X (gén. 3) : une courbe ne peut pas dépasser **64 points** (message `KrakenX can not save fan curve because curve length can not over than 64`), écrite sur l'appareil (`saveCurveOnDevices`).
- Sources de température proposées : CPU, GPU, Liquid, PSU (Liquid seulement pour les appareils qui la mesurent ; PSU seulement avec une alim E-Series).
- Commande directe : `setFixedDutyCycles` (pourcentage), erreur `DutyCycleOutOfRange {got, valid_range}` : plage numérique côté Rust INCONNUE hors de la table §5.1.

### 5.3 Rapport d'état Kraken X3/Z3 (RECOUPÉ, décompilation `index.node` fonction `0x1813e4b30` + liquidctl `kraken3.py`)
Rapport entrant de 64 o, premier octet `0x75`, deuxième `0x01` ou `0x02` (longueur minimale testée : `0x40`). Offsets lus par CAM :
- octets 15 et 16 : température liquide = `octet15 + octet16 × 0,1` (constante flottante lue en `0x18200aeec`) ; invalide si octet15 ou octet16 = `0xFF` ou si le bit 0 de l'octet 63 est à 1 ;
- octets 17-18 : vitesse de pompe, entier 16 bits petit-boutiste ; octet 19 : rapport cyclique de pompe ; bits 1 et 2 de l'octet 63 = « non valide » pour ces deux champs ;
- octet 21 (`0x15`) : nombre de blocs ventilateur (0 à 2) ; bloc 1 : octets 22 (type), 23-24 (régime, 16 bits petit-boutiste), 25 (rapport cyclique), 26 ; bloc 2 en 27…31. Concorde avec liquidctl (23-24 = régime, 25 = rapport cyclique).
Autres rapports entrants décodés par cette fonction : `0x71 0x01` (≥ 22 o) et `0x71 0x03` (≥ 16 o), 12 octets bruts copiés (octets 2 à 13) plus un octet 14 : INCONNU (sémantique non établie ; liquidctl les donne comme réponses de version).

### 5.4 Rapport Smart Device V1 (CONFIRMÉ partiel)
Rapport entrant `0x04`, ≥ 20 o, octet 1 = numéro de canal (0 = ventilos 1-3 selon décodage), octets 11 et 14 : tension/consigne (selon CAM : `voltage`, `current`), octet 15 bas 4 bits = type de ventilateur (valeur < 3 = type connu, sinon invalide), octets 3-4 régime (16 bits gros-boutiste), octets 7-10 : valeurs flottantes dérivées (`octet7 + octet8 × facteur`). Non recoupé octet par octet ; à ne pas réimplémenter sans matériel (liquidctl décrit ce rapport plus clairement : voir `nzxt.md`).

## 6. Trames d'écriture (couleur, vitesse) : ce qui est établi

CAM construit ses requêtes dans `index.node` via une couche de sérialisation typée (messages `FieldSet`, « FieldSet corrupted (this is a bug) », champs bitfield). Les octets d'en-tête des requêtes **n'ont pas pu être extraits de façon fiable** (voir §8).

| Point demandé | Statut | Détail |
|---|---|---|
| Couleur Smart Device V2 / RGB & Fan / Hue 2 | INCONNU dans CAM | Source tierce (liquidctl `smart_device.py`) : `22 10 <canal> 00` + 60 o RGB puis `22 11 …`, mode `22 A0 …`, activation `22 03 <canal> 08`. NON confirmé dans CAM |
| Vitesse ventilateur Smart Device V2 | INCONNU dans CAM | liquidctl : `62 01 <1 << canal> 00 00 00 + consigne`. NON confirmé dans CAM |
| Couleur Kraken X3 / Z3 / 2023 | INCONNU dans CAM | liquidctl : en-tête `2A 04 <canal> <canal>`. NON confirmé dans CAM |
| Vitesse pompe/ventilo Kraken | INCONNU dans CAM | côté CAM : commande « kraken cooling curve » (`Failed to send kraken cooling curve command message to device`) |
| Mode statique vs direct (LED v2) | CONFIRMÉ (existence) | messages typés « LED Strip Controller Static Mode Information » et « Direct Mode Information » : le protocole v2 distingue modes matériels (statique) et temps réel (direct) ; format INCONNU |
| Lecture de la configuration | CONFIRMÉ (existence) | `DeviceInfoRequest` / `DeviceInfoRequestV2`, `DeviceLightChannelInfoRequest`, `Fan Controller Information`, `Redetect Initial` (redétection), `Detailed Noise Measurement` ; octets INCONNUS |
| Accusés / keep-alive | CONFIRMÉ (existence) | `Ack`, `NAck {unknown_report_id, unknown_subreport_id}`, `KeepAlive {keep_alive_count}` pour la variante v2_with_ack ; la présence de `acknowledged_report_id/subreport_id/counter` indique des messages à (rapport, sous-rapport) |
| Format de message v2 | CONFIRMÉ (principe) | adressage par (ID de rapport, sous-rapport) : octet 0 = ID (ex. `0x71`, `0x75`), octet 1 = sous-ID ; décodage conditionné par la longueur reçue (« Content length mismatch… Received length ») |

## 7. Carte mère, RAM, alimentation (hors périmètre, présence seulement)
`cam_driver_mb.sys` + `mobo-driver-class` : lecture de régimes, `SetDutyCycles` sur canaux nommés `CPU_FAN`, `AIO_PUMP`, `SYS_FAN1..5`, `W_PUMP`, `MOS_FAN`, `NZXT_8PIN1/2` (cartes N7), et `ApplyRamLightingMode`. Alimentations E-Series : `SeaSonicsESeries64.dll`. Registres exacts : INCONNU.

## 8. Ce qui est INCONNU (liste honnête)

1. Octets des requêtes d'écriture couleur / vitesse / mode pour Smart Device V2, RGB & Fan, Hue 2, Kraken (X3, Z3, 2023, Elite) : non extraits. Cause : Rust async sans symboles, sérialisation générique ; tentative de remonter depuis les chaînes de panique : n'atteint que des fonctions de gestion d'état. Piste suivante : chercher les constructeurs via les types `LightingSettings`/`CoolingSettings` dans une session Ghidra complète (analyse complète > 1 h), ou capture USB sur matériel.
2. Table complète accessoire id → nombre de LEDs (renvoyée par le matériel).
3. Modèles associés aux PID 2005, 200C, 2016, 201B, 2022, 2023, 2029, 3016.
4. Interface HID / usage page / longueurs de rapports de sortie.
5. Plafond de LEDs par message / par canal côté firmware (seul le 96 des données de test est vu).
6. Bornes de rapport cyclique côté firmware (§5.1 = bornes UI).
7. Écrans LCD Kraken (hors sujet ici, voir `docs/screens-protocols/nzxt.md`).
8. Séquence d'initialisation exacte (liquidctl donne `60 03`, `10 01`, `20 03` pour le Smart Device V2 ; non confirmé dans CAM).

## 9. Difficulté de réimplémentation (estimation)

| Famille | Difficulté | Remarque |
|---|---|---|
| HUE 2 / Smart Device V2 / RGB & Fan (protocole `22`/`62`/`60` de liquidctl) | faible-moyenne | déjà natif en partie dans PureRGB ; CAM n'ajoute rien de confirmé sur les octets |
| Kraken X3/Z3 statut (0x75) | faible | offsets §5.3 recoupés |
| Kraken 2023/Elite (v2 avec accusé) | élevée | protocole typé non extrait ; ne pas le deviner |
| Lecture de la liste d'accessoires | moyenne | message « Strip Layout Response » à décoder (INCONNU) |

## 10. Tests d'octets possibles sans matériel

- Décodage du rapport Kraken 0x75 : fixture 64 o avec octet15 = 30, octet16 = 5 → 30,5 °C ; octets 17-18 = `E8 03` → 1000 tr/min ; octet 19 = 50 ; octet 63 = 0 pour champs valides, bit 0 = 1 → température invalide.
- Table §5.1 : tests unitaires de clamp (Kraken gén. 3 : ventilo ≥ 25, pompe ≥ 60 ; Kraken X3/Z3/Elite : ≥ 20 ; autres : 20-100 ; H1 Refresh pompe refusée).
- PID : test de présence de chaque PID du §3 dans la table de PureRGB (`known.rs`) ; 300C = Kraken 2023 Elite, 300E = Kraken 2023.

## 11. Nettoyage
Installeur, extraits, projet Ghidra et scripts supprimés de `D:\Projet\Analyses\rgb\nzxt\` (seule cette note est conservée).
