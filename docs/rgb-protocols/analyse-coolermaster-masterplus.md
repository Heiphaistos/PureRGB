# Cooler Master MasterPlus+ : analyse du logiciel officiel (ARGB Gen 1/Gen 2, RGB 12 V, MasterLiquid ML240P/ML360)

Date : 2026-10-23. Complète `coolermaster.md` (qui reposait sur OpenRGB/SignalRGB). **[V]** lu dans le binaire cité, **[S]** déduit (hypothèse signalée),
**INCONNU** non confirmé. Aucun code ni asset copié ; seuls des faits de protocole sont notés. Rien n'a été installé ni exécuté.

## Source analysée

- **MasterPlus+ v1.9.6** (Windows 10/11), page officielle `https://www.coolermaster.com/en-global/masterplus.html`, lien « Download Now » `https://linkto.cm/masterplus`
  (redirige vers `coolermaster.egnyte.com/dd/J3FzMxaWXr`, aucun compte requis).
  - `Installer_MasterPlus_v1.9.6.zip` : 366 120 507 o, SHA-256 `821bd692ba9446a1048c4b2f5a705c7280398da48b7faf7e72bca7d5f1767675`.
  - `Installer_MasterPlus_v1.9.6.exe` (Inno Setup 6.1.0 unicode) : 366 524 536 o, SHA-256 `2f0fc6495e899c6aff6a0fc08093da2b90e45fe899af76a717316e2c5a4ffef5`.
  - Extraction : `7z` (zip) puis `innoextract 1.9 -e` (pas d'exécution). L'installeur embarque les `.pdb` des DLL maison, donc les noms de fonctions/classes sont fiables.
- **Fichiers étudiés** (dans `app\`) :
  - `config\DeviceList.cfg` : table des périphériques (VID, PID, numéro d'interface HID, catégorie).
  - `json\<Modèle>\component.json` : description UI (ports, effets, identifiants d'effets).
  - `CMUOT.dll` (x86, 6 Mo, `CMUOT.pdb`) : couche matériel. Classes `CMUOT::ARGB_BOX`, `ARGB_BOX2`, `RGB_BOX`, `ARGBGEN2`, `ARGBGEN2V2`, `ARGBGEN2LITE`, `ML240`, `ML360`, `usbHid`, `CmdQueue`.
  - `cmt.dll` (x86, 4 Mo, `cmt.pdb`) : « middleware », classes `CMIOT::NodeArgb`, `NodeRgb`, `NodeArgbGen2*`, `NodeML240P`, `NodeML360` (traduction UI -> CMUOT).
  - `hidapi.dll` : accès HID (`hid_open_path`, `hid_write` ; trames de 65 octets = ID de rapport 0 + 64).
- Décompilation Ghidra 12.1.4 headless (chargement PDB), lecture des fonctions listées ci-dessous. Version Ghidra = pseudo-code : les offsets d'octets sont déduits des écritures dans un tampon `uchar[0x41]`.

## Matériel couvert (VID 0x2516 pour tous)

Source : `app\config\DeviceList.cfg` (champs `PID`, `INTERFACE`, `deviceType`). La colonne classe = classe CMUOT qui l'implémente [S pour le lien PID -> classe : il vient du nom de dossier
JSON `deviceType` et des classes `Node*` de cmt.dll, pas d'une table PID -> classe lisible dans le binaire].

| PID | Produit (DeviceList.cfg) | Interface HID | Classe CMUOT | Statut |
|---|---|---|---|---|
| 0x1011 | ARGB LED Controller (`ArgbController`) | 0 | `ARGB_BOX` | [V] |
| 0x1000 | A-RGB LED SMALL CONTROLLER (`Mini_ArgbController`) | 0 | `ARGB_BOX` [S] (aucune classe « mini » dans CMUOT.pdb) | [S] |
| 0x1007 | **ARGB2 LED Controller** (`Argb2Controller`), nouveau par rapport à `coolermaster.md` | 0 | `ARGB_BOX2` | [V] |
| 0x1001 | H500M (`H500M_ArgbController`), 4 ports | 0 | `ARGB_BOX` [S] | [S] |
| 0x004F | RGB LED Controller (`RgbControlBox`), 4 ports RGB 12 V | 1 | `RGB_BOX` | [V] |
| 0x010B | ACER RGB LED Controller (`AcerRgbControlBox`) | 1 | INCONNU (Acer_RgbController non étudié) | INCONNU |
| 0x0173 | LED Controller A1 (`ArgbGen2Controller`), 3 canaux | 0 | `ARGBGEN2` | [V] |
| 0x01C9 | LED Controller A1 (`ArgbGen2Rev2Controller`), 3 canaux | 0 | `ARGBGEN2V2` | [V] |
| 0x01CB | LED Controller A1 Mini (`ArgbGen2LiteController`), 1 canal | 0 | `ARGBGEN2LITE` | [V] |
| 0x01C1 | **HAF700 EVO** (catégorie `Case`, `isSupportLed:0`, UUID 63) : écran LCD « Iris » du boîtier HAF 700 EVO | 1 | `CMUOT_HAF700` / `HAF700` (**pas HID**, voir section dédiée) | [V] |
| 0x0155 | **ML240P** (catégorie `Fan`, AIO MasterLiquid, écran LCD) | 1 | `ML240` | [V] |
| 0x020A | **ML360** (catégorie `Fan`, AIO MasterLiquid, écran LCD) | 1 | `ML360` | [V] |

Usage page HID : INCONNU pour tous (la table `device_map` de CMUOT.dll est construite à l'exécution par du code d'initialisation, pas par des données lisibles ; `usbHid::scanDevice`
compare `vid`, `pid`, `interface_num` et `usage_page`, avec `usage_page == 0xFFFFFFFF` = « ne pas filtrer »). Pour les PID Gen 2 l'usage page 0xFF01 vient de `coolermaster.md` (OpenRGB), non reconfirmée ici.

## Trames : généralités [V]

- Toutes les trames CMUOT font **65 octets** (`usbHid::txData(..., 0x41)`) : octet 0 = ID de rapport `00`, puis 64 octets. Les octets ci-dessous sont indexés dans les 65 (index 1 = premier octet utile).
- Ordre d'envoi temps réel : `CmdQueue::sendCmdQueue` empile les 3 trames d'une image ; `CmdQueue::emptyQueue` vide avant l'init. Cadence réelle de la file : INCONNU (le paramètre passé
  à `sendCmdQueue` est un index calculé à partir du port, 0 à 20, pas une durée lisible).
- Couleurs : ordre R, G, B tel que fourni par l'appelant ; aucun GRB dans ces fonctions.

## ARGB Gen 1 (PID 0x1011, classe `ARGB_BOX`) [V : CMUOT.dll `ARGB_BOX::*`]

### Mode matériel d'un port ARGB (`setARGBEffectParameter` @10157b70)
- Port logique 1..4 (le code ajoute 0xFF, soit -1) -> masque en octet 4 : port1 `01`, port2 `02`, port3 `04`, port4 `08` ; autre valeur : erreur.
- Trame : `[1]=80 [2]=0B [3]=02 [4]=masque port [5]=mode [6]=hw_rgb [7]=vitesse-1 [8]=luminosité [9]=R [10]=G [11]=B`.
  - `[6]` (hw_rgb) : `10` = couleur fixe fournie, `00` = couleur aléatoire (valeur par défaut côté appelant : `10`, `NodeArgb::setArgbPortLightingEffect`).
  - `[7]` : l'API passe la vitesse UI (1..5) et le code écrit « vitesse - 1 » : plage 0..4. Défaut UI = 3 (JSON).
  - `[8]` : luminosité 0..255 (défaut 255).
- Modes acceptés par `CMUOT` (octet `[5]`) : 0..6, 8, 9, 10, 11 ; **7 refusé** ; 12 = bascule en passthrough carte mère.
  - Table UI -> CMUOT (`NodeArgb::LedIndexTransf_UiToSdk_ARGBPort`, cmt.dll @100f2ba0) : entrée 0 -> 1, 1 -> 2, 2 -> 3, 3 -> 4, 4 -> 5, 5 -> 6, 6 -> 8, 7 -> 9, 8 -> 10, 9 -> 11, 11 -> 7.
  - Valeurs filaires = celles de `coolermaster.md` (spectrum 01, reload 02, recoil 03, breathing 04, refill 05, demo 06, fill-flow 08, rainbow 09, static 0A, off 0B), **confirmées** par
    la plage de validation. Mode 00 : accepté tel quel (sens INCONNU).
- Avant chaque mode normal (0..11), le logiciel envoie `[1]=80 [2]=01 [3]=01` (sortie du passthrough), puis la trame ci-dessus. Mode 12 (passthrough carte mère) : une seule trame `80 01 02`.

### Entrée / sortie du mode temps réel (« direct »)
- Entrée (`setLightingInit` @101587d0) : deux trames, `80 01 01 02` puis `80 01 03 02` (`[1]=80 [2]=01 [3]=01 ou 03 [4]=02`), puis démarrage du thread d'envoi.
- Sortie (`setLightingUninit`) : voir ci-dessous « INCONNU » (la fonction restaure l'état ; octets non relevés).

### Trame de couleurs temps réel (`setSyncLightingData` @101599b0) : **en-tête du mode direct**
- Un port (1..4) = 3 trames de 65 octets envoyées d'affilée (tampon de 195 o) :
  - Trame 1 : `[1]=00 [2]=07 [3]=port-1 (0..3) [4]=30` puis couleurs à partir de `[5]` : LED 0..19 en `[5+3*i .. 7+3*i]` (R, G, B).
  - Trame 2 : `[1]=01`, couleurs de la LED 20 à 40 (21 LED) à partir de `[2]` (index `2+3*(i-20)`).
  - Trame 3 : `[1]=82`, LED 41 et suivantes à partir de `[2]` (index `2+3*(i-41)`).
  - Octet de contrôle `[1]` = compteur de trame ; bit 7 (`0x80`) = dernière trame (`82` = trame n°2 + dernière). `[4]=0x30` est une constante du code (48), ne dépend pas du nombre de LED.
- Capacité vue : indices LED jusqu'à 60 (trames de 62 octets utiles) ; nombre de LED utile par port réglé séparément (voir « Nombre de LED »).

### Lecture d'état du port RGB 12 V (`getRgbLightingEffectParameter` @10158540)
- Requête `80 0A 01` (+ `00`), la réponse contient R, G, B aux octets 9..11 du rapport reçu [V].
- **Écriture du port RGB 12 V : INCONNU** dans CMUOT. `ARGB_BOX::setARGBEffectParameter` ne sait pas adresser le masque `10` ; aucune fonction `setRgb...` n'existe dans la classe (seulement
  `getRgbLightingEffectParameter` et `getRgbEffectMode`). L'écriture du `FE`/commande `04` donnée par `coolermaster.md` (OpenRGB) reste l'unique source, non reconfirmée.

### Nombre de LED / mode personnalisé
- `CMUOT_ARGB::setLEDSize(uchar,uchar,uchar)` existe ; octets envoyés : INCONNU (non relevés).
- Mode personnalisé (`setARGBEffectCustomize` @10157e90) : précédé de `80 01 01` ; trames de 62 octets utiles avec `[1]=00 [2]=82` sur la dernière ; couleurs par LED ; détail INCONNU.

### Point « Small ARGB Controller (PID 0x1000) » : mode statique et octet 7
- Il n'existe **aucune classe dédiée** au Small controller dans `CMUOT.pdb` (pas d'objet `mini`) : le logiciel utilise `ARGB_BOX` [S]. Le JSON `json\Mini_ArgbController\component.json` liste les effets :
  id 0 MOTHERBOARD, 1 SPECTRUM, 2 RELOAD, 3 RECOIL, 4 BREATHING, 5 REFILL, 6 DEMO MODE, 7 CUSTOM, 8 TURN OFF, 9 STATIC (le contrôleur 1011 : …, 7 FILL FLOW, 8 RAINBOW, 9 MULTILAYER, 10 STATIC).
- **Octet 7 de la trame de mode = vitesse - 1 (0..4)** [V pour `ARGB_BOX`] ; l'octet 6 est le drapeau couleur fixe/aléatoire (`10`/`00`), l'octet 8 la luminosité, 9..11 la couleur.
  `coolermaster.md` (OpenRGB) place la vitesse en octet 6 pour le Small : à corriger si le Small suit bien `ARGB_BOX` (à tester sur matériel).
- **Mode statique du Small : valeur filaire INCONNU (confirmation matérielle requise).** Déduction [S] : si la table `UiToSdk_ARGBPort` (ci-dessus) est appliquée à l'indice `id-1` du JSON, statique (id 9) -> `0A`
  et « turn off » (id 8) -> `09`, ce qui concorde avec le `off 09` d'OpenRGB pour le Small ; mais pour le 1011 la même règle donnerait `0B` pour STATIC (id 10), ce qui contredit OpenRGB (`0A`) : la règle d'indexation n'est donc pas établie.

## ARGB2 (PID 0x1007, classe `ARGB_BOX2`) [V partiel]
- Trame temps réel (`setSyncLightingData` @10166c80) : début `00 80 16 02`, puis `[4]=02` et `[5]=port-1` (cas port-1==4 : `[4]=01 [5]=00`), couleur sur 3 octets plus loin (R en `[8]` d'après le décalage, ordre exact
  INCONNU). Un seul triplet de couleur par port (pas de trame par LED).
- Commande de mode : `80 0A 02 <port>` (`setRGBEffectParameter` @10166380) ; reste INCONNU.
- Non présent dans `coolermaster.md` ; aucun support OpenRGB connu.

## RGB 12 V « RGB LED Controller » (PID 0x004F, classe `RGB_BOX`) [V : CMUOT.dll `RGB_BOX::*`]
Protocole différent (commandes ASCII) : tous les rapports 65 octets, `[1]` = commande.
- **Effet matériel** (`setRGBDeviceEffectParameter` @1023f190), séquence : `41 01` ; puis `51 2B` avec `[3]=port`, `[5]=mode`, `[6]=vitesse (voir table)`, `[7]=20` (mode 1 seulement), `[8]=19` (mode 3) sinon `FF`, `[9]=FF`,
  `[10]=luminosité (voir table)`, `[11..13]=R G B` (mode 2 : FF FF FF) ; puis `51 28` (`[5]=mode`) ; puis `50 55` ; enfin `41 00`.
  - Octet vitesse `[6]` selon (mode, niveau 1..5) : mode 1 -> 3C, 37, 31, 2C, 26 ; mode 2 -> 96, 8C, 80, 6E, 68 ; mode 3 -> 46, 41, 3C, 37, 32 ; autres modes : le niveau tel quel.
  - Octet luminosité `[10]` (niveau 1..5) : 3F, 5A, 7F, 99, FF (mode 2 : niveau 5 -> DF).
  - Modes UI (JSON `RgbControlBox`) : 1 Static, 2 Breathing, 3 Star, 4 Color Cycle, 5 Multiple Color (couleur par ventilateur), 6 Turn Off ; correspondance exacte JSON -> octet `[5]` : INCONNU pour 4..6.
- **Entrée temps réel** (`setLightingInit`) : `41 03`, puis `C0 F0`. **Sortie** (`setLightingUninit`) : `41 00`. **Extinction** : `C0 02`.
- **Couleur directe** (`setLightingPortColor` @1023f850) : une trame de 65 octets tenue en mémoire est mise à jour pour les 4 ports ; l'entrée de chaque port écrit son triplet à une rotation d'octets
  différente (port 1, 2, 3, 4 = décalages +1, +0, +3/+0/+1, +2/+3/+0). En-tête/commande de cette trame : INCONNU (initialisée hors de la fonction).
- **Couleurs par port** (`setRGBDevicePortMultiColor`) : `41 80`, `51 28` (`[5]=04`), `51 30`, `51 A8` (4 couleurs à partir de `[5]`), `51 2C` (luminosité en `[10]`) ; sauvegarde à la fermeture si configuré.

## ARGB Gen 2 A1 (PID 0x0173 `ARGBGEN2`, 0x01C9 `ARGBGEN2V2`, 0x01CB `ARGBGEN2LITE`) [V : CMUOT.dll `ARGBGEN2::*` ; V2 et LITE présentent les mêmes fonctions]
Format de commande : `[1]=80 [2]=commande [3]=opération (01 lecture, 02 écriture) [4]=masque de canal [5]=masque de sous-port ou valeur …`.
Masque de canal (index `1..3`) : `01 << (canal-1)` (valeurs `01`, `02`, `04`), `FF` = tous. Masque de sous-port (1..5) : `01 << (sp-1)`, `FF` = tous.
Identifiant logique de port dans les listes de LED : `10..16`, `20..26`, `30..36` (canal = dizaine : 1 -> `01`, 2 -> `02`, 3 -> `04` ; unité 0..6 -> masque de sous-port 00, 01, 02, 04, 08, 10, 20) et `FF`.

### Séquence d'entrée en mode temps réel
1. `CmdQueue::emptyQueue` puis **`[1]=80 [2]=01 [3]=02 [4]=01`** (`setLightingInit` @10169fa0). La classe repose ensuite sur le thread d'envoi `CmdQueue`.
2. Sortie (`setLightingUninit` @1016a010) : **`80 01 02 00`**.
(Pour V2/LITE : même corps, non rediffé octet par octet.) Le pré-roll à 2 puis 5 trames cité par `coolermaster.md` n'apparaît pas dans ces fonctions : INCONNU / non confirmé.

### Trame de couleurs temps réel (`setSyncLightingData` @1016a8a0)
- Une « image » = **3 trames de 65 octets** par (canal, sous-port) :
  - Trame 1 : `[1]=00 [2]=09 [3]=masque canal [4]=masque sous-port [5]=3C (60 LED)` puis octets de couleur à partir de `[6]` : LED 0..18 à `[6+3*i]` (R,G,B) ; LED 19 : R en `[63]`, G en `[64]`, B en `[3]` de la trame suivante.
  - Trame 2 : `[1]=01 [2]=09`, la LED 19 finit en `[3]`, LED 20..39 à partir de `[4]` (`4+3*(i-20)`), LED 40 : R en `[64]`, G,B en `[3],[4]` de la trame 3.
  - Trame 3 : `[1]=82 [2]=09`, LED 40 -> `[3]`,`[4]`, LED 41..59 à partir de `[5]`.
  - Le flux est donc continu de 180 octets (60 LED x 3) découpé en 59 + 62 + 62 octets ; `[1]`=compteur, bit 7 sur la dernière.
- Couleur fixe d'un port (`setLightingPortColor` @1016a070) : mêmes trois formes, avec le triplet répété (rotation d'ordre pour garder la phase de 3 octets sur la frontière des trames).
- Cadence : INCONNU côté logiciel (le thread `CmdQueue` espace les envois ; `coolermaster.md` cite 40 ms / 2 ms entre paquets sans confirmation dans ce binaire).

### Commandes de configuration (octet `[2]`) [V]
| Cmd | Rôle | Trame |
|---|---|---|
| `03` / `04` | effet / lecture effet (`setARGBEffectMode`) | `80 03` : `[4]=canal [5]=sous-port [6]=mode [7..13]` = paramètres de l'effet (7 octets) ; lecture `80 04` |
| `06` | taille Gen 1 (`setGen1Size`) | `80 06 02 [4]=canal [5]=nb ventilateurs (1..10) [6]=LED/ventilateur (4..60 ; si omis : 60/nb)` |
| `07` | lecture taille Gen 2 | `80 07 01 [4]=canal`, la réponse donne le nombre de sous-ports |
| `0A` | identification (clignotement) | `80 0A 02 [4]=canal [5]=sous-port` |
| `0B` | type ARGB d'un canal | `80 0B 02 [4]=canal [5]=type`, suivi d'un rafraîchissement |
| `0D` | remap des sous-ports Gen 2 | `80 0D 02 [4]=canal` + liste |
| `B2` | rafraîchissement des infos de port | `80 B2 …` (lecture) |
Le même schéma d'enveloppe est utilisé pour les superpositions (`setOverlay*`), l'UID (`setUID`/`getUID`), la sauvegarde (`saveConfig`) et la remise à zéro (`resetConfig`) : octets exacts INCONNU (non relevés).

### Nombre de LED par ventilateur
- Configuré par l'utilisateur ou lu du matériel : `80 06 02` (écriture) ; la lecture des sous-ports Gen 2 (`80 07 01`) donne les sous-ports. Valeur par défaut : 60 / nombre de ventilateurs. Table par modèle de ventilateur :
  `json\Accessory\NewFanList.json` (présent dans l'installeur, non dépouillé ici) : INCONNU.

## HAF 700 EVO (PID 0x01C1, classe `HAF700`) : écran Iris, transport ADB/TCP, pas de RGB HID [V] (passe 2026-10-26, CMUOT.dll 1.9.6)

Absent de la première passe ; trouvé dans `DeviceList.cfg` (`deviceName`/`deviceType` « HAF700 EVO », `deviceCategory` `Case`, `INTERFACE` 1, `isSupportLed` 0) et `json\HAF700\component.json`.
- **Le 01C1 est l'afficheur LCD du panneau avant, pas un contrôleur de LED.** `component.json` : profil `lcdDisplay` (modes : CM LOGO, SYSTEM [CPU/GPU clock, CPU/GPU temp, usage, APM, FAN RPM, PING, état micro], TIME, MULTIMEDIA, MULTI DISPLAY, TURN OFF ; rotation d'écran ; économiseur) [V].
- **Les LED du boîtier ne passent pas par lui** : le composant porte `hasArgbGen2: 1` et délègue l'éclairage (`lightingDelegate`, composant « HAF700-SIDE ») au contrôleur ARGB Gen 2 A1 intégré (famille 0173/01C9/01CB, déjà pilotée) [V pour la délégation, S pour le PID exact du contrôleur intégré].
- **Transport** [V : `HAF700::HAF700`, `TcpClient::InitADB`, `ProtocolWriteTCP`] : le logiciel n'ouvre PAS de rapport HID. Il lance `adb forward tcp:8888 tcp:9900` (adb.exe fourni dans `appdb`, nettoyage préalable `adb forward --remove tcp:8888`), puis se connecte en TCP à `127.0.0.1:8888` (0x22B8). L'écran est donc un périphérique Android/ADB (interface 1 = ADB) et le protocole est un service TCP côté appareil (port 9900).
- Trame TCP : en-tête puis charge utile ; la longueur (u16 big-endian) précède la charge, charge max 0x4FFA octets, 3 tentatives, reconnexion ADB au 2e échec [V]. Disposition exacte des 4 premiers octets de l'en-tête : [S] (champ commande + drapeau) non relevée.
- Codes de commande vus (1er paramètre de `ProtocolWriteTCP`) : `10` rotation (valeur+1), `12` info de mode LCD, `14` (7 o, réglage lumineux/LCD), `15` lecture, `16` sauvegarde de valeur, `18` multi-affichage, `19` version firmware (réponse u16 big-endian), `F2` liste de fichiers, `F5`/`F6`/`F7` envoi d'animations/images, `F8` ping, `FC` synchro de l'heure (9 o) [V pour les codes, INCONNU pour le sens octet par octet].
- `CMUOT_HAF700::setLighting`/`setLedMode`/`getLighting` sont des **fonctions vides qui journalisent et renvoient « succès »** [V] : aucun octet de LED n'est émis pour ce PID.
- **Décision PureRGB** : rien à coder en HID. Ajouter un pilote ADB serait de l'installation d'un composant tiers et des écritures sur un appareil Android (profils, fichiers) : hors périmètre. Le 01C1 est seulement nommé dans le catalogue (`known.rs`, type Boîtier). Les LED du HAF 700 EVO se pilotent par le contrôleur Gen 2 A1 intégré.

## Ventilateurs / pompes MasterLiquid
- **ML240P (0x0155) et ML360 (0x020A)** : écran LCD + LED, classes `ML240`/`ML360` (mêmes fonctions). Aucune fonction de vitesse de ventilateur ou de pompe n'existe dans ces classes ni dans les classes
  `CMUOT_ML240`/`CMUOT_ML360` (liste complète lue dans `CMUOT.pdb`) : **le logiciel ne pilote pas la pompe ni les ventilateurs de ces AIO via USB** [V par absence ; la vitesse relève du PWM carte mère]. Aucune plage de sécurité à documenter.
- **Transport** (`ML360::ProtocolWrite` @101e8430, `ProtocolRead`, `ProtocolReadReg`) : **rapports de 513 octets** (`txDirectData(..., 0x201)` : ID de rapport 0 + 512), non les 65 des contrôleurs ARGB.
  - Enveloppe : octet 0 du rapport utile = commande ; octets 1..3 = compteur de segments avec bit de poids fort de l'octet 3 = 1 sur le premier paquet (`(nbSegments | 0x800000)`), puis arguments (jusqu'à 8 octets), puis données ; 508 octets de données par segment ;
    segments suivants : `commande` + compteur sur 23 bits. Trois tentatives d'écriture.
  - Commandes vues : `FB` écriture de registres (arguments = adresse u32 + longueur u32), `FA` lecture de registres (même format), `FC` horodatage/commit (4 o), `FD` sauvegarde, `E0`/`E1`/`F1`/`F2`/`F4`..`F7` animations et fichiers (écran).
  - Carte de registres (base de lecture `0`, taille `0x220`) : `+0x000` angle de rotation de l'écran (1 o), `+0x002` **mode des LED (0..8)** (1 o, `setLedMode` @101e5370), `+0x1DA` économiseur d'écran (1 o), `+0x004`.. paramètres personnalisés (0x24 o), `+0x1D0` multi-affichage (0x1C o),
    `+0x214` (0x0C o). Les modes LED valides sont 0..8 (valeur > 8 refusée).
- Un mutex inter-processus (`ml360_mutext_name`) protège les accès : MasterPlus+ et un autre client ne doivent pas écrire en même temps.
- Pas de contrôle LED par LED des ML240P/ML360 trouvé (seulement un mode prédéfini 0..8) ; `CMUOT_ML360::setLightingPortColor`/`setSyncLightingData` : non analysés -> INCONNU.

## Contrôle de firmware (séries USB 0023 / 0028)
- La lecture de la version de firmware existe (`ARGBGEN2::getFWVersion`, `ARGBGEN2V2::getFWVersion`, `ARGBGEN2LITE::getFWVersion`, `CMUOT_ARGB::getVersion`), ainsi que la mise à jour via `FWAutoUpdateSDK.dll`
  (téléchargement serveur Cooler Master) : octets de requête INCONNU. Le test sur la chaîne de série USB `A202011171238` (0023) / `A202105291658` (0028) vient d'OpenRGB ; **non retrouvé** dans les chaînes de `CMUOT.dll` ni `cmt.dll`
  (le contrôle côté MasterPlus+ ne compare pas ces littéraux en clair).
- Ne pas mettre à jour le firmware depuis PureRGB : la procédure passe par le SDK d'auto-mise à jour du constructeur.

## Accès bas niveau
- HID uniquement (`hidapi`), pas de SMBus. L'installeur dépose aussi des pilotes (`CMUAC.sys`, `CMTAC.sys`, `WinRing0*.sys`) pour d'autres périphériques (audio, capteurs) ; **aucun n'est nécessaire** aux contrôleurs ci-dessus.
- Conflit avec le logiciel officiel : MasterPlus+ garde le périphérique ouvert ; fermer son service (`MPService`) avant de piloter en HID.

## Ce qui est INCONNU
- 01C1 : en-tête TCP exact et sens des commandes (inutile tant qu'aucun pilote ADB n'est prévu) ; passe 2026-10-26 : port RGB 12 V du 0x1011 et trame Small non revus (pas de nouvelle fonction trouvée dans `ARGB_BOX`).
- Usage page HID de chaque PID.
- Valeur filaire du mode statique du Small (0x1000) et mapping complet des ids d'effets JSON -> octet.
- Écriture du port RGB 12 V du 0x1011 ; en-tête de la trame directe de `RGB_BOX` ; octets de sortie `ARGB_BOX::setLightingUninit`.
- Cadence des envois (`CmdQueue`), pré-roll Gen 2, tables LED par modèle de ventilateur, UID, overlays, multicouche, mise à jour de firmware.
- Trames par LED des ML240P/ML360 ; ARGB2 (0x1007) : couleur et commandes.
- Vitesse de pompe/ventilateur des AIO : non pilotée par USB (cf. ci-dessus).

## Estimation de difficulté de réimplémentation
- ARGB Gen 1 : **faible** (trames fixes de 65 octets, direct = 3 trames). Gen 2 A1 : **moyenne** (enveloppe uniforme, flux continu 180 octets, masques canal/sous-port). RGB_BOX : **moyenne** (séquences ASCII, en-tête de trame directe à relever).
- ML240P/ML360 : **élevée** (rapports 512 o, registres, écran) mais utile seulement pour le mode LED 0..8.

## Tests d'octets possibles sans matériel
- Gen 1 mode : `00 80 0B 02 01 0A 10 02 FF 00 00 FF` (port 1, static `0A`, couleur fixe, vitesse 3, luminosité 255, bleu) ; longueur 65 ; mode 7 rejeté ; mode 12 = trame `00 80 01 02`.
- Gen 1 direct : initialiser par `00 80 01 01 02` et `00 80 01 03 02`, puis 3 trames : `[1]=00 [2]=07 [3]=port-1 [4]=30` ; `[1]=01` ; `[1]=82` ; LED k aux positions calculées ci-dessus.
- Gen 2 : `00 80 01 02 01` (entrée), `00 80 01 02 00` (sortie) ; image 60 LED = 180 octets découpés 59/62/62 avec `[1]` = `00`,`01`,`82` et `[2]=09` ; ports logiques 10..16/20..26/30..36 -> masques ci-dessus.
- RGB_BOX : séquence `41 01` / `51 2B …` / `51 28 …` / `50 55` / `41 00` ; tableau de vitesses et de luminosités.
- ML360 : écrire le mode LED n : `FB` + (adresse `02 00 00 00`, longueur `01 00 00 00`) + `n` dans un rapport de 513 octets.
