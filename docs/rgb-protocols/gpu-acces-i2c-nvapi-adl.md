# Cartes graphiques : comment atteindre le contrôleur RGB (NVAPI I2C, ADL I2C, NVAPI Illumination)

Voir `smbus-i2c-acces-et-securite.md` pour les règles générales ; les registres par constructeur sont dans `gpu-controleurs-par-constructeur.md`.

## Source analysée et limites

- Source : code source OpenRGB (`D:\Projet\Analyses\rgb\openrgb-src`, git `bbc77470`) : `i2c_smbus/Windows/i2c_smbus_nvapi.cpp`, `i2c_smbus_amdadl.cpp`,
  `dependencies/NVFC/nvapi.{h,cpp}`, `dependencies/display-library/` (en-têtes ADL), `Controllers/NVIDIAIlluminationController/*`, `i2c_smbus/i2c_amd_gpu.h`.
- Les logiciels officiels des constructeurs (ASUS GPU Tweak / Armoury Crate, MSI Center / Afterburner, Gigabyte Control Center / RGB Fusion, EVGA Precision X1, Zotac FireStorm, Sapphire TriXX,
  PNY VelocityX, PowerColor DevilZone, ASRock Polychrome...) **n'ont pas été téléchargés ni extraits**. Les commentaires d'OpenRGB indiquent que plusieurs séquences viennent de captures
  de leur trafic (ex. EVGA Precision X1 : trames `NvAPI_I2CWriteEx` reproduites en commentaire) ; le détail de leurs pilotes est `INCONNU (non vérifié)`.
- À la différence de la RAM, **aucun accès au SMBus de la carte mère ni pilote tiers (PawnIO/WinRing0) n'est nécessaire** pour la plupart des GPU : on passe par la bibliothèque du pilote graphique (NVAPI, ADL), à privilèges utilisateur standard (à confirmer : la source ne teste pas l'élévation pour NVAPI/ADL).

## 1. NVIDIA : NVAPI « I2C Ex »

### Chargement (`dependencies/NVFC/nvapi.cpp`)

