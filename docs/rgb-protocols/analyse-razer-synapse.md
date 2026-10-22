# Razer Synapse 3 / Razer Central / Chroma : analyse d'interopérabilité

Document de faits (VID/PID, octets, dimensions). Aucun code ni asset Razer copié. Complète `razer.md` (trame, CRC, effets, déjà documentés). Légende : **[SYN]** = relevé dans les fichiers de Synapse ; **[OGB]** = OpenRGB commit `bbc7747` (`Controllers/RazerController/`) ; **[ORZ]** = OpenRazer (`driver/`). Tout fait sans source précise est `INCONNU`.

## 1. Source analysée

Rien n'a été installé ni exécuté. Téléchargé le 2026-10-23, extrait avec `7z`, `ilspycmd` et `strings` dans `D:\Projet\Analyses\rgb\razer\`.

| Élément | Version | Origine | Taille (o) | SHA-256 |
|---|---|---|---|---|
| Stub `RazerSynapseInstaller.exe` | Synapse 4 (le lien Synapse 3 y redirige) | `https://rzr.to/synapse-4-pc-download` puis `https://dl.razerzone.com/drivers/Synapse4/RazerSynapseInstaller.exe` | 14 923 464 | `e4eabf1a0cf030b773a1a14ed22f4cb337c5f85638863260215b6315d83eb570` |
| `RazerSynapseSetup_v3.9.1231.112613.exe` (Synapse 3, NSIS) | 3.9.1231.112613 | `https://cdn.razersynapse.com/1732600232FFkCuOA7RazerSynapseSetup_v3.9.1231.112613.exe` | 164 493 216 | `59755c454136f72db59fc338c5666bb728417084b11c43cb7dd158e767d7ca23` |
| `RazerSynapseConfigurationDataSetup_v3.9.1130.112605.exe` (NSIS) | 3.9.1130.112605 | `https://cdn.razersynapse.com/17325863935tg0xvJmRazerSynapseConfigurationDataSetup_v3.9.1130.112605.exe` | 7 578 944 | `669825c8d5092f114d9982c12a5207b8363fe5f6fb15ce1c772b41142be31424` |
| `RazerCentral_v7.17.0.732.exe` (Setup Factory) | 7.17.0.732 | `https://cdn.razersynapse.com/17323272942R1wGyAsRazerCentral_v7.17.0.732.exe` | 120 782 992 | `befb944b790afd0d1a2c8d31d442dd0fd5f8569f4dd003906b316d463b0dd5a1` |

- Le site officiel ne propose plus Synapse 3 : `https://www.razer.com/synapse-3` ne donne que Synapse 4, et `https://rzr.to/synapse-3-pc-download` redirige vers Synapse 4 (en-têtes HTTP relevés). Les trois URL `cdn.razersynapse.com` (domaine de distribution de Razer) viennent de la page du paquet Chocolatey `razer-synapse-3` 1.22.0.737 (source tierce) ; fichiers téléchargés tels quels, sans authentification ni contournement. Signature Authenticode : non vérifiée.
- **Razer Central : NON analysé.** Conteneur Setup Factory de 116 Mo que `7z` ne déballe pas ; application d'interface, pas de protocole matériel attendu.
- Stub Synapse 4 [SYN] : `InstallerConfiguration.xml` désigne `discovery3.razerapi.com` et `manifest3.razerapi.com` ; les composants sont téléchargés à l'exécution (non reproduit). Le stub embarque seulement des listes de PID JSON (section 6) et `rzS3detmgr.dll` (détection).
- Synapse 3 (`$PLUGINSDIR` de l'installeur NSIS) contient `RzComDriver_v1.0.52.0.exe` (pilote composite Razer, non extrait) et `RzLightMW_v24.0.0.031210.exe` (moteur d'effets `RzLightingEngine.dll`, extrait ; sans accès matériel, ses chaînes de log montrent `Init : %08X PID:%04X`, `ReSize: GridSize : (Rows:%d, Cols:%d)`, `SetBrightnessState :PID:%04X region: %08X state: %d`).
- Les modules qui écrivent sur le fil sont chargés **par PID** hors de ces installeurs : `RzCtl_ManagedWrapper.dll` (chaîne `CreateRzDeviceEnum_%04X`) et `RSy3_LightingEffects.dll` (chaîne `RzCtl_F001.dll`) ne contiennent que le chargeur. Aucun `RzCtl_<PID>.dll` dans les quatre fichiers => **les trames telles qu'émises par Synapse 3 sont INCONNUES côté Synapse** ; la section 3 repose sur OpenRazer/OpenRGB (ils reproduisent le protocole du firmware).
- Ce qu'apportent vraiment les fichiers Synapse : (a) 907 fichiers `Devices\LedLayout\Common\PID####_EID##_##.dat` (grilles de LED par PID) ; (b) listes de PID des dongles sans fil ; (c) familles d'appareils (`DeviceFamilyData.xml`, `DeviceRegionData.xml`).
- Chroma SDK public : les pages `assets.razerzone.com/dev_portal/REST/...` répondent 403 (accès refusé, non contourné). Dimensions de grille du SDK, format couleur, types d'effet : `INCONNU` ici.

## 2. Format `LEDD` des fichiers de disposition [SYN]

Source : `ConfigData\$APPDATA\Razer\Synapse3\Service\Staging\Devices\LedLayout\Common\PID<pid hex>_EID<2 hex>_<2 hex>.dat`, little-endian. Structure DÉDUITE en comparant 907 fichiers (aucune spécification Razer), validée par trois recoupements indépendants : Huntsman Mini = 61 touches ; Mamba Elite = 2 + 18 = 20 LED ; Lancehead = 2 + 14 = 16 LED, identiques à OpenRGB.

| Offset | Taille | Champ |
|---|---|---|
| 0x00 | 4 | magique `LEDD` |
| 0x04 | u16 | PID |
| 0x06 | u16 | version du format = 2 (tous les fichiers) |
| 0x08 | u16 | type d'appareil : 1 souris, 2 clavier/portable, 3 casque, 4 pavé, 5 tapis, 7 accessoire/ARGB, 8, 9 support, 11 (0068) |
| 0x0E | u16 | EID (0x00 ; 0x01..03, 0x80, 0x81 sur certaines souris) |
| 0x10 | u16 | variante de disposition (claviers : 1..0x12 = suffixe `_NN` du nom, variantes de langue) |
| 0x14 | u16 | taille du corps (= taille du fichier - 48 pour claviers et souris vérifiés) |
| 0x16 | u16 | **lignes** de la grille déclarée |
| 0x18 | u16 | **colonnes** de la grille déclarée |
| 0x1A | u16 | nombre d'enregistrements (LED) |
| 0x1C | n x 14 | claviers/portables (types 2, 4) : 7 u16 = {2 (touche), code touche Razer, 0, **ligne**, 0, **colonne**, 0} |

