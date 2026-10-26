# Logitech G HUB : analyse d'interopérabilité (HID++ 2.0, RGB claviers / souris / casques)

Document d'interopérabilité. Aucun code ni asset Logitech copié ; uniquement des faits de protocole.
Complète `logitech.md` (même dossier) : celui-ci apporte les tables et séquences demandées (codes de touches, activation/restitution du mode logiciel, récepteurs).

## 1. Source analysée et limite majeure (à lire d'abord)

**Le contenu de G HUB n'a PAS pu être analysé. Aucun fait ci-dessous ne vient d'un binaire, d'un JSON ou d'un plugin de G HUB.**

| Élément | Valeur |
|---|---|
| Installeur officiel | `lghub_installer.exe`, `https://download01.logi.com/web/ftp/pub/techsupport/gaming/lghub_installer.exe` |
| Taille / date serveur | 70 346 392 octets, `Last-Modified: 2026-10-08` |
| SHA-256 | `4b2f9903b27c8434afcd52fe65845632fcae47cc50432fb6b3b1637144e811e1` |
| Version (ressource PE `version.txt`) | FILEVERSION 2026.4.9028.0 / PRODUCTVERSION 2026.4.1524.0 |
| Type | Bootstrapper WiX (Burn) + interface .NET/WPF « lghub_setup », **stub en ligne** |

