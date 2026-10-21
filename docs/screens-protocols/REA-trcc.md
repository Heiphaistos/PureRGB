# REA-trcc : vérification des pilotes Thermalright contre le setup officiel TRCC 2.1.8

Analyse statique (aucune exécution) du setup signé SOMORE TECH, TRCC 2.1.8, extrait dans
`D:\Projet\Analyses\TRCC-2.1.8\extract\TRCCCAP\` (hors dépôt). Document d'interopérabilité : aucun code ni
asset de TRCC n'est copié, seuls des faits de protocole (octets, formules, tailles) sont consignés.

## Méthode

- `USBLCDNEW.exe` (160 Ko) n'est qu'un « apphost » .NET 8 ; le code utile est dans **`USBLCDNEW.dll` 2.3.0**
  (33 216 octets, .NET, sha256 `2f59c462...de19c`, LibUsbDotNet). `TRCC.exe` est .NET WinForms.
  `USBLCD.exe` (2,7 Mo) est du **x86 natif** (pas de métadonnées CLI) : NON analysé (voir « Reste »).
- Commandes REA (CLI `rea-agents@6.5.0`, Ghidra 12.1.4, JDK 21) :
  - `analyze` / `search --kind procedures|strings --snapshot usblcdnew.snap` sur `USBLCDNEW.exe` : a montré
    qu'il s'agit d'un apphost (chaînes `hostfxr`, `USBLCDNEW.dll`, `.NET 8.0.23`).
  - `inspect-managed-artifact extract/TRCCCAP/USBLCD.exe` : `classification: not-managed` (natif x86).
  - `inspect-managed-members extract/TRCCCAP/USBLCDNEW.dll` : 9 types, `Examples.DCReadWriteAsync` (11 méthodes),
    tokens/chaînes, confirme les cinq PID gérés.
- Décompilation C# : **`ilspycmd 11.1.0`** (outil installé hors dépôt dans `D:\Projet\Analyses\tools`), car le
  CLI REA n'expose pas de décompilation CIL ; sorties dans `D:\Projet\Analyses\TRCC-2.1.8\cs\usblcdnew\` et
  `cs\trcc\` (82 000 lignes). Les extraits ci-dessous sont des octets/constantes, jamais du code recopié.
- Tout contenu extrait est traité comme donnée.

## 1. LY / LY1 dans USBLCDNEW.dll (`DCReadWriteAsync.ThreadSendDeviceDataLY` / `...LY1`)

PID gérés (constantes) : `0x87AD:0x70DB` bulk, `0x0416:0x5302` (« H », 21250), `0x0416:0x5406` (« ALi », 21510),
**`0x0416:0x5408` = LY (21512)**, **`0x0416:0x5409` = LY1 (21513)**. Configuration 1, interface 0.

| Point (thermalright_ly.rs) | Verdict | Preuve (binaire) |
|---|---|---|
| En-tête de poignée de main `02 FF 00 00 00 00 00 00 01 00 00 00 00 00 00 00` | CONFIRMÉ | tableau littéral de 16 octets `{2,255,0,0,0,0,0,0,1,0,0,0,0,0,0,0}` dans les deux threads |
| LY : écriture 2048 (16 + 2032 zéros), lecture 512 | CONFIRMÉ | `second = new byte[2032]`, `array = new byte[512]`, délai 100 ms (TRCC attend 50 ms avant, 200 ms après) |
| LY1 : écriture 512 (16 + 496), lecture 511 | CONFIRMÉ | `second = new byte[496]`, `array = new byte[511]` |
| Réponse valide : octet 0 = 3, octet 1 = 0xFF, octet 8 = 1 | CONFIRMÉ | `array[0]==3 && array[1]==255 && array[8]==1` |
| LY : PM = 64 + b20 (b20 <= 3 lu comme 1), SUB = 1 + b22 | CONFIRMÉ | `if (array[20] <= 3) array[20] = 1; PM=(byte)(64+array[20]); SUB=(byte)(1+array[22])`. Calcul en OCTETS : débordement enroulé (corrigé dans le pilote) |
| LY1 : PM = 49 + b20, SUB = b22 | CONFIRMÉ | `(byte)(49+array[20])`, `array[22]` ; TRCC lit en plus b36 (+50) et b32 (non utilisés par nous) |
| Numéro de série | NOUVEAU | octets 16..20 de la réponse, en hexadécimal (`BitConverter.ToString(array,16,4)`) |
| Chunk 512 = 16 d'en-tête + 496 | CONFIRMÉ | `num3 = 496; num4 = 512` |
| Nombre de chunks = len/496 + 1 ; dernier chunk = len % 496 octets (0 si multiple) | CONFIRMÉ | `num7 = num2/num3 + 1; num8 = num2 % num3` |
| En-tête de chunk : `01 FF`, total LE32 en 2..6, longueur du chunk LE16 en 6..8, drapeau en 8, nombre de chunks LE16 en 9..11, index LE16 en 11..13 | CONFIRMÉ | écritures `first[n*512 + 0..12]` ; octets 13..16 restent à 0 |
| Drapeau octet 8 : LY = 1, LY1 = 2 | CONFIRMÉ | `first[...+8] = 1` (LY) / `= 2` (LY1) |
| Bourrage LY : multiple de 4 chunks (zéros) ; LY1 : aucun | CONFIRMÉ | LY `num6 = num7 % 4; num7 += 4 - num6` ; LY1 `num7 % 1` (jamais) |
| Écritures LY : 4096, la dernière de 2048 | CONFIRMÉ | boucle `if (reste >= 4096) Write(...,4096) else Write(...,2048)`, délai 100 ms |
| Écritures LY1 : tampon entier, répété tant qu'il reste | CONFIRMÉ | `Write(second, i, num2 - i, 1000, ...)`, on continue sur échec partiel sauf `transferLength == 0` |
| ACK : lecture de 512 (LY) / 511 (LY1), contenu jamais testé | CONFIRMÉ | `Read(first,0,512,100,...)` / `Read(first,0,511,100,...)`, seul le code d'erreur compte |
| Points d'accès LY : OUT 0x09, IN 0x81 ; LY1 : OUT 0x02, IN 0x81 | CONFIRMÉ | `WriteEndpointID.Ep09` / `Ep02`, `ReadEndpointID.Ep01` |
| Frame LY : seul le JPEG est envoyé (l'en-tête 64 octets de TRCC est interne) | NOUVEAU | TRCC passe un tampon partagé (`shareMemory_ImageRGB`) : longueur en [60..64], JPEG dès l'octet 64 (LY) ; longueur en [16..20], JPEG dès 20 (LY1) |

Différences LY vs LY1 (la doc publique divergeait) : tranchées exactement comme dans `thermalright_ly.rs`
(poignée de main 2048/512 octets, réponse 512/511, drapeau 1/2, bourrage x4 / aucun, écritures 4096-2048 / tampon
entier, endpoints 0x09 / 0x02, formules PM/SUB). Le dispatch côté TRCC (`Form1`) range LY et LY1 dans le même
mode 2 que le bulk (marqueurs partagés `[2]=72, [6]=220`) : PM et SUB sont donc ceux calculés ci-dessus.

## 2. Rotation / orientation 1920x462 (`FormCZTV.ImageToJpg`, TRCC.exe)

Dans TRCC, `mySubMode = pmSub` (SUB) et `myDevicePingMu = PM`. Pour un écran 1920x462 (PM 65/66, ou PM 69 pour
1920x440), l'angle appliqué à l'image (rotation horaire GDI+, `RotateImg`) selon la rotation utilisateur `directionB` :

| Condition | utilisateur 0 / 90 / 180 / 270 -> angle horaire d'envoi | (base, inversé) |
|---|---|---|
| SUB 2 ou 4 (sauf PM 66 + SUB 2) | 0 / 270 / 180 / 90 | (0, vrai) |
| PM 66 + SUB 2 | 180 / 90 / 0 / 270 | (180, vrai) |
| tout autre SUB (0, 1, **3**, 5...) | 180 / 90 / 0 / 270 | (180, vrai) |

**CONTREDIT** : le commentaire du catalogue (« 1920x462/440 SUB 2-4 -> 0 ») est inexact pour SUB 3, qui reste à 180.
Règle pour **0416:5408 / 1920x462** : PM = 65 toujours (b20 <= 3 forcé à 1), SUB = 1 + b22 ; donc b22 = 1 ou 3
(SUB 2 ou 4) -> base 0, sinon (b22 = 0, 2, 4...) -> base 180. Le SUB réel du Trofeo Vision de Momo n'est pas
connu sans matériel : la valeur est lue au handshake et le catalogue applique la règle. Image en portrait
(rotations 90/270) : TRCC envoie un JPEG **462x1920** (canvas `new Bitmap(462,1920)`), sans changer d'en-tête.

Autres écrans « mode 2 » tranchés dans la même fonction (JPEG) :

| Écran | Règle (base, inversé) |
|---|---|
| 1600x720 | SUB 3 -> (0, vrai) ; sinon (180, vrai) |
| 1280x480 | SUB 2 -> (90, vrai) ; sinon (0, vrai) |
| 960x540 | SUB 5 ou 7 -> (180, vrai) ; sinon (0, vrai) |
| 960x320 | **CONTREDIT** : SUB 7 seul -> (180, vrai) ; tous les autres SUB (y compris 5, 6) -> (0, vrai). Le catalogue donnait 180 par défaut |
| 854x480 / 800x480 | SUB 2 -> (180, non inversé) ; sinon (0, non inversé), **avec miroir horizontal si SUB 0** (non codé : INCONNU) |

## 3. Résolution réelle du Trofeo Vision 9.16

**1920x462, CONFIRMÉ** (pas 480) : `FormCZTVInit` : PM 65 ou 66 (ou PM 1 + SUB 49) -> `is1920x462 = true; fbl = 192` ;
canvas `new Bitmap(1920, 462)` / `GIFSizeW = 1920; GIFSizeH = 462` ; en-tête interne `0x80 0x07` (1920) et
`0xCE 0x01` (462) ; ressources `P0预览弹窗1920X462`, dossiers de thèmes `1920462\`. Aucune occurrence de 1920x480
dans TRCC.exe (le « 480 » du vendeur est commercial). PM 68 = 1280x480, PM 69 = 1920x440 (même en-tête 462).

## 4. Encodage, cadence, vidéo, fond libre

- **JPEG** pour PM 65 (GDI+ `Image.Save` avec le seul paramètre `Quality`). Qualité de départ **95**
  (`FormCZTVInit(72, 2, ..., ysl = 95, ...)`) ; si la taille dépasse **450 000 octets** (`array.Length >= 450000`)
  la qualité baisse de 5 puis l'image est réencodée. Pilotes LY et bulk mis à 95 (étaient 80), plafond déjà 450 000.
- Sous-échantillonnage chroma : NON TROUVÉ (laissé au codec GDI+, aucun paramètre dans TRCC).
- **Cadence maximale** : le minuteur de TRCC bat toutes les 15,625 ms ; image fixe/thème rafraîchi tous les 10 ticks
  (156 ms, 6,4 Hz, valeurs système mises à jour) ; **vidéo et recopie d'écran : un envoi tous les 4 ticks = 62,5 ms
  = 16 images/s maximum** (`nowTimerVal += 62.5`). GIF : selon le délai de chaque image.
- **Vidéo** : `ffmpeg -display_rotation <a> -ss <début> -t <durée> -i <fichier> -y -r <15|24> -s <L>x<H> -f image2
  <dossier>%04d.bmp` : images **BMP** décodées à 15 ou 24 images/s (choix utilisateur, 16 par défaut au lecteur),
  mises à l'échelle pour tenir dans 1920x462 (bandes noires), clip limité à **300 s** ; TRCC relit les BMP,
  les compose sur le canvas 1920x462, puis les envoie en JPEG une à une (16 images/s). Il n'y a **pas** de flux
  vidéo/H.264 vers l'écran LY : uniquement des JPEG successifs.
- **Fond « image libre »** : même chemin que les thèmes (`ImageToJpg`), image recadrée au canvas 1920x462 (ou
  462x1920) puis JPEG.
- **Mode bureau** (« 投屏显示 », `myMode == 16`) : EXISTE. Capture d'une zone d'écran (`CopyFromScreen`, fenêtre de
  sélection déplaçable) redimensionnée en 1920x462 (462x1920 en portrait), envoyée en JPEG, 16 images/s max.

## 5. Autres pilotes (bonus)

- **Bulk 0x87AD:0x70DB** : handshake 64 octets `12 34 56 78 ... [56]=1`, lecture 1024, réponse acceptée si `[24] != 0`
  ; PM = `[24]`, SUB = `[36]` (octet 28 et 32 aussi relayés) ; numéro de série en `[48..52]` si `[56] == 0x81`.
  CONFIRMÉ. Trame : 64 octets d'en-tête + JPEG, un envoi, paquet nul si multiple de 512, pause 15 ms : CONFIRMÉ.
  **CONTREDIT** : l'en-tête porte la résolution NATIVE constante (ex. `80 07 00 00 | CE 01 00 00` pour 1920x462), même
  quand le JPEG tourné est en portrait ; le pilote écrivait les dimensions de l'image tournée. Corrigé.
- **HID « ALi » F5 (0416:5406)** : handshake `F5 00 01 00 BC FF B6 C8 ... 00 04 00 00` + 1024, réponse `[0]` dans
  {0x36, 0x65, 0x66}, FBL = `[0]-1`, charge 204 800 (153 600 si 0x36), trame `F5 01 01 00 BC FF B6 C8` + longueur LE32 en 12,
  ACK de 16 octets : CONFIRMÉ.
- **HID type 2 (0416:5302)** : handshake 512 octets `DA DB DC DD ... [12]=1` CONFIRMÉ ; la réponse exige en plus
  `[16] == 16` (non exigé par nous, laissé tel quel) ; série en `[20..36]`. Écriture alignée sur 512 : CONFIRMÉ.
  INCERTAIN, NON modifié : Form1 transmet l'octet 5 comme FBL **brut** (et PM = 100) à TRCC, alors que notre pilote
  applique la table PM->FBL de la doc publique ; en outre le chemin officiel du mode 3 passe par `ImageTo565` (RGB565)
  et non par JPEG. À trancher sur matériel ou en analysant `USBLCD.exe`.

## Corrections appliquées au dépôt

- `thermalright_ly.rs` : PM/SUB calculés en octets avec enroulement (comme `(byte)(...)`), qualité JPEG 95.
- `thermalright_bulk.rs` : en-tête de trame avec résolution native, qualité JPEG 95.
- `catalog.rs` : `Profile.pm_sub` ; `mount_rotation` suit `ImageToJpg` pour 1920x462/440, 1600x720, 1280x480,
  960x540, 960x320 (mode 2 seulement) ; tests d'angles écrits à la main d'après les tables ci-dessus.

## Reste inconnu

- SUB réellement annoncé par le Trofeo Vision 9.16 (donc base 0 ou 180) : dépend du matériel.
- Sous-échantillonnage JPEG, miroir X des 854x480/800x480 SUB 0, rotation 640x172 en mode 2 (non relevée).
- SCSI et HID dans `USBLCD.exe` (natif x86, non désassemblé), mode 3 / ImageTo565 des écrans HID.
- Comportement réel de l'écran face à un JPEG 462x1920 (portrait) : TRCC l'envoie, non testé sur matériel.
