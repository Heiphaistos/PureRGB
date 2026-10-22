# SteelSeries GG / Engine : analyse du logiciel officiel + protocoles HID

Document d'interopérabilité (PureRGB, MIT). Aucun code ni asset SteelSeries copié ; uniquement des faits de protocole et d'inventaire.
Complète `steelseries.md` (qui reposait sur OpenRGB seul). Marqueurs de source :
- **(G)** = lu dans l'installeur officiel SteelSeries GG extrait sans exécution (chemins relatifs à `D:\Projet\Analyses\rgb\steelseries\x\`).
- **(O)** = OpenRGB commit `bbc7747`, `Controllers/SteelSeriesController/` (source tierce, NON officielle ; reste à valider sur matériel).
- `INCONNU` = non confirmé. Rien n'est deviné.

## 1. Source analysée

| Champ | Valeur |
|---|---|
| Logiciel | SteelSeries GG 122.0.0 (Windows x64, NSIS) |
| URL officielle | `https://engine.steelseriescdn.com/SteelSeriesGG122.0.0Setup.exe` (CDN SteelSeries ; URL relevée dans le manifeste winget `SteelSeries.GG` 122.0.0, date de sortie 2026-10-20) |
| SHA-256 | `F7708EA437B431FF1941782D9840985E6C2EFDC6B9E61A65FC28C019011B6345` (calculé localement, identique au manifeste winget) |
| Méthode | `7z x` sur l'exe NSIS ; aucune exécution, aucune installation. `@electron/asar extract` sur `resources/app.asar` (UI uniquement) |
| Taille extraite | 1,3 Go (supprimée en fin de mission) |