Ce qui a été fait (sans exécuter l'installeur) :
- `7z x` : sections PE `.text` (69 Mo), `.rsrc` (icônes, manifeste, version), `CERTIFICATE`.
- Découpe de 6 conteneurs CAB par signature `MSCF` (offsets 17350069, 17556197, 31299689, 31506097, 56941661, 57145685). Contenu vérifié : uniquement des **prérequis Microsoft** (MSI « Visual C++ 2022 X86/X64 Minimum/Additional Runtime », exécutables de redistribuables) et les ressources de l'interface Burn (u0 à u31). **Aucun fichier Logitech de périphérique, aucun JSON de définition, aucun plugin.**
- `strings` sur l'installeur : le code de téléchargement référence `https://pipeline.logitech.io`, `https://stg-pipeline.np.logitech.io`, `https://updates.ghub.logitechg.com`, `https://util.logitech.io/brand`, des chemins `/pipeline/v2/access/`, `/content.json`, `/details.json`, `/iat.json`, `/update.json`, et un schéma protobuf `logi/common_protocol/manifest.proto` (messages Manifest, Resource, DeviceResource, ReleaseNotes). Le logiciel réel est donc téléchargé au lancement depuis un service d'accès (`/pipeline/v2/access/`, nom suggérant un jeton d'accès).
- Recherche web d'un installeur complet hors ligne officiel : **aucun lien officiel trouvé** (seulement des listes tierces et un paramètre `--silent` qui reste un installeur en ligne).

Décision : récupérer la charge utile en rejouant le client du service d'accès reviendrait à obtenir/utiliser un jeton d'accès, ce que la consigne interdit. **Blocage non contourné.** Les pistes restantes pour obtenir les définitions de G HUB légitimement : un installeur hors ligne fourni par Logitech, ou un PC où G HUB est déjà installé (lecture seule des fichiers de `C:\Program Files\LGHUB\` et `%ProgramData%\LGHUB`) ; l'utilisateur doit en décider.

Faits tirés à la place de logiciels tiers d'interopérabilité, lus localement dans `D:\Projet\Analyses\rgb\` (supprimés ou non selon §10) :
- **(O)** OpenRGB commit `bbc7747`, `Controllers/LogitechController/` (G815, G915, G213, G203L, GLightsync, Lightspeed, G933, G600, HIDPP20).
- **(G)** g810-led commit `efa3c35`, `src/classes/Keyboard.{h,cpp}`.
- **(S)** Solaar commit `e7304c4`, `lib/logitech_receiver/hidpp20_constants.py`.
- Certains commentaires d'OpenRGB déclarent reprendre des **captures de trames de l'application officielle** (G HUB) ; je les signale « capture G HUB (dit par OpenRGB) » : ce n'est pas vérifié par moi, donc non confirmé au sens strict.

Convention : « octet N » = index dans le rapport HID complet (octet 0 = report ID). `idx` = index de feature (propre à chaque appareil/firmware). Un octet de fonction `0xF S` = fonction F (nibble haut), swId S (nibble bas).

## 2. Cadre HID++ 2.0 (rappel utile, O + S)

| Rapport | ID | Taille | Usage |
|---|---|---|---|
| Court | 0x10 | 7 | registres HID++ 1.0, requêtes courtes |
| Long | 0x11 | 20 | la quasi-totalité des commandes RGB |
| Très long | 0x12 | 64 | trames de touches 0x8080 |

`[report][device_index][feature_idx][fn<<4|swId][paramètres...]`, device_index 0xFF = appareil filaire/direct, 1 à 6 = appareil derrière récepteur. Réponse d'erreur : feature_idx 0x8F en HID++ 1.0 (O, `getWirelessDevice` et `set8071Effects`), 0xFF en HID++ 2.0 (non revérifié ici).

Découverte d'un index : IRoot idx 0, octet de fonction `0x01` (fn 0, swId 1 : `LOGITECH_CMD_ROOT_GET_FEATURE = 0x01` chez OpenRGB), paramètres `[featureId_hi, featureId_lo]` → `[idx, type, version]` (O, `LogitechProtocolCommon.h` lignes 36-37). Table d'index fixes codés en dur par OpenRGB pour des modèles anciens ci-dessous ; **ne pas les coder en dur côté PureRGB : toujours interroger IRoot** (ils varient entre G915 filaire 0x09/0x0A/0x0E et G915 sans fil 0x0A/0x0B/0x0F).

Features utiles (O `LogitechHIDPP20Controller.h` lignes 34-52 ; S) : 0x0001 FEATURE_SET · 0x0003 FW_INFO · 0x0005 DEVICE_NAME_TYPE · 0x1004 batterie unifiée · 0x1D4B statut sans fil · 0x4522 DISABLE_KEYS_BY_USAGE · 0x4540 KEYBOARD_LAYOUT · **0x8070** COLOR_LED_EFFECTS · **0x8071** RGB_EFFECTS · **0x8080** PER_KEY_LIGHTING · **0x8081** PER_KEY_LIGHTING_V2 · 0x8100 ONBOARD_PROFILES · 0x8101 PROFILE_MANAGEMENT · casques Centurion 0x0600 / 0x0620 (HEADSET_RGB_HOSTMODE) · 0x0100/0x0101 infos Centurion.

Versions de feature observées fonctionnelles (O `LogitechHIDPP20Controller.cpp` lignes 117-126) : 0x0620 v1 ; 0x4540 v0-1 ; 0x8070 v0,3,5 ; 0x8071 v0,4 ; 0x8080 v0 ; 0x8081 v0,2.

## 3. Point (1) : claviers

### 3.1 PID et collections HID

VID **0x046D**. Sources : (O) `LogitechControllerDetect.cpp` lignes 57-64 et 1530-1538 ; (G) `Keyboard.h` lignes 47-60 pour les modèles absents d'OpenRGB. CONFIRMÉ comme identifiants publiés par ces projets ; non revérifiés par un binaire G HUB.

| PID | Modèle | Protocole | Collection HID (O) |
|---|---|---|---|
| C336 | G213 | rapports fixes, 5 régions | interface 1, page 0xFF43, usage 0x0602 |
| C33A | G413 | zones/effets natifs, sans clé par touche | INCONNU (non listé dans OpenRGB) |
| C330 | G410 | 0x8080 (rapports 0x11/0x12) | page 0xFF43 |
| C342 | G512 | 0x8080 | page 0xFF43 |
| C33C | G513 | 0x8080 | page 0xFF43 |
| C333 / C338 | G610 | 0x8080 | page 0xFF43 |
| C331 / C337 | G810 | 0x8080 | page 0xFF43 |
| C232 | G813 | famille G815 (OpenRGB le route comme G815) | interface 1, page 0xFF43 |
| C33F | G815 | rapports fixes 0x11 (idx 0x08/0x0D/0x0F/0x10) | interface 1, page 0xFF43 (deux handles 0x11 et 0x12) |
| C32B / C335 | G910 | 0x8080 | page 0xFF43 |
| C339 | G Pro (clavier) | 0x8080 | page 0xFF43 |
| C33E | G915 filaire | 0x8071 + 0x8081 (idx 0x09/0x0A) | interface 2, page 0xFF00, usage 2 |
| C541 / C547 | G915 via récepteur 1 / 2 (device_index 1) | idem (idx 0x0A/0x0B) | interface 2, page 0xFF00, usage 2 |
| C343 | G915 TKL filaire | idem | idem |
| C545 | G915 TKL via récepteur (PID « virtuel » 0x408E, O `HIDPP20_DEVICE_QUIRK_TABLE`) | idem | idem |
| C355 | G515 Lightspeed TKL (virtuel 0x40B4) | 0x8071 + 0x8081, HID++ 2.0 dynamique | page 0xFF00 usage 2 ; Bluetooth page 0xFF43 |

**G Pro X TKL et G Pro X (clavier) : PID INCONNU** (absents de toutes les sources lues). Le champ `modelId` du FW info (0x0003) ou le nom (0x0005) permettrait de les identifier à l'exécution ; ne pas les ajouter à une liste de PID sans confirmation matérielle.

Détecteur générique d'OpenRGB (O lignes 1770-1796) : page 0xFF00 usage 2 = HID++ long moderne ; page 0xFF43 (tout usage) = claviers G213/G512/G610/G810/G813/G815/G910/G Pro ; page 0xFFA0 usage 1 = Centurion (G522, PRO X 2). Sous Windows, **usage 2 est la collection qui accepte les écritures HID++ longues** (commentaire OpenRGB, ligne 1773).

### 3.2 Codes de touches : table CONFIRMÉE (source O + G, concordantes)

Il y a deux espaces de codage différents selon la famille. Ne pas les confondre.

**A. Famille 0x8081 / rapports G815-G915 (G813, G815, G915, G915 TKL, G515, claviers modernes).** Code de LED (« zone », un octet) = **usage clavier HID − 3** pour le bloc principal. Sources : (O) `RGBController_LogitechG815.cpp` lignes 270-290 et 330-360 ; `RGBController_LogitechG915.cpp` lignes 450-490 ; table équivalente `hidpp20_key_name_to_zone` dans `RGBController_LogitechHIDPP20.cpp` lignes 162-310 (qui cite les KEYCODES de Solaar).

| Groupe | Code de LED sur le fil | Remarque |
|---|---|---|
| Lettres A à Z | 1 à 26 (A=1 … Z=26) | usage 0x04..0x1D moins 3 |
| 1 à 9, 0 | 27 à 36 | |
| Entrée, Échap, Retour arrière, Tab, Espace | 37, 38, 39, 40, 41 | |
| - = [ ] | 42, 43, 44, 45 | |
| `\` ANSI | 46 | ISO : `#` = 47 et la touche `\` ISO = 97 |
| ; ' ` , . / | 48, 49, 50, 51, 52, 53 | |
| Verr Maj | 54 | |
| F1 à F12 | 55 à 66 | |
| Impr écran, Arrêt défil, Pause | 67, 68, 69 | |
| Inser, Début, Pg préc | 70, 71, 72 | |
| Suppr, Fin, Pg suiv | 73, 74, 75 | |
| Flèches droite, gauche, bas, haut | 76, 77, 78, 79 | |
| Pavé : Verr Num, /, *, -, +, Entrée | 80, 81, 82, 83, 84, 85 | |
| Pavé 1 à 9, 0, . | 86 à 94, 95, 96 | |
| Menu (touche contexte) | 98 | |
| Positions 99 à 103 (POWER, = pavé, F13, F14, F15) | 99 à 103 | décodées par OpenRGB, aucune carte connue ne les équipe |
| Ctrl G, Maj G, Alt G, Win G | 104, 105, 106, 107 | = usage 0xE0..0xE3 moins 0x78 |
| Ctrl D, Maj D, Alt D, Win D (Fn droite) | 108, 109, 110, 111 | usage 0xE4..0xE7 moins 0x78 |
| Luminosité (indicateur) | 153 (0x99) | PRO X RAPID : 150 |
| Lecture/pause, Muet, Suivant, Précédent | 155, 156, 157, 158 (0x9B..0x9E) | PRO X RAPID : 152 (lecture), 154 (suivant) |
| Touches G1 à G5 | 180 à 184 (n + 0xB3) | |
| Logo | 210 (0xD2 = 1 + 0xD1) | |

La numérotation des touches hors bloc principal est **propre à la carte** (OpenRGB signale que la même étiquette peut avoir un autre id sur le PRO X RAPID). L'énumération réelle (§3.4) prime sur cette table.

**B. Famille 0x8080 (G410, G512, G513, G610, G810, G910, G Pro) via rapports 0x11/0x12.** Adresse = (type de touche u16 BE, id u8). Source (G) `Keyboard.h` lignes 108-170 et `Keyboard.cpp` lignes 580-640 ; (O) `LogitechHIDPP20Controller.h` lignes 100-118 et 175-180.

| keyType (masque) | Groupe | keyId |
|---|---|---|
| 0x0001 | clavier | **usage HID** : A=0x04 … Z=0x1D, 1..9,0 = 0x1E..0x27, Entrée 0x28, Échap 0x29, Retour 0x2A, Tab 0x2B, Espace 0x2C, F1..F12 = 0x3A..0x45, flèches 0x4F..0x52, pavé 0x53..0x63, `\` ISO 0x64, Menu 0x65, modificateurs 0xE0..0xE7 |
| 0x0002 | multimédia | Suivant 0xB5, Précédent 0xB6, Stop 0xB7, Lecture 0xCD, Muet 0xE2 |
| 0x0004 | touches G | G1 à G9 = 0x01 à 0x09 |
| 0x0008 | boutons | INCONNU (listé comme type, ids non documentés ici) |
| 0x0010 | logo | 0x01 (et 0x02 sur G910 : « logo2 ») |
| 0x0040 | indicateurs / autres | rétroéclairage 0x01, mode jeu 0x02, Verr Maj 0x03, Arrêt défil 0x04, Verr Num 0x05 |

Particularités confirmées par les sources : G910 sur-déclare le bit multimédia ; G410 n'accepte que les ids 1 et 2 pour le type 0x40 ; G512 ABNT2 : la touche `/` ABNT2 s'allume en 0x88 et non 0x87 (« hardware-verified » selon un commentaire OpenRGB, `RGBController_LogitechHIDPP20.cpp` lignes 618-628).

### 3.3 Longueurs et séquences de trames

**G815 / G813 : rapports fixes (O `LogitechG815Controller.cpp`)**, device_index 0xFF, tout en 20 octets, écriture puis lecture (timeout 300 ms) sur le handle usage 0x0602 :

| Action | Trame (octets 0..) |
|---|---|
| Commit | `11 FF 10 7F` + zéros |
| Init mode direct, étape 1 | `11 FF 08 3E` |
| étape 2 | `11 FF 08 1E` |
| étape 3 | `11 FF 0F 1E`, octet 0x10 = `01` |
| étape 4 | `11 FF 0F 1E`, octet 4 = `01`, octet 0x10 = `01` |
| Touche amorce | `11 FF 10 1F 29 00 00 00` (code brut 0x29 envoyé tel quel, couleur noire ; OpenRGB n'applique pas ici le moins 3) puis commit, pour « engager » le direct |
| Trame « little » | `11 FF 10 1F` + jusqu'à 4 x `[code R G B]` ; si moins de 4, le code suivant = `FF` (fin) |
| Trame « big » | `11 FF 10 6F R G B` + jusqu'à 13 codes de touches de CETTE couleur ; fin = `FF` |
| Mode matériel | `11 FF 0D 3D <zone> <mode> R G B ...` (octet 0x3D marqué « à vérifier » par OpenRGB) |

Zones de mode : 0 clavier, 1 logo, 2 multimédia, 3 touches G, 4 modificateurs. Modes : 0 off, 1 static, 2 breathing, 3 cycle, 4 wave, 5 direct. Vitesse BE16 (valeur x100 ms) : breathing octets 9-10 + luminosité octet 12 = 0x64 ; cycle octets 11-12 + octet 13 ; wave octets 11-12, direction octet 13, octet 15 = vitesse haute (pour G213 voir plus bas).

**G915 / G915 TKL / G515 (O `LogitechG915Controller.cpp`)** : idem mais device_index/idx dynamiques (G915 filaire : dev 0xFF, 4522=0x0E, 8040=0x13, 8071=0x09, 8081=0x0A ; sans fil : dev 0x01, 4522=0x0F, 8040=0x14, 8071=0x0A, 8081=0x0B). Séquence « direct » (octets fn, tous en 20 octets) :
1. feature 0x4522, fn `3E` puis fn `1E` (paramètres vides) : poignée de main de désactivation de touches par usage.
2. feature 0x8071, fn `1E` avec octet 0x10 = `01` (cluster 0 → effet off, persist).
3. feature 0x8071, fn `1E`, octet 4 = `01`, octet 0x10 = `01` (cluster 1 → off).
4. touche amorce + commit (feature 0x8081, fn `7F` = FRAME_END, swId F).
5. Boucle : trames `1F` (4 touches) / `6F` (13 touches) sur 0x8081 (taille de données de 4 à 16 octets), puis `7F`.
Trames de mode : feature 0x8071, fn `1E` (SET_EFFECT, swId E) : `[zone, mode, R, G, B, ...]`, octet 0x10 = `01` (persist) ; static : octet 9 = `02` ; ripple : octet 11 = vitesse basse. Zones G915 : logo 0, clavier 1, multimédia 2, touches G 3, modificateurs 4 (différent de G815). Modes G915 : 0 off, 1 static, 2 breathing, 3 cycle, 4 wave, 5 ripple, 0xFF direct. Avant un changement de mode matériel : `4522` fn `3E`/`1E` puis 8071 `5E 01 03 07` (SW_CONTROL mode 3, flags 7).

**Famille 0x8081 générique (O `LogitechHIDPP20Controller.cpp`)** : GET_INFO fn 0x00 paginé : requête `[0x00, page, 0x00]`, page 0..2 ; réponse = bitmap de 112 bits par page (14 octets à partir de data[2], bit de poids faible d'abord) ; `zone_id = page*112 + octet*8 + bit` ; zone 0 ignorée ; zone > 255 inatteignable (lignes 3006-3084). SET_INDIVIDUAL fn 0x10 : 4 entrées `[zone, R, G, B]` max par trame longue ; SET_RANGE fn 0x50 : `[début, fin, R, G, B]` x 3 par trame (zones absentes ignorées) ; FRAME_END fn 0x70 : 16 octets nuls, attendre l'ACK de la même fonction (lignes 5081-5264). Amorçage clavier : mettre chaque cluster 0x8071 à l'effet 0 (off) avec persist = 1, envoyer 1 touche (première zone énumérée, noire) puis FRAME_END (lignes 4440-4480).

**Famille 0x8080 (O `LogitechHIDPP20Controller.h` 100-118 ; G `Keyboard.cpp` 1062-1131)** : SET_KEY_COLORS = fn 3 sur rapport 0x12 (64 octets) : `12 FF <idx> 3A 00 <keyType> 00 <count> ` puis `count` x `[keyId R G B]` ; **14 touches max** (8 + 14x4 = 64). Premier octet de l'adresse = type : `00 01` clavier avec count `0E`, `00 02` multimédia avec `05`, `00 40` indicateurs avec `05`, `00 10` logo en rapport 0x11 (20 octets) avec count `01`. Commit : fn 5 `5A` (`11 FF 0C 5A` G213/G410/G512/G513/G610/G810/G Pro ; `11 FF 0F 5D` G910 ; `11 FF 10 7F` G815). Index dans les trames de g810-led : 0x0C pour G410..G810/G Pro ; G910 : logo et clavier en 0x0F (`3A`/`3D`), touches G `3E` ; **valeurs d'index différentes par modèle : interroger IRoot**.
Transport g810-led (libusb, Linux) : `SET_REPORT` classe 0x21/0x09, wValue 0x0211 (rapport 0x11) ou 0x0212 (rapport 0x12), interface 1, lecture de la réponse sur l'endpoint interrupt 0x82 (G `Keyboard.cpp` lignes 1025-1060). Sous Windows via hidapi : `hid_write` sur la collection page 0xFF43 (dont les deux handles 0x11/0x12 chez OpenRGB).

**G213 (O `LogitechG213Controller.cpp`)** : 5 régions numérotées 1 à 5 (g810-led `setAllKeys` boucle `rIndex` 0x01 à 0x05). Couleur fixe d'une région : `11 FF 0C 3A <région> 01 R G B 02`. Mode : `11 FF 0C 3C <zone=0> <mode> R G B ...` modes 0 off, 1 static, 2 breathing, 3 cycle, 4 wave ; vitesse x100 BE16 (breathing octets 9-10, octet 12 = 0x64 ; cycle octets 11-12, octet 13 = 0x64 ; wave octets 11-12, direction octet 13, octet 15). Directions wave : gauche 0x06, droite 0x01, centre vers bord 0x03, bord vers centre 0x08.

Effets natifs (G `Keyboard.cpp` 877-1020) : trame 20 octets `11 FF <i1> <i2> <partie> <groupe> R G B période(2) période(2) variante 64 période-haute storage ...` avec `<i1> <i2>` = `0C 3C` (G213/G413), `0D 3C` (G410/G512/G513/G610/G810/G Pro), `0F 1C` (G815 avec préambule `11 FF 0F 5C 01 03 03`), `10 3C` (G910). Parties : 0 clavier, 1 logo, 0xFF toutes. Groupes : 0 off, 1 couleur, 2 respiration, 3 cycle, 4 vagues, 5 ripple. Octet de stockage : 0 = non persistant, 1 = « utilisateur » (persistant).

### 3.4 Activation du mode logiciel et retour au mode matériel (CONFIRMÉ par OpenRGB ; « capture G HUB » selon OpenRGB)

Chaîne générique 0x8071 (O `ClaimSWControlIfNeeded`, lignes 4000-4163 ; `SetSWControl`, lignes 3925-3960) :
1. `SW_CONTROL` = feature 0x8071, fn 0x50, paramètres `[0x01, mode, flags]`. Prise de contrôle observée par OpenRGB : `mode = 3`, `flags = 6` (« two-phase claim »), puis `flags = 5` après la première trame par touche complète. Cycle d'état d'économie d'énergie : 7 (init) → 5 (actif) → 3 (inactif) → 5 (réveil). Signification des bits de `flags` (commentaire OpenRGB tiré d'une capture réseau) : **bit 0 = contrôle des effets, bit 1 = gestion d'alimentation, bit 2 = config non volatile**. G203 Lightsync et G815/G915 utilisent `mode 3, flags 7` (O `LogitechG203LController.cpp`, `LogitechG915Controller.cpp` `InitializeModeSet`).
2. Poignée de main 0x4522 (claviers G815/G915/G Pro) : fn 0x30 puis fn 0x10, charges vides, entre SW_CONTROL et le premier mode (lignes 4264-4294).
3. `SetRGBPowerMode(1)` : feature 0x8071 fn 0x80 `[0x01, mode]`.
4. Bascule en « host mode » : si 0x8101 existe, fn 0x60 `[0x05]` ; sinon si 0x8100 existe, fn 0x10 `[0x02]` (lignes 3978-3998). **Exception G915 TKL** (PID C343 / 408E) : ne PAS passer en host mode, sinon la rangée F (touches G de profil embarqué) casse ; garder le mode embarqué.
5. Ensuite : 0x8081 SET_INDIVIDUAL / SET_RANGE + FRAME_END.

Pour 0x8070 (anciens appareils, G810 famille zones, G560/G933 zones, souris 0x8070) : `SW_CONTROL` = fn 0x80, charge `[1,1]` pour prendre la main, `[0,0]` pour rendre (lignes 2554-2562, 3935-3946). Les claviers 0x8070 qui exposent aussi 0x8080 (G810 famille) écrivent par touche **sans aucune prise de contrôle** ; une prise partielle casse le par-touche sur G810 (FIXME OpenRGB, ligne 4039).

**Restitution au matériel** (arrêt propre, lignes 3845-3895) : arrêter les envois, `SetSWControl(0, 0)`, puis `0x8101` fn 0x60 `[0x03]` (profile management : mode embarqué) ou `0x8100` fn 0x10 `[0x01]` (onboard : mode embarqué) sauf pour le G915 TKL ; casques 0x0620 : fn 0x80 `[0x00]`. g810-led G815 : `11 FF 11 1A <1|2>` = mode embarqué (1) / logiciel (2) (G `Keyboard.cpp` 865-875, `OnBoardMode::board = 1`, `software = 2`).

Autres commandes de G815 vues dans g810-led (G `Keyboard.cpp`, ligne indiquée), utiles mais secondaires : touches MR `11 FF 0C 0C <v>` (G815, l.730) ; MN `11 FF 0B 1C <1|2|4>` (G815, l.760-768) ; mode touches G `11 FF 0A 2B <v>` (G815, l.805) ; démarrage `11 FF 0D 5A 00 01 <wave=1|color=2>` (G213/G410/G610/G810/G Pro) et `11 FF 10 5E 00 01 <..>` (G910).

### 3.5 Collection HID à ouvrir

- G815/G813/G213 : interface 1, page **0xFF43**, usage **0x0602** (rapports 20 octets) ; un second handle (usage 0x0604, 64 octets) est ouvert par OpenRGB pour les rapports 0x12 (O lignes 1530-1532 ; note : le code de détection lit « usage 0x0602 » seulement pour G213 ; le second handle G815 est un fait de `LogitechG815Controller.h`, constructeur `dev_handle_0x11, dev_handle_0x12`).
- G915 / G915 TKL / G515 / souris modernes : page **0xFF00**, **usage 2** (long), usage 1 (court) pour les registres du récepteur.
- G410/G512/G513/G610/G810/G910/G Pro : page 0xFF43 (tout usage) chez OpenRGB ; g810-led écrit sur l'interface **1** (USB).
- Casques G633/G635/G733/G935 : interface 3, page 0xFF43, usage 514 (0x0202) ; G933 pareil.

## 4. Point (2) : souris

PID (O lignes 69-96 et 120-130) : G203 C084 · G203 Lightsync C092 / C09D · G303 C080 · G403 C083 · G403 Hero C08F · G403 Lightspeed C082 (virtuel 405D) · G502 Proteus Spectrum C332 · G502 Hero C08B · G502 Lightspeed C08D (virtuel 407F) · G502 X Plus C095 (virtuel 4099) · G600 C24A (page 0xFF80, non HID++) · G703 C087 (virtuel 4070) / Hero C090 (4086) · G900 C081 (4053) · G903 C086 (4067) / Hero C091 (4087) · G Pro C085 · G Pro Hero C08C · G Pro Wireless C088 (virtuel 4079) · Powerplay C53A (virtuel 405F). G102, G305 : **PID INCONNU** (absents des sources lues ; G102 est probablement un GLightsync mais non confirmé). Un « PID virtuel » est celui qu'un récepteur annonce pour l'appareil appairé.

### 4.1 Feature d'effet

CONFIRMÉ : les souris à LED utilisent **0x8070** (anciennes) ou **0x8071** (récentes) ; l'index est découvert via IRoot (O `LogitechProtocolCommon.cpp` lignes 696-760 : `RGB_EFFECTS1` = 0x8070, `RGB_EFFECTS2` = 0x8071). Table de fonctions :

| | 0x8070 (Solaar/OpenRGB `LOGITECH_FP8070`) | 0x8071 (`LOGITECH_FP8071`) |
|---|---|---|
| GET_INFO | 0x00 (infos), 0x10 (infos zone), 0x20 (infos effet) | 0x00 (toutes lectures par paramètres) |
| SET_EFFECT | 0x30 | 0x10 |
| NV config | 0x40 / 0x50 | 0x30 |
| SW_CONTROL | 0x80 (`[1,1]` / `[0,0]`) | 0x50 (`[1, mode, flags]`) |
| Autres | 0x60 bin, 0x70 GET_SW_CTL, 0x90 statut, 0xA0 clear, 0xB0 dir, 0xC0 colour, 0xD0 sync, 0xE0 get effect, 0xF0 set bin | 0x40 BIN_INFO, 0x60 SYNC, 0x70 PWR_CFG, 0x80 PWR_MODE, 0x90 SHUTDOWN |

Énumération (O lignes 2574-2706) : 0x8071 GET_INFO `[0xFF, 0xFF, 0x00]` → data[0] numRgbZones (clusters), data[1-2] capacités étendues BE16, data[3-4] effectBlockCount BE16, data[5] supportedClusterIndex (indexation depuis data[2] de la trame complète). Info cluster : `[cluster, 0xFF]` → emplacement BE16 (1 Primary, 2 Logo, 3 Left Side, 4 Right Side, 5 Combined, 6 à 11 Primary 1 à 6) et nombre d'effets. Info effet : `[cluster, index, 0, 0]` → [2-3] id d'effet BE16, [4-5] masque de capacités, [6-7] période par défaut ms.

Id d'effets (O `LogitechProtocolCommon.h` 83-92 et `SetZoneEffect` 5645-5810) : 0x0000 off, 0x0001 static, 0x0003 spectre/cycle, 0x0004 wave, 0x0005 star, 0x0007 audio, 0x000A breathing, 0x000B ripple, 0x000C custom, 0x000E decomposition, 0x000F / 0x0010 signature 1/2, 0x0015 cycle (variante saturation), 0x0016 wave (saturation), 0x0017 ripple (saturation).

### 4.2 Trame d'effet / couleur fixe (CONFIRMÉ, O)

`SET_EFFECT` (0x8071 fn 0x10 ; 0x8070 fn 0x30), 16 octets de charge : `[cluster, effectIdx, 10 octets de paramètres, persist]` avec **persist à l'index 12 de la charge** (bits 1-0 = persistance, bits 3-2 = alimentation ; 0 = volatil ; 1 = enregistré). Pour la couleur fixe : `effectIdx` = index (dans la liste énumérée du cluster) de l'effet d'id 0x0001 ; octets 2-4 = R, G, B ; octet 5 = 0x02 si la couleur n'est pas noire (marqueur « couleur fixe »).
Paramètres (10 octets, charge[2..11]) : breathing 0x000A : RGB (2-4), période BE16 ms (5-6), luminosité 0-100 (8) ; spectre 0x0003 : période BE16 (7-8), luminosité (9) ; ripple 0x000B : RGB (2-4), période BE16 (6-7) ; wave 0x0004 (sans saturation) : période basse (8), direction 1..8 (9), luminosité 1..100 (10), période haute (11) ; audio 0x0007 : octet 2 = 0 couleur fixe / 1 cycle, RGB 3-5, période 7-8.

Anciennes souris à index fixes codés par OpenRGB (O lignes 473-526, `LogitechGLightsyncController.cpp`) : G203, G303, G403 : idx **0x0E**, octet de fonction `0x3A`, device_index 0xFF ; G Pro (C085/C08C) : idx 0x0E, octet `0x3C`. Trame (long) : `11 FF 0E 3A <zone> <mode> R G B ...` ; modes Lightsync : 0 off, 1 static, 2 cycle, 3 breathing ; static : octet 9 = `02` ; cycle : vitesse BE16 octets 11-12, luminosité octet 13 ; breathing : vitesse octets 9-10, luminosité octet 12. Mode direct pour ces modèles : rapport COURT `10 FF 0E 8A 01 01` = sans écriture en flash (« direct »), `... 8A 00 00` rend la main. G203 Lightsync (C092/C09D), idx 0x0E et 0x12 (O `LogitechG203LController.cpp`) : SW_CONTROL `11 FF 0E 50 01 03 07` ; mode `11 FF 0E 10 00 <mode> ...` ; couleur d'une LED `11 FF 12 10 <led> R G B FF` ; commit (FRAME_END) `11 FF 12 70`. G600 : feature report de 8 octets `F1 R G B <mode> <vitesse>` (non HID++), page 0xFF80.

**Valeurs `mode`/`flags` demandées** : SW_CONTROL 0x8071 `[1, 3, 7]` (init anciens appareils) ; `[1, 3, 6]` puis `[1, 3, 5]` (prise de contrôle fluide et actif) ; `[1, 3, 3]` (inactif) ; `[1, 0, 0]` libération ; 0x8070 `[1,1]` / `[0,0]`. `mode` = 3 dans toutes les prises observées ; la signification de `mode` seule reste **INCONNU** (seulement 0 et 3 vus).

### 4.3 Powerplay et récepteur Lightspeed

Powerplay C53A et récepteur Lightspeed C539 : même détection (page 0xFF00 usage 1, interface 2). Un mutex est nécessaire quand un Powerplay et la souris appairée partagent le récepteur (O). Pour la souris appairée, device_index = indice 1..6 donné par le récepteur (voir §5).

## 5. Point (3) : casques et récepteurs Lightspeed

**Casques** (O lignes 101-107, 1553, 1763-1768) : G633 0A5C · G635 0A89 · G733 0AB5 / 0AFE / 0B1F · G933 0A5B · G935 0A87 ; interface 3, page 0xFF43, usage 514 (0x0202), HID++ longs 20 octets, device_index 0xFF, idx fixes pour G933 : `11 FF 04 3A <zone> <mode> R G B ..` (modes 0 off, 1 direct, 2 cycle, 3 breathing ; octet 9 = 0x02 pour direct), « direct » = `11 FF 04 CA <zone>`, off = `11 FF 04 3F <zone>` ; l'écriture est refaite jusqu'à 3 fois avec lecture de 20 ms.

**Casques Centurion** (G522 Lightspeed : USB 0B19, dongle 0B18, G321/G315 etc. ; page 0xFFA0 usage 1) : feature **0x0620** (O `LogitechHIDPP20Controller.h` 127-137 ; `.cpp` lignes 4000-4036 et 5840-5912) : fn 0x00 GET_INFO, 0x10 GET_RGB_ZONE_INFO, 0x20 SET_INDIVIDUAL, 0x30 SET_CONSECUTIVE, 0x40 SET_RANGE, 0x50 SET_SINGLE_VALUE, 0x60 FRAME_END, 0x70 GET_HOST_MODE_STATE, 0x80 SET_HOST_MODE_STATE.
- Prise de contrôle : fn 0x80 `[0x01]` (collante). Libération : `[0x00]`.
- Couleur : fn 0x50 `[R, G, B, n, zone1..zonen]` (une trame par couleur distincte, n jusqu'à 12) puis FRAME_END fn 0x60 `[0x01, 0, 0, 0]` ; **0x01 uniquement** (0x00 est ignoré, 0x02 « cassé » sur G522 selon OpenRGB).
- G522 : 8 LED, deux écouteurs 2x2 ; identifiants de zone : gauche `8 7 / 4 3`, droite `6 5 / 2 1` (vue extérieure).
La famille 0x0600 (CENTURION_RGB) partage le même schéma de fonctions que 0x8071 (O lignes 2549-2552).

**Récepteurs** (O `LogitechControllerDetect.cpp` 113-119 et `LogitechProtocolCommon.cpp` 38-141) : Unifying C52B / C532 · Nano C52F · G700 C531 · G602 C537 · **Lightspeed C539** (interface 2, page 0xFF00 usage 1 pour les registres, usage 2 pour HID++ long) · Powerplay C53A. Enchaînement d'énumération des appareils appairés (registres HID++ 1.0, device_index 0xFF) :
1. Lire le registre 0x00 (`10 FF 81 00 00 00 00`) : bit 0 de data[1] = notifications sans fil ; sinon l'écrire à 1 (`10 FF 80 00 00 01 00`).
2. Lire le registre 0x02 (`10 FF 81 02 ...`) : data[1] = nombre d'appareils appairés.
3. Écrire `10 FF 80 02 02 00 00` : le récepteur rejoue de fausses notifications de connexion, une par appareil (+1 pour le récepteur). Chaque notification : device_index (1 à 6), PID sans fil = data[2]<<8 | data[1], bit 0x40 de data[0] = appareil appairé mais non lié (en veille).
4. Nom (registre 0xB5 sous-registre 0x4N) et numéro de série (0xB5, 0x3N) lisibles même appareil endormi.
Un récepteur Lightspeed n'accepte qu'un appareil ; un Unifying jusqu'à 6.
Tous les accès à ce récepteur doivent être sérialisés (mutex partagé), PureRGB devra le faire.

## 6. Point (4) : G410 / G810 / G910 et la feature 0x8080

CONFIRMÉ (O + G, concordants) : adressage (keyType u16, keyId u8) ; keyId = **usage HID** pour keyType 0x01 (voir §3.2 B). Tailles : 14 touches par trame 0x12 (64 octets). Commit = FLUSH_LEDS fn 0x50. Enumération par appareil : fn 0x00 GET_INFO (masque de types), fn 0x10 GET_KEY_TYPE_INFO, fn 0x20 GET_KEY_COLORS (paginé 14 par trame). Pour les touches particulières (rangée logo/indicateurs/multimédia) des G810/G610, OpenRGB ajoute un bandeau de touches propre au modèle ; la table d'ids par modèle des touches spéciales hors typeFlags 0x01 est **INCONNU** (se déduit par énumération fn 0x20). Mode de démarrage G810 : `11 FF 0D 5A 00 01 <1 wave|2 color>`.

## 7. Point (5) : sécurité (écritures à ne pas envoyer)

Faits tirés de OpenRGB (aucun accès mémoire flash de G HUB observé) :
- Le byte persist de SET_EFFECT (charge[12] = 1) et l'octet « stockage » des effets natifs g810-led (valeur 1) **enregistrent dans la mémoire embarquée** : toujours 0 pour le pilotage temps réel (O ligne 5819-5823 ; G `NativeEffectStorage`).
- Les trames d'amorçage de OpenRGB mettent les clusters sur « off » avec persist = 1 (O `LogitechHIDPP20Controller.cpp` lignes 4447-4454 ; `InitializeDirect` G815/G915 : octet 0x10 = 1) : ceci **écrit en non volatile** ; à éviter ou tester d'abord sur le matériel.
- Ne pas utiliser sans raison : 0x8071 fn 0x30 (NV_CONFIG), 0x40 (BIN_INFO), 0x60 (SYNC), 0x70 (PWR_CONFIG), 0x90 (SHUTDOWN) ; 0x8070 fn 0x40/0x50 (CFG), 0x60 (BIN_INFO), 0xF0 (SET_BIN_INFO) ; 0x8100 / 0x8101 (profils embarqués : LOAD, READBUFFER) ; toute écriture de profil ou mise à jour de firmware (non analysées). 0x8101 fn 0x60 est une bascule de mode, pas une écriture de profil (d'après l'usage qu'en fait OpenRGB).
- Écrire `01 03 07` sur 0x8071 SW_CONTROL puis ne jamais rendre la main laisse l'appareil sans effets matériels : rendre toujours le contrôle (`[1,0,0]`, 0x8100/0x8101) à la fermeture, y compris sur kill brutal (prévoir un mécanisme de rattrapage, par exemple un réarmement au prochain lancement).
- Passer en host mode sur un G915 TKL casse les touches G/F : conserver le mode embarqué (§3.4).
- Gestion de l'énergie : flags 5 ↔ 3 déclenchent des événements d'activité ; envoyer des trames pendant un fondu de mise en veille peut réveiller ou ne pas être pris en compte selon le modèle (quirk G515 : écritures acceptées).
- Rapports 0x11 vs 0x12 : un rapport trop court ou mal ciblé est ignoré ; les erreurs HID++ 1.0 reviennent avec feature_idx 0x8F.
- Cadence : sondage de l'ACK de FRAME_END (O : 10 ms, 3 ms en approche) ; au plus HIDPP20_PERKEY_WINDOW paquets non acquittés, sinon le firmware perd des réponses sur USB. Valeur exacte INCONNU ici.

## 8. Ce qui est INCONNU (ou non confirmé)

- Tout le contenu de G HUB (définitions de périphériques JSON, plugins, ressources, séquences propres) : INCONNU, installeur en ligne (§1).
- PID des G Pro X TKL, G Pro X, G102, G305, G512 SE, G413 SE/TKL, G Pro X Superlight, etc. : INCONNU.
- Collections HID exactes du G413 (C33A) et de G Pro (C339) : INCONNU.
- Signification de `mode` du SW_CONTROL 0x8071 au-delà des valeurs 0 et 3 : INCONNU.
- Id des touches spéciales (G810/G610) et du type `0x08` « boutons » de 0x8080 : INCONNU.
- Cadence maximale réelle par modèle : INCONNU (à mesurer).
- Valeur de `HIDPP20_PERKEY_WINDOW` : INCONNU (non lue).
- Octet `0x3D` du mode matériel G815 (marqué à vérifier par OpenRGB) : non confirmé.
- Format de l'erreur HID++ 2.0 (0xFF) : non revérifié dans cette session.
- Si G HUB envoie des commandes absentes d'OpenRGB (profils, scripts) : INCONNU.

## 9. Difficulté de réimplémentation et tests d'octets sans matériel

Difficulté : **moyenne**. Le cadre HID++ est uniforme ; le travail se concentre sur la découverte IRoot, la prise/restitution de contrôle, la table de zones par appareil (énumérée) et les récepteurs. Ordre facile → difficile : G815/G915 (0x8081, table usage − 3) · G203 Lightsync et GLightsync (0x8070/0x8071 + SET_EFFECT 16 octets) · G213 (5 régions) · 0x8080 (G410..G910) · casques 0x0620 · récepteur et Powerplay.

Tests d'octets possibles sans matériel :
1. `11 FF 10 7F` + 16 zéros = 20 octets (commit G815).
2. Trame « little » G815 : `11 FF 10 1F` + 4 x `[code R G B]` = 20 octets ; code de la touche A = 0x01, Échap = 0x26 (usage 0x29 − 3 = 0x26), Verr Num = 80, Ctrl G = 104, logo = 210.
3. Trame 0x8080 SET_KEY_COLORS à 14 touches : `12 FF <idx> 3A 00 01 00 0E` + 14 x 4 octets = 64.
4. Trame SET_EFFECT static 0x8071 : `11 <dev> <idx> 1<S> <cluster> <idx_effet> R G B 02 ...` avec octet de charge 12 = 00 (volatil).
5. IRoot : `11 FF 00 0<S> 80 71` (feature 0x8071) / `80 70` / `80 81` / `80 80`.
6. SW_CONTROL : `11 <dev> <idx8071> 5<S> 01 03 05` (actif), `01 00 00` (libération) ; 0x8070 : fn `8<S>` `01 01` / `00 00`.
7. Validation du calcul de zone : assert `usage − 3` pour tout le bloc principal, `− 0x78` pour les modificateurs, `+ 0xB3` G-keys, `+ 0xD1` logo.

## 10. Nettoyage

Installeur et extraits de `D:\Projet\Analyses\rgb\logitech\` supprimés ; clones tiers `D:\Projet\Analyses\rgb\_src\` et `openrgb-src\` laissés en place (ils appartiennent à d'autres analyses).

## 7. Complément 2026-10-25 : récepteurs LIGHTSPEED

Aucune nouvelle analyse de G HUB n'a été faite dans ce passage (pas de téléchargement). L'état du §5 reste : séquence de découverte des récepteurs (`10 FF 80 02 02 00 00`, notifications de connexion, mutex partagé, un seul appareil par récepteur Lightspeed) recoupée avec OpenRGB et Solaar, non confirmée dans G HUB ; pas d'octet inconnu nouveau levé. Danger inchangé : ne jamais écrire de registre de jumelage/appairage du récepteur (déconnecte l'appareil) ; aucune écriture de profil sur mémoire embarquée n'est recommandée tant que la feature de profil embarqué n'est pas lue.

## 11. Passage 2026-10-26 (branche logi3) : PRO X / PRO X 2 LIGHTSPEED et Sharkoon

**G HUB toujours NON analysable** : aucun installeur hors ligne officiel (recherche web refaite : seul `lghub_installer.exe` en ligne, connexion requise [V]). La charge utile passe par le service d'accès `/pipeline/v2/access/` : la récupérer = rejouer un jeton d'accès, écarté (cf. §1). Rien téléchargé, rien extrait, Ghidra/REA non lancés (rien à décompiler : le stub ne contient aucun descripteur d'appareil).

| Question | Résultat |
|---|---|
| PRO X 2 LIGHTSPEED (046D:0AF7) a-t-il un éclairage piloté ? | **INCONNU.** Les fiches revendeur (Micro Center, Currys, etc.) ne mentionnent ni LIGHTSYNC ni LED [V, absence de mention seulement, pas une preuve]. Indice contraire [S] : OpenRGB range « PRO X 2 » dans la famille Centurion (page 0xFFA0 usage 1, feature 0x0620 `HEADSET_RGB_HOSTMODE`), mais ce libellé de détection ne prouve pas la présence de LED sur ce modèle. Page officielle Logitech inaccessible (HTTP 403). |
| PRO X Wireless (046D:0ABA) | **INCONNU** (aucune source ; ancien PRO X filaire/Lightspeed, sans LED connue). |
| Trames HID++ exactes | **INCONNU** pour ces deux PID : rien d'implémenté. Les trames 0x0620 du §5 (issues d'OpenRGB, G522) ne sont pas à rejouer sur 0AF7 sans confirmation matérielle (énumération 0x0620 GET_INFO / GET_RGB_ZONE_INFO en lecture seule, à faire par Momo sur la tour). |
| `NO_RGB` (`rgb_catalog.rs`) | **Laissé tel quel** : l'analyse ne prouve ni la présence ni l'absence de RGB (consigne : ambigu = ne pas toucher). |

**Sharkoon 1EA7:0064 / 0066 (souris 2.4G)** : (mis à jour : voir analyse-empire-gaming.md, classés sans RGB) ancien constat : **aucune source.** Aucun logiciel officiel Sharkoon relié à ces PID trouvé (Drakonia / Skiller existent pour des modèles filaires ; seule mention de 1ea7:0066 = fil Arch Linux où usbhid suffit, sans lien avec un logiciel RGB). Rien téléchargé. Les PID restent hors catalogue (test existant `rgb_catalog`).
