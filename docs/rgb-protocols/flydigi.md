# Flydigi (Vader 4 / Vader 4 Pro) : éclairage

## Verdict (analyse REA statique du 2026-10-25)

**Protocole d'éclairage : PARTIEL.** Le format des trames, la sérialisation de la configuration LED (500 octets) et le séquencement sont établis depuis le code décompilé de Flydigi Space Station 3.4.4.3. Mais :

1. **Le transport en mode XInput (045e:028e) n'est PAS du HID standard.** Space Station écrit via un pilote noyau filtre propriétaire (`Xbox360Filter64.sys` + `XboxComm64.dll`) qui injecte les trames de 15 octets sur le pipe USB OUT de la manette. hidapi ne peut pas reproduire cela. Aucun canal sûr côté PureRGB n'est établi.
2. **Aucun mode « temporaire » pour la lumière n'est prouvé.** Le seul chemin LED du logiciel est l'écriture de la configuration dans l'emplacement de profil (commandes « Flash »). Voir « Dangers ».
3. L'identité exacte de la Vader 4 vue par PureRGB (DeviceId 85 ou autre) n'est pas confirmée sans matériel.

**Pilote natif sûr : NON, pas maintenant.** Aucune écriture HID vers cette manette.

## Source analysée

- Installeur officiel : `https://tencent-android.cdn.flydigi.com/PC/Space%20Station3.0/FlydigiSpaceStation_setup_3.4.4.3_Release.exe`, lien publié par la page officielle `https://shops.flydigi.com/pages/game-center` (Flydigi Space 3.4.4.3, appareils : Vader 4/3, Apex 4/3, Direwolf 3, BS1, FS68, Apex 5).
- SHA-256 : `f6e480b75016288797080ba5257aa8cbc46ebc724401ee841680f8c755ca0d50` (105 741 288 octets). Inno Setup 6.3.0 (Unicode).
- Authenticode : signature présente (GlobalSign GCC R45 EV CodeSigning CA 2020, organisation de Shanghai, n° 91310110682265240Q), mais certificat valide du 22/05/2024 au 23/05/2025 : Windows répond « hors période de validité » (statut UnknownError), sans horodatage valide. Intégrité non prouvée au-delà du hash ci-dessus.
- Jamais exécuté. Extraction statique `innounp` (`D:\Projet\Analyses\flydigi-2026-10-25\extract`), décompilation `ilspycmd` (`...\decomp`, hors du dossier d'extraction). Les assemblies .NET sont livrées avec leurs PDB.
- Chemins ci-dessous relatifs à `D:\Projet\Analyses\flydigi-2026-10-25\decomp\` (classes et méthodes citées ; la décompilation n'a pas de numéros de ligne stables). Préfixes : `Main/` = `FlydigiSpaceStation.dll`, `Svc_/` = `Svc_GameController.ServiceHandler/` (service Windows), `LibCommon/`.
- « Space Station 2 » (version 2.5.7.8) non analysée : le lien publié pour cette version est en réalité l'Activation Tool.

## Architecture (établi)

`FlydigiSpaceStation.dll` (UI WPF) envoie du JSON par TCP local au service Windows `GameControllerService.exe`. C'est le service qui parle à la manette.

- UI : `Main/FlydigiSpaceStation.ViewModels.GamePad.Config/NewBasicViewModel.cs` (`UpdateGamepadLedInfo`) → `Main/FlydigiSpaceStation.Common.Database/ConfigManager.cs` (`UpdateConfig` : debounce 350 ms, puis `SendFlashConfig` : différence avec l'état précédent, seuls les paquets modifiés partent) → `Main/FlydigiSpaceStation.Common.Services.Protocol/FlashGamepadConfigService.cs` (`ApplyLEDConfig`, action `ApplyTempLEDConfig`).
- Service : `Svc_/GameController.ServiceHandler.DataManager/FDGTransData.cs` (case `ApplyTempLEDConfig`) → `Svc_/GameController.ServiceHandler.Service/FlashGamepadConfigService.cs` (`ApplyFlashLEDConfigInfo`) → `Svc_/GameController.ServiceHandler.DataManager/XInputDataManager.cs` (`WriteLEDFlash`, `WriteGamePadConfig`, `SendFullConfigDataPacket`, `SendByteArray`).
- Construction des trames : `LibCommon/LibCommon.Command/XinputCommand.cs`, `LibCommon.Command/CommandDic.cs`, `LibCommon.Utils/XInputDataUtils.cs`, `LibCommon.Utils/GamepadConfigUtilsV2.cs` (`GetNewLedByteList`).

## Identification (établi / partiel)

- Mode XInput : `Svc_/.../Service/HID.cs` (`GetDeviceListAndOpen`) repère un chemin contenant `vid_045e` et `pid_028e`, lit le nom amical, `ConnectMode = "XInput"`. Aucune ouverture HID ni écriture HID sur ce chemin.
- Mode D-Input (dongle 2,4 GHz) : `Svc_/.../Config/FDGDevice.cs` : `vid_04b4` ou `vid_04b5`, PID `2410`..`2415` (hexadécimal), interface `mi_02`. HID via HidSharp, autre cadrage (plus bas). PID réel de la Vader 4 dans ce mode : INCONNU.
- DeviceId lu dans la réponse « infos manette » (octet 16 de la réponse) :
  - `LibCommon/LibCommon.Config/GameHandleListDic.cs` : 85 = « F4 » (黑武士 4 Pro = Vader 4 Pro), 91 = F4ACC (édition Assassin's Creed), 105 = variante F4. `GamePadVersionUtils.F4Serial = {85, 91, 105}`.
  - Repli par le nom produit (`FDGDeviceInfo.UpdateDeviceIdByDeviceInfo`) : `vader` et `4` donne DeviceId 85.
  - `GameConfigDb.cs` teste `DeviceId == 88 && ProductName contient "VADER4"` : 88 est de la série F3 (Vader 3 Pro EVA, IP). Une manette « VADER4 » d'id 88 serait un cas IP particulier : non éclairci. **DeviceId de « Controller (Flydigi VADER4) » : INCONNU.**
- Éclairage : `GamePadVersionUtils.IsSupportNewLedConfig` vrai pour F3, K2, F4 (donc Vader 4 Pro).
- Nombre de LED : `FDGDeviceInfo.ledNum` = 5 par défaut (non surchargé pour 85/91/105). Zones physiques réelles : INCONNU.

## Transport XInput (principe établi, pilote non désassemblé)

`LibCommon/LibCommon.Services.Driver/XBoxFilter.cs` : `XboxComm64.dll` (`InitXboxMonitorRes`, `WriteData2Usb(ptr, len)`, `IsCurrentUsbCanWrite`). Pilote `Xbox360Filter64.sys` (chaîne : `IOCTL_XBOX_FILTER_GET_WRITE_STATE Enter:Out Pipe:%x,In Pipe:%x`). `XInputDataManager.SendByteArray` passe le tableau tel quel à `WriteData2Usb`. Les réponses reviennent dans le flux d'entrée de 32 octets : `readBuffer[14] == 0xA5`, `readBuffer[15]` = sous-commande, `readBuffer[16..]` = arguments.

Conséquence : pas d'équivalent hidapi/WinUSB tant que le pad reste lié à xusb. Remplacer son pilote par WinUSB casserait son usage XInput : à proscrire. Longueur réellement émise sur le pipe (15 octets ou remplis) : INCONNU.

## Trame XInput (établi, `XinputCommand.cs`)

Trame de 15 octets (`PerPackageMaxLength = 15`) :

| Octet | Contenu |
|---|---|
| 0 | `0xA5` |
| 1 | sous-commande |
| 2..11 | arguments (10 octets utiles, `PerPackageDataLength = 10`) |
| 12 | `0xA0` pour les paquets de données (`getSendData(sub, args, index)`) |
| 13 | index du paquet |
| 14 | somme 8 bits des octets 0..13 (`CrcData`) |

Sous-commandes (`CommandDic.cs`) : `0x10` infos manette, `0x20` ID de config courante, `0x26` lire config LED par ID (`ReadLedConfigById`), `0x27` retour de lecture LED, `0x28` début d'écriture LED « Flash » (`StartFLashLEDDataKMPC`), `0x29` paquet LED (`WriteLed`), `0x2A` début d'écriture LED « normale » (`StartWriteLed`), `0x50 0x03 <id16>` enregistrement (`SubFunc_SaveFlashConfig`), `0x13` `LedTest`, `0x15` `LightTest`, `0xA2` changement de config.

Réponses (octet 15 de la lecture) : `0x28`/`0x2A` = ack de début ; `0x29` = ack de paquet (`HandleWriteLEDConfigCBK`). Un paquet n'est envoyé qu'après l'ack du précédent ; retransmission sur délai, l'octet de somme étant incrémenté du numéro d'essai (`SendFullConfigDataPacket(IsReSend)`).

## Configuration LED (établi)

`GetNewLedByteList` (`GamepadConfigUtilsV2.cs`) : blob de **500 octets** = 20 d'en-tête + 16 groupes × 10 unités × 3 octets (R, V, B), découpé en 50 paquets de 10 octets (index 0..49).

En-tête (offsets dans le blob) : 0..1 Version (lue sur la manette, jamais fixée par l'UI), 2 Type (« MapType » du mode), 3 Loop_start (jamais fixé par l'UI : à relire de la manette), 4 Loop_end, 5 Loop_time, 6 Light_scale, 7 Rgb_num, 8 LedMode, 9..19 réservés (11 octets, `GetRevData(11)`, valeurs INCONNU, probablement 0).
Unités : offset `20 + groupe*30 + unité*3` = R, +1 = V, +2 = B. Valeurs 0..255 (sélecteur de couleur) ; l'atténuation (x0,8) est calculée côté logiciel.

Modes (`Main/.../Common.Model.UI/NewLedTypeDto.cs`, `GetDefaultConfig` ; `GetDefaultConfigF4ACC` pour 91) :

| LedMode | Nom | Type | Loop_end défaut | Loop_time | Light_scale |
|---|---|---|---|---|---|
| 1 | flux | 0 | 5 (4 sur F4ACC) | 15 | 50 |
| 2 | respiration | 0 | 9 | 15 | 50 |
| 3 | dégradé | 0 | 1 | 15 | 50 |
| 4 | réaction aux touches | 1 | 1 | 15 | 50 |
| 5 | fixe | 0 | 0 | 15 | 50 |
| 6 | éteint | 0 | 0 | 15 | 50 |

Mode fixe (5), `GetLedGroupListByCustom` : groupes 0..4, unité 0 = couleur choisie, unités 1..9 = 0 ; groupe 5 = zéros. Groupes 6..15 non remplis par ce chemin. Bornes min/max de Light_scale et Loop_time acceptées par le micrologiciel : INCONNU.

Séquence « écriture Flash LED » (`ApplyFlashLEDConfigInfo`), pour chaque groupe de paquets contigus modifiés :

1. Début : sous-commande `0x28`, octets 2..5 = `[startPkgIndex, nbPaquets, 0xA0, cfgId]`, somme. Attendre l'ack `0x28`.
2. Chaque paquet : sous-commande `0x29`, 10 octets aux positions 2..11, octet 12 = `0xA0`, octet 13 = index **relatif au groupe** (`XInputDataUtils.FormatterWriteFlashLEDFullData` écrase l'octet 13), octet 14 = somme. Attendre l'ack `0x29`.
3. **Anomalie :** après l'écrasement de l'octet 13, la somme n'est pas recalculée (elle date de l'index absolu). Soit la manette ne vérifie pas la somme, soit pas strictement : INCONNU. Un pilote qui recalcule la somme pourrait diverger du comportement observé : ne pas trancher sans matériel.
4. Délai de garde global 3 s côté service.
5. Enregistrement : `0x50 0x03 <id16 gros-boutiste>` (`SendSaveFlashConfig`). Déclencheur exact côté UI et effet (RAM vers mémoire permanente) : INCONNU.

Taux d'envoi : un paquet par aller-retour d'ack, debounce UI 350 ms. Débit maximal toléré : INCONNU.

## Chemin D-Input (dongle, non applicable à 045e:028e)

`LibCommon/LibCommon.Command/DongleCommand.cs`, `LibCommon.Utils/DInputDataUtils.cs` : rapports commençant par `0x05` : début LED normal `05 E7 cfgId nb`, début LED flash `05 E6 cfgId startIdx nb`, paquet LED `05 33 <10 octets> A0 idx`. Longueur et ID de rapport HID exacts : INCONNU.

## Dangers matériel

- **Tout le chemin LED du logiciel écrit dans le profil « Flash » de la manette** (`StartFLashLEDDataKMPC`, `WriteLEDFlash`, ID de config). Space Station le fait à chaque changement de couleur, mais **aucun chemin purement volatil n'est prouvé**. Animation continue : INTERDIT (usure mémoire possible).
- `LedTest (0x13)` / `LightTest (0x15)` sont définis mais **jamais appelés** dans l'application : peut-être un mode volatil, arguments INCONNU, ne pas essayer à l'aveugle.
- Un blob avec en-tête faux (Version, Loop_start) peut produire une config invalide : relire (`0x26`) puis ne modifier que les octets utiles.
- Les mêmes trames `0xA5` portent aussi les commandes de mise à jour du micrologiciel (`0x30` sous-cmd `0x0B`/`0x0C`, `0x40`) : ne jamais envoyer d'opcode non listé ci-dessus.

## INCONNU restant

- DeviceId et zones LED réelles de « Controller (Flydigi VADER4) ».
- Persistance exacte des paquets `0x28/0x29` avant `0x50 0x03`.
- Longueur émise, pipe cible, handshake requis (le service lit d'abord `0x10`, puis `0x20`) : pilote filtre non désassemblé.
- Format de `0x13`/`0x15`.
- Réaction du micrologiciel à une somme invalide.

## Prochaines étapes

1. Capture USB (`usbcapture.rs`) d'un changement de couleur depuis Space Station sur la Vader 4 : longueur de trame, pipe, présence de `0x28/0x29` et moment du `0x50 0x03`. Seule cette capture dit si une écriture est volatile.
2. Désassembler `Xbox360Filter64.sys` (REA) : longueur et pipe.
3. Canal d'envoi : sans pilote filtre signé, aucun. Réaliste : ne rien faire côté PureRGB tant que 1 et 2 ne concluent pas.

## Ce que fait PureRGB aujourd'hui

Détection par la chaîne produit (voir `manettes.md`) : listée « Manette », vendeur Flydigi, note « éclairage non pris en charge pour l'instant ». Aucune écriture HID vers cette manette. Cette analyse ne change pas cette décision.

## Sources externes déjà écartées (recherche web du 2026-10-25)

- Patch noyau `xpad` (octobre 2024) : boutons supplémentaires Vader 3/4 Pro, aucune trame d'éclairage.
- `ahungry/vader3` (Vader 3, D-Input Bluetooth) : pas d'éclairage.
- SDL : HIDAPI Vader 5 Pro, autre modèle.
- `pipe01/flydigictl` (GPL-3.0, Vader 3 Pro), `BANANASJIM/flydigi-vader5` : autres modèles, licence incompatible, non exploités.
