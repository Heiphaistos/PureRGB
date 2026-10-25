# Corsair iCUE 5.52 : analyse d'interopérabilité (hubs, AIO, LINK, claviers/souris)

Date : 2026-10-23. Légende : **[C]** CONFIRMÉ dans les binaires/données iCUE (source citée) · **[X]** repris de `corsair.md` (liquidctl/OpenRGB/OpenLinkHub), NON confirmé par iCUE · `INCONNU` non établi.
Aucun code ni asset copié ; faits de protocole seulement. Logiciel non installé, non exécuté.

## 1. Source analysée

- **iCUE 5.x, canal « CUE_V5 » (paquets 5.52.93, date des DLL 2026-09-23)**. Le gros installeur 4.x (`iCUESetup_4.33.138_release.msi`, 1,2 Go) existe mais est antérieur à LINK : non utilisé.
- Installeur officiel = « stub » : `https://www3.corsair.com/software/CUE_V5/public/modules/windows/installer/Install%20iCUE.exe`
  (3,2 Mo, SHA-256 `7ecefe7badd2b80d33dce7a82d8abc4caf2404793f17c472b6d06a7cef8145b4`), jamais exécuté. Il ne contient rien : il télécharge un gestionnaire de paquets.
- Les vrais composants viennent du CDN officiel (chaîne de découverte lue dans le stub par `strings`, puis `curl`) :
  `.../modules/icue_initial_installer.json` -> bootstrap `.../windows/installerpackage/cuepkg_bootstrap.zip` (SHA-256 `3f7343d1f0be3932783cd3a58342f0ff28ea737751b1c9ba10b7a8bf6ba9b9e0`)
  -> index `.../windows/packages/cuepkg-metadata.json` (SHA-256 `bb2b4fc471ff3dd5d4eb170133afb70343de5bc3ce583cefd4299d2c0a026ce9`, 524 paquets `.cuepkg` = zip).
