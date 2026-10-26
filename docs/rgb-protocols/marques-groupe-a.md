# Marques groupe A : Trust, Mars Gaming, Empire Gaming, Genesis/Natec, Endorfy, Krux, Spirit of Gamer, Hama uRage, Speedlink, Mad Catz/Cyborg, Savio, Tracer, Ozone, Sharkoon — 2026-10-26

Tags : **[V]** lu dans la source ou le logiciel officiel, **[S]** supposé, **INCONNU** non établi. Aucun logiciel installé ni exécuté : tout a été extrait SANS exécution (`innoextract`, `innounp`, `7z`) dans `D:\Tools\analyse\<marque>`. Donc rien à désinstaller (vérifié : aucun processus, aucun pilote `pnputil` lié à ces marques). Aucune trame USB envoyée, aucun code fournisseur copié.
Les IDs ajoutés sont dans `src-tauri/assets/rgb_usb_catalog_extra_a.json` (61 appareils, 5 fabricants). Tous les installeurs ci-dessous viennent des sites officiels en HTTPS ; presque tous sont NON signés (Authenticode `NotSigned`), sauf Trust (signé `Trust International BV`, certains avec chaîne non vérifiable car certificat expiré) : l'intégrité n'est donc pas prouvable par signature, SHA-256 notés.

## Source des IDs (méthode)
Les logiciels « familles OEM » embarquent leur table VID/PID en clair : `Cfg.ini` (`VID=`/`PID=`), `Data.ini`, `apConfig.bin` (texte INI), profils `.prof` (`vid1/pid1`), ou appels `Set_VIDPID(vid,pid)` (immédiats `push` lus au désassemblage).

| Marque / modèle | VID:PID | Source | Tag |
|---|---|---|---|
| Trust GXT 160X | 145F:02BF (+ 04D9:A1EE, 04D9:8008) | `apConfig.bin` du logiciel officiel `trust.com/23797` | [V] |
| Trust GXT 165 Celox | 145F:0276 | `Cfg.ini` (`trust.com/23092`) | [V] |
| Trust GXT 950 | 145F:02A7 ; 260D:1025 | table du .exe (`trust.com/23645`) | [V] |
| Trust GXT 980 / 981 Redex | 145F:02FB, 145F:02F2 | `Set_VIDPID` dans `Config.exe` / `TrustREDEX.exe` | [V] (rôle filaire/récepteur [S]) |
| Trust GXT 155, 168, 830, « Gaming Mouse » | 145F:01BC, 023F, 01E5, 01C1 | treexy (ID matériel), usb.ids | [V] |
| Trust GXT 158 Orna, Sharkoon Vintorez, SHARK ZONE K30 | 1EA7:0030, 1002, 2007 | usb.ids | [V] |
| Sharkoon Light2 200 | 2EA8:2203 | `Cfg.ini` (`sw_light2_200_210720.zip`) | [V] |
| Sharkoon SKILLER SGM25W | 248A:FA02, 248A:FB01 | profil par défaut `vid1/pid1`, `vid2/pid2` (9354/64002, 9354/64257) | [V] (récepteur = FB01 [S]) |
| Empire Gaming HELLFIRE | 258A:1007 (= Genesis Xenon 200) | `Cfg.ini` | [V] |
| Empire Gaming STARDUST (OE-KB-198) | 258A:0016 | `Cfg.ini` | [V] |
| Empire Gaming HELLHOUNDS souris | 258A:0013 | PID `Cfg.ini`, VID 258A = immédiat dans `OemDrv.exe` | [V] partiel |
| Savio Valiant / Rampage | 258A:0029 / 258A:0049 (= Everest GT-100 RGB / FL ESPORTS F11) | `Cfg.ini` | [V] |
| Savio Gambit, Hex-R ; Ozone Neon X50 | 04D9:A09F | `Data.ini` / `apConfig.bin` (nom interne « XPERT M100 ») | [V] |
| Savio Blackout X2 / Whiteout X2 | 352D:2380 | `Cfg.ini` | [V] |
| Savio RIFT | 1D57:FA61, 1D57:FA60 | `Set_VIDPID` dans `Config.exe` | [V] |
| Savio Blackout / Whiteout (casques) | 0D8C:01DA (C-Media) | INF du pilote fourni | [V] |
| Endorfy GEM / GEM Plus / LIX / LIX Plus (+ Onyx White) | 3299:000F, 000E, 000B, 000A, 0067, 0066, 000D, 000C | `Data.ini` de chaque logiciel EY6A0xx | [V] |
| Endorfy LIV / LIV Onyx White | 3738:00B3 / 00B4 | `Data.ini` | [V] |
| Endorfy LIV Wireless / Plus Wireless | 3738:00AF,00B1 (+récepteurs 00B0,00B2) ; 3299:00A6,00A7 ; 3738:00AD,00AE | `Config.ini` (base64) | [V] ; appariement VID<->PID de Plus Wireless [S] |
| Endorfy souris sans fil EY6A007/8/9/10/13/15 | 3299:0010,0064,0068,0073,0011,0065,006A,006C,006E,0074,006B,006D | `Cfg.ini` (`PID`, `PID2`) | [V] (modèle par PID INCONNU) |
| Speedlink ORIOS (souris, tapis, clavier) | 0C45:5F01, 763D, 763E | `signalrgb-devices.csv` | [V] |
| Tracer Gamma Ivory | 1C4F:0002 | usb.ids | [V] (clavier simple, RGB INCONNU) |
| Mad Catz Cyborg Gaming Light | 06A3:0DC5 | déjà au catalogue (OpenRGB) | [V] |
| Genesis Xenon 200 / Thor 300 | 258A:1007 / 258A:0090 | déjà au catalogue (OpenRGB) | [V] |
| Trust GXT 114 / 180, Endorfy Omnis | déjà au catalogue (OpenRGB) | | [V] |