- Souris (type 1) : corps différent ; 0x16/0x18 valent 7..9 et 5..6 (non interprétés), 0x1A = nombre de **zones** (1 logo, 2 logo + molette, 3 avec pavé ou bande). Une signature `{1, 11, 0, 1, N, N}` suit pour les modèles à bande : N = nombre de LED de la bande (section 5). Tapis, casques, ARGB : seuls les en-têtes lignes x colonnes sont exploitables.
- La grille déclarée peut dépasser la grille occupée (Huntsman Mini 5 x 14 = occupée ; BlackWidow Chroma 7 x 23 déclarée, 6 x 23 occupée). Une colonne de valeur 0x4001 apparaît sur BlackWidow Elite (0228) : drapeau non interprété.
- Le code touche est un identifiant Razer interne, pas un code HID (tables `RzKB_xxxx.dat`, `kbddef.dat` : non analysées).
- Ces grilles servent à l'aperçu de Synapse. Elles ne sont **pas** garanties identiques à la matrice USB du firmware (écarts au tableau 4).

## 3. Protocole d'écriture (compléments à `razer.md`)

- **Rapport HID** : feature report de 91 octets (report-id + 90). Report-id 0 sauf Leviathan V2/V2X (0x07) [OGB `RazerController.cpp:54-59`]. `hid_send_feature_report`, puis `hid_get_feature_report` après 1 à 5 ms [OGB `RazerController.cpp:2086-2128`]. Une écriture est fiable seulement si la réponse a le même `transaction_id`/classe/commande et un statut différent de NEW/BUSY (utilisé pour les appareils derrière un récepteur, jusqu'à 50 essais de 2 ms).
- **Valeurs `MATRIX_EFFECT_*` (famille standard, classe 0x03 cmd 0x0A)** : off 0x00, wave 0x01, reactive 0x02, breathing 0x03, spectrum 0x04, customframe 0x05, static 0x06, starlight 0x19 [ORZ `razercommon.h:68-75`]. (Comble l'INCONNU de `razer.md`.) Stockage : NOSTORE 0x00, VARSTORE 0x01 [ORZ `razercommon.h:33-34`].
- **Mode appareil** : classe 0x00 cmd 0x04, taille 2, args {mode, param}. 0x00 matériel, 0x03 logiciel [OGB `razer_create_device_mode_report`]. **Retour au mode matériel = renvoyer le mode 0x00.** OpenRGB n'envoie le mode logiciel au démarrage que pour Charging Pad, Mug, Firefly Hyperflux [OGB `RazerController.cpp:68-76`] ; s'il est indispensable pour chaque autre PID : `INCONNU`.
- **Trames couleur par ligne** [OGB `razer_create_custom_frame_*`] : standard = classe 0x03 cmd 0x0B, taille 4+3n, args {0xFF, ligne, col_début, col_fin, RGB...} ; étendu = classe 0x0F cmd 0x03, taille 5+3n, args {0, 0, ligne, col_début, col_fin, RGB...} ; linéaire = classe 0x03 cmd 0x0C, taille 0x32, args {col_début, col_fin, RGB...}. Ordre R, G, B.
- **Luminosité** : standard classe 0x03 cmd 0x03 ; étendu classe 0x0F cmd 0x04 ; taille 3, args {stockage, led_id, valeur}.
- **Appareils `CUSTOM`** [OGB `razer_set_custom_frame`] : Tartarus Chroma (rétro, 1 `set_led_rgb`), DeathAdder Chroma (molette puis logo), Naga Epic Chroma (molette, 1 ms, rétro), Mamba 2012 (molette), Orbweaver Chroma (matrice standard décalée de +1 ligne et +1 colonne, présentée en 5 x 5).
- **transaction_id** : colonne « TID » des tableaux ci-dessous [OGB `RazerDevices.cpp`, 5e champ de chaque `razer_device`] : 0x3F, 0x1F, 0x9F (claviers sans fil HyperSpeed), 0xFF (Mouse Dock Pro), 0x60 (Kraken V4/Kitty V3 Pro), 0 (Kraken classiques/V3, protocole non Razer-report).
- **Interface HID** : colonne « IPU » = interface / usage page / usage de la collection à ouvrir [OGB `RazerControllerDetect.cpp`, macros `REGISTER_HID_DETECTOR_IPU`]. Deux motifs dominent : `02 01/02` (interface 2, page 0x01, usage 0x02) et `03 0C/01` (interface 3, page 0x0C, usage 0x01) pour les modèles récents ; souris : `00 01/02` ou `03 0C/01`.
- **Récepteurs sans fil** : commande classe 0x00, id 0xBF (`GET_WIRELESS_CONNECTIONS`), taille 0x50 ; réponse : arg0 = nombre d'emplacements, puis 3 octets par emplacement (statut 1 = connecté, PID en grand-boutiste, 0xFFFF = vide) [OGB `GetPairedDevicePIDs`, `RazerController.cpp:1273-1306`]. Un appareil appairé est piloté en envoyant sur le récepteur les trames avec **le transaction_id de l'appareil appairé**, et en interrogeant l'état jusqu'à fin de commande [OGB `RazerControllerDetect.cpp:41-80`].
- **Contrôleur ARGB 0F1F** : deux interfaces HID, interface 0 = rapports Razer de 90 octets, interface 1 = rapports ARGB de 320 octets [OGB `RazerControllerDetect.cpp:DetectRazerARGBControllers`].


## 4. Claviers (VID 0x1532) : PID, matrice, TID, interface

Colonnes : **Mat.** = type de matrice OpenRGB (STA standard = classe 0x03 cmd 0x0B ; EXT étendue = classe 0x0F cmd 0x03 ; LIN linéaire) ; **TID** = transaction_id ; **Matrice OGB** = lignes x colonnes envoyées par OpenRGB [OGB `RazerDevices.cpp`] ; **Synapse** = grille déclarée / grille occupée / nombre de LED du fichier LEDD [SYN], `absent` si Synapse (package config 3.9.1130) n'a pas de fichier pour ce PID ; **IPU** = interface, usage page/usage [OGB `RazerControllerDetect.cpp`]. « BT » = pas de détection HID USB (Bluetooth). Tous : retour au mode matériel = mode 0x00 (section 3).

| PID | Modèle | Mat. | TID | Matrice OGB | Synapse | IPU |
|---|---|---|---|---|---|---|
| 0241 | BlackWidow 2019 | EXT | 0x1F | 6x22 | absent | 02 01/02 |
| 0203 | BlackWidow Chroma | STA | 0x3F | 6x22 | 7x23 decl, max 6x23, 111/115 LED | 02 01/02 |
| 0211 | Blackwidow Chroma Overwatch | STA | 0x3F | 6x22 | 7x22 decl, max 6x22, 106/110 LED | 02 01/02 |
| 0209 | BlackWidow Chroma Tournament Edition | STA | 0x3F | 6x22 | 7x18 decl, max 6x18, 86/87/90/91 LED | 02 01/02 |
| 0221 | BlackWidow Chroma V2 | STA | 0x3F | 6x22 | 7x23 decl, max 6x23, 111/115 LED | 02 01/02 |
| 0228 | BlackWidow Elite | EXT | 0x1F | 6x22 | 7x22 decl, max 6x16385, 106/107/111 LED | 02 01/02 |
| 024E | Blackwidow V3 | EXT | 0x3F | 6x22 | absent | 03 0C/01 |
| 025A | BlackWidow V3 Pro (Wired) | EXT | 0x3F | 6x22 | absent | 02 01/02 |
| 025B | BlackWidow V3 Pro (Bluetooth) | EXT | 0x3F | 6x22 | absent | BT |
| 025C | BlackWidow V3 Pro (Wireless) | EXT | 0x3F | 6x22 | absent | 03 0C/01 |
| 0A24 | BlackWidow V3 TKL | EXT | 0x3F | 6x18 | 6x18 decl, max 6x18, 88/89/92 LED | 02 01/02 |
| 0258 | BlackWidow V3 Mini (Wired) | EXT | 0x1F | 5x16 | absent | 03 0C/01 |
| 0271 | BlackWidow V3 Mini (Wireless) | EXT | 0x9F | 5x16 | absent | 03 0C/01 |
| 0287 | Blackwidow V4 | EXT | 0x1F | 8x23 | absent | 03 01/00 ; 03 0C/01 |
| 028D | Blackwidow V4 Pro | EXT | 0x1F | 8x23 | absent | 03 01/00 ; 03 0C/01 |
| 02B3 | Blackwidow V4 Pro 75% (Wired) | EXT | 0x1F | 6x18 | absent | 03 0C/01 |
| 02B4 | Blackwidow V4 Pro 75% (Wireless) | EXT | 0x1F | 6x18 | absent | 02 01/02 |
| 02A5 | Blackwidow V4 75% (Wired) | EXT | 0x1F | 6x18 | absent | 03 01/00 |
| 0293 | Blackwidow V4 X | EXT | 0x1F | 6x22 | absent | 02 01/02 |
| 02D7 | Blackwidow V4 TKL (Wired) | EXT | 0x1F | 6x18 | absent | 03 0C/01 |
| 02D5 | Blackwidow V4 TKL (Wireless) | EXT | 0x9F | 6x18 | absent | 02 01/02 |
| 02D4 | Blackwidow V4 Low Profile TKL (Wired) | EXT | 0x1F | 6x18 | absent | 03 0C/01 |
| 02D2 | Blackwidow V4 Low Profile TKL (Wireless) | EXT | 0x9F | 6x18 | absent | 02 01/02 |
| 0216 | BlackWidow X Chroma | STA | 0x3F | 6x22 | 7x22 decl, max 6x22, 106/110 LED | 02 01/02 |
| 021A | BlackWidow X Chroma Tournament Edition | STA | 0x3F | 6x22 | 7x23 decl, max 6x23, 111 LED | 02 01/02 |
| 022A | Cynosa Chroma | EXT | 0x3F | 6x22 | 7x22 decl, max 6x22, 106/110 LED | 02 01/02 |
| 025E | Cynosa v2 | EXT | 0x1F | 6x22 | absent | 02 01/02 |
| 023F | Cynosa Lite | EXT | 0x3F | 1x1 | 1x1 decl, max 2x2, 106/110 LED | 02 01/02 |
| 021E | Ornata Chroma | EXT | 0x3F | 6x22 | 7x23 decl, max 6x23, 110/111/115 LED | 02 01/02 |
| 025D | Ornata Chroma V2 | EXT | 0x1F | 6x22 | absent | 02 01/02 |
| 028F | Ornata V3 | EXT | 0x1F | 1x10 | absent | 02 01/02 |
| 02A1 | Ornata V3 rev2 | EXT | 0x1F | 1x10 | absent | 02 01/02 |
| 02A3 | Ornata V3 TKL | EXT | 0x1F | 1x8 | absent | 02 01/02 |
| 0294 | Ornata V3 X | EXT | 0x1F | 1x1 | absent | 02 01/02 |
| 02A2 | Ornata V3 X Rev2 | EXT | 0x1F | 1x1 | absent | 02 01/02 |
| 0204 | DeathStalker Chroma | LIN | 0x3F | 1x12 | absent | 02 01/02 |
| 0295 | Deathstalker V2 | EXT | 0x1F | 6x22 | absent | 03 0C/01 |
| 0298 | DeathStalker V2 Pro TKL (Wired) | EXT | 0x1F | 6x17 | absent | 03 0C/01 |
| 0296 | DeathStalker V2 Pro TKL (Wireless) | EXT | 0x9F | 6x17 | absent | 02 01/02 |
| 0290 | DeathStalker V2 Pro (Wireless) | EXT | 0x9F | 6x22 | absent | 02 01/02 |
| 0292 | DeathStalker V2 Pro (Wired) | EXT | 0x9F | 6x22 | absent | 03 0C/01 |
| 0227 | Huntsman | EXT | 0x3F | 6x22 | 6x22 decl, max 6x22, 105/109 LED | 02 01/02 |
| 0226 | Huntsman Elite | EXT | 0x3F | 9x22 | 11x24 decl, max 7x23, 106/107/111 LED | 02 01/02 |
| 0266 | Huntsman V2 Analog | EXT | 0x3F | 9x22 | absent | 03 0C/01 |
| 0257 | Huntsman Mini | EXT | 0x3F | 5x15 | 5x14 decl, max 5x14, 61/62 LED | 02 01/02 |
| 0282 | Huntsman Mini Analog | EXT | 0x1F | 5x15 | absent | 03 0C/01 |
| 0243 | Huntsman Tournament Edition | EXT | 0x3F | 6x18 | absent | 02 01/02 |
| 026B | Huntsman V2 TKL | EXT | 0x3F | 6x17 | absent | 03 0C/01 |
| 026C | Huntsman V2 | EXT | 0x3F | 6x22 | absent | 03 0C/01 |
| 02A6 | Huntsman V3 Pro | EXT | 0x3F | 6x22 | absent | 03 0C/01 |
| 02A7 | Huntsman V3 Pro TKL White | EXT | 0x3F | 6x19 | absent | 03 0C/01 |
| 02E4 | Huntsman V3 HE Magnetic Mini | EXT | 0x3F | 6x22 | absent | 03 0C/01 |


Lecture et écarts :
- **Matrice 6 x 22** = clavier pleine taille (BlackWidow, Huntsman, Cynosa, Ornata, DeathStalker V2, V4 X) ; **6 x 18 / 6 x 17 / 6 x 19** = TKL ; **6 x 18** = 75 % (V4 75 %) ; **5 x 16 / 5 x 15** = 60 % (V3 Mini, Huntsman Mini) ; **8 x 23** = BlackWidow V4 / V4 Pro (6 x 23 touches + 2 bandes `lbl`/`lbr` de 1 x 9 LED sous le châssis) [OGB zones de 0287]. **9 x 22** = Huntsman Elite (0226) et Huntsman V2 Analog (0266) : zone clavier 6 x 22 + zone `underglow` 3 x 22 [OGB zones de ces deux PID].
- Écarts Synapse/OpenRGB (à trancher sur matériel, `INCONNU` lequel reflète le firmware) : BlackWidow Chroma/V2, Ornata Chroma = OGB 6 x 22, Synapse 6 x 23 occupées ; Huntsman Mini = OGB 5 x 15, Synapse 5 x 14 ; Huntsman Elite = OGB 9 x 22, Synapse 7 x 23 occupées (grille déclarée 11 x 24) ; BlackWidow Chroma TE (0209) = OGB 6 x 22, Synapse 6 x 18. Ligne de conduite : suivre OGB (validé matériel par la communauté) et traiter les cases supplémentaires comme inactives.
- Claviers Synapse sans entrée OGB (nom `INCONNU`) : 022C (7 x 24 déclarée, 106/108 LED), 0236 (6 x 22, 105/109 LED), 023E (6 x 18, 87 LED), 0305 (6 x 18, 87 LED).
- **Choix de l'interface** : motif `02 01/02` (interface 2) sur les générations Chroma/Chroma V2, Cynosa, Ornata, Huntsman, V4 sans fil et V4 X ; motif `03 0C/01` (interface 3) sur BlackWidow V3, V3 Mini, V3 Pro (dongle), V4 TKL filaire, DeathStalker V2, Huntsman V2/V3 ; **BlackWidow V4 et V4 Pro (0287, 028D) et V4 75 % filaire (02A5)** : `03 01/00` si firmware < 1.5, `03 0C/01` si firmware >= 1.5 [OGB lignes 270-273]. BlackWidow V3 Pro Bluetooth (025B) : non pilotable en USB, détecteur commenté dans OGB (ligne 265).
- **TID** : 0x3F pour la génération Chroma et les Huntsman/V3 ; 0x1F pour BlackWidow 2019/Elite, V4, Cynosa V2, Ornata V2/V3, V3 Mini filaire ; 0x9F pour les claviers sans fil HyperSpeed (V3 Mini sans fil, V4 TKL sans fil, V4 Low Profile TKL sans fil, DeathStalker V2 Pro). DeathStalker V2 Pro filaire/sans fil : 0x9F aussi pour le filaire (0292) selon OGB.
- Type de clavier : tous EXT sauf Chroma/TE/Overwatch/X (STA) et DeathStalker Chroma (LIN, 1 x 12).
- Portables Blade/Stealth/Book (même protocole STA, matrice 6 x 16 ou 6 x 25 Blade Pro 2016-2017, ou 1 x 1 / 1 x 16 pour les modèles à zone unique), interface `02 01/02`, TID 0x3F (0x1F pour 2022-2023 et 027A) : liste complète dans la section portables de `razer.md` et `RazerDevices.cpp`. Synapse : 0233, 023A, 0245, 024B, 024C, 024D, 0234, 0240 = 6 x 15 occupées ; 0253, 0256, 026A, 026D, 026E, 0276, 0279, 028A, 028B, 029E, 029F, 02A0 = 7 x 15 ; modèles « SingleLED » (zone unique) : 023B, 0239, 0246, 024A, 0255, 0252, 0259, 0268, 026F, 027A [SYN `DeviceFamilyData.xml`, catégorie 0x10].
- Pavés : Orbweaver Chroma 0207 (CUSTOM 4 x 5, TID 0x3F), Tartarus Chroma 0208 (CUSTOM 1 LED), Tartarus V2 022B (EXT 4 x 6, TID 0x1F), Tartarus Pro 0244 (EXT, 4 x 5 + touche K20) ; Synapse : 022B grille 4 x 6 (21 LED), 0244 4 x 6 déclarée / 4 x 5 occupée / 2 enregistrements.

## 5. Souris (VID 0x1532) : zones et nombre de LED

Colonnes : **LED OGB** = largeur de la matrice 1 x N d'OpenRGB ; **zones OGB** = découpage (nombre de LED entre parenthèses implicite) ; **Synapse** = nombre de zones du fichier LEDD et, si présent, nombre de LED de la bande. TID et interface : voir la fin du tableau.

| PID | Modèle | TID | Mat. | LED OGB | Zones OGB | Synapse |
|---|---|---|---|---|---|---|
| 0064 | Basilisk | 0x3F | EXT | 2 | logo 1, molette 1 | 2 zones |
| 0086 | Basilisk Ultimate | 0x1F | EXT | 14 | molette 1, logo 1, left 8, right 4 | 3 zones, bande 16 |
| 0088 | Basilisk Ultimate (Wireless) | 0x1F | EXT | 14 | molette 1, logo 1, left 8, right 4 | 3 zones, bande 16 |
| 0085 | Basilisk V2 | 0x3F | EXT | 2 | molette 1, logo 1 | absent |
| 0099 | Basilisk V3 | 0x1F | EXT | 11 | logo 1, molette 1, ledstrip 9 | absent |
| 00CB | Basilisk V3 35K | 0x1F | EXT | 11 | logo 1, molette 1, ledstrip 9 | absent |
| 00AA | Basilisk V3 Pro (Wired) | 0x1F | EXT | 13 | molette 1, logo 1, ledstrip 11 | absent |
| 00AB | Basilisk V3 Pro (Wireless) | 0x1F | EXT | 13 | molette 1, logo 1, ledstrip 11 | absent |
| 00CC | Basilisk V3 Pro 35K (Wired) | 0x1F | EXT | 13 | logo 1, molette 1, ledstrip 11 | absent |
| 00CD | Basilisk V3 Pro 35K (Wireless) | 0x1F | EXT | 13 | logo 1, molette 1, ledstrip 11 | absent |
| 00D6 | Basilisk V3 Pro 35K Phantom Green Edition (Wired) | 0x1F | EXT | 13 | molette 1, ledstrip 11 | absent |
| 00D7 | Basilisk V3 Pro 35K Phantom Green Edition (Wireless) | 0x1F | EXT | 13 | molette 1, ledstrip 11 | absent |
| 00D8 | Basilisk V3 Pro 35K Phantom Green Edition (Bluetooth) | 0x1F | EXT | 13 | molette 1, ledstrip 11 | absent |
| 00AC | Basilisk V3 Pro (Bluetooth) | 0x1F | EXT | 13 | molette 1, logo 1, ledstrip 11 | absent |
| 00E2 | Basilisk V4 Pro (Wired) | 0x1F | EXT | 13 | logo 1, molette 1, ledstrip 5 | absent |
| 00E3 | Basilisk V4 Pro (Wireless) | 0x1F | EXT | 13 | logo 1, molette 1, ledstrip 5 | absent |
| 00AF | Cobra Pro (Wired) | 0x1F | EXT | 11 | logo 1, molette 1, zone 9 | absent |
| 00B0 | Cobra Pro (Wireless) | 0x1F | EXT | 11 | logo 1, molette 1, zone 9 | absent |
| 0043 | DeathAdder Chroma | 0x1F | CUS | 2 | logo 1, molette 1 | 2 zones |
| 005C | DeathAdder Elite | 0x3F | EXT | 2 | logo 1, molette 1 | 2 zones |
| 006E | DeathAdder Essential | 0x3F | EXT | 2 | logo 1, molette 1 | 2 zones |
| 0098 | DeathAdder Essential V2 | 0x1F | EXT | 2 | logo 1, molette 1 | absent |
| 0071 | DeathAdder Essential (White Edition) | 0x3F | EXT | 2 | logo 1, molette 1 | 2 zones |
| 0084 | DeathAdder V2 | 0x1F | EXT | 2 | molette 1, logo 1 | absent |
| 004C | Diamondback Chroma | 0x3F | LIN | 21 | led_strip 19, logo 1, molette 1 | absent |
| 0059 | Lancehead 2017 (Wired) | 0x3F | EXT | 16 | right 7, left 7, logo 1, molette 1 | 3 zones, bande 14 |
| 005A | Lancehead 2017 (Wireless) | 0x3F | EXT | 16 | right 7, left 7, logo 1, molette 1 | 3 zones, bande 14 |
| 0070 | Lancehead 2019 (Wired) | 0x3F | EXT | 16 | right 7, left 7, logo 1, molette 1 | 3 zones, bande 14 |
| 006F | Lancehead 2019 (Wireless) | 0x3F | EXT | 16 | right 7, left 7, logo 1, molette 1 | 3 zones, bande 14 |
| 0060 | Lancehead Tournament Edition | 0x3F | EXT | 16 | molette 1, logo 1, right 7, left 7 | 3 zones, bande 14 |
| 0024 | Mamba 2012 (Wired) | 0x3F | CUS | 15 | zone 1 | absent |
| 0025 | Mamba 2012 (Wireless) | 0x3F | CUS | 15 | zone 1 | absent |
| 0044 | Mamba 2015 (Wired) | 0x3F | LIN | 15 | zone 15 | absent |
| 0045 | Mamba (Wireless) | 0x3F | LIN | 15 | zone 15 | absent |
| 0073 | Mamba 2018 (Wired) | 0x3F | EXT | 2 | molette 1, logo 1 | 2 zones |
| 0072 | Mamba 2018 (Wireless) | 0x3F | EXT | 2 | molette 1, logo 1 | 2 zones |
| 006C | Mamba Elite | 0x1F | EXT | 20 | molette 1, logo 1, left 9, right 9 | 3 zones, bande 18 |
| 0046 | Mamba Tournament Edition | 0x3F | LIN | 16 | left 7, right 7, logo 1, molette 1 | 3 zones, bande 14 |
| 0069 | Mamba Hyperflux (Wired) | 0x3F | EXT | 2 | molette 1, logo 1 | 2 zones |
| 0053 | Naga Chroma | 0x3F | LIN | 3 | molette 1, logo 1, numpad 1 | 3 zones |
| 0093 | Naga Classic | 0x1F | EXT | 3 | logo 1, molette 1, numpad 1 | absent |
| 0050 | Naga Hex V2 | 0x3F | STA | 3 | logo 1, molette 1, numpad 1 | 3 zones |
| 008D | Naga Left Handed | 0x1F | EXT | 3 | logo 1, molette 1, numpad 1 | 3 zones |
| 0067 | Naga Trinity | 0x3F | EXT | 3 | molette 1, logo 1, numpad 1 | 3 zones |
| 008F | Naga Pro (Wired) | 0x1F | EXT | 3 | molette 1, logo 1, numpad 1 | 3 zones |
| 0090 | Naga Pro (Wireless) | 0x1F | EXT | 3 | molette 1, logo 1, numpad 1 | 3 zones |
| 00A7 | Naga Pro V2 (Wired) | 0x1F | EXT | 2 | logo 1, numpad 1 | absent |
| 00A8 | Naga Pro V2 (Wireless) | 0x1F | EXT | 2 | logo 1, numpad 1 | absent |
| 00E7 | Naga Pro V3 (Wired) | 0x1F | EXT | 3 | logo 1, molette 1, numpad 1 | absent |
| 00E8 | Naga Pro V3 (Wireless) | 0x1F | EXT | 3 | logo 1, molette 1, numpad 1 | absent |
| 003E | Naga Epic Chroma | 0x1F | CUS | 2 | molette 1, numpad 1 | absent |
| 0F1D | Mouse Bungee V3 Chroma | 0x1F | EXT | 8 | zone 8 | absent |


- **Interface** : `00 01/02` (interface 0, page 0x01, usage 0x02) pour presque toutes les souris ; `03 0C/01` pour Basilisk V3, V3 35K et V4 Pro ; Naga Pro V3 (00E7/00E8) : usage page 0x01 usage 0x03 sans interface imposée [OGB lignes 411-412]. Modèles sans fil : même PID par mode (filaire / dongle / BT distincts) ; les PID dongle et filaire figurent séparément ci-dessus.
- **Mamba Elite (006C)** : 20 LED = molette 1 + logo 1 + gauche 9 + droite 9 ; Synapse confirme 2 zones + bande de 18 (`{1,11,0,1,18,18}`) [SYN `PID006c_EID00_00.dat`]. **Lancehead 0059/005A/006F/0070/0060** : 16 LED = droite 7 + gauche 7 + logo + molette ; Synapse : bande 14 [SYN `PID0059_EID00_00.dat`]. **Mamba TE 0046** : 16 LED ; Synapse bande 14. **Naga** (0050/0053/0067/008D/008F/0090/0093/00A7/00A8/00E7/00E8) : 3 LED (logo, molette, pavé) ; Naga Pro V2 : 2 (logo, pavé) ; Synapse : 3 zones (pas de bande).
- **Basilisk V3** (0099, 00CB) : 11 LED (logo 1 + molette 1 + bande 9) ; **V3 Pro et V3 Pro 35K** (00AA/00AB/00CC/00CD/00D6/00D7/00AC/00D8) : 13 LED (bande 11) ; **V4 Pro** : 13 LED (bande indiquée 5 dans le nom de zone OGB, total 13 : incohérence interne `INCONNU`) ; **Basilisk Ultimate** (0086, 0088) : OGB 14 LED (molette, logo, gauche 8, droite 4) contre **bande de 16 chez Synapse** : écart `INCONNU` (14 ou 18 LED). **Cobra Pro** : 11 LED (OGB). **Diamondback Chroma 004C** : 21 LED (bande 19 + logo + molette, LIN).
- Souris à zone unique (logo seul) : Viper 0078, Viper Ultimate 007A/007B, DeathAdder V2 Pro 007C/007D, Basilisk Essential 0065, Abyssus 006A/006B, Basilisk V3 X HyperSpeed 00B9 (molette seule).
- Souris dont Synapse a un fichier mais OGB aucune entrée : 0061, 0062 (récepteurs, 0 LED), 0063, 006D, 0074, 0075, 008E, 0092, 0904 (noms `INCONNU`).
- **Formes legacy** (demandées) : **0053 Naga Chroma** : LIN, TID 0x3F, 3 LED (molette, logo, pavé), classe 0x03 cmd 0x0C [OGB] ; **0046 Mamba Tournament Edition** : LIN, TID 0x3F, 16 LED en une seule trame linéaire (taille 0x32, soit jusqu'à 16 pixels RGB) ; **0044 Mamba 2015 filaire** (0045 sans fil) : LIN, TID 0x3F, 15 LED ; **0024/0025 Mamba 2012** : CUSTOM, 1 LED (molette via `set_led_rgb`) ; **0043 DeathAdder Chroma** : CUSTOM, TID 0x1F, molette + logo ; **003E Naga Epic Chroma** : CUSTOM ; **0050 Naga Hex V2** : STA 3 LED.
- Luminosité maximale 100 (0x64) pour DeathAdder Essential V2 (0098), 255 pour tous les autres [OGB `RazerController.cpp:GetMaxBrightness`].

## 6. Tapis, casques, Base Station, ARGB, Core, autres

| PID | Appareil | Mat. | TID | Matrice / zones | IPU | Source |
|---|---|---|---|---|---|---|
| 0C00 | Firefly | LIN | 0x3F | 1 x 15 | 00 01/02 | OGB ; Synapse : en-tête 9 x 7, 0 LED (disposition absente) |
| 0C04 | Firefly V2 | EXT | 0x3F | 1 x 19 | 00 01/02 | OGB ; Synapse : en-tête 9 x 7, 0 LED |
| 0C08 | Firefly V2 Pro | EXT | 0x1F | 1 x 17 | 00 01/02 | OGB ; Synapse absent |
| 0068 | Firefly Hyperflux | EXT | 0x3F | 1 x 14 : molette 1, logo 1, tapis 12 | 00 01/02 | OGB ; mode logiciel envoyé à l'ouverture |
| 0C01 / 0C02 / 0C06 | Goliathus Chroma / Extended / 3XL | EXT | 0x3F / 0x3F / 0x1F | 1 x 1 | 00 01/02 | OGB ; Synapse 1 x 1 |
| 0C05 | Strider Chroma | EXT | 0x3F | 1 x 19 | 00 01/02 | OGB |
| 0F08 | Base Station Chroma | EXT | 0x3F | 1 x 15 | 00 01/02 | OGB ; Synapse en-tête 5 x 7 |
| 0F20 | Base Station V2 Chroma | EXT | 0x1F | 1 x 8 | 00 01/02 | OGB |
| 0215 | Core | STA | 0x3F | 1 x 9 : côté 1, bande 8 | 00 FF00/01 | OGB |
| 0F1A | Core X | EXT | 0x3F | 1 x 16 : côté 1, bande 15 | 02 01/02 | OGB |
| 0F1F | Chroma Addressable RGB Controller | EXT_ARGB | 0x3F | 6 canaux x 80 LED max | I0 + I1 (voir ci-dessous) | OGB ; Synapse : 6 régions x 80, grille 3 x 160 |
| 0F2D | variante ARGB | `INCONNU` | `INCONNU` | Synapse : 6 zones x 80 (même fichier que 0F1F) | `INCONNU` | SYN seulement |
| 0F2C | « Glitter Wireless Dongle » (DeviceRegionData 3884) | `INCONNU` | `INCONNU` | Synapse : 3 zones x 120 | `INCONNU` | SYN seulement |
| 0F09 / 0F0E | Chroma HDK / PC Case Lighting Kit | EXT | 0x3F | 4 x 16 / 2 x 16 (Synapse : grille 2 x 32, 4 enregistrements) | 02 01/02 | OGB + SYN |
| 0F07 | Chroma Mug Holder | LIN | 0x3F | 1 x 15 | 00 01/02 | OGB |
| 0F0D / 0F2B | Laptop Stand Chroma / V2 | EXT | 0x1F | 1 x 15 | 00 01/02 | OGB ; Synapse 10 x 15 |
| 007E / 00A4 | Mouse Dock Chroma / Pro | EXT | 0x3F / 0xFF | 1 x 1 / 1 x 9 | 00 01/02 | OGB |
| 0F26 | Charging Pad Chroma | EXT | 0x1F | 1 x 10 | 00 0C/01 | OGB ; mode logiciel à l'ouverture |
| 0F1D | Mouse Bungee V3 Chroma | EXT | 0x1F | 1 x 8 | 00 01/02 | OGB |
| 0F13 | Lian Li O11 Dynamic Razer Edition | EXT | 0x1F | 4 x 16 (Synapse 24 x 3) | 02 01/02 | OGB + SYN |
| 0F1B | Seiren Emote | EXT | 0x3F | 8 x 8 (OGB zone) | page 0x0C usage 1, sans interface | OGB |
| 0F21 / 0F52 | Thunderbolt 4 / 5 Dock Chroma | EXT | 0x3F | 1 x 12 | 01 01/03 ou 0C/01 | OGB |
| 0F35 | Hanbo Chroma (AIO) | EXT | 0x3F | 70 LED : pompe 16 + 3 ventilateurs x 18 | 00 FF00/01 | OGB ; protocole propre (ci-dessous) |
| 0532 / 054A | Leviathan V2 / V2 X (barres de son) | EXT | 0x1F | 2 x 9 / 1 x 14 | 02 0C/01 / 00 0C/01 | OGB ; report-id 0x07 |
| 0517 / 0518 | Nommo Chroma / Pro | EXT | 0x3F | 2 x 24 / 2 x 8 | 01 01/03 ou 00 01/00 | OGB ; Synapse 12 x 24, 2 enreg. |
| 0F12 / 0F28 | moniteurs (Synapse « Monitor ») | `INCONNU` | `INCONNU` | Synapse 4 x 6 | `INCONNU` | SYN seulement |

**Contrôleur ARGB 0F1F** [OGB `RazerController.{h,cpp}`] :
- Deux interfaces HID : interface 0 pour les commandes de 90 octets (format Razer standard), interface 1 pour les rapports ARGB de 320 octets.
- Rapport ARGB : octet 0 = hid_id 0, octet 1 = report_id (0x04 si ligne < 5, 0x84 sinon), octets 2 et 3 = numéro de canal (répété), octet 4 = 0, octet 5 = dernier index (colonne de fin), octets 6.. = RVB (3 x n, jusqu'à 315 octets) ; écrit en feature report (320 octets) sur l'interface 1 [OGB `razer_create_custom_frame_argb_report`, `razer_usb_send_argb`].
- Configuration du nombre de LED par canal : classe 0x0F cmd 0x08 taille 0x0D, args = {6, puis pour chaque canal i : (0x01+i-1 si taille 0, sinon 0x19), taille}, soit 13 octets [OGB `razer_create_addressable_size_report`]. Détection au démarrage : classe 0x00 cmd 0x44 taille 1 (arg0 = activer) [OGB `razer_create_addressable_startup_detect_report`].
- LED ids des canaux : 0x1A à 0x1F (canaux 1 à 6) pour la luminosité (classe 0x0F cmd 0x04). 80 LED maximum par canal [OGB `RGBController_RazerAddressable.cpp`, tampon `80 * 6`].

**Casques / Kraken** (aucune version Synapse de leur protocole d'écriture, source OGB uniquement) :
- **Kraken classiques** (0501, 0506, 0504 7.1 Chroma, 0510 V2, 0527 Ultimate, 0560 Kitty Black V2 ; interface `03 0C/01`) : contrôleur dédié `RazerKrakenController`, `hid_write` d'un rapport de 37 octets : {report_id 0x04, destination, longueur, adresse haute, adresse basse, arguments[32]} ; destination 0x20 = lecture (firmware : adresse 0x0030 longueur 2 ; série : 0x7F00 longueur 0x16), 0x40 = écriture mémoire. Adresses d'effet : Kraken V2 / Ultimate / Kitty Black V2 : mode LED 0x172D, couleur custom 0x1189, respiration 0x1741/0x1745/0x174D ; Kraken classique : mode LED 0x1008, custom 0x1189, respiration 0x15DE. Octet d'effet : bit0 on/statique, bit1 respiration simple, bit2 spectre, bit3 sync, bit4 respiration 2 couleurs, bit5 respiration 3 couleurs. Réponse : `hid_read`, report_id 0x05 [OGB `RazerKrakenController.{h,cpp}`].
- **Kraken V3 / V3 X / V3 HyperSense** (0549, 0537, 0533) : rapport de 15 octets, report_id 0x40, commande 0x01 mode / 0x02 luminosité / 0x03 couleurs (RVB à partir de l'argument 1) ; mode direct 0x08 (argument 2, argument 1 = 0x0F), vague 0x03 ; 1 LED [OGB `RazerKrakenV3Controller.{h,cpp}`].
- **Kraken V4 / Kitty V3 Pro** (056B, 056C, 0587, 0588) : rapport de 64 octets (report_id 0x02, status, TID, TID 0x60, drapeau sans fil 0x80 pour les versions sans fil), classe 0x0F cmd 0x03 pour les couleurs (args {…, 0, 0, dernière LED, RVB...}), commandes 0xC0 mode / 0xC1 luminosité, firmware 0x02, série 0x00 ; CRC XOR des 61 premiers octets défini mais **non appliqué à l'envoi** dans OpenRGB [OGB `RazerKrakenV4Controller.cpp`] : `INCONNU` si le firmware l'exige. 9 LED (V4), 10 (Kitty V3 Pro : casque 9 + oreilles 1). Interface `05 FF14/01`.
- **Kitty V2 Pro (0554)** : 4 LED (oreilles gauche/droite, logos gauche/droite), détecté par le contrôleur Kraken V3 (`DetectRazerKrakenV3Controllers`), interface `03 0C/01`. **Kitty Edition 0F19** : contrôleur Razer standard, EXT, TID 0x1F, 1 x 4, interface `01 01/03`.
- **Tiamat 7.1 V2 (0F03)** : EXT, TID 0x3F, 17 LED (contrôleur 15 + oreilles 1 + 1), interface 0 ; Nommo : 2 x 24 / 2 x 8.

**Hanbo (0F35)** [OGB `RazerHanboController.cpp`] : rapport de 65 octets (2 octets d'en-tête, 63 d'arguments), interface 0 usage page 0xFF00 usage 0x01. Mode direct : séquence 0x82/arg 0x01, 0x80/{1,0,1}, 0x80/{1,1,1}, 0x82/arg 0x01. Couleurs : en-tête 0x32 (zone pompe) ou 0x40 (ventilateurs), arguments = 01 07 00 00 00 00 0z (z = index de zone) puis octets **G, R, B** (ordre inversé par rapport au standard). Luminosité : en-tête 0x70 {1, zone>0, valeur}. Cadence : pause de 2 ms par zone.

## 7. Dongles sans fil (HyperSpeed) [SYN]

Source : `x1\dongle.json`, `x1\dongleV2.json`, `x1\dockEID.json`, `x1\PatchExceptionalEID.json` du stub (PID en décimal dans les fichiers, convertis). Rôle exact des fichiers (liste de récepteurs gérés par le gestionnaire « DualDongle » / assignation d'EID) : déduit des noms, non confirmé par le code (non décompilé).

| PID | Appareil (si connu) | `cmdSteer` | Fichiers |
|---|---|---|---|
| 025C | BlackWidow V3 Pro (sans fil) | 0x80 | dongle, dongleV2 |
| 0271 | BlackWidow V3 Mini (sans fil) | 0x80 | dongle, dongleV2 |
| 027B | `INCONNU` | 0x80 | dongle, dongleV2 |
| 0290 | DeathStalker V2 Pro (sans fil) | 0x80 | dongle, dongleV2 |
| 0296 | DeathStalker V2 Pro TKL (sans fil) | 0x80 | dongleV2 |
| 02BA | BlackWidow V4 Mini HyperSpeed (sans fil) [ORZ `razercommon.h`] | 0x80 | dongleV2 |
| 02D2 | BlackWidow V4 Low Profile TKL (sans fil) | 0x80 | dongleV2 |
| 02D5 | BlackWidow V4 TKL (sans fil) | 0x80 | dongleV2 |
| 02C9, 02C2 | `INCONNU` | 0x80 | dongleV2 |
| 00A4 | Mouse Dock Pro | 0xE0 | dongleV2 |
| 00B3 | HyperPolling Wireless Dongle [ORZ] | 0xE0 | dongleV2 |

- `cmdSteer` : 0x80 pour les claviers, 0xE0 pour les récepteurs souris (00A4, 00B3). Signification exacte (octet d'orientation de commande vers le dongle, probablement l'équivalent du TID relayé) : `INCONNU`.
- `dockEID.json` : 00B9 (Basilisk V3 X HyperSpeed), 00AB (Basilisk V3 Pro sans fil), 00B7 (DeathAdder V3 Pro sans fil [ORZ]) ; `PatchExceptionalEID.json` : 0060 (Lancehead TE) EID {145}, 00B2 (DeathAdder V3 [ORZ]) EID {145, 146}. EID = « edition id » de Synapse, utilisé pour distinguer des variantes du même PID ; son rôle sur le fil : `INCONNU`.
- `dummyProt.json` : 0D09 (nom `INCONNU`), protocole factice de test.
- **Enchaînement côté OpenRGB** (seule source de séquence) : (1) ouvrir l'interface du dongle ; (2) envoyer classe 0x00 id 0xBF taille 0x50 ; lire les emplacements ; (3) pour chaque PID appairé connu, créer un contrôleur sur le **même chemin HID** avec le TID du PID appairé et attente de fin de commande (jusqu'à 50 lectures de 2 ms) [OGB `RazerControllerDetect.cpp:41-80`, `RazerController.cpp:2086-2128`]. Les claviers sans fil à TID 0x9F (V3 Mini, V4 TKL, DeathStalker V2 Pro) sont adressés directement par TID, pas par appairage.
- Appairage Bluetooth : VID 0x068E, non pilotable ; USB uniquement.

## 8. Ce qui est INCONNU

- Trames exactes émises par Synapse 3 : les modules `RzCtl_<PID>.dll` ne sont dans aucun des 4 fichiers téléchargés (chargés à la demande, hors installeur).
- Chroma SDK : grilles par type d'appareil, format couleur, types d'effets (documentation REST en 403).
- Razer Central : non extrait (Setup Factory) ; rôle dans le pilotage matériel jugé nul, non vérifié.
- Mode logiciel 0x03 indispensable ou non pour chaque PID ; le retour mode 0x00 est supposé valable partout (commande universelle chez OpenRazer/OpenRGB), non testé sur matériel.
- Matrice USB réelle des claviers là où Synapse et OpenRGB divergent (BlackWidow Chroma, Huntsman Mini/Elite, BlackWidow Chroma TE) ; nombre de LED du Basilisk Ultimate (14 OGB / 18 Synapse) et V4 Pro.
- Signification de `cmdSteer` (0x80 / 0xE0), des EID, du PID 0D09.
- PID sans nom : 027B, 02C9, 02C2, 0F2C (nom commercial), 0F2D, 0061/0062/0063/006D/0074/0075/008E/0092/0904, 022C, 0236, 023E, 0305, 0F12, 0F28.
- Protocole des Kitty V2 Pro (octets exacts), des variantes ARGB 0F2C/0F2D, des moniteurs 0F12/0F28, cadence maximale stable, CRC obligatoire sur Kraken V4.
- Enregistrements souris du LEDD : champs 0x16/0x18 (7..9, 5..6) non interprétés ; code touche Razer -> position physique : tables `RzKB_*.dat` non lues.

## 9. Difficulté de réimplémentation

- **Claviers/souris/tapis/accessoires à rapport Razer 90 octets** : facile à moyenne. Une seule couche de trame (CRC XOR), trois familles de matrice (STA/EXT/LIN), tableau de PID avec TID, interface et dimensions (fourni ci-dessus pour les 201 appareils d'OpenRGB).
- **Sans fil par dongle** : moyenne ; tout repose sur le TID de l'appareil appairé et l'attente de fin de commande.
- **Contrôleur ARGB 0F1F** : moyenne (deux interfaces, rapports de 320 octets, tailles de canaux).
- **Kraken classique / V3 / V4, Hanbo** : chacun a son propre protocole court ; faisable mais sans validation Synapse.
- **Piège Windows** : Synapse retient les interfaces HID (pilote `RzComDriver` + service) ; arrêter le service avant d'ouvrir les collections, et refermer proprement (retour mode matériel 0x00).

## 10. Tests d'octets possibles sans matériel

1. Construire « mode appareil logiciel » (classe 0x00, cmd 0x04, taille 2, args 03 00) ; CRC = XOR des octets 2..87 ; vérifier 90 octets (91 avec report-id 0).
2. Ligne de trame étendue 22 colonnes : taille 5 + 66 = 71 = 0x47 ; standard : 4 + 66 = 70 = 0x46 ; linéaire : taille fixe 0x32.
3. Rapport ARGB : 320 octets, report-id 0x04 pour les lignes 0 à 4 et 0x84 au-delà.
4. Table : chaque `transaction_id` du tableau appartient à {0x3F, 0x1F, 0x9F, 0xFF, 0x60, 0}.
5. Parseur LEDD : relire `PID0257_EID00_01.dat` (902 octets) -> 61 enregistrements, lignes 0..4, colonnes 0..13 ; `PID006c_EID00_00.dat` (244 octets) -> 3 zones, bande 18 ; `PID0f1f_EID00_00.dat` -> 6 zones de 80.
6. Recouper les listes PID de cette note avec `RazerDevices.h` (OpenRGB) et `driver/razercommon.h` (OpenRazer).

## 11. Nettoyage

Installeurs et extraits supprimés après la rédaction ; conservés dans `D:\Projet\Analyses\rgb\razer\tools\` : scripts d'analyse et tables générées (sans binaire Razer).