- Paquets analysés (SHA-256 complet disponible dans `D:\Projet\Analyses\rgb\corsair\dl\sha256.txt` tant que l'on garde les notes ; préfixes) :
  `bragi-proto` 5.52.93 (`713e450f...`) · `clink` 5.32.99-5.43.3 (`3acd05c1...`, .NET) · `clink-proto2` 5.52.93 (`74b723a3...`) ·
  `legacy-proto` 5.52.93 (`9da55225...`) · `dev-link-system-hub` 5.48.1 (`d22d83c5...`) · `dev-commander-core-xt` 5.34.1 (`cba1b52d...`) ·
  `dev-aio-pro-cooler` 5.31.99 (`3f011db8...`) · `dev-h150i-elite` 5.31.99 (`0b91e8a0...`) · `predefined-data` 5.52.93 · ~390 `dev-*` (tables d'identifiants).
- Méthodes : 7z (paquets), `ilspycmd` (modules .NET du paquet `clink`, noms de types non obfusqués), Ghidra 12.1.4 headless sur `BragiProtocols.dll` (C++/Qt, PDB présent seulement en chemin), parseur Qt-rcc maison pour les `.rcc` de périphériques (manifestes JSON `*.cuebmf`).

### Architecture iCUE (utile pour savoir où chercher)
- **Protocole « CLink » (managé .NET, paquet `clink`)** : Hydro Pro/Platinum/Pro XT/Elite RGB, Commander Pro, Lighting Node, Coolit (H80i..H115i), PSU. Lisible en clair.
- **Protocole « Bragi » (natif `BragiProtocols.dll`)** : Commander CORE / CORE XT / CORE (AIO Capellix), iCUE LINK System Hub, claviers/souris récents, casques. Gabarit HID `Bragi_lighting_node` / `Bragi_kbd` / `Bragi_mouse` [C : champ `HidProto` des manifestes].
- **Protocole « Legacy/Nxp2000 »** (natif `LegacyProtocols.dll`) : claviers K55/K70 MK.2/K95 Platinum, Harpoon/Scimitar [C : `HidProto: Nxp2000_kbd|Nxp2000_mouse`]. Non décompilé : INCONNU.

## 2. Matériel couvert (VID 0x1B1C) [C : `meta/auxiliary.json` de chaque paquet `dev-*`]

Liste issue de `244` PID (échantillon : ~50 paquets `dev-*` non lus faute de temps, donc liste NON exhaustive : PID absents = INCONNU). Type = `device-type`, lister = `lister-stage2`.

### 2.1 Refroidissement, hubs, éclairage

| PID | Modèle iCUE | Type | Famille |
|---|---|---|---|
| 0C02 | H80iGT | liquid-cooler | CLink |
| 0C03 | H100iGTX | liquid-cooler | CLink |
| 0C07 | H110iGTX | liquid-cooler | CLink |
| 0C08 | H80i v2 | liquid-cooler | CLink |
| 0C09 | H100i v2 | liquid-cooler | CLink |
| 0C0A | H115i | liquid-cooler | CLink |
| 0C0B | LIGHTING NODE PRO | led-controller | CLink |
| 0C0C | LIGHTING NODE PRO | led-controller | CLink |
| 0C10 | Commander PRO | led-controller | CLink |
| 0C11 | COMMANDER PRO | led-controller | CLink |
| 0C12 | H150i PRO | liquid-cooler | CLink |
| 0C13 | H115i PRO | liquid-cooler | CLink |
| 0C15 | H100i PRO | liquid-cooler | CLink |
| 0C17 | H115i PLATINUM | liquid-cooler | CLink |
| 0C18 | H100i PLATINUM | liquid-cooler | CLink |
| 0C19 | H100i PLATINUM SE | liquid-cooler | CLink |
| 0C1A | LIGHTING NODE CORE | led-controller | CLink |
| 0C1B | LIGHTING NODE CORE | led-controller | CLink |
| 0C1C | Commander CORE | led-controller | Bragi |
| 0C1D | Commander CORE | led-controller | Bragi |
| 0C1E | LS100 STARTER KIT | led-controller | CLink |
| 0C1F | LS100 STARTER KIT | led-controller | CLink |
| 0C20 | H100i RGB PRO XT | liquid-cooler | CLink |
| 0C21 | H115i RGB PRO XT | liquid-cooler | CLink |
| 0C22 | H150i RGB PRO XT | liquid-cooler | CLink |
| 0C23 | LT100 STARTER KIT | led-controller | CLink |
| 0C24 | LT100 STARTER KIT | led-controller | CLink |
| 0C29 | H60i RGB PRO XT | liquid-cooler | CLink |
| 0C2A | Commander CORE XT | led-controller | Bragi |
| 0C2D | H100i RGB PRO XT | liquid-cooler | CLink |
| 0C2E | H115i RGB PRO XT | liquid-cooler | CLink |
| 0C2F | H150i RGB PRO XT | liquid-cooler | CLink |
| 0C30 | H60i RGB PRO XT | liquid-cooler | CLink |
| 0C32 | Commander CORE | led-controller | Bragi |
| 0C33 | Elite LCD | pump-display | LCD |
| 0C35 | H100i ELITE | liquid-cooler | CLink |
| 0C36 | H115i ELITE | liquid-cooler | CLink |
| 0C37 | H150i ELITE | liquid-cooler | CLink |
| 0C39 | Elite LCD | pump-display | LCD |
| 0C3C | Commander CORE | led-controller | Bragi |
| 0C3F | iCUE LINK System Hub | led-controller | Bragi |
| 0C40 | H100i ELITE WHITE | liquid-cooler | CLink |
| 0C41 | H150i ELITE WHITE | liquid-cooler | CLink |
| 0C4E | iCUE LINK LCD Cap for LINK coolers | pump-display | LCD |
| 0C55 | NAUTILUS LCD CAP | pump-display | LCD |
| 0C56 | Commander DUO | led-controller | CLink |
| 0C57 | NAUTILUS LCD CAP | pump-display | LCD |
| 1D00 | OBSIDIAN 1000D | led-controller | CLink |
| 1D01 | OBSIDIAN 1000D | led-controller | CLink |
| 1D05 | SPEC-OMEGA RGB | led-controller | CLink |

Familles : Hydro Pro/Platinum/Pro XT/Elite = **CLink managé** (`ClinkPlatinumCoolers`) ; Commander CORE/XT/ST + LINK = **Bragi** (`lister-stage2: bragi-device-listing`) ; Commander Pro / Lighting Node / Coolit = CLink managé.
Correspondance PID -> classe iCUE (Hydro) [C : `PlatinumDeviceFactory.cs`] : 0C17, 0C18 = Platinum ; 0C19 = Platinum SE ; 0C20, 0C21, 0C29, 0C2D, 0C2E, 0C30 = Pro XT ; 0C22, 0C2F = Pro XT H150i (3e ventilateur) ;
0C34, 0C35, 0C36 = « Tamriel » (Elite RGB) ; 0C37 = Tamriel H150i ; 0C40 = « Whiterun » (Elite blanc) ; 0C41 = Whiterun H150i ; 0C14 = Corsair ONE.
Noms internes [C : `CoolingLoggerNamingScenario.cs`] : 0C34 H60i ELITE, 0C35 H100i ELITE, 0C36 H115i ELITE, 0C37 H150i ELITE, 0C40 H100i ELITE WHITE, 0C41 H150i ELITE WHITE.
Les PID 0C3D/0C3E cités par OpenRGB sont absents des paquets iCUE 5.52 : INCONNU. Les écrans LCD Elite (0C33, 0C39) et le « LCD Cap » LINK (0C4E) sont listés `pump-display` : hors périmètre ici (voir `screens-protocols`).

### 2.2 Claviers, souris (PID VID 1B1C)

**Claviers** (74) : 1B11 K95 RGB ; 1B13 K70 RGB ; 1B17 K65 RGB ; 1B2D K95 RGB PLATINUM ; 1B33 K70 LUX RGB ; 1B36 K70 LUX ; 1B37 K65 LUX RGB ; 1B38 K70 RGB RAPIDFIRE ; 1B39 K65 RGB RAPIDFIRE ; 1B3A K70 RAPIDFIRE ; 1B3D K55 RGB ; 1B3F K68 ; 1B40 K63 ; 1B41 K66 ; 1B42 K83 WIRELESS ; 1B45 K63 WIRELESS ; 1B49 K70 RGB MK.2 ; 1B4F K68 RGB ; 1B50 K63 WIRELESS ; 1B55 K70 RGB MK.2 LP ; 1B59 K70 RGB SE RAPIDFIRE ; 1B6B K70 RGB MK.2 SE ; 1B6D K83 WIRELESS ; 1B6E K57 RGB WIRELESS ; 1B73 K70 RGB TKL Champion Series ; 1B7C K100 RGB ; 1B7D K100 RGB ; 1B82 K95 RGB PLATINUM SE ; 1B83 K60 PRO ; 1B87 K70 RGB MK.2 IG ; 1B89 K95 RGB PLATINUM XT ; 1B8C K63 WIRELESS ; 1B8D K60 RGB PRO SE ; 1B8F K63 WIRELESS ; 1BA0 K60 RGB PRO ; 1BA1 K55 RGB PRO XT ; 1BA4 K55 RGB PRO ; 1BA5 K68 SE ; 1BAB K100 RGB Air Wireless ; 1BAD K60 RGB PRO LP ; 1BAF K65 RGB Mini ; 1BB3 K70 RGB PRO ; 1BB6 K70 RGB PRO Mini Wireless ; 1BBD K65 RGB Mini ; 1BC0 K70 MAX ; 1BC3 K65 RGB Mini ; 1BC4 K70 RGB PRO ; 1BC5 K100 RGB ; 1BC6 K70 RGB PRO ; 1BC7 K60 PRO TKL RGB ; 1BCF K65 RGB Mini ; 1BD3 K70 RGB PRO Mini Wireless ; 1BD4 K70 RGB PRO ; 1BD7 K65 PRO Mini ; 1BED K60 PRO TKL RGB ; 1BFD K70 CORE RGB ; 1BFE K55 CORE RGB ; 1BFF K70 CORE RGB ; 2B01 K70 CORE RGB TKL ; 2B02 K70 CORE RGB TKL WIRELESS ; 2B04 MAKR 75 ; 2B05 K55 CORE RGB ; 2B09 K70 CORE SE RGB ; 2B0A K70 CORE RGB ; 2B10 K65 PLUS WIRELESS ; 2B11 K65 PLUS WIRELESS ; 2B12 K65 PLUS WIRELESS ; 2B14 K70 PRO RGB TKL ; 2B1A MAKR PRO 75 ; 2B20 K55 CORE RGB TKL ; 2B23 K65 PLUS WIRELESS ; 2B36 K65 PLUS WIRELESS ; 2B37 K65 PLUS WIRELESS ; 2B5B MAKR PRO 75.

**Souris** (52) : 1B12 M65 RGB ; 1B14 SABRE RGB OPTICAL ; 1B19 SABRE RGB LASER ; 1B1E SCIMITAR ; 1B22 KATAR ; 1B2E M65 PRO RGB ; 1B2F SABRE RGB ; 1B32 SABRE ; 1B34 GLAIVE RGB ; 1B35 DARK CORE RGB ; 1B3C HARPOON RGB ; 1B3E SCIMITAR PRO RGB ; 1B4B DARK CORE SE ; 1B4C IRONCLAW RGB WIRELESS ; 1B51 DARK CORE SE ; 1B5A M65 RGB ELITE ; 1B5C NIGHTSWORD RGB ; 1B5D IRONCLAW RGB ; 1B5E HARPOON RGB WIRELESS ; 1B64 DARK CORE RGB ; 1B70 M55 RGB PRO ; 1B74 GLAIVE RGB PRO ; 1B75 HARPOON RGB PRO ; 1B79 SABRE RGB PRO CHAMPION SERIES ; 1B7A SABRE PRO CHAMPION SERIES ; 1B7E DARK CORE PRO SE ; 1B80 DARK CORE PRO ; 1B88 M65 RGB ELITE IG ; 1B8B SCIMITAR RGB ELITE ; 1B93 KATAR PRO ; 1B94 KATAR PRO WIRELESS ; 1B98 SABRE RGB PRO WIRELESS ; 1B9E M65 RGB ULTRA ; 1BAC KATAR PRO XT ; 1BAE KATAR PRO WIRELESS ; 1BB2 DARKSTAR WIRELESS ; 1BB5 M65 RGB ULTRA Wireless ; 1BB8 Nightsabre Wireless ; 1BBA KATAR ELITE Wireless ; 1BD6 M65 RGB ULTRA Wireless ; 1BDB SCIMITAR RGB ELITE WIRELESS ; 1BE3 SCIMITAR RGB ELITE ; 1BEF M65 RGB ULTRA ; 1BF0 M75 ; 1BF1 M75 WIRELESS ; 1BF2 M75 AIR WIRELESS ; 1BFB M55 WIRELESS ; 2B03 M55 ; 2B08 M75 WIRELESS ; 2B1E M55 WIRELESS ; 2B22 SCIMITAR ELITE WIRELESS SE ; 2B32 IRONCLAW WIRELESS SE.

**Récepteurs sans fil** : 0A6B HS80 WIRELESS RECEIVER ; 0A73 HS80 WIRELESS RECEIVER ; 1B62 K57 WIRELESS RECEIVER ; 1B65 HARPOON WIRELESS RECEIVER ; 1B66 IRONCLAW WIRELESS RECEIVER ; 1B7F DARK CORE PRO SE WIRELESS RECEIVER ; 1B81 DARK CORE PRO WIRELESS RECEIVER ; 1BB0 K57 HARPOON RGB WIRELESS BUNDLE ; 2B07 K65 PLUS WIRELESS RECEIVER ; 3A09 SCUF ENVISION PRO WIRELESS RECEIVER.

Protocole par modèle (échantillon manifestes [C]) :

| Modèle (PID) | `DeviceEnumerator` / `HidProto` | `LightingCommandSupport` | Ping requis |
|---|---|---|---|
| K55 RGB (1B3D) | Legacy / Nxp2000_kbd | Zone (3 zones) | - |
| K70 RGB MK.2 (1B49) | Legacy / Nxp2000_kbd | PerColorCommand | - |
| K95 RGB Platinum (1B2D) | Legacy / Nxp2000_kbd | PerColorCommand | - |
| Harpoon RGB (1B3C), Scimitar RGB Elite | Legacy / Nxp2000_mouse | Zone | - |
| K70 RGB TKL (1B73), K65 RGB Mini (1BAF) | **Bragi** / Bragi_kbd | PerColorCommand | 60 000 ms |
| Dark Core RGB Pro (1B80) | **Bragi** / Bragi_mouse | Zone | 60 000 ms |

Le champ « Ping requis » (`RequiredPingInterval`) est la période max entre deux échanges pour que le firmware reste en mode logiciel : 60 000 ms (Bragi clavier/souris), **30 000 ms** (Commander CORE, CORE XT, LINK System Hub), 0 (sous-appareils LINK) [C : manifestes]. OpenRGB cite ~10 s pour le CORE [X] : la valeur iCUE est l'intervalle d'envoi de ping de l'application, pas le délai de repli du firmware : délai réel INCONNU.

## 3. Hydro Platinum / Pro XT / Elite RGB (CLink, 0C17..0C41) [C : DLL `Corsair.Module.PlatinumDevices`]

### 3.1 Trame
- Rapport HID de **64 octets de charge + 1 octet d'ID (0x00)** en écriture (`WriteBuffer` copie la trame à l'offset 1 d'un tampon `MaxOutputReportLength`, octet 0 = 0) ; lecture = 64 octets sans ID. Timeouts de lecture/écriture HID 20 ms. `Constants.PacketLenght = 64`, `XMCPayloadLenght = 63` (0x3F).
- Octet 0 = `0x3F` (longueur de charge) ; octet 1 = **séquence | commande** ; octets 2..62 = données ; **octet 63 = CRC-8 CCITT** (poly 0x07, init 0, sans réflexion, table standard : 0,7,0E,9,1C...) calculé sur les **octets 1 à 62 inclus**. Réponse vérifiée de même (`data[63]` = CRC de 1..62).
- Séquence : compteur démarrant à 8, **+8 à chaque trame** (modulo 256, jamais 0) ; les 3 bits de poids faible portent la commande (champ « feature » de liquidctl). Octet 1 des commandes : fan/pompe `seq|0`, LED mode `seq|1`, index palette `seq|2` et `seq|3`, couleurs palette `seq|4..7`.
- Lecture : trame `[3F][seq][FF][code]...` + CRC, `code` = `GetStatus 00`, `GetPumpFan F8`, `GetLedMode F9`, `GetLedIndex0 FA`, `GetLedIndex1 FB` (sert aussi à lire le 3e ventilateur), `GetLedPalette0..3 FC..FF`. Codes d'écriture de l'énumération : `SetPumpFan 80`, `SetLedMode 81`, `SetLedIndex0/1 82/83`, `SetLedPalette0..3 84..87`.
- Synchronisation/statut : trame `[3F][seq][FF][00]` + CRC (`OutgoingSyncPacket`) -> réponse d'état.

### 3.2 Réponse d'état (code 00) [C : `IncomingStatePacket`]
Offsets (sur la trame de 64 o lue) : 2 version firmware, 3 révision, 4 statut (`50` OK ; erreurs = exception), 5-6 compteur de paquets u16 LE, 7-8 température liquide (octet 8 = °C entiers, octet 7 = fraction /256 : `8 + 7/256`), 9 compte à rebours (watchdog),
puis ventilo 0 : 10 mode, 11 % cible (0-255), 12-13 RPM cible, 14 % actuel (0-255), 15-16 RPM actuel ; ventilo 1 : 17..23 (même structure) ; pompe : 24 mode (0 silence, 1 équilibré, 2 performance), 25 % cible, 26-27 RPM cible, 28 % actuel, 29-30 RPM actuel ; ensuite 1 octet état de sécurité (failsafe) et un hash u32.
Les pourcentages sont stockés 0-255 (`pct = round(v*100/255)` ; écriture `(pct*255+50)/100`). Ce sont exactement les offsets de liquidctl [X] (14/15, 21/22, 28/29).

### 3.3 Profil de refroidissement (commande 80, `OutgoingFanPacket[V2]`) [C]
Deux versions de firmware (choix par version : majeur<=1 et (majeur!=1 ou mineur<1) -> V01, sinon V02 ; `ProtocolVersionHelper`).
Champs communs (trame de 64 o, remplie de `FF` au départ) : [2] compte à rebours (défaut **20** : `DefaultCountdownTimer`), [3] `0x55` = enregistrer en EEPROM / `00` = RAM seule, [4] `FF`, [5] `05`.
Profil de sécurité ventilateur (bit 8 du type) : [6]=00, [7] Tdésactivation, [8] Tactivation, [9] Trampstart, [10] Tmax.
- **V01** : ventilo 0 à l'offset 17 ; ventilo 1 à 23 ; mode pompe à **29** ; température externe pompe u16 LE (valeur x256) à 32-33 ; nombre de points `7` à 35 ; courbe à 36.
- **V02** : ventilo 0 à l'offset **11**, ventilo 1 à **17** ; mode pompe à **23** ; température externe aux 26-27 ; nb points `7` à 29 ; courbe ventilo 0 à **30**, ventilo 1 à **44**.
- Bloc ventilo (6 octets) : mode, vitesse cible u16 LE (RPM), température externe u16 LE (x256, seulement modes « externe »), puissance cible (0-255).
  Modes : `00` courbe (temp interne), `01` courbe (temp externe), `02` % fixe, `03` % fixe avec repli, `04` RPM fixe, `05` RPM fixe avec repli, `08` courbe CPU/GPU interne, `09` courbe CPU/GPU externe, `FF` non défini.
- Courbe : 7 points (température °C entière, puissance 0-255) consécutifs.
- Pompe : seulement 3 modes (`00` Quiet, `01` Balanced, `02` Performance). **iCUE n'expose aucun % ni RPM de pompe pour ces modèles.** Le mode « vitesse variable » d'iCUE choisit le mode selon la charge CPU : Quiet 0-30 %, Balanced 31-60 %, Performance 61-100 %, au plus un changement toutes les **900 ms** [C : `TamrielVariableSpeedDefinitions`, `PumpVariableSpeedStateHolder`].
- Limites iCUE (`PlatinumCoolerPoints`) : ventilos **20 % min**-100 %, RPM 400-3000, températures de courbe 20-60 °C ; Pro XT : % min 20, RPM 300-3000 (`AsetekCoolerProPoints`). Courbes par défaut par PID (Quiet/Balanced/Performance) dans `PlatinumCoolerPoints` (non recopiées).
- Sécurité : le firmware a un watchdog (octet [2] / compte à rebours 20) et un profil de sécurité ventilo (Tdésact/Tact/Trampstart/Tmax) ; trame envoyée par iCUE avec pause adaptative (`AdaptiveSleep`) entre écritures. Après erreurs répétées iCUE exécute une procédure de « reboot EFM8 » : écriture HID de `[00 00 80]` puis `[00 00 C0]` (changement de baud-rate du pont UART/EFM8). **Ne pas reproduire sans besoin.**

### 3.4 Éclairage [C : paquets LED]
- **Mode LED (cmd `seq|1`)** : [2]=00, [3]=01, [4]=FF ; [17..19]=7F ; [20],[21] = index de couleurs aléatoires (<=79 sinon 7F) ; jusqu'à **6 cycles** de 5 octets à partir de l'offset 22 : mode de cycle, vitesse u16 LE, premier index palette, dernier index palette ; offset 52 = luminosité (`(pct*255+50)/100`).
  Mode « réactif » (température) : [4]=00 puis 3 triplets (T°C, B, G, R) à partir de 5 ; [19]=00.
  Modes de cycle (`LedCycleMode`) : 00 off, 01 shift up, 02 shift down, 03 ping-pong, 05/06 copy up/down, 09/0A/0B/0D/0E variantes « blend », 23 (35) ping-pong 2 couleurs, 2B (43) blend ping-pong 2 couleurs, 40 copy-all up, 80 copy-all down.
- **Palette d'index** : 40 index/trame, 2 trames max, commandes `seq|2`, `seq|3`, remplissage `FF`. **Palette de couleurs** : 20 couleurs/trame en **B, G, R**, 4 trames max (80 couleurs), commandes `seq|4..7`, remplissage 00.
- **Éclairage direct** (`OutgoingDirectLedPacket`) : `seq|1`, [2]=1 (activer)/0, [3]=01, [17..20]=7F, octets 22,27,32,37,42,47 = 0, [52]=luminosité. Ceci est la trame que iCUE envoie pour passer l'éclairage en mode « direct » ; je n'ai PAS retrouvé les 3 trames « magiques » de liquidctl [X] (features 1,2,3) comme constantes de l'application : leur origine est INCONNUE. La trame ci-dessus est confirmée.
- La couleur par LED en mode logiciel passe ensuite par la palette (couleurs + index) : le mode direct par pixel via trames successives n'est pas détaillé ici (INCONNU).
- Nombre de LED : Platinum DefaultLedChannelCounts 2 / type `MlFanHub` [C : manifeste H115i Platinum], Elite RGB 1 canal [C : H150i Elite] ; nombre exact de LED par modèle : voir `corsair.md` [X].

### 3.5 Écran LCD
Les Elite LCD (0C33, 0C39) apparaissent comme `pump-display` ; protocole non analysé ici : INCONNU (voir `docs/screens-protocols/`).

## 4. Commander CORE / CORE XT / ST (Bragi) 

### 4.1 Ce qui est CONFIRMÉ
- PID : 0C1C et 0C1D (« Commander CORE », paquet `dev-aio-pro-cooler`, 1 port pompe + 6 ports ventilateur : `DeviceCoolerPortTypeMapping {0:PumpPort, 1..6:FanPort}`) ; 0C32 et 0C3C « Commander CORE » ; **0C2A « Commander CORE XT »** (6 ports ventilateur `FanPort`, pas de pompe) [C : `auxiliary.json`/manifestes].
- Énumération `DeviceEnumerator: Bragi`, `HidProto: Bragi_lighting_node`, `RequiredPingInterval: 30000`, `DirectLightingsImplementations: ["RealTimeLightingsBuffer"]`, 2 canaux de géométrie pour le XT (6 ventilos + 1 port externe), `DefaultLedChannelCounts [6,1]`, `DefaultLedChannelTypes ["QlRgbFanHub","Autodetect"]`, `DynamicGeometryLedIdLast Led_Cmd_Led2099` (jusqu'à 2100 LED adressables).
- Types de ventilateurs reconnus (CORE XT) : `QlRgbFanHub` **34 LED/ventilo**, `LlFanHub`, `MlFanHub`, `HdFanHub`, `Fan8LedSeries`, `RgbPumpCap13Leds`, bandes `Strip15/21/27/40/49/84Leds` (noms = nombre de LED), `RgbStrip` 10, `RgbPowerExtensionCable` 25 [C : gabarits de géométrie]. Max par canal bandes : 138 (ExternalRGBLightStrip), 204 (LS Aurora) ; chaîne « HydroX » : 69.
- Tampon temps réel des couleurs : ressource Bragi `RealTimeLightingsBuffer`. `BragiFwWorkarounds` du CORE XT : `saveCoolerSettingsForDisconnectedSensors`. `CompatibleToImportUsbIds` (anciens PID dont iCUE importe les profils) : 0C0B, 1D04, 1D00, 0C1A, ...
- `SupportsCoolersAutoDetection: true`, `SupportsLedAutoDetection: true`, `DoChannelAutodetectOnlyOnce: true` : le matériel sait auto-détecter ventilos et types de LED (ressources `CoolersAutodetectionSettings/Results`, `LightingChannelsAutodetectionSettings/Results`).
- **Limites de refroidissement iCUE** [C : `predefined-data/commonCoolingDefaults`] : profil « FetaPlus » (Commander CORE) et « CommanderCoreXt » : ventilo **% fixe 20-100 (pas 1)**, RPM fixe **300-5000** (défaut 600 RPM / 40 %), température de courbe 20-100 °C ; courbe par défaut FetaPlus (T°C -> %) : 29->25, 31->33, 34->43, 35->55, 36->68, 37->85, 38->100 ; CommanderCoreXt : minPercents 20.

### 4.2 Ressources Bragi (identifiants numériques)
Les ressources Bragi sont des « fichiers/propriétés » adressés par un octet. Constantes retrouvées dans le code (appels `écrire-ressource(id, objet)` dans `BragiProtocols.dll`) [C] :

| ID | Ressource | Fonction iCUE (adresse VA Ghidra) |
|---|---|---|
| 0x18 | `CoolersTargetRotationSettings` (consignes de rotation) | `FUN_1800a2e70` (message « Coolers Target Rotation Settings », écrit via `FUN_1800814b0`) |
| 0x1D | nombre de contrôleurs/LED par canal (`LightingChannelsControllersQuantity`) | `FUN_1800c6250` (« SetGeometryToDevice ») |
| 0x1E | géométrie/types par canal | idem |
| 0x1F | `LightingChannelsBrightnesses` | `FUN_1800d7a30` |
| 0x2B | `GamepadSensitivitySettings` | `FUN_1801012c0` |
| 0x37 | (non nommé par le journal) | `FUN_180141850` |
| 0x48, 0x4B | `GameModeActuationSettings`, `AnalogSwitchSettings` (claviers) | `FUN_1800f61c0`, `FUN_1800e4230` |
| 0x4F | seuil PinProtect+ (PSU) | `FUN_18016cff0` |

Les identifiants cohérents avec OpenRGB/OpenLinkHub [X] : 0x17 RPM actuels (`CoolersActualRPMs`), 0x21 températures (`TemperatureMeasurementsStatus`), 0x22 tampon de couleurs (`RealTimeLightingsBuffer`), 0x20 résultats d'auto-détection. L'ordre des noms dans la table d'énumération de la DLL (début à `1802ce650`) est identique avec un décalage constant (+2/+3) vis-à-vis de ces ID, **mais la table de valeurs n'a pas été retrouvée** : les ID 0x17/0x20/0x21/0x22 restent [X] (recoupés, pas lus).

### 4.3 Objet « ventilateurs » (lecture/écriture)
- **Écriture d'une consigne** (`CoolersTargetRotationSettings`, 0x18) [C : `FUN_1800a2e70`] : le client construit un vecteur d'éléments de **4 octets** `{canal u8, estRPM u8 (0 = %, 1 = RPM), valeur u16 LE}`, un élément par canal ventilateur/pompe ; le journal d'iCUE affiche « Cooler channel N : valeur Rpm|% ». L'index de canal suit `DeviceCoolerPortTypeMapping` : sur CORE XT canaux 0..5 = ventilos 1..6 ; sur CORE (AIO) canal 0 = pompe, 1..6 = ventilos.
  C'est exactement la forme `[N] puis N x [canal,00,pct lo,hi]` d'OpenLinkHub [X], donc le **format 4 octets est confirmé** ; l'octet de compte `N` en tête est [X].
- **Lecture des RPM** : réponse de 0x17 [X] : octet 6 = N, puis N x (statut u8 `00`=OK, u16 LE RPM) à partir de 7 (OpenLinkHub). Confirmation iCUE : classe `ChanneledCoolersRotationSpeedSensorsController` (capteurs par canal) ; offsets exacts INCONNU côté iCUE.
- **Réécriture sûre** : iCUE réécrit TOUT le vecteur (les canaux non modifiés reprennent leur valeur) ; message d'échec « Commit coolers target rotation settings failed ». Le firmware CORE XT garde en plus des modes persistants (`CoolersMode`, `CoolersFixRotateSpeed`, `CurveRPM` : noms de ressources présents) : format INCONNU.
- Courbe « matérielle » CORE (liquidctl `60/61/62 6D` [X]) : non retrouvée dans iCUE 5.52 (noms `CoolersCurveDescription`, `CoolersMode` existent, IDs INCONNU).
- Taille de rapport HID par PID : **INCONNU côté iCUE** (OpenRGB : 97/65 o CORE/ST, 385 o XT [X]).
- Détail de la sécurité pompe : INCONNU en dur. Valeurs iCUE (courbes par défaut pompe) : voir §5.4.

## 5. iCUE LINK System Hub (0C3F, Bragi)

### 5.1 Identification [C : manifeste `link_system_hub_manifest.cuebmf`]
- PID **0C3F** (3135), `HidProto: Bragi_lighting_node`, `DeviceEnumerator: Bragi`, `IsMasterController: true`, `EmulatedDeviceType: LinkSystemHub`, `SdkDeviceType: LightingNodePro`.
- `MaxSubdevicesOnSide: 12` (maximum 12 appareils de chaque côté, 2 chaînes de 12), `MaxLedsCountAcrossAllChannels: 476`, `LightingsBufferHunkSize: 4096` (le tampon d'éclairage est écrit par morceaux de 4096 octets), `SensorsChannelsRange: 26`, `RequiredPingInterval: 30000`.
- Couleurs : `DirectLightingsImplementations ["RealTimeLightingsBuffer"]`, `HostControlsColorBrightness: true` (luminosité appliquée par l'hôte), niveaux 0/33/66/100, **carte de réduction de luminosité** `BrightnessReductionMap` (courant -> % : 238:100, 273:90, 308:80, 343:70, 378:60, 413:50, 448:40, 483:30, 519:22) : la luminosité maximale diminue quand la somme des LED allumées dépasse ces seuils (valeurs sans unité confirmée, vraisemblablement mA/10 : INCONNU).
- `BragiFwWorkarounds`: `clearUnusedFiles`, `resetToDefaultOnOperatingModeChangeFail`, `useCommonBufferIndices`. Firmware : `DisallowUnzippedFirmware`, checksum CRC32_MPEG2, outil `BragiFwUpdateTool`. **Ne pas implémenter la mise à jour firmware.**
- `SupportedCoolingImplementations: ["OnBoardControl","DirectControl"]`, `CoolingConfigurationType: LinkMasterController`, `SupportsFanTypeSelection`, `SupportsHwLightings`.

### 5.2 Énumération de la chaîne
- iCUE pose des ressources « `ConnectedSubdevicesIds` », « `PairedSubdevicesIds` », `ConnectedSubdevicesBitmap(2)` (noms présents dans la DLL) ; le sous-appareil est créé par `BragiLinkEcosystemSubdevice` (HidProto `Bragi_link_ecosystem_subdevice`, ping 0). Commande/format exacts : INCONNU côté iCUE ; la séquence `0D 01` / `08 01` / `36` d'OpenRGB/OpenLinkHub [X] reste l'unique description (voir `corsair.md`).
- **Identifiants de type de sous-appareil** (`SupportedCorsairSubdevices`, id = type + 256 x modèle) [C] :

| id | classe | id | classe | id | classe |
|---|---|---|---|---|---|
| 1,2,3,4,10,15,19,26,28 | Fan | 5, 261, 517 | LightingNode | 6,7,9,12,14,17,25,263,273,282,519,529,775,785,1031,1041,1287,1297 | LiquidCooler |
| 16 | PumpFan | 13, 20, 27 | LightingNode | 11,21,29,267,277,523 | PSU |
| 2074 | DashboardLCD | | | | |

  Cela confirme que le couple (type, modèle) lu dans la réponse [X] se lit en id u16 = type | modèle << 8 (ex. 261 = 0x0105 = type 05 modèle 01).
- **Nombre de LED par type** (`LedChannelTypeLedsCount`) [C] : QX RGB (`QxRgbFanHub`) 34 ; LX RGB (`LxRgbFan`) 18 ; RX RGB (`RxRgbFan`) 8 ; RX MAX (`RxRgbMax`) 8 ; `RxFanNoLeds`/`RxMaxNoLeds`/`LinkGenericFanNoLeds` 0 ; LL (`LlFanHub`) 16 ; pompe `CoolerPump20Leds` 20 ; `LinkTitanPump` 20 ; `LinkCoolerScreen40Leds` (pompe LCD) 40 ; XC7 Elite (`CpuBlockXC7Elite`) 24 ; XD5/XD6 Elite (`XD5ElitePumpRes`/`XD6ElitePumpRes`) 22 ; `EliteLCDXD` 22 ; XG3 Hybrid 18 ; XG7 RGB 16 ; XD3 16 ; XD7 36 ; `OriginOA` 32 ; châssis 5000T 204, 9000D châssis 22 / IO panel 18 / logo 4 ; `Strip*Leds` = leur nombre ; `RgbPowerExtensionCable` 25 ; `UsbArgbFanController` 0 (variable).
  Écarts avec la table d'OpenRGB [X] : OpenRGB donne 22 LED au 9000D (= châssis seul) et ne distingue pas IO/logo ; 5000T = 160 (OpenRGB) contre **204** (iCUE).
- Configuration d'un canal du hub : jusqu'à 24 extensions (`SupportedExtensionsCount` 24 par type).
- Identification pompe/ventilateur : par la classe (`Fan`, `PumpFan`, `LiquidCooler`) ci-dessus ; les AIO LINK (H100i/H150i...) sont des `LiquidCooler` à pompe (`CoolerPump20Leds`, `SupportsCoolingPumpTemplates`).

### 5.3 Trames de couleur
- Le hub reçoit **un tampon unique RVB** dans la ressource `RealTimeLightingsBuffer` (0x22 [X]), contenant dans l'ordre les LED de tous les appareils de la chaîne (somme des LED ci-dessus), découpé en morceaux de **4096** octets maximum [C : `LightingsBufferHunkSize`]. Entête/offset de chaque morceau et ordre RVB : [X] (voir `corsair.md`), non lus dans le code.
- Gabarit géométrique : `DynamicGeometryLedIdFirst/Last = Led_Cmd_Led0..Led_Cmd_Led1019` (1020 LED max pour la géométrie dynamique, même si 476 utilisables).
- `RequiresApplyGeometryCommand: true` : après modification de la chaîne, il faut une commande « apply geometry » (ressources 0x1D/0x1E).

### 5.4 Refroidissement LINK
- Écriture d'une consigne : ressource 0x18, éléments 4 octets (§4.3) [C]. Les ventilos LINK ont besoin d'un type configuré (`SupportsFanTypeSelection`).
- Valeurs par défaut d'iCUE (`LinkMasterController`, fichier `commonCoolingDefaults`) : % fixe **20-100**, RPM fixe **300-5000**, température 20-100 °C. Courbe pompe par défaut (T°C -> %) linéaire : 29->20, 30->25, 31->34, 33->42, 34->54, 35->66, 37->70 ; « exponentielle » : 34->0, 35->20, 36->34, 37->43, 38->52, 40->58, 42->60 ; points de charge : 0->0, 25->36, 50->39, 75->48, 100->58. Courbe ventilos par défaut : 44->20, 52->30, 57->38, 60->45, 63->54, 66->66, 70->70.
  **Le plafond « pompe 70 % » des courbes par défaut et le minimum de pompe 20 % (50 % chez OpenLinkHub [X]) ne sont pas des limites matérielles prouvées** : INCONNU. Recommandation : ne jamais descendre la pompe sous 50 % et ne jamais couper un ventilateur d'AIO.
- Capteur : `CoolingTemperatureSensor: Package` (CPU) : iCUE alimente la courbe avec la température logicielle ; le matériel gère aussi `OnBoardControl`.

## 6. Hydro Pro/Platinum (CLink) - références croisées
Voir §3. Les Commander Pro (0C10/0C11) restent décrits dans `corsair.md` [X] (manifeste : `ClinkLightingNode`, 408 LED max, capteurs température + tension, `sharedStorage CommanderPro`).

## 7. Claviers et souris

- Deux protocoles : **Legacy/Nxp2000** (K55 RGB, K70 RGB MK.2, K95 RGB Platinum, K65 RGB, K63, Harpoon, Scimitar, Dark Core RGB, M65...) et **Bragi** (K70 RGB TKL, K65 RGB Mini, K100 RGB, K70 Core/Pro, K65 Plus Wireless, Dark Core RGB Pro, Katar Pro, M75...). La liste des PID est au §2.2.
- Pour les modèles Bragi : couleurs par touche via la ressource `RealTimeLightingsBuffer` (K70 TKL) ou `RealTimeRgbLightings` (Dark Core RGB Pro) ; K65 Mini : en plus `IndicationRealTimeLightingsBuffer` (LED d'indication) [C : manifestes]. Le retour au mode matériel : `RequiredPingInterval` 60 000 ms (au-delà le firmware reprend la main, délai exact INCONNU) ; iCUE possède aussi un mode `SelfOperated` (message « Can't return device to SelfOperated mode » dans `BragiProtocols.dll`) et des « operating modes » (Host/SelfOperated, commande `SetOperatingMode` du client Clink) : valeur numérique INCONNU.
- Pour les modèles Legacy (`Nxp2000`) : trames par touche non analysées (`LegacyProtocols.dll` non décompilé) : INCONNU. Les formats déjà publics (ckb-next, OpenRGB) restent l'unique source.
- `SupportedLightingTypes` des claviers : effets matériels = spirale arc-en-ciel, vague arc-en-ciel, color shift, color pulse, ... (`SpiralRainbow/RainbowWave/ColorShift/ColorPulse/...LightingTypeData`) ; effets stockés dans des ressources `HostControlledLightingsList`/`SolidLighting`, `ColorWaveLighting`, `StaticColorLighting`... (noms).

## 8. Faits de sécurité
- Ne jamais écrire un profil Hydro sans le watchdog (octet [2] = compte à rebours 20) et sans CRC correct ; **ne jamais envoyer** le « flush baud-rate » `[00 00 80]`/`[00 00 C0]` ni la mise à jour firmware (`BragiFwUpdateTool`, CRC32_MPEG2, modes Bootloader/SecondaryBootloader) : risque de brique.
- Hydro : pompe 3 modes seulement ; ventilos >= 20 %, RPM 400-3000 (Platinum) ; ne pas écrire l'EEPROM (`0x55` à l'octet [3]) à chaque changement : usure ; envoyer `00` (RAM) pour un usage dynamique.
- Bragi : ne pas laisser expirer le ping (30 s hub/CORE, 60 s clavier) ; ne pas écrire `Firmware`, `Bootloader`, `SecondaryChipFirmware`, `ScreenModuleFirmware`, `WirelessEncryptionKey`, `PairingId*` (ressources présentes dans la table de noms).
- Bragi : `resetToDefaultOnOperatingModeChangeFail` indique que le passage de mode logiciel/matériel peut échouer et réinitialiser l'appareil.
- Luminosité hub LINK : respecter `BrightnessReductionMap` (courant limité par le bus d'alimentation).
- iCUE détient les HID : fermer le service avant (`Corsair.Service.exe`).

## 9. INCONNU (récapitulatif)
- Offsets exacts des réponses Bragi (lecture RPM, température, énumération LINK), taille des rapports par PID, en-têtes de lecture/écriture : non lus (DLL C++ Qt, pas de table de constantes exploitable en l'état).
- Valeurs numériques des ID 0x17, 0x20, 0x21, 0x22 (déduits/recoupés avec [X] seulement), `CoolersMode`, `CoolersFixRotateSpeed`, `CurveRPM`.
- Trames Legacy/Nxp2000 (claviers K55/K70 MK.2/K95), retour au mode matériel de ces modèles.
- Éclairage direct complet Hydro (3 trames d'activation de liquidctl) et nombre de LED par modèle.
- Écran LCD Elite/LINK ; PID 0C3D/0C3E ; minimum de sécurité de la pompe.

## 10. Difficulté de réimplémentation
Hydro Platinum/Pro XT : **moyenne-faible** (trame complète, CRC, offsets confirmés). Commander CORE/XT : **moyenne** (format 4 octets confirmé, reste Bragi à lire sur matériel). LINK : **moyenne-élevée** (énumération non confirmée, mais LED/types/limites confirmés). Claviers Bragi/Legacy : **élevée** sans analyse supplémentaire.

## 11. Tests d'octets possibles sans matériel
- CRC-8 : table 0,7,0E,9,1C,1B,12,15,38,3F... ; `crc(octets 1..62)` placé à l'octet 63 ; une réponse valide redonne le CRC égal à l'octet 63.
- `[3F][08|0][00]...` : séquence = 8 puis +8 ; commande = `seq | n` (n de 0 à 7) ; trame de lecture état : `3F 10 FF 00 ... CRC`.
- Conversion pourcentage : `(v*255+50)/100` ; température pompe externe : `T*256` u16 LE.
- Vecteur LINK/CORE : `[canal, estRPM, lo, hi]` x N.
- Somme LED d'une chaîne LINK <= 476 ; trames de couleur multiples de 3 octets, morceaux <= 4096.

## 12. Complément 2026-10-25 : couche transport Bragi (passe REA ciblée, arrêtée tôt)

Source : `bragi-proto_5.52.93` retéléchargé depuis `https://www3.corsair.com/software/CUE_V5/public/modules/windows/packages/` dans `D:\Projet\Analyses\icue-2026-10-25\dl` (SHA-256 du .cuepkg `713e450fa10b2fb8b4dc94a0b8b4a5b7e7e140b6f2449182c0dc05075263b925`, identique au premier téléchargement ; rien exécuté). `BragiProtocols.dll` (4 115 096 o, SHA-256 `501976887f54886ee8a407ccdb2c73ac6b7499de36a5712db10a0f27d91a3f8a`) : Authenticode **Valid**, signataire Corsair Memory, Inc. Outil : REA/Ghidra 12.1.4, pseudocode de `FUN_1800a2e70`, `FUN_1800814b0`, `FUN_18024fad0`, `FUN_1802401f0`, `FUN_180239290`.

### 12.1 Établi [C]
- **Écriture de propriété** `FUN_18024fad0` (message « Set property %1 failed. ») : prend un **identifiant de propriété u16** et une **valeur de 4 octets** (copiés en un bloc `{u16 id, u32 valeur}` = 6 octets), plus un second tampon de **32 octets** (8 x u32), puis appelle `FUN_1802401f0` (émission + attente de réponse). Donc la forme « propriété = id u16 + valeur » est confirmée côté iCUE ; la taille du tampon associé (32 o) est observée, sa sémantique INCONNUE.
- `FUN_1802401f0` : sérialise la requête (`FUN_180239290`), l'envoie via un objet transport (appel virtuel +0x10), puis, si une réponse est attendue (paramètre `param_4 != 0`), valide « Invalid reply buffer. » (`FUN_180214300`) et désérialise (`cereal`, archive binaire) la réponse ; vérifie un code d'erreur (`FUN_1802463a0`) avec relance possible (`FUN_18024e500`/`FUN_18024e4d0`).
- `FUN_180239290` : construit l'en-tête de requête `{octet = champ objet+8, u16 = champ objet+0xa8, 0x01, drapeau « réponse attendue »}` sur un `bragiprotocol::Report::StreambufAdapter` (le rapport HID est un `streambuf` sur le tampon d'un rapport). Les valeurs numériques de l'octet de commande et de l'identifiant d'appareil sont des **champs d'objet** initialisés ailleurs : non lues.
- `CoolersTargetRotationSettings` (0x18) confirmé comme vecteur d'éléments 4 octets `{canal, estRPM, u16 valeur}` (déjà §4.3) ; le cadre est écrit via `FUN_1800814b0(device, &nom, 0x18, vecteur, ...)` = écriture de **ressource** (flux), distincte de l'écriture de **propriété** ci-dessus. Aucune des deux ne montre d'écriture en mémoire persistante : le caractère temporaire/persistant n'est PAS établi (INCONNU).

### 12.2 Limite de la lecture iCUE (arrêt documenté)
La sérialisation passe par des gabarits C++ (`cereal` + `std::ostream` sur un `bragiprotocol::Report::StreambufAdapter`), imbriqués sur une dizaine de niveaux (`FUN_18006d890` -> `FUN_180219960` -> `FUN_180219320` ; `FUN_180247b70` -> `FUN_180248270` -> `FUN_1802011a0` -> `FUN_180202220` -> `FUN_1801edbd0`, tous de simples relais). Deux faits ont pu en être extraits [C] :
- `FUN_180219320` choisit un **en-tête de commande de 2, 3 ou 4 octets** (`FUN_1802040c0(descripteur)` ; autre valeur : erreur « Invalid header size: N ») ; chaque variante (`FUN_180218bc0` = 2 o, `FUN_180218aa0` = 3 o, `FUN_180218980` = 4 o) dépaquette 5 champs d'une structure « command » (offsets +0..+4, champ nommé `"command"`), les octets +3 et +4 étant copiés tels quels (`FUN_180219720` = simple recopie d'un octet).
- Les **valeurs numériques des opcodes** ne sont pas des constantes de ces fonctions : elles viennent des champs de l'objet appareil (octet +8, u16 +0xa8 de l'objet passé à `FUN_180239290`). Elles n'ont **pas** été lues dans iCUE. Les opcodes retenus ci-dessous viennent donc d'OpenRGB [X] et sont seulement RECOUPÉS (cohérence de structure et de plusieurs identifiants), pas lus dans iCUE.
Suite possible : trouver les sites d'appel de `FUN_18024fad0` / `FUN_1800814b0` et lire l'initialisation de l'objet appareil, ou capturer le trafic USB d'un hub réel.

### 12.3 Trames Bragi RECOUPÉES avec OpenRGB (source [X] : `Controllers/CorsairICueLinkController/CorsairICueLinkProtocol.h` + `CorsairICueLinkController.cpp`, `Controllers/CorsairCommanderCoreController/CorsairCommanderCoreController.cpp`, commit local `bbc77470`)
Recoupements avec iCUE [C] qui renforcent la confiance :
- **Identifiants de sous-appareil LINK** : OpenRGB `known_devices {type, model}` -> id iCUE = type | model<<8. 5000T `{05,02}` = 0x0205 = **517** et 9000D `{05,01}` = **261** (classe LightingNode iCUE) ; ventilateurs `01/02/03/04/0A/0F/13` = 1,2,3,4,10,15,19 (classe Fan) ; `10` VRM = 16 (PumpFan) ; refroidisseurs `06,07,09,0C,0E,11,19` = 6,7,9,12,14,17,25 (classe LiquidCooler). Les deux tables coïncident donc entièrement : RECOUPÉ.
- **Ressources** : OpenRGB écrit sur les points d'accès `0x1D` (nombre de LED, type de données `0C 00`) et `0x1E` (types de ventilateurs/géométrie, type `0D 00`), ce qui correspond exactement aux ressources 0x1D/0x1E que j'ai lues dans iCUE (§4.2). `0x20` (résultats d'auto-détection, lu par `GetLedCounts`), `0x22` (couleurs, type `12 00`), `0x18` (consigne), `0x17` (RPM), `0x21` (températures) et `0x36` (appareils LINK, type `21 00`) sont cohérents avec les noms vus dans iCUE : les IDs 0x17/0x20/0x21/0x22 passent de [X] à RECOUPÉ (1 source indépendante + noms iCUE).
- Écarts de nombre de LED : pompe LCD `{06,00}` 24 (OpenRGB) contre 40 (`LinkCoolerScreen40Leds`, iCUE) ; 5000T 160 contre 204 ; le reste identique. Retenir iCUE.

**Cadre HID** (hub LINK, rapport de **513 o** en écriture / **512 o** en lecture, ID 0 ; Commander CORE : paquet de taille V1/V2/V3 selon le PID/firmware, XT : taille propre ; `packet_size - 4` octets de réponse) :
- Hub : `[0]=00 [1]=00 [2]=01 [3..]=commande (2 ou 3 o) puis données`. Réponse : en-tête de lecture de 3 o, type de données à l'octet 4 (`waitForDataType`), jusqu'à 5 relectures, délai 1000 ms.
- Commander CORE/XT : `[0]=00 [1]=08 [2..3]=commande [4..]=données` ; réponse lue en boucle jusqu'à `buf[0]==0x00`, données utiles à partir de `buf[3]`.
- Commandes (hub et CORE identiques) : `01 03 00 02` = mode logiciel ; `01 03 00 01` = mode matériel (rendu à la fermeture) ; `02 13` = lire firmware (octets 4..7 du hub : major, minor, patch u16 LE ; CORE : octets 0..2 de la réponse) ; `0D 01 <ressource 2 o>` ouvre (hub, point d'accès normal) / `0D 00 <ressource>` ouvre le point d'accès couleur (hub) et tout point d'accès (CORE) ; `05 01 01 <ressource>` ferme (hub) / `05 01` (CORE) ; `08 01` / `08 00` lit ; `06 01 <len u16 LE (données+2)> 00 00 <type 2 o> <données>` écrit (hub, normal) ; `06 00 …` premier morceau couleur, `07 00 …` morceaux suivants (508 o max par requête, hub ; CORE : `packet_size-10` puis `packet_size-4`).
- **Couleurs** : tampon RVB (R,G,B) de toutes les LED à la suite, écrit sur la ressource `22 00` avec le type `12 00` ; CORE : 34 LED réservées par port (le tampon saute `3*(34-n)` octets pour chaque zone sauf la première), cadence minimale 33 ms entre envois, **un envoi de couleurs remet à zéro le délai de repli** ; sans couleur à envoyer, keep-alive `02 13` toutes les 10 s (OpenRGB). Délai de repli réel du firmware : INCONNU.
- **Énumération LINK** [X, lecture seule, sûre] : lire `36` (type `21 00`) ; octet 6 = nombre de canaux, puis par canal une fiche de 8 o (type = octet 2, modèle = octet 3, **longueur de l'identifiant = octet 7**) suivie de l'identifiant ASCII (numéro de série). Fiche à longueur d'id 0 = canal vide.
- **Mode ventilateurs CORE** : écriture sur `1E 00` (type `0D 00`), 15 o `07 <b1> <b2> (01 06)x6` : CORE (AIO) `b1=01 b2=08`, XT sans port externe `b1=00 b2=00` (paires décalées d'un octet), XT avec port RGB externe `01 01` ; puis réveil `01 03 00 02`. Nombre de LED : `1D 00` (type `0C 00`), 15 o `07 <n>`.

### 12.4 Dangers (écritures possiblement persistantes) pour Bragi
Aucune source (iCUE ou OpenRGB) ne dit si ces écritures vont en flash ; **traiter comme persistantes tant que non prouvé** : `1D 00`/`1E 00` (géométrie, nombre de LED, mode ventilateurs : iCUE les note « matériel » et prévoit `RequiresApplyGeometryCommand`) ; `01 03 00 01|02` (mode) ; tout ce qui touche `CoolersMode`, `CoolersFixRotateSpeed`, `CurveRPM` (formats non lus). Écriture temporaire/sûre présumée : couleurs `22 00`/`12 00`, lecture `08 xx`, firmware `02 13`, énumération `36`. Écrire le moins possible sur 1D/1E : une fois, au premier démarrage, après lecture de la valeur actuelle, et seulement si elle diffère.
