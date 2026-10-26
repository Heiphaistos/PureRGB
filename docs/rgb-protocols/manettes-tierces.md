# Manettes tierces : VID/PID, réglages exposés, verdict de pilotage

Analyse du 2026-10-26 (logiciels officiels décompilés ou lus, jamais exécutés contre une manette ; aucune manette branchée sur ce PC). Aucun code ni asset fournisseur copié. Légende : **[V]** = lu dans le code/données du logiciel officiel ; **[S]** = source tierce (Linux `xpad.c`, consulté le 2026-10-26, faits d'identifiants seulement, driver GPL non repris) ; **INCONNU** = tenté, non établi.

## Verdict global

**Aucun pilote natif implémenté.** Pour toutes ces marques, le logiciel officiel :

1. écrit chaque réglage (vibration, sticks, gâchettes, remap, LED) dans un **profil gardé en flash de la manette** (8BitDo, GameSir) ou envoie ses commandes en **GIP via `Windows.Gaming.Input.Custom`** (Thrustmaster, PowerA, HORI, Nacon, GameSir Nexus, 8BitDo Ultimate Software X, Xbox Accessories), transport qui n'est pas du HID et que `hidapi` ne peut pas atteindre ;
2. n'expose aucune commande « en direct » (RAM seule) vérifiée pour la lumière ou un réglage.

Les règles (rien de persistant, aucune trame inconnue, pas d'envoi USB réel) interdisent donc d'écrire un pilote. Seule la détection (`backends/hid/gamepads.rs`) est améliorée : marque et raison affichées pour 8BitDo, GameSir, Thrustmaster, PowerA, HORI, Nacon/Bigben, Razer.

## Logiciels obtenus et traitement

| Logiciel | Source officielle | Méthode | Résultat |
|---|---|---|---|
| 8BitDo Ultimate Software V2 1.37 (Windows, portable) | `support.8bitdo.com/bd-uploads/files/ultimate_soft/8BitDo_Ultimate_Software_V2_Windows_V1.37.zip`, SHA-256 `5121512b889176edd3819f581e23ec0ac7901b3060ecef40d7fddaa1cb5cbfca` | zip extrait (aucune installation), bundle .NET décompilé (`ilspycmd`) | VID/PID + structures de profil [V] ; trames dans `8BitDoAdvance.dll` natif (non désassemblée) |
| GameSir Connect 1.14.5 | `pc-connect-update.gamesir-cdn.com/1.14.5/GameSir Connect Setup 1.14.5.exe`, SHA-256 `6524cdee294a2e7705bc0491cd5644d61fd08c1dba553494f77e7ea588979277` | NSIS extrait (7z), `app.asar` extrait (Electron, JS lisible) | table des modèles + trames HID du Cyclone 2 [V] |
| ThrustmapperX 1.0.9 (Store) | Microsoft Store 9P3SR9K3V8NR | installé par `winget`, fichiers copiés, désinstallé ; `Tm.Devices.Gip.Custom.dll` désassemblée (Ghidra headless) | barre LED + intensité des modules, transport GIP |
| PowerA Gamer HQ 3.1.14, PowerA PC HQ 1.4.2 | Store 9NMZHRCX9WSB, XPFCVS0B6Q1K3S | installés, copiés, désinstallés | PowerA PC HQ (Qt, `hidapi`/`WT_USB.dll`, pages Remap/Stick/Trigger/Light/Vibrate/Profils) non désassemblé : INCONNU. Gamer HQ : GIP |
| Xbox Accessories (Microsoft.XboxDevices) | Store 9NBLGGH30XJ3 | idem | PID des firmwares embarqués [V] ; GIP |
| HORI Device Manager (Xbox Series), Nacon Revolution-X, Nacon Pro Compact, GameSir Nexus 2.5.8, 8BitDo Ultimate Software X 1.5 | Store 9N02W73BVLXV, 9MX3BC5HB91W, 9PJ2CFT54GTB, 9PMJKF5NSTDR, 9PMC9MN3ZZ85 | idem | tous reposent sur `D4XGaming.Devices.D4XDevice` (fabrique `GameControllerFactoryManager`) : GIP. Binaires .NET Native non désassemblés |

Tout ce qui a été ajouté a été retiré : paquets Appx supprimés, `unins000.exe` de PowerA PC HQ exécuté, aucun pilote ajouté (`pnputil` : aucun fournisseur de ces marques), aucun service ni processus restant. Copies d'analyse : `D:\Tools\analyse\manettes\`. Non retirés : paquets cadres `Microsoft.UI.Xaml.2.3/2.5` et source `Microsoft.Winget.Source` (dépendances partagées, d'autres logiciels les utilisent).

## 8BitDo (VID 0x2DC8)

PID [V] (`AdvanceSuper.Hid/VIDPID.cs`, nom = nom de la constante ; recoupé avec `xpad.c` [S] pour 3106, 3109, 310A, 310B, 2000/200F/201E, 6001) :

| PID | Constante | PID | Constante |
|---|---|---|---|
| 2DC8:3109 | IDLE (= « Ultimate Wireless Bluetooth » [S]) | 2DC8:310B | Xinput (= « Ultimate 2 Wireless » [S]) |
| 2DC8:3100 | USB_Ultimate | 2DC8:310A | QINGCHUN2 (= « Ultimate 2C Wireless » [S]) |
| 2DC8:3105 | USB_Ultimate2 | 2DC8:301C | QINGCHUN2RR (récepteur) |
| 2DC8:3104 | USB_UltimateClasses | 2DC8:3010 | Pro2_Wired |
| 2DC8:3011 | Ultimate_PC | 2DC8:3012 / 3013 | Ultimate2_4 / récepteur |
| 2DC8:6003 / 6006 | Pro2 / Pro2_CY | 2DC8:6007 / 3106 | UltimateBT / UltimateBTRR (3106 = « Ultimate Wireless / Pro 2 Wired » [S]) |
| 2DC8:6009 / 600A / 600D | Pro3 / Pro3USB / Pro3DOCK | 2DC8:6012 / 6013 | Ultimate2 / récepteur |
| 2DC8:600F / 6011 | UltimateBT2 / récepteur | 2DC8:301A | UltimateBT2C |
| 2DC8:2100 / 2101 | Xcloud / Xcloud2 | 2DC8:6002 | SN30Plus |
| 2DC8:2066 / 207D | Ultimate3CPLUS / adaptateur | 2DC8:208C / 208F | Ultimate3C / adaptateur |
| 2DC8:200F, 201E, 2000 | Ultimate 3-mode, Pro 2 Wired for Xbox [S] | 2DC8:207C / 207A | SN30Pro / USB |

Autres PID de la liste (arcade, N64, NGC, HitBox, claviers, souris, Lite/Lite 2 5111/5112) : voir `VIDPID.cs`.

Réglages exposés [V] (structures C# de `AdvanceSuper.Advances`, une copie par emplacement de profil) : `rumble_params_record_t` (2 flottants de gain moteur gauche/droit), `joy_params_record_t` (2 plages début/fin d'octet = zone morte / saturation des sticks), `trigger_params_record_t` (2 plages = course des gâchettes), `joy_trigger_special_feature_t` (drapeaux), `sixaxis_params_record_t` (gyro : touches, mode, sensibilité, compensation), `xinput_rumble_params_record_t`, remap (`mode_map_key_record_t_*`), macros, et thèmes LED : `single_theme` (24 couleurs `uint`), `dynamic_theme` (couleur, fond), `hotwheel_theme` (couleur, fond, vitesse). Lecture batterie et version firmware via `GetBattery`/`GetFWVer` (DLL).

**Pilotage : refusé.** Chaque écriture (`writeUltimate2Vibration`, `writeUltimate2Sticks`, `writeUltimate2theme`…) prend un index d'emplacement (`cur_slot`) et un enregistrement CRC16 (`ANSI_CRC_16_Ultimate2`) : c'est une écriture de **profil dans la manette**. Les octets réels sont construits dans `8BitDoAdvance.dll` (9,4 Mo) que je n'ai pas désassemblée : trames **INCONNU**. Aucun mode temporaire (RAM) trouvé.

## GameSir (VID 0x3537)

Table `ConfigModePid` de GameSir Connect 1.14.5 [V] (le nom est celui du champ `ProductName` du bloc ; une même entrée couvre filaire et dongle/sans fil) :

| Modèle (champ `Model`) | PID |
|---|---|
| Cyclone 2 (C2) | 3537:100B, 101D, 102A, 1053 |
| G7 Pro 8K (G7ProCE) | 3537:1032-1035, 10B7, 10B9, 10C5-10CE, 10FB-10FE |
| K2 8K | 3537:108E, 109F |
| Nova Vision (N2S) | 3537:10BD, 1141 |
| Tarantula CE | 3537:090D, 103D |
| Tegenaria Lite (T3Lite) | 3537:1093 |
| Tarantula Pro (T3Pro) | 3537:103E, 1050 |
| T3XS | 3537:105A, 10BE |
| Nova 2 Lite (T4N2) | 3537:100F, 1098 (100F = « Nova 2 Lite » aussi dans `xpad.c` [S]) |
| Nova Pro (T4NPro) | 3537:100E, 103E |

Attention : 103E apparaît pour Tarantula Pro et Nova Pro (le parseur a rattaché les PID au dernier `Model` lu ; à confirmer sur le matériel). Autres [S] `xpad.c` : 3537:1004 T4 Kaleid, 1010 G7 SE, 1040 Nova Lite. Modèles Xbox sous licence (GameSir Nexus 2.5.8, GIP) : T4k Kaleid, G7, G7 SE, K1, T7, T7 Pro W, G7 Pro, T3 Pro.

Réglages [V] : profils (4 + « Shift »), remap/macros, courbes de sticks, gâchettes, deux niveaux de vibration poignées, lumière (cadres d'animation, 5 zones RGB), stick/D-pad, gyro, batterie.

Protocole HID du Cyclone 2 / Tegenaria Lite [V] (`Ro.*` dans `main.js` d'Electron) : rapport de sortie `0x0F`, collection de page d'usage `0xFFF0` (`UsagePage 65520`), paquets de 58 octets de données (`EveryOutPacketLength`), profil de 680 octets, profil lumière de 635 octets. Commandes (octet 1) : 1 entrer config, 2 sortir config, 3 écrire profil, 4 lire profil, 7 changer de profil, 9 lire firmware, 11 profil courant, 13 lire/commander lumière `[0x0F, 0x0D, lecture ? 1 : 0, image]`, 16 rafraîchir, 22 macro, 23 mise à jour rapide `[0x0F,0x17,0x55,0x88]`, 32 vibration d'essai `[0x0F,0x20,0x66,0x55,gauche,droite]`, 242 battement, 252 mode mise à jour, 253 calibration.

**Pilotage : refusé.** Seuls deux ordres sont « en direct » : l'essai de vibration (échelle des octets gauche/droite et arrêt automatique non établis [S], rien ne garantit l'arrêt du moteur si l'application tombe) et la lecture/pause de l'animation lumineuse (la lumière elle-même vit dans le profil 635 octets, écrit en flash). Aucun ne donne de couleur à la demande. Les familles T3/T4N Pro/N2S/T3XS passent par `sapjoyuniapis_*.dll` (C++ `Sapientia::Joystick`, appels virtuels) : `SetLightColorHSV`, `SetVibrateLevel`… sont toujours suivis de `SaveConfigPage` (écriture 500 ms après, fonction `Be` du JS) ; le contenu des trames et le caractère RAM seule de `Set*` sans `Save` sont **INCONNU**.

## Thrustmaster (VID 0x044F) : eSwap X Pro et autres

PID [S] `xpad.c` : 044F:D01E « ESWAP X 2 ELDEN RING EDITION » ; 044F:B326 Gamepad GP XID ; Ferrari 458 / GPX / XB1 Classic sous 24C6 (5B00, 5B02, 5B03, 581A). PID de l'eSwap X Pro : **INCONNU** (non lu dans ThrustmapperX).

ThrustmapperX (Unity IL2CPP + `Tm.Devices.Gip.Custom.dll`) [V] expose : couleur de la barre LED (`SetLedBarColorState`, requête interne 0x15, 3 emplacements, mode 1 à 6), intensité LED des modules, sensibilité des mini-sticks, mode DS4 des mini-sticks, vibration et impulsions, remap par module avec **mode aperçu** (`EnterMappingPreview` / `Commit` / `Rollback`), verrou de profil.

Transport [V] (désassemblage Ghidra de `FUN_180019720` = `SendOutputOnlyVendorGipMessage`) : message GIP vendeur de classe 0x1F, 60 octets (`[séquence, type, mot16, mot16, longueur, charge ≤ 53 octets]`) envoyé par l'objet fournisseur `GipGameControllerProvider` de Windows. Passage de la requête 0x15 aux octets de charge : **INCONNU** (le code intermédiaire n'est pas résolu). Pas du HID : pas de pilote natif possible dans `Hid`/`MockHid` ; une implémentation demanderait un transport Windows Gaming Input à part et du matériel pour vérifier.

## PowerA (VID 0x20D6), HORI (0x0F0D), Nacon/Bigben (0x3285, 0x146B), Razer (0x1532), Xbox Accessories

- PowerA Gamer HQ 3.1.14 (Fusion Pro 3/4, Lumectra RGB) : GIP (`Dashine.Devices.Input`), la chaîne `20D6` figure dans le binaire. Trames : INCONNU. PowerA PC HQ 1.4.2 (Qt, `hidapi` + `WT_USB.dll`, modèles 1531/1532/1581, pages Light/Vibrate/Profils) : plusieurs onglets mais slots de moc non résolus : INCONNU.
- PID [S] `xpad.c` : PowerA 20D6:2009, 2064, 281F, 400B (FUSION Pro 4), 890B (MOGA XP-Ultra), 24C6:541A/543A/551A/561A, 15E4:3F00 ; HORI 0F0D:0067 HORIPAD ONE, 00C5, 0078, 0151/0152 ; Nacon 3285:0603, 0607 GC-100, 0614/0646 Pro Compact, 0662 Revolution5 Pro, 0663 Evol-X, 11C9:55F0 GC-100XF, 146B:0601/0604 (Bigben) ; Razer 1532:0A29 Wolverine V2, 0A57/0A59 Wolverine V3 Pro (filaire / dongle 2,4 GHz), 0A03 Wildcat ; Microsoft 045E:02E3 Elite, 0B00 Elite 2, 0B0A Adaptive Controller.
- Xbox Accessories [V] (noms de fichiers de firmwares embarqués) : 045E:02D1, 02D2, 02DD, 02E3 (Elite), 02EA (One S), 02F3, 0B00 (Elite Series 2), 0B0A (Adaptive Controller), 0B12 (Series), 0B17, 0B1A, 0B32. Réglages Elite : remap, palettes, verrous de gâchette, courbes de sticks, vibration, profils : enregistrés dans la manette, GIP. Pas de RGB (seulement la luminosité du bouton Xbox).
- Revolution 5 Pro (PlayStation/PC) : l'application PC annoncée n'a pas été trouvée (Revolution-X = Xbox). Wolverine V2 Chroma : seul le Synapse analysé ailleurs (`analyse-razer-synapse.md`) pourrait l'éclairer ; non traité ici.
- Nintendo Switch Pro : inchangé (LED de joueur, cf. `manettes.md`).

## Ce qui reste utile à intégrer

- Les PID ci-dessus pour les photos/noms (catalogue `assets/rgb_usb_catalog*.json` non modifié ici, à intégrer par l'appelant).
- Pistes si l'on veut aller plus loin (matériel requis) : désassembler `8BitDoAdvance.dll` pour les trames de profil (écriture persistante, à ne pas utiliser) ; GIP vendeur eSwap X Pro pour la barre LED (si une écriture sans `Commit` est prouvée) ; capter le trafic USB réel d'un Cyclone 2 pour l'échelle des moteurs.