### Ce que contient réellement l'installeur (G)
- `apps/engine/SteelSeriesEngine.exe` (16,7 Mo) : binaire **Go** embarquant un interpréteur « GoLisp » (chaînes `SteelSeries/GoLisp Debugger`, `load-golisp-handlers`, `DeviceDriver not initialized`). Le pilotage des périphériques est écrit en Lisp, évalué à l'exécution.
- `apps/engine/HIDDLL.dll` (2,1 Mo) et `SSEdevice.dll` (0,8 Mo) : couche d'accès HID (`HidD_GetFeature`, `HidD_SetFeature`, `HidD_GetInputReport`, `HT_GetFeature`, `HT_SetFeature`) et flash de firmware (`FlashFirmwareSenseiRivalPro`). Pas de tables de trames lumineuses.
- `apps/engine/deviceSpecifications/*.edevice` : **489 fichiers**, un par modèle ou « base » (`base_apex_2022_wired`, `base_apex_2022_wireless`, `base_apex_2024_tkl_layout_lighting`, `libsteel_lighting_engine`, `common_lighting`, `linear-lighting-device`...). **Tous chiffrés** (bloc `-----BEGIN DESCRIPTOR-----` + base64 haute entropie, 0 fichier en clair sur 489). Ce sont eux qui portent VID/PID, rapports, tailles de tampon et séquences de lumière. **Non déchiffrés** (voir §8 : le déchiffrement relèverait du contournement d'une protection ; refusé par la consigne).
- `apps/engine/firmware/<VID<<16|PID en décimal>/` : 310 dossiers, 1 par PID, avec `version.json` (clés `firmware_version`, `device_name`) et les images de firmware. **Source fiable des PID officiels** (§2).
- `apps/engine/firmware/<PID>/layout.bin` : table de géométrie des touches des Apex Pro/7/5 (§4) : **seul fichier de protocole lisible en clair**.
- `resources/app.asar` : UI Electron (React, 1443 PNG) ; ne contient aucun protocole (grep `apex_pro|aerox` : noms d'affichage seulement).
- `apps/engine/driver/` : installeur de pilote (`win_driver_installer.exe`, `Win10/`, `legacy/`), non exécuté.
- `apps/sonar`, `apps/moments`, `apps/prismSyncV2`, `engineApps/*` : audio, capture, Prism Sync (écran) : hors périmètre RGB périphériques.
- Le moteur écoute en local (GameSense) : voir `steelseries.md` §GameSense (coreProps.json) ; non réanalysé.

## 2. Matériel couvert : PID confirmés par le dossier `firmware/` (G)

VID **0x1038** sauf mention. Identifiant du dossier `firmware/` = `(0x1038<<16)|PID` en décimal. Colonne FW = `firmware_version` de `version.json`.

### 2.1 Claviers
| PID | `device_name` (G) | FW (G) | Remarque |
|---|---|---|---|
| 1610 | apex_pro | 4.16.8/4.16.8 | + `layout.bin` |
| 1612 | apex_7 | 4.16.8/4.16.8 | + `layout.bin` |
| 1614 | apex_pro_tkl | 4.16.8/4.16.8 | + `layout.bin` |
| 1618 | apex_7_tkl | 4.16.8/4.16.8 | + `layout.bin` |
| 161A | apex_3 | 0.40.0.0 | pas de `layout.bin` |
| 161C | apex_5 | 2.1.1/2.1.1 | + `layout.bin` |
| 161E | apex_pro_mini | 1.19.7 | absent de `steelseries.md` |
| 1620 | apex_9_mini | 1.19.9 | |
| 1622 | apex_3_tkl | 3.0.4 | |
| 1624 / 1626 | apex_pro_mini_wireless_dongle / apex_pro_mini_wireless | 3.24.1 | + VID 0x0111 PID 0x1626 `apex_pro_mini_wireless_bluetooth` (3.13.4) |
| 1628 | apex_pro_tkl_2022 | 1.19.7 | = « Apex Pro TKL 2023 » d'OpenRGB |
| 1630 / 1632 | apex_pro_tkl_wireless_dongle / apex_pro_tkl_wireless | 3.24.1 | 1630 = dongle, 1632 = clavier |
| 1634 | apex_9_tkl | 1.19.9 | |
| 1640 | apex_pro_2024 | 4.16.8/4.16.8 | + `layout.bin` (115 entrées) |
| 1642 | apex_pro_tkl_2024 | 1.19.7 | = « Apex Pro TKL Gen 3 » d'OpenRGB |
| 1644 / 1646 | apex_pro_tkl_wireless_2024_dongle / apex_pro_tkl_wireless_2024 | 3.24.1 | |
| 1648 | apex-pro-mini-2024 | 1.19.6 | nouveau |
| 1650 | apex_5_2024 | 2.2.3 | + `layout.bin` ; nouveau |
| 1652 | apex_7_2024 | 4.16.8 | + `layout.bin` ; nouveau |
| 1660 / 1662 | apex_pro_tkl_2024_cyber / _steel | 1.20.00 | nouveau |
| 0616 / 0617 | apex_m750 / apex_m750_tkl | 1.20 / 1.7 | |
| 1200, 1202, 1206, 1208, 120A | apex-raw, apex, apex-350, apex-300, hp_omen_apex-350 | 1.16 / 0.56 / 0.64 / 1.18 / 0.64 | anciens |
| 1600-160E, 1616 | apex_m800, m500, m400, apex_100, apex_150 | divers | anciens |

### 2.2 Souris (extrait)
| PID | `device_name` (G) | FW (G) |
|---|---|---|
| 1700 / 1730 | rival_700 / rival_710 | 0.114 |
| 1702, 170C, 1814, 1816, 1729 | rival_100, dota2_rival_100_lenovo, rival_105, rival_106, rival_110 | divers |
| 1710, 1712, 1716, 1718, 171A, 171C, 1394 | rival_300 et éditions | 0.104 / 1.8 |
| 1720, 171E, 1736, 1722 | rival_310, howl, pubg, sensei_310 | 1.36 |
| 1724 / 172E | rival_600 / dota2_rival_600 | 2.52 / 2.51 |
| 1726 / 172B | rival_650_wireless_dongle / _mouse | 1.26 |
| 1824 | rival_3 | 0.81 |
| 184C | **rival_3_nvidia_reflex** (OpenRGB l'appelle « Rival 3 actuel ») | 0.81 |
| 1870 / 1872 | rival_3_v2 / rival_3_wireless_v2 | 1.1.6 / 1.5.0 | nouveaux |
| 1830 | rival_3_wireless | 1.04 |
| 183C / 183E | rival_5 / rival_5_destiny_2 | 1.0.16 |
| 1832 / 1834 | sensei_ten / csgo_neon_rider_sensei_ten | 0.11 |
| 1836 | aerox_3 | 1.0.22 |
| 1838 / 183A | aerox_3_wireless_dongle / aerox_3_wireless (souris) | 1.11.4 |
| 1850 | aerox_5 (filaire) | 1.0.1 |
| 1852 / 1854 | aerox_5_wireless_dongle / aerox_5_wireless | 1.11.4 |
| 185C / 185E, 1860 / 1862 | aerox_5_wireless_destiny_2 / diablo_4 (dongle / souris) | 1.11.4 |
| 1858 / **185A** | aerox_9_wireless_dongle / **aerox_9_wireless (souris)** | 1.2.4 |
| 1874 / 1876 | aerox_9_wireless_we_dongle / _we | 1.5.2 | nouveaux |
| 1878 / 187A | aerox_3_wireless_dle_dongle / _dle | 1.14.1 |
| 1890 / 1892 | aerox_3_wireless_gen2_dongle / _gen2 | 1.1.6 |
| 1880 / 1882 | sensei_rival_pro_base_station / dongle | 2.14.2 | nouveaux |
| 182A, 182C, 182E, 1840-184D, 1856 | prime_*, prime_wireless*, prime_mini* | 1.0.x / 1.1.7 | Prime (non couverts par OpenRGB) |
| VID 0x0111 (Bluetooth) | `aerox_3_wireless_bluetooth` = 0x0111:0x183A (6202 déc.), `aerox_5_wireless_bluetooth` = 0x0111:0x1854 (6228), `apex_pro_mini_wireless_bluetooth` = 0x0111:0x1626 (5670), `stratus_xl` = 0x0111:0x1419 (5145) | 0.0.0 / 1.0.0.0 / 3.13.4 / 1.69 | VID 0x0111 = Bluetooth SIG ; parlé en BLE, pas en HID USB |

Remarque : **résolution du doublon d'OpenRGB** (§ `steelseries.md`, Aerox 9 = 0x185A) : (G) confirme **0x1858 = dongle, 0x185A = souris Aerox 9 Wireless en mode filaire**. L'Aerox 5 filaire pur est 0x1850. Le `#define` d'OpenRGB `AEROX_9_PID = 0x185A` n'est donc pas un doublon mais le même appareil.

### 2.3 Tapis QCK
(G) 150A QCK Prism Cloth M, 150D XL, 1512 dota2, 1514 csgo_neon_rider XL, 1516 3XL, 1518 4XL, 151A 5XL, 151C XL Neo Noir, 151E XL Destiny 2, 1520 XL Destiny 2 Lightfall (FW 1.2 ; 1.5 pour 1520), 1507 `qck_prism` (0.39). Confirme la liste de `steelseries.md`.

### 2.4 Casques (G)
Siberia : 1210, 1211, 1224, 1226, 1227, 1228, **1229 (siberia_350)**, 1240. Arctis : **1250 arctis_5, 1251 dota2_arctis_5, 1252 arctis_pro, 12AA arctis_5_2018, 12A8 pubg_arctis_5_2018**, 1260/1261 arctis_7, 12AD/12AE arctis_7_2018, 12B1 arctis_9x, 12B3-12B7 arctis_1w/1x, 12C0/12C2 arctis_9, 12CB/12CD nova_pro, 12D0-12D8, 12E0-12E8 nova_pro_wireless, **12EC arctis_nova_3**, 12F0-12F6 nova_4/4x, 12FA nova_pro_v2, 2200-22AD (Nova 7, 7P, 7X, 7+ , Nova 5, Nova 5X, Nova 3 Wireless 2267/2269, Nova 3X Wireless 226B/226D, Elite 2244/2249, Pro Omni 2290/2296, Nova 7 Gen2 227C/227E...), 230A/230C Gamebuds, 2317/231A Gamebuds X. Liste complète : `firmware/*/version.json`.

## 3. Génération de firmware / distinction Apex : ce qui est confirmé

### 3.1 Réponse à la requête de version (octet 0x90) : (O)
- Requête : sortie 65 octets (`[0]=0` report id, `[1]=0x90`), puis lecture 64 octets (timeout 2 ms).
- Réponse : **chaîne texte ASCII** `x.y.z` dans le tampon ; si `ibuf[0] == 0x90` (Gen2+), le premier caractère (écho) est supprimé avant l'analyse. Gen1 seulement : seconde requête avec `[2]=0x01` pour la version LED (`KBD: x.y.z / LED: x.y.z`).
- Règle OpenRGB (Apex Pro TKL 1614, Apex 9 TKL 1634, Apex 9 Mini 1620) : **Gen3 si version ≥ 1.19.7** (comparaison major, puis minor, puis patch).
- Cohérence avec (G) : 161E/1628/1642 expédient exactement **1.19.7** dans GG, 1620/1634 **1.19.9**, 1648 **1.19.6**. Le seuil 1.19.7 d'OpenRGB correspond donc à la première version « 2022/2024 » livrée par GG. **Piège confirmé (G)** : 1614 (Apex Pro TKL d'origine) expédie **4.16.8** (numérotation Prism) ; avec la règle OpenRGB (`major > 1`), il est classé Gen3 et enverrait id direct 0x40 + init 0x4B. Pas vérifié que le matériel 1614 l'accepte : `INCONNU`.
- Codes de région Gen2+ : requête `[1]=0xF5` ; réponse `ibuf[0]==0xF5`, `ibuf[2]` = code (1 -> « 64740 », 3 -> « 64741 », 4 -> « 64743 », 6 -> « 64744 » ...) ; (O, `SteelSeriesApexController.cpp`). Sert à choisir le patch de disposition.

### 3.2 Nomenclature GG vs OpenRGB
| GG (G) | PID | OpenRGB (O) |
|---|---|---|
| apex_pro_tkl_2022 | 1628 | « 2023 » (Gen2) |
| apex_pro_tkl_wireless (dongle 1630 / clavier 1632) | 1630, 1632 | « 2023 WL » (Gen3 dans le tableau) |
| apex_pro_tkl_2024 | 1642 | « Gen3 » |
| apex_pro_tkl_wireless_2024 (dongle 1644 / clavier 1646) | 1644, 1646 | « Gen3 WL » |

## 4. Table des touches Apex : `layout.bin` (G)

**Fichier** : `apps/engine/firmware/272111120/layout.bin` (PID 1610) ; octets **identiques** (SHA-256 `add6333f1aa90dd30a3b442f7ebf28ee538d0e0ea33733f4811f765948106cd8`) pour 1610, 1612, 1614, 1618, 161C, 1650, 1652. `272111168/layout.bin` (PID 1640 Apex Pro 2024) : SHA-256 `ef760d371c0b236a320e994b1a2f68ed0ba686b965623782643b11e2cc10045f`, différence = une entrée de plus (0xFB). Les autres Apex (9, Pro 2022/2024 TKL, 3, 3 TKL, mini) n'ont **pas** de `layout.bin` dans le paquet.

**Format** (déduit et validé : le comptage retombe exactement sur l'octet 3 de l'en-tête et sur la fin des données, offset 604 / 608) :
- Fichier de 1024 octets, zéro-rembourré après les données.
- Octets 0-1 : `5A 1A` (marqueur). Octet 2 : `04` (semantique `INCONNU`, constant). Octet 3 : **nombre d'entrées** (0x72 = 114 ; 0x73 = 115 pour 1640).
- À partir de l'offset 4, entrées successives. Entrée simple (4 octets) : `[usage HID][0x00][x][y]`. Entrée étendue (12 octets) : `[usage][0x01]` suivi de **5 paires (x, y)** (touche multi-LED : Entrée 0x28, Backspace 0x2A, Espace 0x2C, ISO 0x31/0x32, 0x64, 0x87-0x8B, Maj gauche 0xE1, Ctrl gauche/Alt/GUI/Alt Gr/Menu/Fn : 0xE1 E2 E3 E5 E6 E7, 0xF0).
- Unité de x/y : grille arbitraire, `INCONNU` (x 0x02..0x58, y 0x02..0x50 environ). Exemples : A (0x04) = (9, 16), Échap (0x29) = (2, 2), F1 (0x3A) = (12, 2).
- Clé d'une touche = **usage clavier USB HID** (0x04..0x64 : lettres, chiffres, ponctuation, F1-F12, pavé), plus les extensions : **0x85** (pavé numérique virgule), **0x87-0x8B** (touches internationales JIS), **0x90-0x91** (Hangul), **0xE0-0xE7** (modificateurs), **0xF0** (Fn), et **0xFB** présent seulement sur l'Apex Pro 2024 (1640 ; touche supplémentaire).
- Ensemble complet : 04..64 (97), 85, 87..8B (5), 90, 91, E0..E7 (8), F0 = 114 ; 1640 ajoute FB = 115.
- **Écart avec OpenRGB (O)** : sa table `keys[]` (112 codes) n'a **pas** 0x85, 0x90, 0x91 (touches absentes des claviers testés) et contient 0xFB pour tous. Ordre d'envoi = ordre de `keys[]`, sans importance : chaque entrée du paquet direct nomme sa touche.

**Conclusion Apex (clé -> LED)** : CONFIRMÉ (G + O) que la trame directe adresse chaque LED par son **usage HID** ; il n'y a pas d'index séparé. La liste exploitable des codes pour Apex Pro / Pro TKL / 7 / 7 TKL / 5 (et 5 2024, 7 2024) est la liste de 114 ci-dessus. Pour Apex 9, Pro TKL 2022/2024 et sans fil : `INCONNU` côté GG (descripteurs chiffrés) ; (O) utilise la même `keys[]` tronquée à `(longueur-3)/4`.

## 5. Protocole Apex : trame directe, longueurs, retour matériel (O)

Source unique : `SteelSeriesApexController.cpp`, `SteelSeriesApexBaseController.h` (OpenRGB). **Non confirmé par GG** (descripteurs chiffrés).

| Élément | Valeur (O) |
|---|---|
| Transport | feature reports (`hid_send_feature_report`) pour les couleurs ; `hid_write` 65 octets + lecture 64 octets (timeout 100 ms) pour les requêtes |
| Trame directe | `[0x00 report id][packet_id][nb_touches][(usage, R, G, B) x nb]`, longueur du feature report = `direct_packet_length` |
| packet_id | Gen1 = **0x3A** ; Gen2/Gen3 filaire = **0x40** ; Gen2/Gen3 **sans fil** (1630, 1632, 1644, 1646) = **0x61** |
| Longueur du report | **643** par défaut (1610, 1612, 1614, 1618, 161C, 1628, 1630, 1632, **1642, 1644, 1646**) ; **513** seulement pour Apex 9 TKL (1634) et Apex 9 Mini (1620). Un report de mauvaise longueur fait caler le transfert de contrôle (commentaire de la source) |
| Nombre max de touches | `(longueur - 3) / 4` : 160 pour 643 ; 127 pour 513 |
| Init Gen3 | feature report 65 octets, `[1]=0x4B` ; réinitialisation (retour au mode matériel) = `[1]=0x41` (Gen2/3) ou `[1]=0x3B` (Gen1), en `hid_write` 65 octets |
| Profil matériel | feature 65 octets `[1]=0x89, [2]=n°` |
| Luminosité | `hid_write [1]=0x23, [2]=0..10` ; lecture `[1]=0xA3` -> réponse `ibuf[0]=0xA3, ibuf[1]=0x00, ibuf[2]=valeur` ; `ibuf[1]=0xFF` = non supporté |

**Longueur du report direct des sans fil (1630, 1632, 1644, 1646) et du Gen3 filaire (1642)** : (O) ne définit **aucune entrée spécifique** dans `direct_packet_length_map` (seuls 1634 et 1620 y figurent) donc **643** pour tous. Non confirmé par le logiciel officiel : `INCONNU` côté GG (la longueur est dans les descripteurs chiffrés). Les firmwares 3.24.1 des sans fil (G) ne permettent pas de la lire sans désassembler l'image du MCU (non fait).

**Trame de retour au mode matériel** : (O) uniquement ; à la fermeture, envoyer `hid_write(65)` avec `[1] = reset_cmd` (0x3B Gen1, 0x41 Gen2/3). Passage onboard -> direct sur Gen2/3 : renvoyer l'init 0x4B. (G) : non confirmé.

## 6. Souris, tapis, casques : trames (O sauf mention)

### 6.1 Constantes communes souris (`SteelSeriesMouseController.h`)
Effets : `0x00` spectrum cycle, `0x01/0x02/0x03` breathing max/mid/min, **`0x04` direct**, `0x05` rainbow breathing, `0x06` disco, `0x07` reactive, `0x08` rainbow. Luminosité max `0x64`. Enregistrement (`Save`) : tampon 10 octets `00 09 ...` (écrit via `hid_write`).

### 6.2 Rival 3 (`SteelSeriesRival3Controller`) : PID 1824, 184C
- Interface 3 (cf. `steelseries.md`). Tailles de tampon : **version 3**, **effet 4**, **couleur 8**, **save 10** (tous incluent le report id 0).
- Version : `00 10 00`, lecture de 2 octets (u16).
- Effet : `00 06 00 <effet>` (valeur envoyée = `mode.value - speed`).
- Couleur : `00 05 00 <zone> R G B <luminosité>`.
- Zones : Front=1, Middle=2, Rear=3, Logo=4. Modes exposés : 0x04, 0x03, 0x00, 0x05.
- Rival 3 Wireless (1830, famille Aerox sans fil) : couleur = `00 13 <zone> R G B` (zone Scroll Wheel=1) ; pas d'effets matériels, pas de save.

### 6.3 Rival 5 (183C) : tampons feature de **64 octets**
- Couleurs (10 zones) : `00 21 FF 03` puis RGB à `4+3i` (i=0..9). Zones : 0 Scroll Wheel, 1/2 Body L1/R1, 3/4 L2/R2, 5/6 L3/R3, 7/8 L4/R4, 9 Logo.
- Arc-en-ciel statique : `00 22 FF 03` ; luminosité : `00 23 <0x00|0x0C|0x19|0x32|0x64>` (5 paliers) ; réactif : `00 26 01 00 R G B`. Pas de version, pas de save.

### 6.4 Rival 600 (1724, 172E) : `hid_write` de 7 octets
- `1C 27 00 <1<<zone_id> R G B` (report id 0 préfixé). `zone_id` côté OpenRGB : LED latérales du 600 = 0x02..0x07 (gauche haut 02, milieu 04, bas 06 ; droite haut 03, milieu 05, bas 07) ; valeur de la LED logo/roue : voir `RGBController_SteelSeriesRival.cpp` (logo 0 ; 0x10/0x11 pour roue/logo des 650/700) ; correspondance exacte `INCONNU`.
- Rival 700/710 (1700, 1730) : feature report de **578 octets** (577 + report id), `[0]=0x05, [2]=zone, [3..5]=RGB, [0x0B]=zone, [0x0C]=1`.
- Rival 650 (172B, 1726 dongle) : quatre `hid_write` de 60 octets (+ report id) ; `03 ...(0x04=0x30, 0x06=0x10, 0x07=0x27, 0x16=1, 0x1E=4, 0x1F-0x21=RGB, 0x22=0xFF, 0x27=0xFF, 0x29=0x54, 0x2C=0xFF, 0x2D=0x54, 0x2E-0x30=RGB, 0x31=0x56)`, puis `03 .. [2]=0x30,[4]=0x2C`, puis `05 .. [2]=zone(0x10..0x17),[3]=0xFF,[8]=0x5C`, puis `1C .. [2]=0x55,[4]=0x46`.
- Rival 100 / 300 : `05 00 R G B` / `08 <zone+1> R G B`; effet `07 ...`; version Rival 300/700 : `10 00`.

### 6.5 Aerox 3 / 5 / 9 filaire (1836, 1850) : tampon **65** octets, timeout 250 ms
- **Aerox 3** (`...Aerox3Controller`) : couleur `00 21 <1<<zone> ...` avec RGB à l'offset `3 + zone*3` (3 zones : Front 0, Middle 1, Rear 2) ; luminosité `00 23 <v>` ; save `00 11 00` (non confirmé fonctionnel) ; version : feature `00 90 00` puis lecture ; init `00 2D` (compte DPI). Mode : `0x04` seulement.
- **Aerox 5** (`...Aerox5Controller`) : zones 0 Front, 1 Middle, 2 Rear, 3 Reactive. Zone 0 : `00 21 01 R G B` (RGB à l'offset 3) ; zone 1 : `[2]=0x02`, RGB à 6 ; zone 2 : `[2]=0x04`, RGB à 9 ; zone 3 : cmd `0x26`, RGB à 4. Délai 2 ms entre écritures.
- **Aerox sans fil / Rival 3 Wireless** (`...AeroxWirelessController`, 1838, 183A, 1852, 1854, 185C-1862, 1858, 185A, 1878, 187A, 1890, 1830) : tampon de **64** octets (pas 65), timeout 250 ms. Couleur : `00 21 01 <zone> R G B` (RGB à l'offset 4) ; Aerox 9, zone 3 (Reactive) : cmd `0x26`, `[3]=0`. Mode **sans fil 2,4 GHz** : l'octet de commande est OR-é avec **0x40** (`0b01000000`) et une **lecture obligatoire** (accusé) suit chaque écriture ; en mode filaire : écriture seule. Luminosité = 10 paliers (`v/10`), `00 23 <v>`. Effet rainbow (0x05) : `00 22 FF`. Save : `00 11 00` (non confirmé).
- Zones de l'Aerox 3 sans fil : Front, Middle, Rear ; Aerox 9 : Front, Middle, Rear, Reactive ; modes 0x04, 0x05.

### 6.6 QCK Prism Cloth (`SteelSeriesQCKMatController`, interface 0)
- Feature report de **525 octets** : `[1]=0x0E, [3]=0x02`, couleur A en `[5..7]` (avec `[8]=0xFF,[9]=0x32,[10]=0xC8,[14]=1`), couleur B en `[0x11..0x13]` (avec `[0x14]=0xFF,[0x15]=0x32,[0x16]=0xC8,[0x19]=1,[0x1A]=1,[0x1C]=1`). Puis `hid_write` 65 octets avec `[1]=0x0D` (validation). **2 zones** (deux couleurs).

### 6.7 Casques
- **Arctis 5** (1250, 12AA) : report id **0x06**, tampon **37** octets, interface 5 ; couleur : `06 8A 42 .. [4]=0x20 [5]=0x41 [7..9]=RGB [0A]=0xFF [0B]=0x32 [0C]=0xC8 [0D]=0xC8` ; sélection de zone : `06 8A 42 .. [4]=0x20 [5]=0x41 [6]=0x08 [7]=zone [8]=0x01` ; init : `06 81 43 01 22` puis `... 23`.
- **Arctis Nova 3** (12EC) : report id 0x06 ; couleur **521 octets** (`[1]=0xAA`, `[2]=1/0` zone), sélection/appli **64 octets** (`[1]=0xA5/0x09/0xA3`) ; paliers de luminosité à `[0x0D],[0x0E]` (1 -> 30 75, 5 -> 1E 41, 10 -> 94 02).
- **Siberia 350** (1229) : `hid_write` de 16 octets `01 00 <cmd> <len-1> ...` ; couleur = cmd `0x83` + RGB.
- Autres casques (Arctis 7, 9, Nova 7 / Pro / 5 / Elite) : `INCONNU`.

## 7. Cadence, difficulté, tests sans matériel
- Cadence : `INCONNU` (non mesurée ; (O) n'impose pas de délai sauf 2 ms entre écritures Aerox 5).
- Difficulté : **Facile** Apex Pro/7/5 (une seule trame, table des codes confirmée en (G)) ; **Facile** Rival 3 / 5 / 600 / Aerox 3/5 ; **Moyenne** Aerox sans fil (accusé de lecture + OR 0x40) ; **Moyenne** détection de génération Apex (parse version texte, piège 1614). Beaucoup de sous-familles.
- Tests d'octets possibles :
  1. Parser `layout.bin` (§4) : doit retrouver 114 entrées, offset final 604 (115 / 608 pour 1640).
  2. Construire la trame Apex 643 octets avec 3 touches : `[1]=0x3A` ou `0x40`, `[2]=3`, entrées à `3+4i`.
  3. Trame Aerox 5 : zone 2 -> `00 21 04 ...` RGB à 9. Zone 3 -> `00 26 01 ...` RGB à 4.
  4. Rival 5 : 64 octets, `00 21 FF 03`, 10 x RGB à 4.
  5. Comparer version texte `1.19.7` -> Gen3, `4.16.8` -> Gen3 (piège), `1.19.6` -> Gen2.

## 8. Blocages et INCONNU
- **Descripteurs `.edevice` chiffrés (489/489)** : toute la spécification officielle des trames (rapports, tailles, zones, valeurs d'effet) y est ; non lisible sans déchiffrement. **Déchiffrement non tenté** : la consigne interdit tout contournement de protection ; un chiffrement des descripteurs en est un. À décider par l'utilisateur ; sinon, passer par sniffing USB légitime sur matériel possédé (hors périmètre de cette mission).
- Longueurs des reports directs des Apex sans fil (1630/1632/1644/1646) et du Gen3 filaire 1642 : **INCONNU (G)** ; (O) = 643 par défaut.
- Format de réponse de version : **texte `x.y.z` CONFIRMÉ (O)**, pas de champ binaire de génération ; la distinction Gen1/2/3 se fait par PID + comparaison de version. (G) fournit les versions livrées, pas le format.
- Tables de touches pour Apex 3, Apex 3 TKL, Apex 9, Pro TKL 2022/2024, Pro Mini, M750 : pas de `layout.bin` : `INCONNU` (G) ; (O) a des contrôleurs séparés non relus dans cette mission (`Apex3`, `Apex8Zone`, `ApexTZone`, `ApexM`, `OldApex`).
- Valeurs d'effet / zones : Rival 3, Rival 5, Rival 600, Aerox 3/5/9 : (O) uniquement.
- Arctis (hors 5 et Nova 3), QCK au-delà du 525 octets : (O) partiel.
- Prime, Sensei Ten, Rival Pro, Aerox 3 Gen2 : (G) PID seulement ; trames `INCONNU`.
- Aucun pilote noyau SteelSeries nécessaire en écriture HID ; `driver/` d'installation (non exécuté) sert à GG (Sonar, audio virtuel).
- Conflit possible avec le service GG qui tient les interfaces 0xFFC0 des sans fil (cf. `steelseries.md`).