Familles OEM découvertes (utile pour reconnaître d'autres marques) : (1) **« Holtek mouse + `apConfig.bin` »** (Qt4, hidapi, rapport 32 o) : Trust GXT 160X/950, Ozone Neon X50 (XPERT M100), Savio Gambit/Hex-R ; (2) **« BY-COMBO/BY8801 + `Cfg.ini` + `OemDrv.exe` »** : Trust GXT 165, Sharkoon Light2 200, Empire HELLFIRE/STARDUST/HELLHOUNDS, Savio Valiant/Rampage/X2, Endorfy EY6A007..015 ; (3) **« DuiLib `Config.exe` + `hiddriver_N.dll` + `Set_VIDPID` »** : Trust GXT 980/981, Savio RIFT ; (4) **« Qt5 + HidDevice.dll + profils `.prof` »** (« J Gaming Mouse ») : Empire RF906 et Sharkoon SGM25W (même éditeur) ; (5) **« Option.exe/Monitor.exe + `hiddriver.dll` »** claviers : Sharkoon SGK4/SGK5, Empire HELLHOUNDS clavier (IDs non lisibles en clair : INCONNU).

## Protocole d'éclairage (famille 1, Trust GXT 950 / 160X, Ozone Neon X50, Savio Gambit)
Établi sur `GXT 950 Gaming Mouse.exe` (x86, SHA-256 `092a84e6…4000`, décompilé Ghidra/REA) :
- [V] Commandes (noms des slots Qt) : `01` SetReportRate, `02` SetCurrentProfile, `03` SetKeyboardoption, `04` SetGunData, `0D` SetRGBLedEffect(mode,x,y), `0E` SetXYSensitive, `10` SetLED8StageColor(index,24 o via sortie), `11` SetDPISet/SetDPISelect, `12` SetKeyMatrix ; lectures `81 83 8B 8D 90 91`.
- [V] Trame de commande = **feature report de 9 octets** : `[00][cmd][a1..a6][chk]`, `chk = NOT(somme(cmd, a1..a6))` (8 bits). Optionnellement chiffrée par XOR si `isEncode=true` (clé `sCode`, ex. Ozone : `2EEB44`). Les données longues (couleurs, matrice) suivent en `hid_write` (rapport de 32 o, préfixe `00`, pause 8 ms).
- Interfaces : mouse `usagePage ff00 / usage ff00`, `reportLength 32`.
- **INCONNU** : sens exact de `a1..a3` pour `0D` (mode/vitesse/luminosité), liste des modes, et surtout **persistance** (le matériel écrit-il en flash ?). Faute de preuve de non-persistance, **aucun pilote natif écrit** (règle : pas de profil persistant).

## Réglages non lumineux proposés par ces logiciels (relevés, non implémentés)
Fréquence d'interrogation (`01`), DPI par palier (`11`, 200..4500/12400 selon capteur), sensibilité X/Y (`0E`), profils (`02`), remap/macros (`12`). SGM25W/RF906 : DPI, polling, `lift`, `angle`, `jitter`, `wake` (mise en veille), batterie. Tous nécessitent une écriture mémoire interne tant que la persistance n'est pas prouvée : non implémentés.

## Ce qui n'a pas pu être fait
- **Mars Gaming** : `marsgaming.net` répond 503/injoignable ; aucun IDs vérifiable, rien ajouté. **Krux**, **Spirit of Gamer** : hôte injoignable. **Genesis/Natec** : `genesis-zone.com` derrière un défi Cloudflare (non contourné) ; IDs OpenRGB déjà au catalogue.
- **Hama uRage** : aucune page de logiciel officielle trouvée ; **Tracer** : page « wsparcie » sans logiciel ; **Speedlink** : pas de page de téléchargement exploitable ; **Mad Catz** : marque disparue, seul l'appareil OpenRGB.
- **Empire RF908** : l'archive officielle `RF908_MOUSE.zip` a été supprimée par Windows Defender (détection PUA) dès le téléchargement ; non contourné, non analysé. **RF906 / W905 / X-BLADES / Sharkoon SGM1,SGM30W,SGM70W,PureWriter,SGK20, Tempest X2** : IDs non lisibles en clair ; analyse approfondie non faite (INCONNU). Le SGK20/Tempest X2 embarquent un profil par défaut `0C45:1007` qui est un gabarit, pas un ID prouvé.
- Casques Empire H1300/H1800/WARCRY GW10 : pilotes C-Media CM108/CM6620 (audio) ; non installés, pas d'IDs Empire.
- Rien n'est classé `NO_RGB` : aucune preuve d'absence d'éclairage pilotable (le tapis/clavier Speedlink ORIOS, Tracer Gamma restent à vérifier).
