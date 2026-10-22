# SignalRGB (WhirlwindFX) : analyse pour PureRGB

Analyse statique de l'installeur officiel, sans exécution ni installation. Les plugins d'appareils sont du JavaScript lisible : ce sont des descriptions de protocole (octets, ordre, adresses). Aucun code ni asset n'est recopié ici, seulement des faits avec leur source.

## 1. Source analysée

| Élément | Valeur |
|---|---|
| Logiciel | SignalRGB (WhirlwindFX / Whirlwind Virtual Realities Inc), Windows |
| Installeur | `Install_SignalRgb.exe`, 322 934 336 octets |
| URL | https://release.signalrgb.com/Install_SignalRgb.exe (lien du site signalrgb.com/download) |
| SHA-256 installeur | `d42099f9cd6cbc9c00d0ebdf4faa01ff2b2cb1ea5efaca116174a33b42c07377` |
| Format | Squirrel.Windows : `7z x` donne `Update.exe`, `RELEASES`, `VortxEngine-2.5.80-full.nupkg` (zip) |
| Version | 2.5.80 (nupkg `VortxEngine-2.5.80-full`) |
| SHA-1 nupkg (fichier RELEASES) | `0C3AA9B73C9444393FC744BBBC1D0B205A2909F1` |
| Application | `lib/net45/Signal-x64/SignalRgb.exe` (C++/Qt 6, 113 Mo, symboles de chemins sources `Signal\Discovery\...` conservés) |
| Plugins embarqués | `Signal-x64/Plugins/` : 462 fichiers `.js` (≈ 452 plugins d'appareil), `Components/` : 209 JSON (accessoires ARGB), `Effects/`, `LCDFaces/`, `Macroscripts/` |
| Dépôt amont des plugins | https://gitlab.com/signalrgb/signal-plugins (liste « builtin » des add-ons de l'application, archive publique `master`). C'est un état plus ancien et plus éclaté que l'embarqué (471 plugins par modèle, dont 31 QMK/VIA, 5 SRGBmods, services WLED/Nanoleaf/Hue). Les VID/PID absents de l'embarqué ont été ajoutés au CSV avec la source `gitlab-signal-plugins/...` |
| Noms de dossiers dans le CSV | Le champ `vendor` est le dossier fabricant des plugins (pas le champ `Publisher()`, qui vaut presque toujours « WhirlwindFX ») |

Limites : le dépôt gitlab `signalrgb/msi` (add-on cartes mères MSI) renvoie 404 (privé) et `myrepo` 403. Le catalogue peut aussi être mis à jour à chaud par un « Plugin CDN » (voir §4) : le catalogue réel d'une machine peut dépasser l'embarqué.

## 2. Moteur et API de plugins

### 2.1 Architecture

- Application C++/Qt 6 (QML pour l'UI, QtWebEngine/Chromium 140 pour les effets « LightScripts » HTML canvas, Ultralight pour les visages d'écrans LCD, `libgit2` pour mettre à jour les add-ons, `libusb`/`hidapi`/WinUSB intégrés, `nvapi64.dll`, ADLX). Sources internes citées par les chaînes : `Products\ThirdParty\Plugin\ThirdpartyJsPlugin.cpp`, `PluginHid.cpp`, `ThreadedJSEngine.cpp`, `Discovery\USBMonitor.cpp`, `Discovery\I2CBusManager.cpp`, `Discovery\I2C\SystemBusManager.cpp`, `Discovery\I2C\Lpc\{LpcIO,IT87XX,Nct677X}.cpp`.
- Les plugins d'appareil sont des **modules ES** exécutés dans un moteur JS Qt (`QJSEngine`, classe `ThreadedJSEngine`), pas dans Chromium. Modules importables : `@SignalRGB/Errors`, `@SignalRGB/lcd`, `@SignalRGB/DeviceDiscovery`, `@SignalRGB/permissions`, `@SignalRGB/systeminfo`, `@SignalRGB/serial` (chaînes + imports comptés dans les plugins).
- Un plugin est rechargé automatiquement quand son fichier change (« Watched Path has Changed. Reloading Plugin... »).

### 2.2 Contrat d'un plugin (exports)

| Export | Rôle |
|---|---|
| `Name()`, `Publisher()`, `Documentation()`, `ImageUrl()`, `Size()` | Métadonnées ; `Size()` = grille de LEDs virtuelle [largeur, hauteur] |
| `VendorId()` / `ProductId()` | VID/PID USB. `ProductId()` renvoie un entier **ou un tableau** (tous les PID gérés par le plugin). Pour ~130 plugins la liste est dérivée des clés d'une table interne (`Object.keys(...PIDLibrary)`) ou écrite en tableau |
| `Type()` | Transport : `"Hid"` (HID supposé quand `Type()` est absent), `"rawusb"` (libusb/WinUSB, `device.bulk_transfer` / `control_transfer`), `"serial"` (port COM, `@SignalRGB/serial`), `"SMBUS"` (bus I2C/SMBus, pas de VID/PID), `"hybrid"` (HID + rawusb) |
| `DeviceType()` | Catégorie UI : keyboard, mouse, mousepad, headphones, lightingcontroller, aio, motherboard, gpu, ram, lcd, dongle, speakers, microphone, case, other |
| `Validate(endpoint)` | Filtre sur chaque collection HID du périphérique : `endpoint.interface`, `.usage`, `.usage_page`, `.collection`. Seules les collections acceptées sont ouvertes. Liste complète : `signalrgb-endpoints.csv` |
| `Scan(bus)` | Plugins SMBUS uniquement : renvoie les adresses I2C 7 bits trouvées sur le bus |
| `Initialize()`, `Render()`, `Shutdown(SystemSuspending)` | Cycle de vie. `Render()` est appelé en boucle (cadence demandée par `device.setFrameRateTarget(n)`, valeurs vues : 30, 40, 60, 90), `Shutdown` à l'arrêt ou à la mise en veille |
| `ControllableParameters()` | Propriétés exposées à l'UI (couleur d'arrêt `shutdownColor`, mode `Canvas`/`Forced`, `forcedColor`, DPI, etc.) |
| `LedNames()`, `LedPositions()`, `DefaultPosition()`, `DefaultScale()` | Géométrie des LEDs sur le canevas |
| `SubdeviceController()`, `SupportsFanControl()`, `ConflictingProcesses()` | Sous-appareils (canaux ARGB), ventilateurs, logiciels concurrents à fermer (iCUE.exe, RGBFusion.exe, LightingService.exe, LEDKeeper2.exe, L-Connect-Service.exe, TRCC.exe, PredatorSense.exe, ...) |

### 2.3 API objet `device` (utilisée dans les plugins, comptes d'appels)

- E/S HID : `write` (1143 appels), `send_report` (798), `read` (321), `get_report` (104), `set_endpoint(interface, usage, usage_page, collection)` (208), `clearReadBuffer`, `getLastReadSize`, `getHidEndpoints`, `getHidInfo`, `input_report`.
- E/S USB brut : `bulk_transfer(endpoint, data, length[, timeout])` (25), `control_transfer(type, request, value, index, data, length, timeout)` (12).
- Couleurs : `color(x, y)` (échantillonne le canevas à la position de la LED), `createColorArray`, `subdeviceColor`, `getChannelPulseColor`, `getBrightness` (déjà appliquée à `color()`), `getImageBuffer`.
- Topologie : `setName`, `setSize`, `setControllableLeds(noms, positions)`, `addChannel(id, ledLimit, defaultCount)`, `channel()`, `createSubdevice`, `setSubdeviceLeds`, `SetLedLimit`, `getLedCount`, `setImageFromUrl`.
- Ventilateurs/capteurs : `createFanControl`, `getNormalizedFanlevel`, `getFanlevel`, `setRPM`, `fanControlDisabled`, `createTemperatureSensor`, `SetTemperature`.
- Divers : `pause(ms)` (imprécis, ±10 %), `log`, `notify`, `addProperty`, `getProperty`, `addFeature` (ex. `battery`), `productId()`, `vendorId()`, `getMotherboardName()`.
- Objet `bus` (plugins SMBUS) : `IsSystemBus/IsIntelBus/IsAMDBus/IsNvidiaBus/IsNuvotonBus/IsAMDGpuBus`, `Vendor/SubVendor/Product/SubDevice` (identifiants PCI du contrôleur ou du GPU), `ReadByte/ReadWord/ReadBlockBytes/ReadByteWithoutRegister/ReadBlockWithoutRegister`, `WriteByte/WriteWord/WriteBlock/WriteBlockWithoutRegister/WriteQuick`, `TransactionLock/Unlock`, `FetchRamInfo`, `pause`. Deux flavours : bus « libre » (adresse passée à chaque appel, utilisé dans `Scan`) et bus « fixe » (adresse trouvée par `Scan`, utilisé dans `Initialize/Render`).
- Les plugins SMBUS portent le commentaire « Modifying SMBUS Plugins is -DANGEROUS- and can -DESTROY- devices ».
- Réseau : `DeviceDiscovery.foundVirtualDevice(...)` (périphériques réseau/virtuels), modules TCP/UDP côté moteur. Hors périmètre du CSV.

## 3. Accès bas niveau : SMBus/I2C, Super I/O, pilotes

### 3.1 Pilote installé : `SignalRgbDriver.sys` (« SignalIO »)

Le pilote noyau est embarqué en clair dans `SignalRgb.exe` (ressource Qt `:/files/Resources/Driver/SignalRgbDriver.sys`, octets 74 028 378 à 74 085 948, 57 570 octets, extrait pour analyse). SHA-256 de ce `.sys` : `66a04f2113c6066ab4d1c0031127f343a1324f76619ae432a02690893089ebf0`. PDB : `D:\Projects\Whirlwind\signalrgbdriver\x64\Release\SignalRgbDriver.pdb`, signé « SignalRGB (Whirlwind Virtual Realities Inc) ».

