# Razer (Synapse / Chroma SDK) : protocole USB des périphériques

Document d'interopérabilité. Aucun code ni asset Razer n'est copié ; seuls des faits (VID/PID, octets, codes de commande) sont consignés.

## Source analysée

- **Synapse 3/4 : NON analysé.** L'installeur officiel est un stub en ligne (télécharge ses composants) et pose pilotes/services ; interdit d'installer (consigne). Rien n'a donc été extrait du logiciel Razer lui-même. Les faits ci-dessous viennent de projets tiers de rétro-ingénierie qui ont documenté le protocole (le firmware parle le même protocole que Synapse) :
  - **OpenRazer**, `github.com/openrazer/openrazer`, commit `477f4d6` : `driver/razercommon.h`, `razercommon.c`, `razerchromacommon.c`, `razerkbd_driver.c`, `razermouse_driver.c`.
  - **OpenRGB**, `gitlab.com/CalcProgrammer1/OpenRGB`, commit `bbc7747` : `Controllers/RazerController/RazerDevices.h`, `RazerControllerDetect.cpp`, `RazerController/RazerController.{h,cpp}`.
  - **Razer Chroma SDK REST** : `assets.razerzone.com/dev_portal/REST/html/index.html` (page d'index seule lue ; les sous-pages ont répondu 403).
- Copies de travail : `D:\Projet\Analyses\rgb\_src\` (hors dépôt).
- SHA-256 d'installeur : sans objet (rien téléchargé).
- Les valeurs extraites par résumé automatique de pages web (OpenRazer `razerchromacommon.c`, `razerkbd_driver.c`) sont marquées (R) = à recouper dans la source avant d'implémenter.

## Matériel couvert

VID Razer **0x1532** (Bluetooth 0x068E). Source des PID : OpenRGB `RazerDevices.h` (~270 PID). Table condensée par famille.

### Claviers (VID 1532)
| PID | Modèle | PID | Modèle |
|---|---|---|---|
| 0203 | BlackWidow Chroma | 0226 | Huntsman Elite |
| 0209 | BlackWidow Chroma TE | 0227 | Huntsman |
| 0211 | BlackWidow Overwatch | 0243 | Huntsman TE |
| 0216 / 021A | BlackWidow X Chroma / TE | 0257 | Huntsman Mini |
| 0221 | BlackWidow Chroma V2 | 0266 / 026B / 026C | Huntsman V2 Analog / TKL / V2 |
| 0228 | BlackWidow Elite | 0282 | Huntsman Mini Analog |
| 0235 / 0237 | BlackWidow Lite / Essential | 02A6 / 02A7 | Huntsman V3 Pro / TKL blanc |
| 0241 | BlackWidow 2019 | 02E4 | Huntsman V3 HE Mini |
| 024E | BlackWidow V3 | 021E / 025D | Ornata Chroma / V2 |
| 0258 / 0271 | V3 Mini filaire / sans fil | 028F / 02A1 / 02A3 | Ornata V3 / Rev2 / TKL |
| 025A / 025B / 025C | V3 Pro filaire / BT / sans fil | 0294 / 02A2 | Ornata V3 X / X Rev2 |
| 0A24 | BlackWidow V3 TKL | 022A / 023F / 025E | Cynosa Chroma / Lite / V2 |
| 0287 / 028D / 0293 | BlackWidow V4 / V4 Pro / V4 X | 0204 | DeathStalker Chroma |
| 02B3 / 02B4 | V4 Pro 75% filaire / sans fil | 0295 | DeathStalker V2 |
| 02A5 | V4 75% filaire | 0290 / 0292 | DS V2 Pro sans fil / filaire |
| 02D7 / 02D5 | V4 TKL filaire / sans fil | 0296 / 0298 / 0297 | DS V2 Pro TKL sans fil / filaire / BT |
| 02D4 / 02D2 | V4 LowProfile TKL filaire / sans fil | 0207 / 0208 / 022B / 0244 | Orbweaver Chroma, Tartarus Chroma / V2 / Pro |

Portables Blade / Blade Pro / Stealth / Book (clavier intégré, même protocole matriciel) : 020F, 0210, 0224, 0225, 022D, 022F, 0232, 0233, 0234, 023A, 023B, 0240, 0245, 0246, 024A, 024C, 024D, 0252, 0253, 0255, 0256, 0259, 026A, 0268, 026D, 026F, 0270, 0276, 0279, 027A, 028A, 028C, 029D, 0205, 0220, 0239.

### Souris (VID 1532)
Abyssus 0020/0042/005B/005E/006A/006B, Atheris récepteur 0062, Basilisk 0064/0065/0083/0085/0086/0088/0099/00AA/00AB/00AC/00B9/00CB/00CC/00CD/00D6/00D7/00D8/00E2/00E3, Cobra 00A3/00AF/00B0, DeathAdder 0016/0037/0038/004F/0054/0043 (Chroma)/005C (Elite)/006E/0071/0084/0098/008C/007C/007D, Diamondback 004C, Imperator 002F, Lancehead 0059/005A/0060/006F/0070, Mamba 0024/0025/0044/0045/0046/0069/006C/0072/0073, Naga 002E/003E/003F/0040/0041/0036/0050/0053/0067/008D/008F/0090/0093/00A7/00A8/00E7/00E8, Orochi 0013/0039/0048, Ouroboros 0032, Taipan 0034, Viper 0078/008A/0091/007A/007B. Détail modèle par PID : `RazerDevices.h`.

### Tapis, casques, accessoires (VID 1532)
- Tapis / socles : Firefly 0C00, V2 0C04, V2 Pro 0C08, HyperFlux 0068, Goliathus Chroma 0C01, Extended 0C02, 3XL 0C06, Strider 0C05, Mouse Dock 007E, Mouse Dock Pro 00A4, Base Station 0F08, Base Station V2 0F20, Charging Pad 0F26, Laptop Stand 0F0D / 0F2B, Mouse Bungee V3 0F1D.
- Casques : Kraken classic 0501/0506, Kraken 0504, Kraken V2 0510, Ultimate 0527, V3 HyperSense 0533, V3 X 0537, V3 0549, Kitty 0F19 / 0F21 / 0560 / 0554, V4 filaire / sans fil 056B / 056C, Kitty V3 Pro 0587 / 0588, Tiamat 7.1 V2 0F03, Nommo Chroma / Pro 0517 / 0518, Leviathan V2 / V2X 0532 / 054A.
- Divers : Chroma Addressable RGB Controller 0F1F (ARGB, 6 canaux), Chroma HDK 0F09, Mug 0F07, PC Case Lighting Kit 0F0E, Core 0215, Core X 0F1A, O11 Dynamic 0F13, Seiren Emote 0F1B, Thunderbolt 4 Dock 0F21, TB5 Dock 0F52, Hanbo 0F35.
- Les casques Kraken, le contrôleur ARGB 0F1F (2 interfaces, rapports 90 et 320 octets) et Hanbo ont des contrôleurs OpenRGB distincts (RazerKraken*, RazerHanbo) **non lus ici**.

### Interface HID à ouvrir (OpenRGB `RazerControllerDetect.cpp`)
Exemples constatés : BlackWidow Chroma / V2 / Elite / 2019 / V3 Pro : interface **0x02**, usage page **0x01**, usage **0x02** ; BlackWidow V3 : interface 0x03, usage page 0x0C, usage 0x01. Varie par modèle : relever la table complète avant d'implémenter. Côté OpenRazer le transfert se fait en contrôle (wIndex) : "usually 0x02, 0 for Firefly" ; 0x03 pour Naga X / Basilisk V3 / V3 35K.

## Protocole

### Trame (struct `razer_report`, 90 octets) : OpenRazer `razercommon.h`
| Offset | Taille | Champ |
|---|---|---|
| 0 | 1 | status (0x00 nouveau, 0x01 occupé, 0x02 OK, 0x03 échec, 0x04 timeout, 0x05 non supporté) |
| 1 | 1 | transaction_id (bits 7-5 device, bits 4-0 id) |
| 2-3 | 2 | remaining_packets (big-endian) |
| 4 | 1 | protocol_type (toujours 0) |
| 5 | 1 | data_size |
| 6 | 1 | command_class |
| 7 | 1 | command_id (bit 7 = direction : 0x80 = lecture) |
| 8-87 | 80 | arguments |
| 88 | 1 | crc |
| 89 | 1 | reserved (0) |

- **Checksum** (OpenRazer `razer_calculate_crc`, source lue) : XOR des octets d'index **2 à 87 inclus** ; octets 0, 1 et 88 exclus.
- **Transport OpenRazer** : SET_REPORT `bRequest=0x09`, `bmRequestType=0x21`, `wValue=0x0300`, `wIndex` = interface ; réponse par GET_REPORT `bRequest=0x01`, `bmRequestType=0xA1`, `wValue=0x0300`. Attente entre envoi et lecture : valeur par modèle (macros `RAZER_*_WAIT_US`, valeurs non lues).
- **Transport OpenRGB** : `hid_send_feature_report` / `hid_get_feature_report` avec la structure précédée d'un octet report-id (champ `report_id`, valeur `report_index` par modèle : INCONNUE ici). Temporisations : 1 à 2 ms avant envoi, 5 ms avant lecture de la réponse (`RazerController.cpp`, fonctions `razer_get_*`).
- **Retry OpenRazer** (`razermouse_driver.c`, (R)) : jusqu'à 5 tentatives, 10 ms entre échecs ; réponse acceptée si command_class / command_id / remaining_packets correspondent et status = SUCCESSFUL ou BUSY.
- **transaction_id par modèle** (OpenRazer `razerkbd_driver.c`, (R), varie même par fonction) : **0x3F** (BlackWidow Lite, Ornata, Cynosa, BlackWidow V3, DeathStalker V2, Chroma V2...), **0x1F** (BlackWidow Elite / V4, Huntsman V2 / V3, Ornata V3, Tartarus V2 / Pro, Joro...), **0x9F** (variantes sans fil V3 Pro / V4 Mini et TKL HyperSpeed / DS V2 Pro), **0xFF** (défaut : la plupart des autres, famille Blade).

### Init / mode logiciel
- **Set device mode** : classe 0x00, cmd 0x04, data_size 2, `arguments[0]` = mode, `arguments[1]` = 0x00. Modes : **0x00 normal/matériel**, **0x03 driver/logiciel** (OpenRGB : `HARDWARE=0x00`, `SOFTWARE=0x03`). OpenRazer n'accepte que 0x00 et 0x03. Get : classe 0x00, cmd 0x84, size 2.
- Autres lectures (constantes OpenRGB) : firmware cmd 0x81, série 0x82, info clavier 0x86, connexions sans fil 0xBF ; commande état LED classe 0x03 cmd 0x00 (args : stockage 0=RAM / 1=sauvé, LED id, valeur).
- LED id (OpenRGB) : 0x01 molette, 0x03 batterie, 0x04 logo, 0x05 rétroéclairage, 0x07 macro, 0x08 jeu, 0x0C-0x0E profil R/V/B, 0x10 côté droit, 0x11 côté gauche, 0x1A-0x1F canaux ARGB 1-6.
- Séquence minimale pour un pilotage direct (déduite, non rejouée sur matériel) : (1) device mode 0x03 ; (2) écrire les lignes de la trame personnalisée ; (3) envoyer l'effet "custom frame" pour l'afficher ; (4) à la sortie, device mode 0x00.

### Effets matériels : classe / commande (OpenRazer `razerchromacommon.c`, (R))
| Famille | Classe / cmd | Détail |
|---|---|---|
| Standard (anciens claviers) | 0x03 / 0x0A | arg0 = id d'effet : none, spectrum (size 1), wave (size 2, direction 1-2), static (size 4, RGB args 1-3), reactive (size 5, vitesse 1-4, RGB 2-4), starlight (type 1/2/3), breathing (size 8, type 1 simple / 2 double / 3 aléatoire), customframe (size 2). Valeurs numériques des macros `MATRIX_EFFECT_*` : INCONNUES. |
| Étendu (matrice récente) | 0x0F / 0x02 | effet **0x00** none, **0x01** static (size 9, RGB args 6-8), **0x02** breathing (aléatoire size 6 ; simple size 9 avec arg3=1 ; double size 12 avec arg3=2), **0x03** spectrum, **0x04** wave (arg3 direction 0-2, arg4=0x28), **0x05** reactive (arg4 vitesse 1-4, RGB 6-8), **0x07** starlight (arg4 vitesse 1-3, arg5 1 simple / 2 double), **0x08** custom frame, **0x0A** wheel (arg3 direction 1-2, arg4 0x28) |
| Souris étendu | 0x03 / 0x0D | effet : 0x00 none, 0x06 static (RGB args 3-5), 0x04 spectrum, 0x02 reactive (vitesse arg3 1-4, RGB 4-6), 0x03 breathing (arg3 : 1 simple, 2 double, 3 aléatoire) |
| Luminosité | 0x03 / 0x03 (get 0x83) ; 0x0F / 0x04 (get 0x84) matrice étendue ; 0x0E / 0x04 Blade ; 0x07 / 0x02 dock | size 3 (standard / étendu), 2 (Blade), 1 (dock) |

### Couleur par touche (trame personnalisée) : (R)
- **Standard** : classe 0x03, cmd **0x0B**, size 0x46 ; arguments = `[0xFF (id de frame), ligne, colonne_début, colonne_fin, R,G,B, R,G,B ...]`. 0x46 = 70 = 4 + 22 x 3.
- **Étendu** : classe 0x0F, cmd **0x03**, size 0x47 par défaut ; arguments = `[0, 0, ligne, colonne_début, colonne_fin, R,G,B ...]`. 0x47 = 71 = 5 + 22 x 3.
- Une trame d'image = une commande par ligne de la matrice (6 lignes pour 22 colonnes), puis effet "custom frame" (0x03/0x0A ou 0x0F/0x02 effet 0x08) pour afficher.
- Ordre des octets couleur : R, G, B (OpenRazer et OpenRGB). Dimensions par modèle (lignes, colonnes, zones, ARGB) : `RazerDevices.cpp` d'OpenRGB (373 Ko), non extraites. Types de matrice OpenRGB : none / standard / étendue / linéaire / ARGB étendue / custom.

### Chroma SDK (API officielle, côté logiciel)
- REST : `http://localhost:54235/razer/chromasdk` (HTTP) ou `https://chromasdk.io:54236/razer/chromasdk` ; POST d'enregistrement d'application renvoie l'URI d'instance ; GET donne la version du SDK. Source : page index REST.
- **Heartbeat** : le serveur coupe la connexion après **15 s** sans activité ; PUT heartbeat périodique (exemple de la doc : toutes les secondes).
- Types d'effet (CHROMA_CUSTOM, CHROMA_CUSTOM_KEY, CHROMA_STATIC...), dimensions (la FAQ dit CHROMA_CUSTOM = canevas 2D ; le changelog évoque une grille clavier 8 x 24 depuis le SDK 3.05) et format couleur : **NON confirmés** par les pages lues (403 sur les sous-pages) ; ne pas supposer 6 x 22 ni BGR sans lecture de la doc. Sources : `developer.razer.com/works-with-chroma/chroma-faq/`, `developer.razer.com/?p=783`.
- Le SDK passe par Synapse (service installé) : pour un pilote natif PureRGB, viser le HID direct ci-dessus, pas le SDK.

## Cadence
Rien de mesuré. OpenRGB insère 1-5 ms de délai par commande ; une trame clavier = une commande par ligne (6) + 1 commande d'affichage, soit ~10-20 ms par image. Cadence maximale stable : INCONNUE (à mesurer).

## Ventilateurs / pompes
Sans objet (périphériques). Le Chroma ARGB Controller 0F1F pilote des bandes ARGB, pas de ventilateur.

## Accès bas niveau
HID/USB uniquement, pas de SMBus. Aucun pilote Razer requis pour écrire des rapports feature sur la bonne interface. Sous Windows, Synapse détient déjà les interfaces ; PureRGB devra gérer le conflit (Synapse arrêté).

## Ce qui est INCONNU
- Octet report-id exact et usage page / interface de chaque modèle (table OpenRGB à relever).
- Valeurs numériques `MATRIX_EFFECT_*` du mode standard ; tailles de matrice par modèle.
- Protocole des casques Kraken, Hanbo, contrôleur ARGB 0F1F (non lus).
- Format couleur, dimensions et types d'effets du Chroma SDK REST.
- Détail des variantes sans fil (transaction_id lu seulement pour 0x9F).
- Contenu réel du logiciel Synapse (non analysé).

## Estimation de difficulté
**Facile à moyenne** pour claviers / souris / tapis : trame fixe 90 octets, CRC XOR, même protocole sur ~270 PID. La difficulté est la table par modèle (transaction_id, interface, dimensions). Casques et ARGB : moyenne, non documentés ici.

## Tests d'octets possibles sans matériel
1. Construire "set device mode 0x03" (classe 0x00, cmd 0x04, size 2, args 03 00), calculer le CRC (XOR octets 2..87) et vérifier taille 90 et octet 88 = XOR.
2. Construire une ligne de trame étendue (0x0F/0x03, size 0x47) pour 22 colonnes : 5 + 66 = 71 = 0x47, CRC cohérent.
3. Vérifier que tout transaction_id utilisé est dans {0x1F, 0x3F, 0x9F, 0xFF}.
4. Comparer la table PID ci-dessus à `RazerDevices.h`.