- Bibliothèque : `nvapi64.dll` (64 bits) / `nvapi.dll` (32 bits), installée avec le pilote GeForce. Point d'entrée unique exporté : `nvapi_QueryInterface(id)` ; chaque fonction s'obtient par un identifiant de 32 bits.
- Identifiants utilisés : `NvAPI_Initialize` **0x0150E828** ; `NvAPI_Unload` 0xD22BDD7E ; `NvAPI_EnumPhysicalGPUs` **0xE5AC921F** ; `NvAPI_GPU_GetPCIIdentifiers` **0x2DDFB66E** ;
  **`NvAPI_I2CWriteEx` 0x283AC65A** ; **`NvAPI_I2CReadEx` 0x4D7B0709** ; `NvAPI_GPU_ClientIllumZonesGetControl` 0x3DBF5764 ; `NvAPI_GPU_ClientIllumZonesSetControl` 0x197D065E.
  (Interfaces non publiques : instables d'une version de pilote à l'autre ; prévoir un test d'existence et une dégradation propre.)
- `GetPCIIdentifiers` renvoie `device_id` (octets bas = vendor, hauts = device) et `sub_system_id` (bas = subvendor, hauts = subdevice) -> c'est la **clé de la liste blanche** (§3).

### Structure `NV_I2C_INFO_V3` (version = `NV_STRUCT_VERSION(NV_I2C_INFO_V3, 3)`, c.-à-d. taille de la structure | 3 << 16)

Champs dans l'ordre : `version` (u32), `display_mask` (u32), `is_ddc_port` (u8), `i2c_dev_address` (u8), `i2c_reg_address` (pointeur u8), `reg_addr_size` (u32), `data` (pointeur u8), `size` (u32),
`i2c_speed` (u32), `i2c_speed_khz` (enum), `port_id` (u8), `is_port_id_set` (u32). La structure est mise à zéro au constructeur.

Valeurs fixes utilisées par OpenRGB :
- `is_ddc_port = 0`, **`port_id = 1`**, `is_port_id_set = 1` : « le port 1 du GPU est celui où se trouvent les contrôleurs RGB » (commentaire de la source). Les ports DDC/écran sont à exclure.
- `i2c_speed = 0xFFFF` et `i2c_speed_khz = NVAPI_I2C_SPEED_DEFAULT`.
- `i2c_dev_address = adresse_7_bits << 1` (adresse 8 bits).
- Écriture / lecture : `NvAPI_I2CWriteEx(handle, &info, &inconnu)` / `NvAPI_I2CReadEx(...)` (3e argument entier non documenté, passé à 0).

Traduction SMBus -> NVAPI (comportement à reproduire) :
| Opération SMBus | `i2c_reg_address` / `reg_addr_size` | `data` / `size` |
|---|---|---|
| send byte (octet seul) | aucun / 0 | l'octet / 1 |
| read/write byte data | `[registre]` / 1 | 1 octet |
| read/write word data | `[registre]` / 1 | 2 octets, poids faible d'abord |
| block write | `[registre]` / 1 | longueur + données (n+1 octets) |
| i2c block (write/read) | `[registre]` / 1 | n octets bruts |
| écriture brute (`i2c_write_block`) | aucun / 0 | tout le paquet (premier octet = « registre ») |
| `write_quick` | **non supporté** | - |

Conséquences : les contrôleurs qui reposent sur `write_quick` (voir RAM) ne peuvent pas être sondés par NVAPI ; EVGA Ampere nécessite `read_i2c_block_data` (« does the i2c interface support... »).

### Énumération

`NvAPI_Initialize` puis `NvAPI_EnumPhysicalGPUs` (jusqu'à 64) ; pour chaque GPU, créer un « bus » nommé `Nvidia NvAPI I2C on GPU n`, remplir `pci_vendor/device/subsystem_vendor/subsystem_device` par `GetPCIIdentifiers`, `port_id = 1`.

## 2. AMD : ADL2 (`atiadlxx.dll`)

- Bibliothèque `atiadlxx.dll` (64 bits ; `atiadlxy.dll` pour 32 bits sur 64 bits), fournie avec Adrenalin. Fonctions résolues par nom : `ADL2_Main_Control_Create`, `ADL2_Main_Control_Destroy`, `ADL2_Adapter_NumberOfAdapters_Get`,
  `ADL2_Adapter_AdapterInfoX2_Get`, `ADL2_Adapter_AdapterInfoX4_Get`, **`ADL2_Display_WriteAndReadI2C`**.
- Énumération : `AdapterInfoX4_Get(context, -1, ...)` puis garder **une seule entrée par numéro de bus PCI** (`iBusNumber`) pour éviter les doublons (une entrée par sortie vidéo). Le vendeur/device/sous-système viennent de la chaîne PNP de l'adaptateur
  (`...VEN_xxxx&DEV_xxxx&SUBSYS_ddddvvvv...`, le sous-système vaut `sous-device` puis `sous-vendeur`).
- Structure `ADLI2C` : `iSize`, `iLine`, `iAddress`, `iOffset`, `iAction` (`ADL_DL_I2C_ACTIONREAD` / `ADL_DL_I2C_ACTIONWRITE`), `iSpeed`, `iDataSize`, `pcData`.
  Valeurs utilisées : **`iLine = 1`** (« emplacement de la puce Aura »), `iSpeed = 100` (kHz), `iAddress = adresse_7_bits << 1`.
- Lecture : `iOffset = registre`, `iDataSize = n`. Écriture : le **premier octet du tampon est le registre**, puis les données, `iDataSize = n + 1`, `iOffset = 0`.
- Le bus est nommé `"AMD ADL"` : **tous les détecteurs AMD vérifient ce nom** (`is_amd_gpu_i2c_bus`, qui accepte aussi les noms Linux `AMDGPU DM i2c OEM bus` et `AMDGPU i2c bit bus OEM 0x97`). Ainsi un bus DDC de l'écran n'est jamais pris pour un contrôleur RGB.
- `write_quick` non supporté.

## 3. Sécurité : la liste blanche PCI est obligatoire

Contrairement à la RAM (où l'on sonde), les GPU sont détectés **uniquement par identité PCI** : un détecteur est enregistré avec `(vendor GPU, device ID, vendor du sous-système, device du sous-système, adresse I2C)` ;
il n'est déclenché que si les quatre valeurs du bus correspondent exactement (`DetectionManager::BackgroundDetectI2CPCIDevices`). Aucun balayage d'adresses I2C d'une carte inconnue.
Vendeurs de sous-système (carte, `pci_ids.h`) : ASUS 0x1043, MSI 0x1462, Gigabyte 0x1458, EVGA 0x3842, Zotac 0x19DA, PNY 0x196E, Galax 0x1B4C, Gainward 0x10B0, Palit 0x1569, PowerColor 0x148C,
Sapphire 0x1DA2 (et 0x174B ancien), ASRock 0x1849, Colorful 0x7377, NVIDIA 0x10DE. Vendeurs GPU : NVIDIA 0x10DE, AMD 0x1002.

Règles pour PureRGB :
1. Même principe : table (vendeur, device, subvendor, subdevice) -> famille de protocole + adresse. Pas de table = pas d'écriture.
2. Pour chaque famille, exiger la **confirmation par lecture** quand la source en prévoit une (magic ASUS 0x1589, réponse `AB xx xx xx` Gigabyte, signature Galax, 0x1F/0x91 EVGA GP102, magic PowerColor, chaîne « PALIT »...), AVANT toute écriture.
3. Ignorer les bus NVAPI qui ne sont pas `port_id = 1` ; ignorer tout bus AMD qui n'est pas `"AMD ADL"`.
4. Cartes à plusieurs bus candidats : certaines Gigabyte AORUS exposent plusieurs bus qui répondent à la même adresse, un seul étant le contrôleur RGB (les autres renvoient `00 00 00 00` ou répètent l'octet envoyé : l'écran LCD ou un bus DDC). Rejeter toute réponse qui reflète l'écho de la commande.
5. Ne **pas** écrire en flash du contrôleur (octets « Save ») à chaque mise à jour (voir fiche constructeurs).
6. NVAPI : l'appel est lent et peut ignorer des appels trop rapprochés (voir §4). Mutex dans PureRGB pour sérialiser, et ne pas appeler pendant une reprise de veille.
7. Les écritures I2C sur le GPU passent par le pilote : un mauvais octet ne peut pas flasher le firmware du GPU, mais peut dérégler le contrôleur LED (ou, sur les ASUS/EVGA, enregistrer un état dans sa mémoire). Les risques SPD de la RAM ne s'appliquent pas ici.

## 4. NVIDIA Illumination (voie officielle NVAPI, sans I2C)

Pour les cartes NVIDIA Founders et quelques cartes partenaires (PNY, Palit, Gainward, Manli), le pilote expose l'éclairage lui-même :
- `NvAPI_GPU_ClientIllumZonesGetControl` / `...SetControl` (ids 0x3DBF5764 / 0x197D065E).
- Structure de contrôle de version **72012** (valeur imposée par OpenRGB, « trouvée par capture »), `bDefault = 0`. Un `GetControl` initial remplit le type et le nombre de zones.
- Types de zone : RGB (1), couleur fixe (2), RGBW (3), couleur simple (4). Emplacements : haut (0x00), face avant (0x08), arrière (0x0C), pont SLI (0x20).
- Mode de contrôle : manuel (0) ou linéaire par morceaux (1). Paramètres : `colorR/G/B(/W)` et `brightnessPct`.
- Pour certaines cartes (liste `treats_rgbw_as_rgb`, 3050/3060/4060...) le canal blanc est ignoré.
- Le pilote n'aime pas les appels répétés : OpenRGB attend `NVAPI_CONTROL_BUFFER_TIME_MS` entre deux appels (valeur numérique non relevée ici : `INCONNU`, mesurer sur matériel).
- Liste blanche aussi (RTX 2060 FE -> RTX 5090 FE et des Palit/Gainward/PNY/Manli ; ~40 entrées dans `NVIDIAIlluminationControllerDetect_Windows_Linux.cpp`). Une carte hors liste n'est pas contrôlée.
- Avantage : aucun accès bas niveau, risque minimal. À privilégier quand la carte est dans la liste.

## 5. Ce qui est INCONNU

- Les pilotes/chemins réellement utilisés par chaque logiciel constructeur (NVAPI direct, service dédié, driver propriétaire) : non extraits.
- Temps minimum entre deux appels NVAPI (valeur de la constante non lue), limites de cadence réelles.
- Comportement d'`NvAPI_I2CWriteEx` sur plusieurs GPU NVIDIA (SLI) : handles multiples énumérés, comportement d'adressage multi-GPU non testé.
- Les droits nécessaires (administrateur ou non) pour NVAPI I2C / ADL I2C sous Windows 11 : à tester.
- Disponibilité de `ADL2_Display_WriteAndReadI2C` sur les GPU Intel Arc ou les APU récentes (la source ne gère pas Intel sous Windows ; en-tête `i2c_intel_gpu.h` présent mais non analysé).

## 6. Estimation de difficulté

**Moyenne** : l'accès est simple (deux DLL du pilote graphique, aucune élévation noyau), la difficulté est de maintenir la liste blanche et de gérer ~15 familles de protocoles différents.

## 7. Tests d'octets possibles sans matériel

1. Conversion d'adresse : 0x68 -> octet NVAPI/ADL 0xD0 ; 0x2D -> 0x5A ; 0x49 -> 0x92.
2. Découpage d'un `read_word_data` : octets reçus `[lo, hi]` -> mot `hi << 8 | lo`.
3. Table de liste blanche : vérifier qu'un quadruplet inconnu ne produit aucune écriture (test d'intégration avec un bus factice qui journalise).
4. Décodage de `GetPCIIdentifiers` : `device_id = 0x2504 10DE` -> vendor 0x10DE, device 0x2504 ; `sub_system_id = 0x3A00 1043` -> subvendor 0x1043, subdevice 0x3A00 (exemple de format uniquement, valeurs fictives).
5. Chaîne PNP AMD : `PCI\VEN_1002&DEV_73BF&SUBSYS_E4101849&REV_C3` -> subvendor 0x1849, subdevice 0xE410 (le sous-système vaut `deviceId` puis `vendorId` dans la chaîne).