- Installation : l'appli copie le `.sys` dans `System32\Drivers\SignalRgbDriver.sys`, crée le service `SignalRgbDriver`, ouvre `\\.\SignalIo`. Options CLI : `--installsignalio`, `--uninstallsignalio` (droits admin requis). Contrôle de version du pilote (« Mismatched SignalIO Driver »). Elle supprime l'ancien pilote WinRing0 (`\Drivers\WinRing0x64.sys`, service `WinRing0x64`) si son SHA-1 correspond : « New SignalIO driver doesn't require Winring0 ».
- Objet : `\Device\SignalIo`, lien `\DosDevices\SignalIo`. Descripteur de sécurité (SDDL lu dans le pilote) : `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)` : SYSTEM et Administrateurs tous droits, **tout utilisateur authentifié en lecture/écriture**. Le pilote référence `\SignalRgb.exe` et `\SignalRgbService.exe` (contrôle de l'appelant, `IoValidateDeviceIoControlAccess`, `ProcNotifyStatus`).
- Interface IOCTL : les codes extraits du désassemblage (`objdump -d`) sont ceux, bien connus, de WinRing0/OpenLibSys : `0x9C402000` version, `0x9C402004` compteur de références, `0x9C402084` lecture MSR (`rdmsr`), `0x9C40208C` lecture PMC, `0x9C4060CC/D0/D4` lecture port E/S octet/mot/dword, `0x9C40A0D8/DC/E0` écriture port E/S octet/mot/dword, `0x9C406144` lecture config PCI, `0x9C40A148` écriture config PCI (importe `HalGetBusDataByOffset`/`HalSetBusDataByOffset`). L'identification par valeurs est ferme ; les noms des IOCTL sont ceux de WinRing0 (déduits).
- **Liste blanche de ports E/S** (routine à `0x1400011A8`, table de 24 plages `base 16 bits + longueur 16 bits` et 8 ports isolés). Plages (base:longueur) : `0x0B00:0x40`, `0x0500:0x20`, `0x0580:0x20`, `0xEFA0:0x20`, `0x3000:0x20`, `0x4000:0x20`, `0x4040:0x20`, `0x5000:0x20`, `0x5040:0x20`, `0x6040:0x20`, `0x8000:0x20`, `0xF000:0x60`, `0xFFE0:0x20`, `0x0200:0x10`, `0x0290:0x10`, `0x02A0:0x10`, `0x02B0:0x10`, `0x02C0:0x10`, `0x0A10:0x10`, `0x0A20:0x10`, `0x0A30:0x10`, `0x0A40:0x10`, `0x0620:0x10`, `0x0A80:0x10`. Ports isolés : `0x2E/0x2F`, `0x4E/0x4F` (index/données Super I/O), `0x87/0x88`, `0xCD6/0xCD7` (index/données PM de l'AMD FCH). Lecture : on déduit des plages `0x0B00`, `0x0500`, `0xF000` etc. des ports de contrôleurs SMBus (PIIX4 AMD à 0xB00/0xB20, i801 Intel) et des plages `0x0290..0x0A80` des moniteurs matériels Super I/O (déduction, non confirmée par une trace d'exécution).
- Le pilote SignalIO sert à : bus SMBus Intel/AMD, Super I/O (ventilateurs), config PCI. Sans le pilote, l'appli saute le scan I2C (« SignalIO driver unavailable; skipping I2C scan », réglage `smbusDisabled`).
- Autres pilotes livrés dans le paquet : `post\CDM21228_Setup` (FTDI CDM 2.12.28 : `ftdibus.sys`, `ftser2k.sys`, installés par `post\vortx_post.exe` pour le matériel propriétaire Vortx/Element de WhirlwindFX) et `STTub30.sys` (pilote USB STMicroelectronics, ressource `:/files/Resources/Firmware/Element/Driver/x64/STTub30.sys`, pour l'appareil Element). Aucun pilote n'est nécessaire pour les périphériques HID/rawusb tiers (hidapi + libusb/WinUSB intégrés).

### 3.2 Types de bus I2C/SMBus (classe `SystemBusManager`, plugins via `bus.Is*`)

| Bus | Détection (chaînes de l'appli) | Usage |
|---|---|---|
| Intel ICH8/i801 (« Intel ICH8 Bus - ») | WMI `Win32_PnPSignedDriver` / `Win32_PnPAllocatedResource` / `Win32_PortResource` : périphérique `VEN_8086&DEV_...`, adresse de départ (`StartingAddress`), contrôle « Host Enable / I2C_EN » | RAM, cartes mères |
| AMD PIIX4 (« AMD PIIX4 Bus - ») | base I/O + lecture du registre d'état (0xFF = contrôleur absent, bus désactivé) | RAM, cartes mères |
| Nuvoton (« Nuvoton Bus ») | SMBus lié à un Super I/O Nuvoton (« FOUND NUVOTON CHIP »), déduit des chaînes | cartes mères AMD/ASUS (`IsNuvotonBus`) |
| NVIDIA GPU | `nvapi64.dll` : `NvAPI_EnumPhysicalGPUs`, ids PCI (vendor/subvendor/device/subdevice) ; I2C par NVAPI, lecture/écriture de blocs **uniquement via NvAPI** | GPU (`IsNvidiaBus`) |
| AMD GPU | ADLX (`AMDI2CBus`, `ADLXInterface`) ; filtre par sous-vendeur PCI (ASRock = 0x1849) | GPU (`IsAMDGpuBus`) |

- Verrous inter-processus (mutex nommés) : `Global\Access_SMBUS.HTP.Method`, `Global\Access_PCI`, `Global\Access_ISABUS.HTP.Method` , `Global\CorsairLinkReadWriteGuardMutex` (iCUE). Si le mutex SMBus ne peut pas être créé, toute communication I2C est coupée.
- Liste d'adresses interdites (« Blacklisted i2c address used ») appliquée à l'appel ; une fonction de vidage (`smbus-dump.txt`) scanne toutes les adresses par Quick Write.
- Scan des bus : par plugin SMBUS, le résultat est mis en cache (`I2CScanResultsCache`). Détection de RAM : `systeminfo.GetRamInfo` (fabricant lu dans les SPD) ; chemin spécifique « HyperX Fury RAM ».
- Carte mère : `systeminfo.GetMotherboardInfo` (fabricant « ASUSTeK » exigé par le plugin ASUS SMBus).

### 3.3 Super I/O (surveillance et ventilateurs, pas RGB)

L'appli (modules `Lpc\LpcIO`, `IT87XX`, `Nct677X`) balaie les ports LPC `0x2E` et `0x4E` (liste blanche du pilote) pour la **surveillance et le contrôle des ventilateurs de carte mère**, pas pour le RGB. Puces reconnues (chaînes) : ITE IT8613E, 8620E, 8625E, 8628E, 8631E, 8638E, 8655E, 8665E, 8686E, 8688E, 8689E, 8696E, 8705F, 8712F, 8716F, 8718F, 8720F, 8721F, 8726F, 8728F, 8771E, 8772E, 8790E, 8791E/8792E/8795E, 87952E ; Nuvoton NCT6102D/6104D/6106D, 6771F, 6776F, 6779D, 6791D, 6792D(-A), 6793D, 6795D, 6796D(-R/-S), 6797D, 6798D, 6799D, 6701D, 6683D, 6686D, 6687D(-R), NCT5577D ; Winbond W83627DHG(-P), W83627EHF, W83627HF, W83627THF, W83667HG(-B), W83687THF ; Fintek F71858, F71862, F71869(A), F71878AD, F71882, F71889AD/ED/F, F71808E ; « Asus ATK0110 ». Opérations IT87XX : `SaveOriginalFanState`, `SetFanSpeed`, `RestoreAllFanStates`/`RestoreOriginalFanState`. Registres exacts : INCONNU (non lus ici).
Le RGB des cartes mères passe, lui, par des contrôleurs USB HID (ITE 0x048D, ASUS Aura 0x0B05, MSI 0x1462/0x0DB0, Gigabyte via ITE, ASRock, NZXT) ou par SMBus (ASUS Aura carte mère).

## 4. Détection automatique des appareils

1. **USB/HID** : `USBMonitor` énumère par WMI (`Win32_USBControllerDevice`) et par l'énumération HID (hidapi) au démarrage (« Initial USB Scan »), puis suit les branchements/débranchements (`OnUSBDeviceConnected/Disconnected`). Chaque appareil (VID/PID) est comparé aux plugins (`hasMatchingPlugin`, tables `VendorId`/`ProductId`). Pour chaque collection HID, la fonction `Validate(endpoint)` du plugin décide si elle est ouverte. Chemins HID listés en liste noire ignorés. Les appareils sans plugin sont signalés comme virtuels non supportés (`DeviceDiscovery.foundVirtualDevice({supported:false, vendorId...})`).
2. **Microsoft Dynamic Lighting (HID LampArray)** : détecté par l'usage `LAMPARRAY` ; ouvre `HKCU\Software\Microsoft\Lighting` ; si l'utilisateur active Dynamic Lighting, SignalRGB demande un redémarrage (`WDLSettingHandler`).
3. **SMBus/I2C** : après création des bus (§3.2), chaque plugin SMBUS exécute `Scan(bus)` ; les adresses trouvées créent des instances. Portes de sécurité : bus système seulement (`IsSystemBus`) pour la RAM, ids PCI exacts pour les GPU (vendor/subvendor/device/subdevice contre une table, voir CSV).
4. **Série** (écrans Turzx) et **rawusb** (Thermalright, Turzx 8.8) : appariement par VID/PID comme HID.
5. **Plugins à jour sans nouvelle version** : `PluginCdnManager` interroge un manifeste (`/plugin_cdn/...`, fichiers JSON, hash, versions de retour arrière), écrit le catalogue dans le dossier de plugins ; activé par la porte Statsig `plugin_cdn`. Les add-ons (dépôts git, liste d URL intégrée : `gitlab.com/signalrgb/signal-plugins` et `myrepo`) sont clonés/fast-forward avec libgit2. URL exactes du CDN : INCONNU (seul le chemin `/plugin_cdn/` est lisible).
6. **Conflits** : `ConflictMitigator` ferme/avertit pour les processus listés par `ConflictingProcesses()` ; `BlacklistedModules` détecte des DLL injectées.

## 5. Catalogue d'appareils (CSV)

Voir `signalrgb-devices.csv` (colonnes `vendor,vid,pid,name,type,protocol_family,leds,source_file`) et `signalrgb-endpoints.csv` (condition `Validate` par plugin).

Conventions du CSV :
- `vid`/`pid` : USB en hexadécimal. Pour les lignes `type=gpu` : identifiants **PCI** du GPU (vid = vendeur du GPU, ex. 0x10DE, pid = device id) ; le nom contient `[PCI subvendor=... subdevice=... i2c=...]` (sous-vendeur/sous-appareil de la carte du partenaire et adresse I2C de son contrôleur). Les lignes de bus SMBus (RAM, carte mère) et les `component:*` n'ont pas de VID/PID (vides).
- `protocol_family` : `<transport>:<nom du plugin>` (ex. `hid:razer-modern-keyboard`, `smbus-i2c-nvapi:asus-ampere-lovelace-gpu`, `component`). Plusieurs plugins partagent un même protocole ; voir §6 pour les familles.
- `leds` : nombre de LEDs lu statiquement dans les tableaux `LedNames`/`vLedNames`/`Leds` du plugin ou de la table interne du modèle. `INCONNU` quand le nombre est calculé à l'exécution (sondage du matériel, canaux ARGB configurables, claviers par disposition). Pour les plugins à PID multiples sans table par modèle, la valeur est celle du fichier (donc indicative).
- Lignes MSI `Msi ... (cle de Library; PID deduit du numero MS-xxxx)` : le plugin MSI 185 octets (embarqué) indexe ses cartes par nom de modèle `(MS-7Dxx)` ; le PID HID déduit est le numéro `MS-xxxx` (cohérent avec les PID listés par l'ancien dépôt gitlab, ex. 0x7D91). Déduction, marquée dans le nom.

## 6. Protocoles par famille (faits extraits des plugins)

Chaque section cite ses fichiers sources.

### 6.1 Razer, Logitech, SteelSeries
Source = `...\Signal-x64\Plugins\<Fabricant>\`. `[n]` = offset d'octet. "INCONNU" = non confirme dans le source.
Convention `send_report(buf,len)` / `write(buf,len)` : buf[0] = report ID (0x00 = sans ID), `len` inclut cet octet ; le moteur complete de zeros.
Cadence : la boucle `Render()` du moteur envoie a chaque frame (frequence moteur = INCONNU). Pas de keep-alive explicite dans les 3 familles, sauf mention.

### 1. RAZER (VID 0x1532)
#### 1.1 Trame de commande 90 octets (Razer_Modern_Keyboard.js : `StandardPacketSend`, `CalculateCrc`, `ConfigPacketSend/Read`)
Transport : HID **feature report** (`send_report(pkt,91)`, report ID 0x00 + 90 o). Reponse : `get_report([0,0,txid],91)` apres `pause(10)`.

| Off. trame (hors report ID) | Contenu |
|---|---|
| [0] | status = 0x00 (reponse : 1 busy, 2 OK, 3 echec, 4 timeout, 5 non supporte) |
| [1] | transaction id (txid) |
| [2..3] | remaining packets = 0x0000 |
| [4] | protocol type = 0x00 |
| [5] | data size (taille des args) |
| [6] | command class |
| [7] | command id (bit 7 = lecture : 0x82, 0x84...) |
| [8..87] | arguments |
| [88] | CRC = XOR des octets [2..87] |
| [89] | 0x00 reserve |

- txid : `getDeviceTransactionID` essaie 0x1f,0x2f,...,0x9f avec la commande serial (size 2, class 0x00, id 0x82) ; garde le txid dont la reponse [8..22] est un serial non nul ; `pause(400)` entre essais, 5 boucles max. Addressable RGB Controller : txid fixe 0x1f.
- Variante "NewWriteLighting" (flag `requiresNewWriteLighting` ou firmware >= 1.5 par defaut) : [1] n'est plus le txid mais un compteur 0x80..0x9F (retour a 0x80 apres 0x9F). Seuil exact par modele = bibliotheque du plugin (INCONNU au cas par cas).
- Pause inter-paquet : `requiredPause` par modele (ex. 1 ms ; 0 par defaut). Legacy mouse : `pause(5)` apres chaque envoi.

#### 1.2 Endpoints (interface / usage / usage_page)
| Cas | Valeur |
|---|---|
| Claviers (23 modeles) | 2 / 0x0002 / 0x0001 |
| Claviers (16 modeles, recents) | 3 / 0x0001 / 0x000C |
| Souris modernes (Modern_Mouse) | 3 / 0x0001 / 0x000C ; Basilisk V3 (PID 0x0099) : interface 3 quel que soit l'usage |
| Mousepad_Controller, Legacy_Mouse (DeathAdder Chroma PID 0x0043) | 0 / 0x0002 / 0x0001 |
| Lecture macros / veille | `set_endpoint(1,0x0000,0x0001)` (16 o) ; `(1,0,1,0x0005)` ou `0x0006` si Hyperspeed ; paquet `05 09 03` = reveil (re-init apres 3 s), `05 09 02` = veille |

#### 1.3 Chemin couleur courant (matrice par ligne : clavier, souris, tapis "Chroma moderne")
Init (`deviceInitialization`, `setSoftwareLightingMode`) :
1. `getModernMatrixEffect` : size 0x06, class 0x0F, cmd 0x82, args `00` -> OK (status 2) = matrice moderne.
2. Mode logiciel (custom frame) : size 0x06, class 0x0F, cmd 0x02, args `00 00 08 00 01`.
   - Sinon (souris "Modern" legacy, claviers anciens, PID 0x0268) : effet legacy custom = size 0x02, class 0x03, cmd 0x0A, args `05 00`.
   - Hyperflux (Modern_Mouse) : en plus mode appareil logiciel = size 2, class 0x00, cmd 0x04, arg `03`.
3. Mode appareil (hardware 0x00 / factory 0x02 / software 0x03) : size 2, class 0x00, cmd 0x04 (lecture 0x84 -> octet [8]).

Envoi de couleurs, **un paquet par ligne** (`setKeyboardDeviceColor`, `setDeviceColor` du mousepad) :
`size = 3N+5, class 0x0F, cmd 0x03, args = 00 00 <ligne> 00 <N-1> + N x (R,G,B)`. N = `LEDsPerPacket` du modele (10, 15, 16, 18, 22 ou 23 ; 22 pour la plupart des claviers pleine taille). Ordre des LEDs par `vKeys`. Pas de commit pour ces modeles. Le plugin boucle sur `ceil(nbLEDs/N)+1` paquets (le dernier peut etre vide).
- Souris (`setMouseLighting`) : meme commande, ligne unique : `size 3N+5, 0x0F, 0x03, 00 00 00 00 <N-1> + RGB`.
- Claviers Chroma/Synapse2 (`requiresApplyPacket: true`, ex. Blackwidow Chroma) : `size 3N+5, class 0x03, cmd 0x0B, args FF <ligne> 00 <N-1> + RGB` (le source ne met que 4 o d'args avant le RGB alors que size=3N+5 : ecart non explique, INCONNU), puis apply apres CHAQUE ligne : effet legacy `0x02,0x03,0x0A,05 00`, ou si matrice moderne `0x06,0x0F,0x02,00 00 08 00 01`.
- Mousepad_Controller (modele Firefly 15 LEDs, `requiresApplyPacket`) : forme moderne ci-dessus ; forme legacy `setDeviceLegacyColor` : `size 3N+2, class 0x03, cmd 0x0C, args 00 <N-1> + RGB` + effet legacy. Souris PID 0x0053 : forme legacy ; PID 0x0046 : `3N+5, 0x03, 0x0C, 00 00 00 00 <N-1> + RGB`.
- Legacy_Mouse (DeathAdder Chroma, `setMouseLighting`, zones `legacyLightingZones` ex. [1,4]) : par zone `size 5, class 0x03, cmd 0x01, args 00 <zone> R G B` puis `size 3, class 0x03, cmd 0x02, args 00 <zone>`.
- Batterie : poll toutes les 15 s (`PollModeInternal`).

#### 1.4 Shutdown / retour materiel
- Clavier et mousepad : veille systeme -> noir ; sinon `shutdownColor` puis `0x06,0x0F,0x02,00 00 03` (retour effet materiel).
- Souris moderne : couleur shutdown, `setDeviceDPIToHardware`, puis mode appareil Hardware (class 0, cmd 0x04, arg 0x00).
- Legacy mouse : couleur shutdown + mode Hardware.

#### 1.5 Autres controleurs
- **Addressable RGB Controller** (PID 0x0F1F ; 6 canaux x 80 LEDs max, limite 240) : config sur endpoint (0, 0x0001, 0x000C), txid 0x1f. Init : lecture mode (`size 2, 0x00, 0x84`), forcer mode 3 si besoin (`0x02,0x00,0x04,03`), puis `0x06,0x0F,0x02,00 00 08 03 88` ; `channelInit` : par canal `size 3, class 0x0F, cmd 0x04, args 01 (0x1A+canal) FF` (luminosite max) ; `size 0x0D, 0x0F, 0x08, args 06 01 50 19 50 19 50 19 50 19 50 19 50 19` puis lecture `0x0D,0x0F,0x88,06`. Flux couleur sur endpoint (1, 0x0001, 0x000C), **rapport 321 o** : `[0]=00, [1]=0x04 (0x84 pour le canal 5), [2]=canal, [3]=canal, [4]=00, [5]=nb LEDs, puis RGB en ligne`. Un rapport par canal a chaque frame. Shutdown = couleur shutdown.
- **Kraken** (PID 0x0527, 0x0533, 0x0537, 0x0549, 0x0560, 0x0554 ; endpoint 3/0x0001/0x000C ; pas de CRC) : mode logiciel `write([40 01 00 00 08], 28)` (variante 4 LEDs : `40 01 00 0F 08`, longueur 15). Couleur `write([40 03 00] + RGB, 9)` (1 LED "Both cans" ; variante 4 LEDs longueur 15).
- **Hanbo Cooler** (PID 0x0F35 ; usage_page 0xFF00, usage 0x0001, interface -1 ou 0) : `write` 64 o. Init : `00 80 01 00 01`, `00 80 01 01 01`, `00 70 01 01 64`, `00 70 01 00 64`. Pompe 16 LEDs : `00 32 01 07 00 00 00 00 00` + RGB en ordre **G,R,B**. 3 canaux ventilos (18 LEDs max) : `00 40 01 07 00 00 00 00 <canal+1>` + GRB. Shutdown : vide.
- Non lus : Blackwidow V3 Pro, Deathstalker V2 Pro, Barracuda X, Kraken V4, BaseStation, Nari, Mouse Dock, etc. Table PID->matrice complete : dans `deviceLibrary` de chaque plugin (INCONNU ici).

### 2. LOGITECH (VID 0x046D)
#### 2.1 HID++ 2.0 moderne (Logitech_Modern_Device.js, `LogitechProtocol`)
Trames : court `0x10` (7 o), long `0x11` (20 o), tres long `0x12` (64 o). Format : `[0]=type, [1]=device index, [2]=feature index, [3]=(fonction<<4)|swId, [4..]=params`. Dans le plugin, `data` = `[featureIdx, octet fonction, params...]` apres le prefixe `[type, devIdx]`.
- Device index : 0xFF = filaire/direct, 0x01 = sans fil (recepteur Lightspeed). `PingDevice` : court `10 FF 80 00 00 01` (activer notifs HID++1.0) ; erreur code 1 = filaire -> 0xFF, sinon 0x01 ; "faux reconnect" `10 FF 80 02 02 00` (reponse : PID transport octets [3],[2]) ; puis notifs coupees `10 FF 80 00 00 00`.
- Endpoints (usage_page 0xFF00) : court = usage 0x0001, long = usage 0x0002. Interface 2 si multipoint/Lightspeed (2/0x0001/0xFF00 present), interface 1 si singlepoint. `Validate` : interface 1 ou 2 avec usage 1 ou 2. Lecture : 7 o (court) / 20 o (long), timeout 20 ms.
- Delai : avant tout envoi non "noDelay" `pause(30)` + vidage buffer de lecture ; les envois de couleur sont `noDelay` sans reponse. Requetes : 5 tentatives (10 pour la decouverte), controle du feature index de retour.
- **Decouverte de features** : IRoot (index 0) fonction 0 : long `[00, 00, pageHi, pageLo]` ; reponse data[1] = index (0 = absent ; > 64 ignore). Pages : 0x0001 FeatureSet, 0x0003 DeviceInfo, 0x0005 DeviceName, 0x0007 FriendlyName, 0x0020 Reset, 0x1001 BatteryVoltage, 0x1004 UnifiedBattery, 0x1300 LEDControl, 0x1D4B WirelessStatus, 0x1010 ChargingControl, 0x2201 DPI, 0x8060 PollingRate, 0x8100 OnboardProfiles, 0x8110 ButtonSpy, 0x1B04 ReprogControlsV4, 0x8010/0x8020/0x8030 G/M/MR keys, 0x8040 Brightness, 0x1814/0x1815 hotes, **0x8070 RGB, 0x8071 RGB "Hero", 0x8080 PerKey, 0x8081 PerKeyV2**.
- Identification : DeviceInfo fn 0 (court `[idx, 00]`, reponse longue) : data[1]=nb entites, [2..5]=id unique, [8..13]=3 model IDs, [15]=serie. ID retenu si present dans `LogitechDevice.DeviceIDs` (table non repertoriee ici).
- Polling : court `[PollingRateIdx, 0x20, 1000/Hz]`.
- Mode direct (`SetDirectMode`) : Hero = court `[RGB8071, 0x50, 01, 03, 05]` ; sans PerKeyV2 en plus long `[RGB8071, 30, 01, 00, 08, 04, 07]`, court `[RGB8071, 20, 00, 03]`, court `[RGB8071, 30, 00, 00, 08]`. 8070 = court `[RGB8070, 0x80, 01, 01]` + `[LEDControl, 0x30, onboard ? 0 : 1]`.
- **Zone unique (souris, `SendSingleZoneLighting`)** : par zone, long sans reponse `[RGB8071 (Hero) ou RGB8070, fn 0x10 (Hero) ou 0x30, zone, 01, R, G, B, 00 (Hero) ou 02]` ; pour Hero et IDs "4067"/"4070", data[14]=0x01. `pause(5)` entre zones. Apply `[00, 20, 01]` (long) seulement pour IDs "4079" et "405d".
- **PerKey V2 (0x8081, claviers)** : long `[idx8081, 0x10, (keyId,R,G,B) x4 max]` (16 o), seulement les touches modifiees ; apply long `[idx8081, 0x70]`. `deltaCompression` : fn 0x40 = id depart + 15 o de deltas 4 bits signes R,G,B pour 10 zones consecutives (sinon repli fn 0x10) ; apply 0x70. Mode "range" (G515 filaire) : fn 0x50 = 3 plages `[first,last,R,G,B]`, fn 0x60 = `[R,G,B, 13 ids]` ; apply 0x70.
- Powerplay (PID 0xC53A) : endpoint (2,0x0002,0xFF00), init par `10 07 0B 10`, `11 07 0B 20 ...`, `10 07 0B 80 01 01` ; couleur long `11 07 0B 30 00 01 R G B 02`.
- Shutdown : couleur shutdown (noir en veille) ; si `settingControl`, `SetOnBoardState(true)` (rend la main au profil embarque). Appareil absent : `PingDevice` reessaye chaque seconde.

#### 2.2 Legacy "8080" : feature index FIGES, device index 0xFF (dossier `8080 Devices (Legacy)`)
Lighting IF = (1, 0x0604, 0xFF43) rapports 64 o ; System IF = (1, 0x0602, 0xFF43) rapports 20 o.
| Modele (PID) | Couleur | Apply (20 o) |
|---|---|---|
| G810 (0xC331, 0xC337) | clavier `12 FF 0C 3D 00 01 00 <n>` + n x (usage HID,R,G,B), n=14 (7 paquets + 9 touches) ; G-keys `12 FF 0C 3D 00 04 00 09` ; logo `11 FF 0C 3D 00 10 00 02` | `11 FF 0C 5D` |
| G910 (0xC335, 0xC32B) | clavier `12 FF 0F 3B 00 01 00 <n>` (n<=14) ; G-keys `12 FF 0F 3F 00 04 00 09` ; logo `11 FF 0F 3D 00 10 00 02` ; `pause(2)` | `11 FF 0F 5B` ; init G/M : `11 FF 08 00`, `11 FF 08 20 01`, `11 FF 09 00`, `11 FF 09 10 00` |
| G513 (0xC33C) | `12 FF 0C 3E 00 01 00 <n>` | `11 FF 0C 5E` |
| Pro TKL (0xC339) | clavier `12 FF 0C 3C 00 01 00 <n>` ; logo `11 FF 0C 3E 00 10 00 01` ; media `11 FF 0C 3D 00 40 00 <n>` | `11 FF 0C 5C` |
| G815 (0xC33F ; Validate 1/0x0602/0xFF43) | `11 FF 10 1A` + 4 x (id,R,G,B) par paquet (20 o), `pause(1)` | `11 FF 10 7A` |
| G213 (0xC336) | 5 zones, 1 paquet/zone : `11 FF 0C 3A <zone 1..5> 01 R G B 02`, `pause(10)` | `11 FF 03 3A` |
| G402 (0xC07E) | pas de RGB : luminosite `11 FF 05 5B 01 00 80 00 <lum>` ; init `10 FF 05 3C 01` (7 o) ; DPI `10 FF 0C 3A 00 hi lo` | - |
- G410/G412/G512/G560 (PID 0x0A78, interface 2 usage 0x0202)/Headset : non detailles (INCONNU). Retour mode materiel au Shutdown : rien trouve, seulement couleur shutdown + Apply (INCONNU).

### 3. STEELSERIES (VID 0x1038)
Aucun CRC. Report ID 0x00.
#### 3.1 Claviers "direct" (Steelseries_Keyboard_Controller.js ; PID 0x1612, 0x1618, 0x161C, 0x1650, 0x1640, 0x1642, 0x1644, 0x1646, 0x1648, 0x1624)
- Endpoint : interface 1 (filaire) ou 3 (sans fil, et 0x1646), usage 0x0001, usage_page 0xFFC0, collection 0.
- Init : `send_report([00 4B], 642)` (mode direct).
- Couleur : `send_report([00, opcode, nbLED, (ledIdx,R,G,B) x nbLED], 642)`. Opcode par modele : 0x3A (gen 2019, interface 1 : Apex Pro/7/Pro TKL/5/7 TKL), 0x40 (Pro filaire : Pro TKL 2023/Gen3, Pro Mini 0x161E), 0x61 (sans fil interface 3), 0x21 (Apex 3 TKL, Pro Mini 0x1626, Pro TKL Gen3 0x1646), 0x0B (Apex 3 par zones, octet nbLED fixe 0x00 sur Apex 3, 0xFF sur Apex 3 TKL).
- OLED (Apex Pro, 128x40, rapport 643 o, commande 0x65) : trame `00 65 00` + image 1 bit. Commit : INCONNU.
- Fichiers dedies : Apex 3 (PID 0x161A, interface 3) `write([00 0B 00 + 10 zones RGB], 33)`. Apex Pro 0x1610, Pro TKL 0x1614, Pro TKL 2023 0x1628/0x1630/0x1632, Apex 3 TKL 0x1622 : non detailles (INCONNU).
#### 3.2 Claviers "513 octets" (Apex 9 TKL 0x1634, 9 Mini 0x1620, M750 0x0616, M750 TKL 0x0617/0x0619)
- 9 TKL : interface 1 usage 1 ; `send_report([00 40 nb, (usage HID,R,G,B) x nb], 513)` a chaque frame.
- M750 : interface 2 ; init = 3 rapports 513 : `00 00 00 00 01 00 85`, `00 00 00 00 03 01 00 FF`, `00 00 00 00 01 00 85` ; couleur : `00 00 00 01 8E 01 03 06 16` puis RGB a l'offset `9 + 3*idxLED` (104 LEDs), 513 o. Commit : INCONNU (commentaire du source : paquets de commit 64 o vus a USBlyzer, non reproduits).
#### 3.3 Souris (un envoi par zone a chaque frame, `pause(1)`)
| Modele (PID) | Interface | Trame couleur |
|---|---|---|
| Rival 3 (0x184C) | 3 | init `00 09` (69 o) ; couleur `write([00 0A 00 0F, 4 x RGB], 69)` |
| Rival 3 Gen2 (0x1870) | INCONNU | `[00 21 07, RGB...]` 65 o ; polling `[00 2B v]`, `[00 23 64]`, fw `[00 90]` |
| Rival 5 (0x183C) | INCONNU | init `00 09` ; `[00 21 FF 03, RGB...]` 65 o |
| Rival 110 (0x1729) | 0 | `[00 05 00, RGB, ..., RGB@18..20]` 65 o |
| Rival 310 (0x1720, 0x171E), Sensi Ten (0x1832) | 0 | **feature report 263 o** : `[00 5B 00 <zone> 10 27]`, [20]=01, [28]=01, [29..34]=R G B R G B ; zones 0 et 1 |
| Rival 600 (0x1724) | 0 | 8 zones, 579 o : `[00 05 00 <zone> 55 00 91 00 <zone> 10 27]` + RGB a [31..33] et [46..48] ; init `write(00 33 00 31 00 00 00 00 32, 33)` ; reste INCONNU |
| Rival 650 (0x172B, 0x1726) | INCONNU | `write` 65 o `[00 03 .. [5]=30 [7]=10 [8]=27 [23]=01 [31]=04 RGB@32..34 et 47..49]` puis `[00 05 .. [3]=16+zone [4]=FF [9]=5C]` ; `pause(3)` |
| Rival Prime (0x182E) | INCONNU | `write` 643 o `[00 62 01 R G B ... RGB@18..20]` ; DPI `[00 61 01 00 dpi/50]` |
| Rival 700 (0x1700) / 710 | INCONNU | feature report 579 o `[00 05 00 <zone> R G B FF 32 C8 ...]` ; haptique `[00 59 01 00 type]` 32 o |
| Aerox 3/5 filaire (0x1836, 0x1850) | 3, usage 1, page 0xFFC0 | 3 zones (ids 1,2,4) : `write([00 21 <zone> R G B], 65)` |
| Aerox 3/5 sans fil (0x1838, 0x183A, 0x1852, 0x1854, 0x1858, 0x185A, 0x185C, 0x185E, 0x1860, 0x1876, 0x233A) | 3, usage 1, page 0xFFC0 | `write([00 61 01 <idx 0..2> R G B], 65)` + lecture ; reponse [1]==0x40 = pas de connexion (pause 1 s) ; `pause(10)` fin de frame ; batterie `[00 D2]` toutes les 15 s ; polling `[00 6B v]`, DPI `[00 6D n stage dpi/100...]`, veille `[00 69 b1 b2 b3]`, dim `[00 63 0F 01 smart 00 b1 b2 b3]`, apply `[00 51]` |
#### 3.4 Tapis QcK (Prism 0x1507/0x1514/0x1518/0x151C ; XL 0x3769/0x1520 ; 3XL 0x1516 ; interface 0, usage 1, page 0xFFC0)
- Prism (12 LEDs) : `send_report(buf, 525)` : `[1]=0x0E, [3]=0x0C` ; LED i : `[5+12i..7+12i]=R G B`, `[14+12i]=01`, `[16+12i]=i` ; commit `write([00 0D], 65)` apres chaque trame.
- Controller (XL/3XL, 2 zones) : `[1]=0x0E, [3]=0x02`, zone 1 RGB a [5..7] + `FF 32 C8` a [8..10], [14]=01 ; zone 2 RGB a [17..19] + `FF 32 C8` a [20..22], [25]=01 ; [26]=01, [28]=01 ; commit `00 0D`.
#### 3.5 Shutdown SteelSeries
Aucune commande "retour mode materiel" dans les plugins lus : Shutdown = renvoi de `shutdownColor` (noir en veille). Reste INCONNU.

### 6.2 Corsair, NZXT, Lian Li, CoolerMaster, Thermaltake, Hyte
Source : `...\Plugins\<fabricant>\...js` (chemins relatifs). Faits seulement, aucun code copie. INCONNU = non confirme dans le plugin.
Convention : "write(N)" = rapport HID de N octets, octet 0 = Report ID. `send_report`/`get_report` = feature report.

### 1. Corsair (VID 0x1B1C)

#### 1.1 Bragi (claviers/souris/dongles modernes) - Modern_Corsair/Corsair_Bragi_Device.js
- PID : `CorsairLibrary.ProductIDList` (ex 0x1BA6 dongle multipoint, 0x0D01 dongle laptop, 0x1B7F/1B7E Dark Core Pro SE). Liste complete non reprise.
- Endpoint (`Validate`) : (interface 1, usage 0x0001, page 0xFF42) ou (interface 2, usage 0x0002, page 0xFF42). `Initialize` : `set_endpoint(0x01,0x01,0xFF42)`.
- Taille rapport : lue via `getHidInfo()` (writeLength/readLength) ; valeur INCONNUE dans le plugin.
- En-tete : `[0x00, deviceID|0x08, cmd, ...]`. deviceID 0 = filaire ou dongle ; 1..7 = enfant (bitmask propriete 0x36, bits 1..7).
- Commandes : 0x01 setProperty, 0x02 getProperty, 0x05 closeHandle, 0x06 writeEndpoint, 0x07 streamEndpoint, 0x08 readEndpoint, 0x09 checkHandle, 0x0D openEndpoint, 0x12 ping (reponse octet[2]=0x12), 0x15 confirmChange.
- Reponse : octet[3] = erreur (0 OK, 1 valeur invalide, 2 paquet trop long, 3 echec endpoint, 5 propriete non supportee, 6 handle non ouvert) (`CheckError`).
- Proprietes : 0x01 polling (1=125Hz ... 4=1kHz ... 7=8kHz), 0x02 lum. HW, 0x03 mode, 0x07 angle snap, 0x0F/0x10 batterie, 0x11 VID, 0x12 PID, 0x13 firmware, 0x1E-0x22 DPI, 0x36 bitmask sous-peripheriques, 0x37 idle timeout, 0x41 layout (1 ANSI, 2 ISO), 0x96 polling max.
- SetProperty : `[0x00,id|8,0x01,prop,0x00,val_lo,val_mid,val_hi]` (`SetProperty`).
- Endpoints : 0x01 Lighting (plans R/G/B), 0x02 Buttons, 0x17 FanRPM, 0x18 FanSpeeds, 0x1A FanStates, 0x1D/0x1E nb LED 3/4 pins, 0x21 sondes, 0x22 LightingController (RGB entrelace). Handles : 0 Lighting, 1 Background, 2 Auxiliary.
- Mode (propriete 0x03) : 1 = Hardware, 2 = Software. Init : ferme les handles ouverts puis Software (`SetMode`). Retour materiel = `SetMode(Hardware)` ; Shutdown du plugin Bragi : couleur d'arret/noir sur veille (bascule HW non confirmee ici).
- Ecriture generique (`WriteToEndpoint`) : open `[..,0x0D,handle,endpoint]` -> write `[..,0x06,handle,len32 LE,donnees]` -> lecture reponse -> close `[..,0x05,0x01,handle]`. Pauses 1-10 ms.
- Trame couleur (`SendRGBData`) : handle Lighting 0x00 (0x01 pour PID 0x1BFE/0x2B05/0x2B20) ouvert sur endpoint 0x01 (ou 0x22). 1er paquet = 8 octets d'en-tete + donnees (len32 = taille totale), suivants en stream `[..,0x07,handle,donnees]` (en-tete 4). Pas de commit separe. Pause 3 ms apres le 1er paquet. Lecture reponse du stream seulement pour K70 Core / K60 Pro.
- Format : endpoint 0x01 = 3 plans consecutifs (tous R, tous G, tous B), espaces de `ledSpacing` par appareil ; endpoint 0x22 = RGB entrelace precede de `[0x12,0x00]`.
- Checksum : aucun. Cadence : 1 trame par `Render` et par appareil ; poll mode/etat/ping sur `PollInterval`.
- Ventilos (hub Bragi) : RPM endpoint 0x17 (octet[4]=6, [6]=nb, u16 LE a partir de [7]) ; etats 0x1A (octet[4]=9) ; duty endpoint 0x18 : `[0x07,0x00,nb,(id,0x00,duty%,0x00)*nb]` (defaut 0x32). Temp endpoint 0x21 : 3 octets/sonde, u16 LE /10.
- Dongle : `ResetDongle` = SetProperty(23,0) + Hardware/Software + 1000 ms (flag non documente).

#### 1.2 Commander Core / Core XT - Modern_Corsair/Corsair_Lighting_Commander_Core.js, _XT.js
- PID Core 0x0C1C (fw dev 2.10.219) et 0x0C32 (2.0.17) ; Core XT 0x0C2A (1.3.54). Endpoint : interface 0, usage 0x0001, page 0xFF42.
- Taille rapport = taille lue sur la reponse a un ping (`FindBufferLength`) ; commentaire "97" pour fw >= 2.10.219, non confirme.
- En-tete `[0x00, 0x08 (filaire), cmd, ...]` (0x09 = sans fil). Memes cmd que Bragi.
- Mode Software a l'init ; Shutdown hors veille = Hardware ; veille = noir.
- LED : Core 1 canal 204 (au-dela de 238 LED le controleur deraille : coupe a 238*3 octets). XT : canal 4 pins 204 + canal 3 pins (max 128, defaut 60), limite device 264.
- Couleur : endpoint 0x22 (handle 0 ouvert a l'init), RGB entrelace + `[0x12,0x00]` ; write `[0,0x08,0x06,0x00,len16 LE,0,0,+data]`, suites `[0,0x08,0x07,0x00,+data]`, lecture reponse a chaque paquet. Pas de commit.
- Config ventilos (`SetFanType`) : endpoint 0x1E (Background) 7 paires (0x01,type) : port 0 type 0x01 (pompe/rien), ports 1-6 type 0x06 (QL, 34 LED). XT : + endpoint 0x1D (3 pins) avec nb LED en 2 octets LE.
- Ventilos/pompe : Core = Pump + Fan1..6 (7 ports), XT = Fan1..6. RPM lu toutes 2 s (endpoint 0x17). Consigne toutes 2 s, endpoint 0x18 : `[0x07,0x00,0x07,(id,0x00,duty%,0x00)*7]`. Pompe plancher 60 % (en dessous : mode failsafe jusqu'au power-cycle). Sondes temp par machine d'etats.
- Pompe Elite Capellix : etat ventilo[0] 7 ou 1 = connectee ; nb LED via ReadEndpoint(0x01,0x20,0x0F)[9].

#### 1.3 iCUE LINK Hub - Modern_Corsair/Corsair_ICUE_Link_Hub.js
- PID 0x0C3F. Endpoint : (interface 0, usage 0x0001, page 0xFF42) ou (interface 1, usage 0x0002, page 0xFF42) ; `set_endpoint(0x00,0x01,0xFF42)`. Taille 513 (commentaires), sinon `getHidInfo`.
- En-tete DIFFERENT : `[0x00, 0x00, deviceID, cmd, handle, len32 LE, donnees]`, deviceID = 0x01 pour le hub. Memes cmd. 1er paquet lumiere : 9 octets d'en-tete, suivants 5.
- Init : Software -> infos/sondes -> Hardware ~11 s (spool des ventilos pour lire les RPM) -> Software -> enumeration enfants (endpoint 0x36, 2 lectures, 34 octets/enfant ; deviceType 6 = LCD, 25 ignore). Shutdown hors veille : Hardware.
- Couleur : handle Lighting, endpoint 0x22, RGB entrelace + `[0x12,0x00]`, donnees concatenees dans l'ordre des enfants. Adaptateurs bandes LS : endpoint 0x20.
- Ventilos : RPM endpoint 0x17 ; consigne endpoint 0x18 `[0x07,0x00,nb,(rpmId,0x00,duty,0x00)*nb]` toutes les 2 s ; pompe (deviceType 0x07) plancher 20 %. Nb LED max : somme dynamique (INCONNU en dur).

#### 1.4 Platinum / Pro XT / Elite AIO (protocole 0x3F) - Platinum_Pro_XT_AIO/Platinum_Pro_XT_Elite_AIO.js
- PID ex 0x0C15/0C18/0C19, 0x0C17 (Platinum), 0x0C29, 0x0C20/0C2D, 0x0C21/0C2E, 0x0C2F/0C22 (Pro XT)... (`PIDLibrary`). Interface -1 ou 0, pas d'usage_page. write(65).
- Trame `[0x00,0x3F,seq|cmd,...]`, octet 64 = CRC-8 (table, poly 0x07 deduit, init 0) sur octets 2..63. seq += 8 par paquet (8..248, repart a 8) ; 3 bits bas = type.
- Couleur : `seq|0b100/101/110`, 60 octets (20 LED) chacun, ordre BGR ; pompe puis anneau ventilos (max 32 LED) ; envoye seulement si la trame change. Activation logicielle : 3 paquets `seq|0b001/010/011` (tables de config).
- Etat : `[0,0x3F,seq,0xFF,0x00]` -> temp liquide u16 LE @8-9 ((v/25.6+0.5)/10), ventilo1 duty@15 rpm@16-17, ventilo2 duty@22 rpm@23-24, pompe mode@25 rpm@30-31, ventilo3 @43-45, statut@5. A emettre au moins toutes ~10 s sinon le refroidisseur coupe ; poll 3 s.
- Ventilo : paquet cmd 0x14 (`seq|0b000`, ventilo 3 : `|0b011`), mode 0x03 (PWM fixe + fallback), duty 0..255 ; pompe par profil Quiet/Balanced/Extreme (0/1/2). Burst 0xFF a l'init puis 15 s.
- Retour materiel : INCONNU.

#### 1.5 Legacy Lighting Controllers - Legacy_Lighting_Controllers/
- PID : Commander Pro 0x0C10 (+0x1D00), Node Pro 0x0C0B, Node Core 0x0C1A, LS100 0x0C1E, LT100 0x0C23, Spec Omega 0x1D04. Interface -1/0/2. write(65), read(17).
- Cmd (octet 1) : 0x02 firmware, 0x10 sondes, 0x11 temp (u16 BE /100), 0x20 etats ventilos (octets 2-7 : 0 absent, 1 DC, 2 PWM, 3 boot), 0x21 RPM (u16 BE octets 2-3), 0x23 set duty `[0,0x23,fan,pct]`, 0x24 set RPM `[..,fan,hi,lo]`, 0x32 couleurs, 0x33 commit `[0,0x33,0xFF]`, 0x34 start `[0,0x34,canal]`, 0x37 reset, 0x38 mode `[0,0x38,canal,1=HW|2=SW]`.
- Couleur : 2 canaux (204 LED), plans R/G/B separes : `[0,0x32,canal,debut,nb,plan(0=R,1=G,2=B),+nb]`, nb <= 50. Sequence : mode SW, start, plans, puis commit unique apres les 2 canaux ; pause 1 ms. Node Core lit 17 octets apres chaque ecriture, Commander Pro non.
- Commander Pro : 6 ventilos, 4 sondes, poll 1 s, pct via 0x23 (bornes INCONNUES). Retour materiel : mode 1 sur chaque canal.

#### 1.6 Legacy claviers/souris - Legacy_Corsair/Corsair_Legacy_Keyboard.js
- Interface normale (1, usage 0x0004, page 0xFFC2), macro (0, usage 0x0002, page 0xFFC0) ; si une interface 2 (0x0004/0xFFC2) existe : normale = 2, macro = 1. Souris : macro page 0xFFC1, wake 0xFFC3. write(65).
- Octet 1 : 0x07 ecriture, 0x0E lecture (feature), 0x7F stream. Sous-cmd : 0x04 mode fonctions, 0x05 mode eclairage `[0x07,0x05,mode,0x00,0x03]` (1 HW, 2 SW, 8 Strafe), 0x0A polling (`1000/Hz`), 0x22 souris/tapis, 0x25 K55, 0x28 clavier 24 bits.
- Clavier : par plan 1=R,2=G,3=B, stream `[0x00,0x7F,n°paquet,nb(<=60),0x00,+data]`, puis apply `[0x07,0x28,plan,0x03,fin]` (fin=1 pour R,G ; 2 pour B). Souris : `[0x07,0x22,nbLED,0x01,RGB...]`. Retour materiel : modes eclairage et fonctions a 1.

#### 1.7 Void headsets - Void_HeadSets/*.js
- PID 0x0A14, 0x0A17, 0x0A19, 0x0A1A, 0x0A1D, 0x1B23 (Pro) ; 0x0A52, 0x0A56, 0x0A75 (Elite). Endpoint interface 3, usage 0x0001, page 0xFFC5 ; init write(20) `[0xC8,0x01]`.
- Couleur write(20) a chaque trame, pause 30 ms : `[0xCB,0x06,...]`, 3 zones : R a zone*6+3, G a +5, B a +7, octets intercalaires constants (0x16,0xDC,0x17,0x9A...). Retour materiel : INCONNU (fonction vide).
- Sidetone Void Pro : feature(64) `[0xFF,0x0B,0x00,0xFF,0x04,0x0E,0xFF,0x05,0x01,0x04,0x00,niveau+200]`.

### 2. NZXT (VID 0x1E71) - Nzxt/

| Appareil | PID | Interface | Source |
|---|---|---|---|
| Smart Device V1 | 0x1714 | 0, usage 0x0001, page 0xFF00 | NZXT_Smart_Device_V1_Controller.js |
| Smart Device V2 / Hue 2 / RGB&Fan / Control Hub | 0x2001,2002,2006,2009,200D-2012,2019,201F-2022 | -1 ou 0 | ..._V2_Controller.js |
| Kraken X3 | 0x2007, 0x2014 | 0 | Nzxt_Kraken_X3_AIO.js |
| Kraken X2/M2 | 0x170E, 0x1715 | non lu | Nzxt_Kraken_X2_M2_AIO.js |
| Kraken Z3 | 0x3008 | 1 | NZXT_Kraken_Z3_AIO.js |
| Kraken Elite / Elite v2 | 0x300E,0x300C / 0x3012,0x3014 | 1 | NZXT_Kraken_Elite*.js |
| Cartes meres N5/N7 | 0x2005,200A,200B,200C,2016,2017,201B,201D | 0 | NZXT_Motherboard.js |

- V1 : 1 canal 40 LED, GRB. write(65) `[0x02,0x4B,0,0,0,+20 LED]` puis write(65) `[0x03,+20 LED]`. Pas de commit. Fans : non implemente (TODO).
- V2 / X3 / Z3 / Motherboard (rapports 64) : donnees `[0x22,0x10|n°paquet,1<<canal,0x00,+GRB]`, 20 LED/paquet (V2 : jusqu'a 120 LED/canal = 6 paquets ; X3 : 60 octets/paquet, 2 paquets). Commit apres le dernier paquet : `[0x22,0xA0,1<<canal,0,0x01,0,0,0x28,0,0,0x80,0,0x32,0,0,0x01]` (0x28 = 40 LED en dur).
- V2 : init `[0x10,0x01]` (fw : reponse 0x11/0x01, version octets 0x11-0x13) puis `[0x20,0x03]` (reponse 0x21/0x03, nb canaux = octet 14). 40 LED/canal, max 4 canaux = 160 LED. Lecture non bloquante a chaque Render.
- Motherboard : N7 Z390 = 3 canaux NZXT ; Z490/B550/Z590/Z690/N5 = 2 NZXT + 1 ARGB + 1 header 12V ; B650E/Z790 = 4 NZXT + 2 ARGB (40 LED/canal). Header 12V : write(64) `[0x2A,0x04,0x08,0x08,0x00,0x32,0,c1,c2,c3]`. Pas de fans.
- Kraken X3 : canal 1 (anneau ext., 40 LED) + canal pompe (`1<<1`, 8 LED, ordre G,R,B) + logo write(64) `[0x2A,0x04,0x04,0x04,0x00,0x32,0x00,G,R,B]` (PID 0x2014 ; 0x21 initial pour 0x2007, commente), octets [56..59]=01 00 01 03. Limite device 80.
- Elite v2 (rapport 512) : pompe 24 LED `[0x26,0x14,0x01,0x01,+GRB]`, canal `[0x26,0x14,0x02,0x02,+GRB]`, commit `[0x26,0x06,0x01,0,0x01,0,0,0x18,0,0,0x80,0,0x32,0,0,0x01]`. Elite v1 : aucune trame LED relevee (LCD + fans) -> LED INCONNU.
- LCD (pour info) : write(64) `[0x36,0x01,...]`, bulk_transfer EP 0x02 (en-tete `12 FA 01 E8 AB CD EF 98 76 54 32 10 <mode> 00 00 00 + len32 LE`, puis image Q565), `[0x36,0x02]`. Z3 : image brute 409600 octets.
- Fans V2 : rapport `0x67 0x02` : mode @16+i (0 absent, 1 DC, 2 PWM), RPM u16 LE @24+2i, duty @40+i, i=0..2. Config poll : `[0x60,0x03]` puis `[0x60,0x02,0x01,0xE8,n,0x01,0xE8,n]` (n = 3*secondes). Consigne `[0x62,0x01,1<<fan,...]` pct (0-100) a l'octet 3+fan, 1 ventilo / 1000 ms.
- Fans/pompe Kraken : lecture `[0x74,0x01]` -> `0x75 0x01` (Z3/Elite) ou `0x75 0x02` (X3) : temp = @15 + @16/10, pompe RPM u16 LE @17-18, duty @19, ventilo RPM @23-24, duty @25. Ecriture : pompe `[0x72,0x01,0,0,+40 octets pct]` (plancher 25 %), ventilos `[0x72,0x02,0x01,0x01,+40 pct]` (0 % ok). Elite : prefixes 0x01 0x00 / 0x01 0x01 selon firmware. Poll 3000 ms, burst 50 % a l'init.
- Retour materiel : aucun envoi ; Shutdown = noir/couleur d'arret.

### 3. Lian Li (VID 0x0CF2) - Lian Li/

| Modele | PID | Canaux x LED | Transport |
|---|---|---|---|
| Uni Fan 1re gen | 0x7750 | 4 x 64 | control_transfer |
| Uni Fan SL | 0xA100, 0xA106 | 4 x 64 | control_transfer |
| Uni Fan AL | 0xA101 | 4 x 80 | control_transfer |
| SL Infinity | 0xA102 | 4 x 80 | HID report 0x06 |
| SL V2 | 0xA103, 0xA105 | 4 x 96 | HID 353 |
| AL V2 | 0xA104 | 4 x 120 | HID 353 |
| Galahad II LCD | VID 0x0416 PID 0x7395 | 1 x 50 | HID iface 1 |
| Uni Fan TL LCD | VID 0x04FC PID 0x7393 | LCD seul | iface 0, usage 0x0001, page 0xFF06 |

- SL V2 / AL V2 (le plus courant, interface 1) : couleur write(353) `[0xE0,id,+RBG]`, id 0x30..0x33 (SL V2, canal 0-3) ; AL V2 : interieur 0x30,32,34,36 (8 LED/ventilo), exterieur 0x31,33,35,37 (12 LED/ventilo), 6 ventilos/canal. Commit feature(353) `[0xE0,id,0x01,0x02]` : id 0x10..0x13 (SL V2) ; AL V2 int. 0x10,12,14,16 / ext. 0x11,13,15,17. Ordre RBG. Pause 5 ms par paquet. Cadence limitee au-dela d'un canal (message du plugin).
- Init : feature `[0xE0,0x10,0x63,0x00,0x01,0x02,0x03,0x08]` (+get), puis par ventilo `[0xE0,0x10,0x60,n,0x06]` + `[0xE0,0x30+2n]`. Passthrough carte mere `[0xE0,0x10,0x61,v]` puis `[0xE0,0x50]`.
- Mode ventilo `[0xE0,0x10,0x62,x]` : x = 0x10,0x20,0x40,0x80 par canal (logiciel) ou 0x11,0x22,0x44,0x88 (PWM carte mere), puis `[0xE0,0x50]`.
- Ventilo : consigne `[0xE0,0x20+canal,0x00,pct]` (0 invalide -> 1 ; plage 1-100). RPM : control_transfer IN (0xC0, req 0x81, wIndex 0xD800/D802/D804/D806, 65 octets), u16 LE octets 0-1. Poll 3000 ms, burst 50 %. Permissions L-Connect "fans"/"lighting" a respecter.
- Anciens (Uni Fan, SL 1.2, AL 1.1) : tout en control_transfer OUT (0x40, req 0x80, wValue 0, wIndex = adresse), pause 1 ms. Adresses : commandes 0xE020 ; donnees canal0 0xE300, canal1 0xE3C0, canal2 0xE480 ; commit canal0 0xE020, canal1 0xE030, canal2 0xE040 ; consigne ventilo 0xD8A0/D8A2/D8A4 (commit 0xD890/91...), RPM 0xD800/D802... Donnees canal 192 octets RBG ; commit 16 octets `[0,0x01,0xFF,0,...,0x01]`. Consigne RPM u16 LE (SL : 1900 x pct, plancher 800). Retour PWM carte mere : `[0,0x31,(0x10<<canal)+0x0F,...]`.
- SL Infinity : write `[0xE0,0x30|(2*canal)(+1),...]`, commit `[0xE0,0x10|(2*canal)(+1),0x01,0x02]`, ventilo `[0xE0,0x20|canal,0,max(pct,20)]`, mode `[0xE0,0x10,0x62,(1<<(canal+4))|(pwm<<canal)]`, RPM par input_report `[0xE0,0x50]` (65 octets).
- Galahad II : petit paquet 64 `[0x01,cmd,0,seqHi,seqLo,len,+data]`, grand 1024 `[0x02,cmd,len32 BE,seq24 BE,len16 BE,+data]`. cmd 0x81 info (RPM ventilo @6-7 BE, pompe @8-9 BE, temp valide si @10=1 : @11+@12/10), 0x8A pompe `[pwm?,pct]`, 0x8B ventilo `[0,pct]`, 0x83/0x85 couleurs pompe/ventilos, 0x14 per-LED, 0x0E image JPEG par blocs 1013 octets. 50 LED, RGB.

### 4. CoolerMaster, Thermaltake, Hyte

#### CoolerMaster (VID 0x2516) - CoolerMaster/
- ARGB Controller PID 0x1011 (iface 0) : 4 canaux x 48 LED (limite 196), write(65). Init `[0,0x80,0x17,0x02]`, `[0,0x80,0x01,0x02]` x4, `[0,0x80,0x01,0x04,0x01]` (header RGB), `[0,0x80,0x01,0x01]`. Couleur RGB entrelace : `[0,0x00,0x10,0x02,canal,0x30,+59]`, `[0,0x01,+63]`, `[0,0x82,+62]`, commit `[0,0x80,0x01,0x03]`. Header 12V `[0,0x80,0x04,0x05,0x10,0x02,0xFF,c1,c2,c3]`. Pause 1 ms entre canaux. Pas de fans.
- Small A-RGB PID 0x1000 : 1 canal 48 LED, init `[0,0x80,0x01,0x01,0x02,0x02]`, paquets d'index 0x00/0x01..0x84, pause 10 ms (detail `sendColors`).
- Gen2 LED A1 PID 0x0173/0x01C9/0x01CB (iface 1) : 3 canaux, GEN1 60 LED / GEN2 80 LED (reglage utilisateur) ; init `[0,0x80,0x01,0x02,0x01]`, `[0,0,0x09,0xFF,0xFF,0x90]`, `[0,1,0x09]`, `[0,0x82,0x09]` ; couleur `[0,0x00,0x09,1<<canal,0,0x90]` puis `[0,0x01|0x82,0x09]`, pause 4 ms. Pas de fans. Retour materiel INCONNU.

#### Thermaltake (VID 0x264A) - Thermal Take/
- Riing Plus PID 0x1FA4-0x1FA7 ; Riing/Ring Controller PID 0x2135, 2136, 2137, 2143, 2144 (iface -1/0). 5 canaux x 54 LED (limite 270), GRB. Init `[0,0xFE,0x33]` (write 193 + read 193) = mode logiciel. Chaque ecriture est suivie d'une lecture.
- Couleur Riing Plus : `[0,0x32,0x52,canal+1,0x18,+GRB]` (1 paquet). Ring : 2 paquets `[0,0x32,0x52,canal+1,0x24,0x03,(1|2),0x00,+57 octets]`.
- Ventilo : consigne `[0,0x32,0x51,canal+1,0x01,pct]` ; RPM `[0,0x33,0x51,canal+1]` -> u16 LE octets 6-7. 5 ventilos, poll 3000 ms, burst 75 %. Shutdown : pause 2000 ms. Retour materiel INCONNU.
- Level 20 Desk PID 0x07D1 : iface 0, usage 0x0001, page 0xFF00 ; write(839) `[0,0xFE,0x33]` puis trame couleur write(839) (en-tete dans le fichier). LEDBOX/E1/DPSG non lus.

#### Hyte (VID 0x3402) - Hyte/
- Aucun controleur fans/AIO. CNVS mousepad PID 0x0B00/0x0B01 en port SERIE (trames `FF DC 08`, `FF DC 05 00`) ; Keeb TKL PID 0x0300 iface 2, write(65) + feature(9) `[0,0x04,0xF0/0xF1,...]`. Details INCONNUS.

### Non couvert
Legacy Mouse (trame propre), Vanguard 96, RAM Corsair, LCD Corsair, HS80/headsets Bragi, claviers/souris CoolerMaster, LEDBOX Thermaltake, tables de positions/maps LED, table CRC-8 complete (poly deduit des premieres valeurs), tailles reelles de rapport Bragi/Commander Core (lues du descripteur HID au runtime).

### 6.3 Cartes mères, ARGB tiers, claviers génériques
Base : `...\Plugins\<fabricant>\`. Faits uniquement, aucun code copie. INCONNU = non confirme dans le plugin.
Convention : `W(n)` = device.write de n octets, `SR(n)` = send_report (feature report, 1er octet = report ID), `RD` = read/get_report. Pour W, l'octet 0x00 initial = pas de report ID.
Aucun plugin ne documente un "retour au mode materiel" explicite sauf mention : a la fermeture ils envoient seulement la couleur d'arret (noire en veille).

### 1. ASUS Aura USB (VID 0x0B05)
Sources : Asus/Motherboard Controllers/Asus_Motherboard_Controller.js (+ _X870E.js), Asus/ARGB Controllers/Asus_Motherboard_ARGB_Controller.js.
| Item | Fait |
|---|---|
| PID carte mere | 0x18F3, 0x19AF, 0x1939, 0x1BED (X870E : 0x1AA6) ; ARGB seul : 0x18A3, 0x18A5, 0x1867, 0x1872 |
| Endpoint | interface 2 (carte mere) ; ARGB seul : interface 2 ou 0. usage_page/usage INCONNU |
| Trame | W(65) toujours, octet0 = 0xEC (pas de report ID separe) |
| Firmware | `EC 82` puis RD(65) : rep[1]=0x02, nom ASCII rep[2..17] (ex. "AULA3-AR32-0207") |
| Table config | `EC B0` RD(65) : rep[1]=0x30 ; rep[6]=nb canaux ARGB ; rep[31]=nb LEDs carte (12V inclus) ; rep[33]=nb headers 12V. Quirks : rep[6]==4 => 3 ARGB + panneau Polymo ; rep[33]==3 => 1 header 12V et LEDs carte -1. Tables d'override par nom de carte / ID firmware (valeurs de la table incoherentes selon BIOS) |
| Mode direct | `EC 35 <canal> 00 00 FF` par canal (0 = carte, 1..N = ARGB, +1 si Polymo). Avant : `EC 52 53 00 01` (coupe gen2). ARGB seul : `EC 3B <canal> 00 FF` pour canal 0..N |
| Trame couleur | `EC 40 <canal, OU 0x80 sur le dernier paquet> <index_led_debut> <nb_leds> RGB...` ; 20 LEDs max/paquet ; 120 LEDs max/canal |
| Commit | bit 0x80 sur le canal du dernier paquet (ARGB seul : paquet separe `EC 40 <0x80|canal>` sans donnees) |
| Canal carte | index direct 4 (pas 0) : LEDs carte puis headers 12V a la suite dans le meme flux. Polymo (ROG) : 19 LEDs sur canal 0 ; X870E : 25 LEDs sur canal 3 |
| Ordre couleurs | defaut RGB ; 6 permutations reglables par zone (carte, 12V, ARGB) |
| Limites | 120 LEDs/canal, 20/paquet ; ARGB seul : 2 canaux utilises (config[6] ignore), 5 declares |
| Retour materiel | INCONNU (arret = couleur noire / shutdownColor) |

#### ASUS Aura Terminal (Asus_Aura_Terminal_Controller.js)
VID 0x0B05 PID 0x1889, interface -1 ou 0. Init : `EC 3B i 00 FF` pour i=0..4 (W65). 4 canaux x 90 LEDs (limite totale 210). Meme trame `EC 40 canal debut nb RGB` (20 LEDs/paquet), commit `EC 40 <0x80|canal>`. Logo = canal 4 : `EC 40 04 00 01 R G B` + commit `EC 40 84`. Ordre defaut RGB par canal.

#### ASUS Strix LC / Ryujin / Multidevice
- Strix LC (ASUS_ROG_Strix_LC.js) : PID 0x879E (LC), 0x1B29 (LC III). Endpoint : interface -1/0 usage 0x0001, OU interface 2 usage 0x00A1 page 0xFF72. Trame `EC 40 <canal|0x80 si apply> debut nb RGB` (20/paquet, 4 LEDs). Init LC : `EC 3B 00 FF` ; LC III : `EC 35 00 00 00 FF 00 FE 01 ...` (22 octets, luminosite max en fin) puis `EC 3B 00 E3 FF`. Bit apply 0x80 uniquement LC III.
- Ryujin (ASUS_Ryujin_AIO_Controller.js) : PID 0x18AE (I), 0x1988 (II), 0x1AA2 (III). Endpoint usage 0x00A1 page 0xFF72, interface 0 (I) / 1 (II, III). Mode logiciel `EC 3B 00 E3 FF`. Ryujin I : un paquet `EC 40 80 00 05 RGB x5`. Ryujin III : LCD 320x240 RGB (230400 octets) par bulk endpoint 0x01 en blocs de 4096 (R/B echanges) puis `EC 7F 03 00 84 03 00 00` ; init LCD : `EC D0`, `EC 51 20 00 00`, `EC 7F ...`. Pompe : `EC AA` -> reponse 0x2A (RPM = octet[4]*100). Firmware : `EC 82`.
- ASUS_Multidevice_Controller.js (PID 0x1866, 0x19B6, boitiers/peripheriques "Aura device") : octet d'en-tete par modele (ex. 0x5E), init = chaine ASCII "ASUS Tech.Inc." + `05 20 31 00 10` + `BC D0` (64 octets, SR ou W selon modele). Protocoles par modele : perledv1 (`hdr BC D0 01 02 00 <n paquet> <nb> 00 RGB`, 16 LEDs/paquet), perledv2/zone (`hdr BC 00 01 <zone> .. debut nb 00 RGB`, 65 octets), single (`hdr B3 00 00 R G B FF`). Endpoints par modele (ex. interface 0 usage 0x0076 page 0xFF31). Detail par modele INCONNU.
- AsusSMBusMotherboardController.js : non couvert (SMBus, pas USB HID).

### 2. Gigabyte RGB Fusion 2 (ITE) : GigaByte/Gigabyte_Motherboard_Controller.js, _0x5711.js
| Item | Fait |
|---|---|
| IDs | VID 0x048D ; PID 0x5702, 0x8297 ; fichier _0x5711 : PID 0x5711 |
| Endpoint | usage 0x00CC, interface -1/0/1 (5702/8297) ; interface 1 seule (5711). usage_page INCONNU. Report ID 0xCC, tout en feature report SR(64) |
| Config | `CC 60` puis get_report(64) : rep[1]=produit, rep[2]=n device, rep[4..7]=firmware, rep[8]=enums LEDs DLED (nibble bas hdr1, haut hdr2), rep[12..39]=chaine description (contient "IT5702-Z790MA" => mode panneau IO) |
| Mode direct ARGB | `CC 32 03` (hdr 1+2) ; 5711 : `CC 32 0B` ; 5711 init zones : SR `CC <id>` pour 0x20..0x27 et 0x90..0x92 puis commit |
| Taille ARGB | `CC 34 <masque>` : enum par header (<=32:0, <=64:1, <=256:2, <=512:3, <=1024:4), hdr1 nibble bas / hdr2 nibble haut ; 5711 : 2 octets (masque, masque2). Renvoye quand le nb de LEDs change |
| Canaux ARGB | ids 0x58 (hdr1), 0x59 (hdr2), 0x62 (hdr3, 5711) |
| Trame ARGB | `CC <id canal> <off_lo> <off_hi> <nb_octets> RGB...` ; offset = LED_debut*3 (16 bits LE), nb = 3 x LEDs ; 19 LEDs max/paquet ; 120 LEDs/canal ; limite 240 (360 en 5711). Pas de commit pour l'ARGB direct |
| Zones carte | `CC <zone 0x20..0x27 ou 0x91/0x92> <2^(zone-0x20)> 00 00 ... [0x0B]=1 (statique) [0x0C]=0x5A (lum. max) [0x0D]=0 ; RGB des l'octet 0x0E`. 5711 : octet3 = 0x02 (zone 0x91) / 0x04 (0x92). Commit `CC 28 FF` (5711 : `CC 28 FF 07`), un commit par zone |
| Ordre | carte : BGR defaut, 12V : RGB, ARGB : GRB. Noms de zones par modele de carte (WMI), table "Auto" sinon |
| Z790 AORUS Master | panneau IO 12 LEDs envoye comme canal ARGB 0x58 ; zones carte forcees a noir |
| Retour materiel | INCONNU |

### 3. MSI Mystic Light (Msi/)
Commun : VID 0x1462 (162 octets et 185 octets) ; variante `MSI_185Byte_Controller_0x0DB0.js` VID 0x0DB0. Interface 2 ou 0 ; usage_page/usage INCONNU. Processus en conflit : Dragon Center, LedKeeper, LightKeeper.
#### 162 octets (162 Byte Motherboard Controller/, PID 0x7B17, 0x7B18, 0x7B85, 0x1720, 0xB926)
- Feature report ID 0x52, 162 octets (get_report(200) : taille lue doit etre 162 sinon aucun envoi). Un seul SR(162) par frame + pause 30 ms : buffer constant initial modifie en memoire (blocs "enable=1, couleur, option 0x28, sync 0x80").
- Blocs de zone de 10 octets : [0]=mode/enable, R,G,B en +1..+3 et +5..+7 (couleur dupliquee). Offsets : JRGB1 0x01, Rainbow1 (ARGB) 0x0B, Corsair 0x15, Mainboard1..10 = 0x29 + 0x0A*n (0x29..0x83), temp 0x8D, JRGB2 0x97, save flag 0xA1. Une couleur par zone (pas de per-LED, ARGB = 1 couleur).
- Init : lecture de taille uniquement. Retour materiel INCONNU.
#### 185 octets (185 Byte Motherboard Controller/, nombreux PID 0x7Bxx/0x7Cxx, table par PID)
- Zones : feature 0x52, 185 octets, meme blocs de 10 octets. Offsets : JRGB1 0x01, JPipe1 0x0B, JPipe2 0x15, Corsair 0x35, Corsair backup 0x40, Mainboard1 0x4A, +0x0A chacun jusqu'a Mainboard10 0xA4. Rainbow1/2 : INCONNU (constantes non lues).
- Detection per-LED : SR([0x53], 725), retour >0 => per-LED. Activation per-LED : SR 0x52 185 octets avec blocs JRainbowN dont un 11e octet = nb de LEDs du header (>=1), flags 0x25/0x29/0x82 ; deux variantes "efficiency" / "high capacity" selon total LEDs.
- Trame per-LED : feature **ID 0x53, 725 octets** : `53 25 <type> <index> 00 RGB...` ; type 0x04 = header ARGB (index 0..2), 0x05 = header Corsair, 0x06 = LEDs carte (zones onboard/JPipe puis 12V concatenees). Ordre RGB. Pause 1 ms entre paquets. Limite 360 LEDs (480 pour 0x0DB0), 120 LEDs en pulse.
- Cartes recentes (V3, noms MS-7Dxx/7Exx via WMI) : coupe gen2 par header avec W(64) `01 84 00 00 00 00 <hdr> 00` (+ pause 1000 ms) ; verif EZ LED du BIOS : W(64) `01 BA CC..` puis lecture, octet[6]==1 sinon notif. Gen2 : W(64) `01 82|80 00 00 00 00 <port>` + read, get_report(0x80, 242) : details INCONNUS.
- Le plugin avertit qu'un mauvais paquet peut briquer le controleur RGB.
#### Autres MSI
- MSI_Keyboard_Controller.js (Vigor GK41/GK50/GK71) : VID 0x0DB0, PID 0x0B41, 0x0B5B, 0x1958, 0x0B7A, interface 1. Init W(65) `00 41 80`, `00 56 20 01`. Trame W(65) `00 56 21 01 00 FF.. (en-tete fixe 51 octets) + RGB par LED + 13 octets de fin` ; apply `00 51 28 00 00 01`.
- MSI_Monitor_Controller.js : VID 0x1462 PID 0x3FA4, interface 0/-1 ; identification W(64) `01 35 38 30 30 31 34 30 0D`, `01 B0` ; couleurs SR `53 25 03 00 00 RGB` (variante MEG). MSI_MAG_Monitor.js : VID 0x1038 PID 0x1128, interface 0 usage 0x0001 page 0xFFC0, SR(525) `00 0C 00 28 00 + 40*4 octets` puis `00 0C 00 0C 00 + reste` (4 octets/LED, role du 4e INCONNU).

### 4. Controleurs ARGB tiers
| Famille (fichier) | VID / PID | Endpoint | Capacite | Ordre |
|---|---|---|---|---|
| Nollie8 v2, Nollie1 | 0x16D2 / 0x1F01 (8 ch), 0x1F11 (1 ch) | interface 0 | 8 x 126 ; 1 x 630 | GRB |
| PrismRGB 8 | 0x16D5 / 0x1F01 | interface 0 | 8 x 126 (meme protocole que Nollie8) | GRB |
| Nollie16v3 (+Nos2.0) | 0x3061 / 0x4716 ; Nos2.0 : 0x16D5 / 0x4716, 0x2A16 | interface 0 | 16 x 256 | GRB |
| Nollie32 (+Nos2.0) | 0x3061 / 0x4714 ; Nos2.0 : 0x16D5 / 0x4714, 0x2A32 | interface 0 | 16 + EXT1-4 = 20 x 256, + Strimer | GRB |
| Nollie CDC Nos2.1 | 0x16D5 / 0x2A01 (1 x 630), 0x2A08 (8 x 126) | serie CDC, interface 1, 115200 8N1 | idem | 8 ch : GRB ; 1 ch : reglable |
| Zhi Yan DRGB V3/CORE | 0x2023 / 0x1226, 0x1221, 0x1209 | interface 0/2/-1 | 16, 25 ou 8 canaux, 80 LEDs defaut, 1800 total | RGB |
| DRGB ELITE / JPU / NEEDMAX / YICO | 0x2023 / ELITE (liste), 0x1412, 0x1410 ; 0x1368 / 0x6077-79 | idem | 1024 / 1320 total (NEEDMAX, YICO 1024) | RGB |
| DRGB V4F, YICO_16 | 0x2486 / liste, 0x1616 | interface 0/-1 | 256/canal, 8192 (V4F) / 4096 | RGB |
| Airgoo | 0x2486 / 0x3F28, 0x3F16, 0x3F08, 0x3216, 0x3204 | interface 0/-1 | 1024/canal, 8192 total | RGB |
| Skydimo | 0x1A86 / 0xE316 | interface 0, usage 0x0001 page 0xFF00 | 1 canal : 144 LEDs (SK0100) ou 93, selon nom produit USB | GRB |
| PrismRGB Mini / S | 0x16D0 / 0x1407 ; 0x16D0 / 0x1294 | interface 2 | Mini 128 LEDs ; S = Strimer 24 br. + 2/3x8 br. | RGB |

#### Nollie8 / Nollie1 / Prism8 (W(65), report ID 0x00)
- Init : version `FC 01` (rep[2]), compte LEDs/canal `FC 03` (8 x 16 bits big-endian), reecriture `FE 03 <lo hi> x8`. Tension SATA `FC 1A` toutes les 150 frames (version >= 2).
- Couleur : `00 <idx = paquet + canal*6> + 63 octets GRB` (21 LEDs/paquet) ; commit en fin de frame `00 FF`. Effet materiel : `00 FE 02 00 R G B 64 0A 00 01` (statique) ; retour materiel a l'arret `00 FE 01 00`. Nollie1 : aussi `00 FE 1A 00|01`.
- CDC : meme bloc de 63 octets precede d'1 octet d'index (pas de report ID) ; commit = 64 octets avec [0]=0xFF ; pause ~10 ms moins 0,6 ms/paquet.
#### Nollie16v3 / Nollie32 (W(1024), report ID 0x00)
- V1 : `00 <canal materiel> <num paquet> <nb_hi> <nb_lo> GRB...`, jusqu'a 256 LEDs/paquet ; correspondance canaux logiques -> index `[5,4,3,2,1,0,15,14,26..31,8,9,13,12,11,10]` ; Strimer ATX = 6 paquets de 20 LEDs (ids 16-19, 6, 7), GPU = ids 20-25 (27 LEDs/paquet). Pas de commit separe.
- V2 (option) : canaux regroupes par blocs <=340 LEDs : `00 40 <canal_debut> <canal_fin> <marqueur 0/1/2> GRB...` ; comptes LEDs par canal via `00 88 <32 x 2 octets>` quand ils changent. Arret : W(513) `00 FF`. Sauvegarde LED materiel : W(513) `00 80 <mos> <03 statique|01 Nollie> R G B`.
#### Zhi Yan DRGB (V3 / ELITE / JPU / NEEDMAX / YICO)
- Firmware : W(65) `00 02` puis read(6) ; fw = octets 2..5.
- En-tete W(65) : `00 60 <AA = compression 4 bits|BB> <2 octets (hi,lo) par canal = nb LEDs>` (canaux 18/24 et 20 : tables de LEDs predefinies) ; donnees en paquets `00 <id 100,101..> + 63 octets` (21 LEDs), id du dernier paquet decale de +100 ; compression optionnelle : 4 bits/composante.
- V4F / YICO_16 / Airgoo : W(1025) `00 <id 100..> <nb paquets> <hi> <lo> + 1020 octets` ; table des comptes LEDs (2 octets/canal, octet 71 forcee a 0 si < 36 canaux) en tete du flux, RGB ensuite. Pas de checksum. Airgoo : effets materiels W(65) `00 30 ...`, sauvegarde `00 30 34 F0 00 00`.
#### PrismRGB Mini
Firmware : W(65) `00 00 00 00 CC`, read(4) : octets 1..3 comparees a la version du plugin. Trame : `00 <n paquet> <nb paquets> 00 AA + RGB` (20 LEDs/paquet, compression 4 bits optionnelle) ; reglage materiel `00 00 00 00 BB <hwl_enable, return, delai, mode 1-4, vitesse, lum., R,G,B, status_led, compression>`. Prism S : flux concatene decoupe en W(65) `00 + 64 octets` ; sauvegarde `00 FE 01 R G B <mos>`.
#### Skydimo
Init (mode logiciel) : W(64) `01 16 00 00 01 ... [63]=0xAF`. Trame W(64) : `01 <segment = 0,20,40..> 00 + 60 octets GRB (20 LEDs)`, [63] = CRC-8 (poly 0x07, init 0, octets 0..62). Commit : `01 FF FF <b3> .. [60]=<b60>` avec (b3, b60) selon modele (SK0100 : 0xC8, 0xEE) ; segment max par modele (SK0100 : 180).

### 5. Claviers / peripheriques generiques
Pas de plugin QMK/VIA generique dans l'arbre : EVision signale seulement qu'un modele VIA doit etre flashe en QMK (layout "QMK" ignore).
| Famille | VID / PID | Endpoint | Init (direct) | Trame couleur | Commit / quirks |
|---|---|---|---|---|---|
| Nuvoton (Nuvoton_Controller.js) | 0x0416 / 0xB23C, 0xC345, 0x7372... | interface 2, usage 0x0091 page 0xFF1B coll. 0 | W(64) `01 07 00 00 00 0E 00 04 03 FF`, `01 17 00 00 00 01 01`, `01 08 00 00 00 0D 02 03 03 FF 00.. 01` | W(65) `01 0F <zone> 00 <n paquet> <nb octets> RGB` (18 LEDs/paquet) | 1 seule zone : 2 paquets vides `01 0F 01 00 00 36` et `01 0F 01 00 01 2D` ; AK820 MAX : `01 0F 00 00 07 12`. Modele/fw `01 0D` (ASCII). Variante TL (ventilateurs) : `01 <opcode> 00 00 00 <len> payload`, 0xB0 couleur groupe (RGB), 0xAA ventilateur |
| Nuvoton Keyboard2 | 0x0416 | interface 1, usage 0 page 0x0001 coll. 6 | W(32) `06 BE 14 00 01 00 0E` + read | W(32) `06 BE 19 00 01 01 0E <5 index LED> <5 x R G B 00>` | pause 2 ms/paquet ; modele `06 BE 02`, fw `06 BE 03` ; commit/pre-commandes `06 BE 15` optionnels |
| Xiaohua | 0x2E88 / 0xB30A | interface 0, usage 0x0091 page 0xFF1B coll. 6 | W(64) `06 17 00 00 00 01 01`, `06 08 00 00 00 0D 02 03 03 FF ... 01` | W(65) `06 0F <zone> 00 <n> <nb octets> RGB` (18 LEDs) | meme quirk 2 paquets vides ; modele `06 0D` |
| Sinowealth 1 | 0x258A / 0x010C | interface 1, usage 0x0001 page 0xFF00 coll. 6 | aucune | SR(520) `06 08 00 00 01 00 7A 01` + RGB (offset LED*3) | pause 1 ms ; modele `06 82 01 00 01 00 06` (octet 13) |
| Sinowealth 2 | 0x258A / 0x0049 | interface 1, page 0xFF00, coll. 5 (modele) puis 8 | set_endpoint coll. 5 : SR(6) `05 05 81 00 00 00`, get_report(8) ; puis coll. 8 | SR(382) `08 0A 7A 01` + RGB | pause 1 ms |
| Royal Kludge | 0x258A / PID RK61, RK84, RK98... | interface 1, usage 0x0001 page 0xFF00 coll. 5 | aucune | SR(65) `0A 07 01 06` + 61 octets, puis `0A 07 <zone 2..7>` + 62 octets (6 zones) | pause 3 ms ; option echange R/G |
| Sonix | 0x05AC (interface 2) ou 0x0C45 (interface 2/3) ; usage 0x0001 page 0xFF13 | | SR(65) `00 04 20 00 00 00 00 00 00 08` + get_report | flux `[index R G B]` 4 octets/LED coupe en SR(65) `00 + 64 octets` | pause 2 ms ; fin : SR `00`, SR `00 04 02`, get_report ; certains modes "None" non supportes |
| EVision | 0x320F, interface 1 ou 2, usage 0x0092 page 0xFF1C coll. 4 | | W(64) `04 8C 00 0B 30 50 01` | W(64) `04 <chk_lo> <chk_hi> 12 <56|48> <off_lo> <off_hi> 00 + RGB` (mono = 1 octet/LED) | checksum 16 bits = somme des donnees + 74 + index*taille (index<5) ou + 99 + (index-5)*taille ; pause 1 ms (3 ms lowFPS) |

### 6.4 SMBus/I2C (RAM, GPU, carte mère ASUS)
Source : `...\Signal-x64\Plugins\<fabricant>\*.js` (lecture seule, aucun code copie). Adresses = I2C 7 bits (valeur passee a `WriteQuick`). `INCONNU` = non confirme dans les sources.
**Avertissement global (en-tete ENE_RAM.js)** : "modifier un plugin SMBUS est DANGEREUX et peut DETRUIRE des peripheriques". Tout write SMBus sur une adresse non identifiee est un risque (SPD/EEPROM, PMIC, regulateur VRM).

### 0. Conventions du moteur
- `Scan(bus)` : bus "libre" (adresse passee a chaque appel) ; `Initialize/Render` : bus "fixe" (adresse deja liee, signatures sans adresse).
- Filtres de bus : RAM = `IsSystemBus()` (AMD/Intel) ; carte mere ASUS = `IsAMDBus()||IsNuvotonBus()` ; GPU NVIDIA = `IsNvidiaBus()` (I2C via pilote, aucun WriteQuick : "No Quick Write test on Nvidia") ; ASRock = `IsAMDGpuBus()`.
- GPU : identification par (vendor, subvendor, device, subdevice) -> adresse I2C fixe par famille (table deja extraite ailleurs). `Scan` renvoie l'adresse sans sondage, sauf Gigabyte Gen1/Gen2/Gen4-"Gen2" (lecture test 0xAB) et ASRock.
- Primitives : WriteQuick, ReadByte(addr,reg), WriteByte, WriteWord, WriteBlock(reg,len,data), WriteBlockWithoutRegister (bloc I2C brut, 1er octet = commande), ReadBlockBytes, TransactionLock/Unlock.
- Tous les plugins couleur : `Shutdown` envoie noir si veille, sinon `shutdownColor` (ASUS GPU/carte mere/Legacy : retour au mode materiel).

### 1. RAM

#### 1.1 ENE (Aura / XPG / T-Force) : `Asus\ENE_RAM.js` (Scan, ENERam, ENEInterfaceFree/Fixed)
- **Registre 16 bits** : ecrire le mot (octets du registre inverses) a la commande 0x00 ; puis valeur a 0x01 (octet), bloc a 0x03, lecture a 0x81. Lecture mot = 2 lectures 0x81, pause 30 ms.
- Adresses candidates AURA : 0x39-0x40, 0x4F, 0x66, 0x67, 0x70-0x76. XTREEM (T-Force Xtreem) : 0x39-0x3C, 0x67, 0x70-0x73, 0x75, 0x76. Adresse de depart a remapper : 0x77 seule (0x74 retiree : "explose sur cartes Gigabyte").
- **Remap** (`CheckForFreeAddresses`, 8 iterations max, tant que 0x77 repond) : cherche une adresse libre (WriteQuick<0), ecrit canal (0..7) puis `adresse<<1` : AURA regs 0x80F8 / 0x80F9 ; XTREEM regs 0xE0F8 / 0xE0F9 ; pause 1 ms. Remap non persistant : le premier logiciel qui touche la barrette le fait.
- **Detection AURA** : WriteQuick==0 (+1 ms) ; motif 0xA0-0xAF == 0x00-0x0F (>14/16 corrects) ou, variante TeamGroup, 0x90-0x9F == 0x10-0x1F ; nom modele (16 car. @0x1000, NUL ignores) present dans la table ; nom fabricant (21 car. @0x1030) ne doit PAS contenir "Micron" (sinon Ballistix, ignore). Pause 30 ms entre lectures.
- Noms -> protocole : `LED-0116`, `AUMA0-E8K4-0101`, `DIMM_LED-0102`, `DIMM_LED-0103` = V1 ; `AUDA0-E6K5-0101`, `AUMA0-E6K5-0104/0105/0106` = V2 ; `AUMA0-E6K5-0107` = "GPU V2" (aucune ecriture couleur prevue pour cette valeur dans ce plugin).
- **Detection XTREEM** (`TestXTREEMRegisterValues`, testee avant AURA) : mot @0xF01C == 0x0000 ; mot @0x4000 == 0x7742 ; octets 0x9B..0x9F == 0x1B..0x1F ; octet 0xA0 == 0x20.
- **Init** : relit nom (@0x1000) + table de config (64 o. @0x1C00, pause 10 ms) ; nb LED = max(table[2], table[3]) ; retry jusqu'a 20x si nom inconnu ou nb LED >= 15. XTREEM : 15 LED fixes. Mode direct : AURA ecrit 0x01 @0x8020 ; XTREEM ecrit @0xE020 (0x00 puis 0x01 a la reg 0x01).
- **Couleurs** : ordre des canaux **R, B, G**. Blocs de 15 octets (V1 base 0x8000, V2 base 0x8100, + offset des octets deja ecrits) ; commit : 0x01 @0x802F. XTREEM : blocs de 30 octets base 0xE300, commit 0xA0 @0xE02F.
- LED/barrette : AURA = lu dans la table de config (valeur typique INCONNUE) ; XTREEM 15.
- Danger : le remap ecrit sur 0x77 ; 0x66 provoque un ecran noir en VGA (note du plugin carte mere).

#### 1.2 Corsair DDR5 : Dominator Platinum / Vengeance RGB (`Corsair\Ram\Corsair_Dominator_Ram.js`), Titanium (`Corsair_Dominator_Titanium_Ram.js`)
- Adresses : 0x18-0x1B, 0x58-0x5F. WriteQuick==0, puis lectures (5 essais) reg 0x43 (vendeur) et 0x44 (modele). **Dominator : 0x43==0x1B** ; **Titanium : 0x43==0x1A** ; modele dans {0x03, 0x04}.
- Modele = prefixe 3 car. du part number WMI (10 essais, pause 500 ms ; defaut CMH / CMP). Dominator.js : CMT Platinum RGB 12 LED, CMH Vengeance Pro SL 10, CMN Vengeance RT 10, CMG Vengeance RS 6. Titanium.js : CMP Titanium 11 LED, CMH Vengeance RGB DDR5 10.
- **Paquet direct** : octet 0 = commande (**0x0C Dominator, 0x0B Titanium**), puis RGB brut, puis **CRC-8 PEC (poly 0x07, table 256 entrees)** de tous les octets precedents. Dominator : CRC a l'index 37, ecrit en 32 o. @reg **0x31** puis 6 o. @reg **0x32**. Titanium : CRC a l'index 34, 32 o. @0x31, pause 1 ms, 3 o. @0x32 ; n'envoie que si les donnees changent. Commit separe : aucun visible.
- Conflit logiciel : `iCUE.exe`.

#### 1.3 Corsair Vengeance Pro (DDR4) `Corsair_Vengeance_Pro_Ram.js`
- Adresses 0x58-0x5F ; WriteQuick==0 ; reg 0x43 == **0x1C** ; reg 0x44 in {3, 4}. 10 LED.
- Init : lit reg 0x44 (version). Version 3 = firmware ancien (notification "mettre a jour via iCUE", debit plus bas).
- **V4** : un `WriteBlock(reg 0x31, 32)` = [0x0A, 10x(R,G,B)] + CRC-8 (meme table) a l'index 31.
- **V3** : suite de WriteByte(commande, valeur) : (0x10,0x64) et (0x28,0x49) en tete ; puis 20 paires, "commande" = 0xB0 | quartet de vert (bas puis haut), valeur = R / B ; CRC-8 cumule sur les paires, termine par WriteByte(0x28, crc). Detail exact : fonction WritePacketV3.

#### 1.4 Crucial Ballistix `Crucial\Crucial Ballistix Ram.js`
- Meme schema registre 16 bits que ENE. Adresses 0x39-0x3C, 0x20-0x23. Remap depuis **0x27** (si WriteQuick(0x27)==0) : 4 canaux, regs 0x82EE (canal) / 0x82EF (adresse<<1), commit 0xF0 @0x82F0.
- Detection : octets 0xA0-0xAF == 0x00-0x0F (stricts), fabricant @0x1025 (21 car.) **doit contenir "Micron"** (inverse d'ENE).
- Init : 0xCF @0x820F (mode statique), 0x00 @0x82EE, 0x10 @0x82EF, 0x84 @0x82F0.
- Couleurs : 8 LED, **trois plans separes** R @0x8300, G @0x8340, B @0x8380 (1 octet/LED, 1 bloc chacun), sous TransactionLock/Unlock. Correction "chaude" : R x0.56, G x0.82, B x1. Pas d'apply apres les blocs (necessite INCONNUE).

#### 1.5 Kingston Fury (Hyper Fury) `Kingston\Kingston Hyper Fury Ram.js`
- Pre-requis : WMI doit lister une barrette Kingston dont le part number commence par "KF" ou "HP". Adresses 0x60-0x63, 0x58-0x5B ; WriteQuick==0 (+30 ms).
- Garde PMIC : lire reg 0xDD ; 0x72 ou 0x78 (RT3672/RT3678, controleur PWM) -> adresse ignoree (evite de dereglementer la tension CPU).
- Identification : 0x53@0x08 (debut), lire mots aux regs 1..4 (octet bas, pause 30 ms) == "FURY" (46 55 52 59) ou 06 55 52 59 ou 07 E0 FF FF, 5 essais ; puis 0x44@0x08 (fin).
- Init : 0x53@0x08 ; 0x00@0x0B (index LED) ; 0x10@0x09 (mode direct) ; 0x01@0x0C (direction) ; 0x50@0x20 (luminosite) ; 0x44@0x08. Writes avec retry (5 ms).
- Couleurs per-LED : 12 LED ; encadre par 0x53@0x08 ... 0x44@0x08 (pauses 3 ms) ; RGB octet par octet aux regs **0x50 + i** (36 octets), seuls les octets modifies sont ecrits. Mode "haute vitesse" (couleur unique) : 0x53@0x08, 0x00@0x09, 0x01@0x30, RGB aux 0x31-0x33, 0x44@0x08 (pauses 1 ms).

#### 1.6 Patriot Viper / Viper Steel `Patriot\Patriot_Viper_RAM.js`, `Patriot_Viper_Steel_RGB_RAM.js`
- Controleur maitre sur **0x77** (WriteQuick). Barrettes 0x50-0x57 : le Scan ecrit (adresse **0x36**, reg 0, 0xFF) puis (0x37, 0, 0xFF) (adresses de selection de page SPD, pause 30 ms), lit reg 0x00 == 0x23, puis signature regs [0x40,0x41,0x61..0x68] : Viper = 85 02 4D 49 43 53 59 53 5F 44 ("MICSYS_D") ; Steel RGB = FF FF 50 44 41 31 00 00 00 00.
- Ecriture sur 0x77, 5 LED, par LED deux WriteByte : **(reg, R) puis (G, B)** (G et B passent en position "registre/valeur"). Regs LED : Viper 0x30-0x34 + commit (0x35 <- 01 00 00) apres chaque LED, pause 1 ms ; Steel 0x17-0x1B, pause 1 ms, sans commit. Init Viper : (0xFF<-FF FF FF), (0x03<-07 00 00), (0x01<-04 00 00) ; Steel : aucune.
- Danger : ecritures aux adresses 0x36/0x37 (zone SPD).

### 2. GPU
| Famille / fichier | Adresse | Detection Scan() | Protocole |
|---|---|---|---|
| ASUS Legacy (Strix Pascal/Turing) `Asus\GPUs\ASUS_Legacy_GPU.js` | 0x29 (Pascal) / 0x2A (Turing) | IDs PCI + mot @0x20/0x21 == 0x1589 | 1 zone. R/G/B = regs 0x04/0x05/0x06 ; mode 0x07 (1 static, 2 breath, 3 flash, 4 cycle) + commit 0x01 @0x0E ; poll du mode toutes 5 s (remis en static) ; arret materiel : mode cycle. Ecriture seulement si couleur changee. |
| ASUS Ampere/Lovelace (Aura GPU) `Asus_Ampere_Lovelace_GPU.js` | 0x67 (subvendor 0x1043) | IDs seuls | Registre 16 bits en bloc (0x00: hi,lo ; donnees @0x03 ; lecture @0x81). Nom @0x1000 (16 car.), table @0x1C00 (64 o.), nb LED = table[3] (borne 30). Mode direct : 0x01 @0x8020 puis 0x01 @0x80A0 (sortie : 0x00). Couleurs : preambule bloc (0x00,[0x81,0x00]) puis paquets <=31 octets (1er = longueur) a reg 0x03, ordre **R,B,G**, uniquement si changement. Effets (arret) : cmd 0x80 0x20 + [vitesse, mode] ; direction 0x80 0x24 ; modes 01 static, 02 breath, 05 rainbow, 07 comet, 0A flash&dash, 0C yoyo, 0D starry. |
| EVGA Pascal `EVGA\GPU's\EVGA_Pascal_GPU.js` | 0x49 (subvendor 0x3842) | IDs | 1 zone. "Transaction" : 0x0E <- E5,E9,F5,F9 ... fin : 0x08<-01, 0x0E<-F0, 0x0E<-E0. Mode reg 0x60 (0 off, 1 static, 0x0F rainbow, 0x22 breathing) ; R/G/B regs 0x09/0x0A/0x0B ; luminosite 0x6F=0x64. Poll 5 s ; arret : rainbow. |
| EVGA Turing `EVGA_Turing_GPU.js` | 0x49 | IDs | Idem Pascal avec R/G/B = 0x6C/0x6D/0x6E (couleur 2 : 0x70-0x73, non utilisee). |
| EVGA Ampere `EVGA_Ampere_GPU.js` | 0x2D (subvendor 0x3842) | IDs | Blocs a registre 8 bits. Firmware : 6 o. @0xB1 (version o[3].o[4].o[5]). Controle logiciel : bloc @0xB2 = [04,C6,EB,EA,15]. Etat : 10 o. @0xC0 (o[1..4] mode des 4 zones MainBar/SideLogo/BackLogo/ARGB ; o[5..8] nb LED). Mode zone : bloc @0xC0 = [09, FF.., mode a l'index zone+1] (1 = statique). Couleur : bloc @0xC1+zone = [04,FF,R,G,B]. Nb LED ARGB : bloc @0xC0 [09,FF x7,count,00]. Poll 5 s ; pause 10 ms/frame. |
| Gigabyte Gen1 "Vision" `GigaByte\Gigabyte_Gen1_GPU.js` | selon carte : 0x32, 0x47, 0x48, 0x50, 0x55, 0x62, 0x63, 0x71, 0x72 (liste de test 0x32,0x46-0x48,0x51,0x52,0x55,0x56,0x62,0x63,0x71) | bloc brut [0xAB] (longueur 8 pour 0x32/0x50/0x62/0x71, sinon 4) puis lecture 4 o. : o[0]==0xAB et o[1] in {0x10,0x11,0x12,0x14} (exception subdevice 0x405E / device 0x2486 : accepte) | 1 zone. Mode : [0x88, mode, 05, 63] (01 static, 02 breath, 04 flash, 08 dual, 11 spectrum) ; couleur : [0x40, R, G, B] ; longueur d'ecriture 8 pour 0x32/0x62, sinon 4. Pause 10 ms/frame. |
| Gigabyte Gen2 "Master" `Gigabyte_Gen2_GPU.js` | 0x32, 0x50, 0x52, 0x62, 0x64-0x66, 0x70-0x72 | test 0xAB (longueur 8) ; 0x50 exempte | Init : zones 0..7 : [0x88, mode, 06, 63, 08, zone]. Cartes avec bibliotheque (cle = subdevice) : per-LED, 4 paquets/zone [0xB0+zone*4+k, 0x01, 6 o. RGB] (max 8 LED/zone). Sinon 4 zones simples : [0x40,R,G,B,led] precede du mode zone led+1. |
| Gigabyte Gen3 "Xtreme" `Gigabyte_Gen3_GPU.js` | 0x60 | IDs seuls | Init : bloc brut longueur 32 = [0xD6]. 5 zones (ids 02-06 ; 3,3,3,3,6 LED) : [0xE6, zoneId, 01, 64, 02, 03, nbLED, RGB...]. |
| Gigabyte Gen4 `Gigabyte_Gen4_GPU.js` | 0x75 (Gen4) ; 0x71 (variante protocole "Gen2") | Gen4 : IDs seuls (checkModel@0x75 deprecie) ; "Gen2" : test 0xAB, 0x71 exempte | Gen4 : 6 zones, bloc brut longueur 0x40 = [header, R, G, B, 00, zoneId], header 12 01 01 06 0A (16 01 00 06 00 pour AERO/EAGLE), pause 9 ms, zone absente = noir. Variante Gen2 : paquets Gen2 (longueur 8 ; SINGLE_GEN2 = LED par LED). |
| MSI `Msi\GPU's\MSI_GPU.js` | 0x68 | IDs seuls | 1 zone, regs 8 bits : mode 0x22 (0x13 static ; 01 off, 08 rainbow, 04 breath, 02 flash...), luminosite 0x36 (0x64), vitesse 0x38, R/G/B 0x30/0x31/0x32, apply 0x3F. Init : 0x00@0x2E ; serie 40 : 0x00@0x2D ; serie 50 : 0x01@0x46 ; puis static + luminosite 100 % + vitesse 0. WriteByte avec retry x4 ; pause 120 ms/frame. |
| MSI Lovelace `MSI_GPU_Lovelace.js` | 0x68 | IDs | Mode "single" = idem MSI. Sinon per-LED : init longue (0x2D<-0, 0x46<-1, 0x22<-0x1C, blocs 3 o. [00,00,FF] @0x29/0x27/0x28, 0x36<-0x14, 0x38<-4, 0x22<-8, 0x3F<-0, repetition, 0x2E<-1) ; trame : un bloc de 136 o. @reg **0x49**, 4 o./LED (ordre **B,G,R**, 4e octet inutilise), jusqu'a 34 LED ; "GAMING X" duplique la couleur a l'index -12. |
| MSI Blackwell `MSI_GPU_Blackwell.js` | 0x68 | IDs | Per-LED : preambule 0x46<-1, 0x06<-0, 0x20<-0 ; trame : un bloc @reg **0x04**, octet 0 = 0x01 puis 5 o./LED [idx, R, G, B, 0x01]. Mode single : init MSI + 0x06<-1. |
| PNY Turing/Ampere `PNY\PNY_Turing_Ampere_GPU_Controller.js` | 0x49 | IDs | 1 zone. Init : 0xE0<-0, 0x60<-1, 0x6F<-0x64 ; R/G/B regs 0x6C/0x6D/0x6E. |
| PNY XLR8 Lovelace `PNY_XLR8_Lovelace_GPU.js` | 0x60 (subvendor 0x196E) | IDs | 3 zones : bloc 7 o. @reg 0x02 = [06,FF,zone,00,R,G,B]. |
| PNY Blackwell `PNY_Blackwell_GPU.js` | 0x60 | IDs | 1 zone, meme paquet 7 o. ecrit aux regs 0x04 ET 0x02. |
| PNY XLR8 REVEL Lovelace `PNY_XLR8_REVEL_Lovelace_GPU.js` | 0x49 | IDs | 4 LED. Init : 0xEF<-0, 0xE0<-0, 0x60<-0. Trame : bloc 16 o. @0xB0 = [05,05,05,05, RGB x4] puis commit bloc 3 o. @0xE0 = [05,03,18]. |
| Colorful Ampere `Colorful\Colorful_Ampere_GPU.js` | 0x61 (subvendor 0x7377) | IDs | Bloc brut de 11 o. (longueur 0x0B) : [AA, EF, m1, m2, 01, FF, R, G, B, crcLo, crcHi], (m1,m2) = (12,03) ou (01,04) pour "Vulcan" ; CRC = somme 16 bits LE des 9 premiers octets. 1 zone. |
| ASRock AMD (subvendor 0x1849) `AsRock\ASRock GPU.js` | **0x36** | bus AMD GPU + subvendor + WriteQuick(0x36)==0 | Bloc 12 o. a reg 0 = [cmd, 0, sub, donnees...]. Canaux actifs : cmd 0x14/sub 1 (masque dans o[4..5]) ; LED/canal : cmd 0x14/sub 2 (o[4..]). Couleur : cmd **0x10**, sub = index canal, donnees [01,R,G,B,0x80,0xFF,00,0x1A]. Canaux : 0 Backplate, 3 ARGB Header, 6 Top Side, 7 Fan. Une couleur par canal, pause 20 ms, seuil de changement 5 %. |

Dangers GPU : sur NVIDIA l'adresse vient des IDs, jamais d'un sondage -> une erreur de table envoie des ecritures a une puce inconnue. Pas d'avertissement de brick explicite hors ENE_RAM.js.

### 3. Carte mere ASUS via SMBus `Asus\Motherboard Controllers\AsusSMBusMotherboardController.js`
- Pre-requis : `motherboardInfo.manufacturer` contient "ASUSTeK" ; bus AMD ou Nuvoton. Adresses **0x39, 0x40, 0x4E, 0x4F** (0x66 retire : ecran noir en VGA).
- Detection : WriteQuick==0 ; nom modele (16 car. @0x1000, pause 3 ms) dans la table (V1 : `LED-0116`, `AUMA0-E8K4-0101` ; V2 : `AUDA0-E6K5-0101`, `AUMA0-E6K5-0104/0105/0106/0107`) ; fabricant (21 car. @0x1025) sans "Micron". Test de motif 0x00-0x0F present mais desactive (commentaire).
- Meme interface 16 bits qu'ENE. Table de config 64 o. @0x1C00 ; nb LED = table[2] ; liste des LED a partir de l'offset **0x13 (V1) / 0x1B (V2)** : chaque octet = id de zone (04 Sync, 05 DRAM_2, 82 Center_Start, 83 Center, 84 Audio, 85 Back_IO, 86/87/91 RGB header, 88 Backplate, 8A DRAM, 8B PCIE, 95 QLED, 98 Power/Reset, 99-9C chipset, A0 IOShield, A2/A3 M.2 cover) ; ids inconnus ignores. Un sous-peripherique par canal.
- Init : mode direct 0x01 @0x8020 + commit 0x01 @0x80A0 (sortie : 0x00 + commit) ; Render bloque tant que le nom lu n'est pas valide.
- Couleurs : un seul bloc, **R,B,G** par LED dans l'ordre de la table ; V1 : base 0x8000 + commit 0x01 @0x80A0 ; V2 : base 0x8100, sans commit. Taille max de bloc INCONNUE.
- Danger : ne pas scanner 0x66 (ecran noir), 0x74 (Gigabyte).

### 4. Non couvert / INCONNU
- Tables d'IDs PCI GPU (omises volontairement) ; detail binaire de WritePacketV3 (Vengeance Pro) ; table PEC (CRC-8 poly 0x07 standard).
- Commit Corsair DDR5 / Ballistix, nb LED reel des barrettes ENE, taille max de bloc ASUS carte mere : non confirmes.

## 7. Écrans LCD (rawusb / série), utile au module écrans

| Appareil | VID:PID | Transport | Faits (source) |
|---|---|---|---|
| Thermalright (Vision, Wonder, Frozen, Hyper, Mjolnir, Stream, Core, TL-M10...) | 0x87AD:0x70DB | rawusb, bulk OUT 0x01 / IN 0x81 | `Thermalright_LCD_Controller.js` : trame de 64 octets `12 34 56 78` ; init = octet 56 à 0x01, réponse 64 octets (modèle = octet 24, sous-modèle = octet 28, `A1 A2 A3 A4` aux octets 4-7 = démarrage en cours, attendre 3 s, 10 essais) ; envoi image : en-tête 64 octets (octet 4 : 0x02 JPEG, 0x03 RGB565 ; largeur LE aux octets 8-9, hauteur LE aux 12-13 ; octet 56 = 0x02 ; longueur des données LE aux 60-63) suivi des données. Résolutions : 0x01/0x03/0x04 = 480x480, 0x05/0x06/0x07 = 640x480, 0x0B = 854x480, 0x40 = 1600x720 JPEG (dalle 2400x1080 réduite par le firmware), 0x41/0x42 = 1920x462, 0x20 = 320x320 RGB565 (sans handshake) |
| Turzx 3.5" (série) | 0x1A86 + PID table | série 9600 bauds | `Turzx_LCD_Controller.js` : commandes 0x65 redémarrage, 0x6C/0x6D écran off/on, 0x6E luminosité, 0xC5 image ; 320x480 ; paquet 6 octets avec x, y, largeur, hauteur compactés |
| Turzx 2.1" / 5" / 8" | 0x1D6B + PID table | série 115200 bauds | `Turzx_Screen_21_Controller.js` : commandes à signature `ef 69`, remplissage à 250 octets ; 480x480 (2.1"), 480x800, 480x1920 |
| Turzx 8.8" | 0x1CBE:0x0088 | rawusb, bulk 0x01/0x81, rapports 512 octets | `Turzx_Screen_88_Controller.js` : en-tête 512 octets chiffré DES (clé statique dans le plugin, non reproduite ici), image JPEG 480x1920 |
Autres écrans : Corsair iCUE LINK/XC7/XD6/Capellix/Nexus, NZXT Kraken Z/Elite, ASUS Ryuo, Lian Li Galahad II/TL LCD/Universal Screen 88, ID-Cooling FX LCD, ASRock LCD, CoolerMaster Atmos II (voir CSV, détails INCONNU).

## 8. Ce qui est INCONNU

- URL exacte du CDN de plugins et format du manifeste ; contenu des add-ons non publics (`signalrgb/msi`).
- Registres Super I/O (IT87XX/Nct677X) et logique de création des bus Intel/AMD/Nuvoton (code natif non décompilé ; seules les chaînes et la liste blanche de ports du pilote sont lues).
- LEDs de nombreux modèles (calculés à l'exécution) ; les `INCONNU` du CSV.
- Voir les sections « Non couvert » de chaque famille ci-dessus (Razer deviceLibrary complète, Logitech DeviceIDs, Corsair Legacy Mouse/Vanguard/LCD, usage pages ASUS/Gigabyte/MSI, etc.).

## 9. Difficulté de réimplémentation (estimation)

| Famille | Difficulté | Remarque |
|---|---|---|
| Razer (trame 90 o, CRC XOR) | Faible | très documenté, un seul format, ~190 PID |
| Contrôleurs ARGB tiers (Nollie, PrismRGB, Zhi Yan, Skydimo, Airgoo) | Faible | trames simples, peu de variantes |
| SteelSeries, Corsair Bragi/legacy, NZXT, Lian Li V2 | Moyenne | opcodes par modèle, découverte de propriétés |
| Logitech HID++ 2.0 | Moyenne | découverte des features, Lightspeed |
| ASUS/Gigabyte/MSI cartes mères USB | Moyenne | offsets de blocs, risque de « brick » MSI |
| RAM ENE/Corsair/Kingston, GPU I2C | Élevée | pilote de port E/S ou NVAPI/ADLX, détection par ids PCI, risque de destruction de matériel |
| Écrans LCD | Moyenne | JPEG/RGB565 sur bulk, handshake de modèle |

## 10. Tests d'octets possibles sans matériel

- Razer : construire la trame 90 octets d'un effet statique et vérifier le XOR des octets [2..87] par un test unitaire.
- Corsair Platinum/Commander Pro : CRC-8 (polynôme 0x07 déduit) sur les trames d'exemple des plugins.
- Thermalright : en-tête 64 octets (magic, commande, dimensions, longueur) comparé aux offsets du §7.
- ENE : conversion de registre 16 bits (échange d'octets) avant écriture.
- CSV : vérifier l'unicité (vid,pid,source) et que chaque `ProductId` d'un plugin est bien présent.
