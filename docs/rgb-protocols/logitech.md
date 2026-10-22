# Logitech G HUB : HID++ 2.0 et RGB des périphériques G

Document d'interopérabilité. Aucun code ni asset Logitech copié ; seuls des faits de protocole.

## Source analysée

- **G HUB : NON analysé.** L'installeur (`lghub_installer.exe`) est un stub en ligne qui installe services et pilotes ; interdit d'installer (consigne), et le stub ne contient pas les binaires. Aucune extraction du logiciel officiel n'a été faite.
- **Spécification HID++ publique de Logitech : NON trouvée.** La recherche web ne retourne aucun document officiel pour les features 0x8071 / 0x8081 (confirmé : seulement des listes de features et du code tiers).
- Faits tirés de code tiers de rétro-ingénierie (lus localement, `D:\Projet\Analyses\rgb\_src\`) :
  - **OpenRGB** commit `bbc7747`, `Controllers/LogitechController/` : `LogitechControllerDetect.cpp`, `LogitechProtocolCommon.h`, `LogitechG815Controller/`, `LogitechGLightsyncController/`, `LogitechHIDPP20Controller/LogitechHIDPP20Controller.{h,cpp}` (très complet : 0x8071, 0x8080, 0x8081, 0x0620 casques).
  - **Solaar** commit `e7304c4`, `lib/logitech_receiver/hidpp20_constants.py` (table des feature IDs).
  - **Noyau Linux** `drivers/hid/hid-logitech-hidpp.c` (IDs de rapport, tailles ; section LED 0x8071 non atteinte dans la partie lue).
  - **g810-led** commit `efa3c35` (claviers G410/G610/G810/G910/G213 : rapports 0x11).
  - **hidpp** (cvuchener) commit `474ac10` et **libratbag** `8235b5b` clonés, non exploités faute de temps (pistes pour compléter).
- Les valeurs marquées (O) viennent d'OpenRGB lu directement ; (S) = Solaar ; (K) = noyau Linux.

## Matériel couvert

VID Logitech **0x046D** (X56 : 0x0738 Saitek/Mad Catz). PID de OpenRGB `LogitechControllerDetect.cpp` (O).

| PID | Modèle | Type | Famille de protocole |
|---|---|---|---|
| C336 | G213 | clavier | zones, rapports 0x11 |
| C232 | G813 | clavier | par touche (0x11/0x12) |
| C33F | G815 | clavier | par touche, rapports 0x11 usage 0x0602 / 0x0604 |
| C33E / C541 / C547 | G915 filaire / récepteur / récepteur 2 | clavier | idem G815 |
| C343 / C545 | G915 TKL filaire / récepteur | clavier | idem |
| C355 | G515 Lightspeed TKL | clavier | HID++ 2.0 |
| C084 / C092 / C09D | G203 / G203 Lightsync (x2) | souris | HID++ 2.0 zones |
| C080 / C083 / C08F / C082 | G303 / G403 / G403 Hero / G403 Lightspeed | souris | zones |
| C332 / C08B / C08D / C095 | G502 Proteus Spectrum / Hero / Lightspeed / X Plus | souris | zones |
| C24A | G600 | souris | spécifique |
| C087 / C090 | G703 Lightspeed / Hero | souris | zones |
| C081 / C086 / C091 | G900 / G903 / G903 Hero | souris | zones |
| C085 / C08C / C088 | G Pro / G Pro Hero / G Pro Wireless | souris | zones |
| C53A | Powerplay (tapis) | tapis | zones |
| 0A5C / 0A89 / 0AB5 / 0AFE / 0B1F | G633 / G635 / G733 (x3) | casque | zones |
| 0A5B / 0A87 | G933 / G935 | casque | zones |
| 0B19 / 0B18 | G522 Lightspeed USB / dongle | casque | Centurion (feature 0x0600 / 0x0620) |
| C52B / C52F / C531 / C532 / C537 / C539 | Récepteurs Unifying 1/2, Nano, G700, G602, Lightspeed | récepteur | pont HID++ (device index 1-6) |
| 405D, 407F, 4070, 4086, 4053, 4067, 4087, 4079, 405F, 4099, 40B4 | PID "virtuels" renvoyés par un récepteur (G403/G502/G703/G900/G903/G Pro Wireless/Powerplay/G502 X Plus/G515) | souris | via récepteur |
| 0738:2221 / 0738:A221 | X56 joystick / manette des gaz | contrôleur | spécifique |

Interfaces : claviers G815-like = usage page **0xFF43**, usage **0x0602** (rapports 20 octets) et **0x0604** (64 octets), deux handles à ouvrir (O). Souris et autres : interface vendeur 0xFF00 (HID++ court/long).

## Protocole

### Cadre HID++ (K, O)
| Rapport | ID | Taille |
|---|---|---|
| Court | **0x10** | 7 octets |
| Long | **0x11** | 20 octets |
| Très long | **0x12** | 64 octets |

Format : `[report_id][device_index][feature_index][function<<4 | swId][params...]`.
- `device_index` : **0xFF** pour un appareil filaire/direct (`LOGITECH_DEFAULT_DEVICE_INDEX`) ; 1 à 6 pour un appareil derrière un récepteur (O, K).
- `swId` (4 bits) : identifie l'émetteur ; le firmware renvoie la valeur reçue. OpenRGB prend 0x07 ; le noyau Linux 0x01 ; Solaar cycle 0x02-0x0F ; 0x00 = firmware (O).
- `feature_index` est obtenu à l'exécution : **IRoot (index 0x00), fonction 0**, params `[featureId_hi, featureId_lo]` → réponse `[index, type, version]` ; index 0 = absent (O, `LOGITECH_HIDPP_PAGE_ROOT_IDX`). Les index diffèrent par appareil : toujours interroger.
- Erreur : réponse avec feature_index 0xFF (HID++ 2.0 standard ; non revérifié dans cette session).

### Features RGB / utiles (S ; IDs sûrs)
0x0001 FEATURE_SET · 0x0003 DEVICE_FW_VERSION · 0x0005 DEVICE_NAME · 0x1000/0x1004 batterie (unifiée) · 0x1300 LED_CONTROL · 0x1981/0x1982/0x1983 BACKLIGHT 1/2/3 · 0x1990 ILLUMINATION · 0x2201 ADJUSTABLE_DPI · 0x8040 BRIGHTNESS_CONTROL · 0x8060 REPORT_RATE · **0x8070 COLOR_LED_EFFECTS** · **0x8071 RGB_EFFECTS** · **0x8080 PER_KEY_LIGHTING** · **0x8081 PER_KEY_LIGHTING_V2** · 0x8100 ONBOARD_PROFILES · 0x8101 PROFILE_MANAGEMENT · casques Centurion : 0x0600 HEADSET_RGB_EFFECTS, 0x0620 HEADSET_RGB_HOSTMODE, 0x0621 ONBOARD_EFFECTS, 0x0622 SIGNATURE_EFFECTS, 0x0635 RGB_STREAMING.

Note : un résumé web avait inversé les noms 0x8070/0x8071 ; Solaar et OpenRGB concordent : 0x8070 = COLOR_LED_EFFECTS, 0x8071 = RGB_EFFECTS.

### Feature 0x8071 (RGB_EFFECTS, clusters de zones) : (O, fonctions = octet de fonction déjà décalé)
| Octet fn | Nom | Remarque |
|---|---|---|
| 0x00 | GET_INFO | infos et nombre de clusters |
| 0x10 | SET_EFFECT | `[cluster, effectIdx, 10 octets de paramètres, persist à l'index 12]` (16 octets) |
| 0x20 | SET_PATTERN | |
| 0x30 | NV_CONFIG | config non volatile |
| 0x40 | BIN_INFO | |
| 0x50 | SW_CONTROL | `[0x01, mode, flags]` : prise de contrôle logiciel |
| 0x60 | SYNC | |
| 0x70 / 0x80 | PWR_CONFIG / PWR_MODE | gestion d'alimentation |

0x8070 (anciens appareils) : SET_EFFECT = fn **0x30**, SW_CONTROL = fn **0x80**, format simple `[activé, persist]` ; envoyer `[1,1]` pour prendre la main, `[0,0]` pour rendre (O).

Paramètres d'effet (10 octets, `data[2..11]`, O) :
- **Static (id 0x0001)** : R, G, B aux octets 2-4 ; octet 5 = 0x02 si couleur non noire.
- **Breathing (id 0x000A)** : RGB (2-4), période BE16 ms (5-6), luminosité 0-100 (octet 8).
- **Cycle/spectre (id 0x0003)** : période BE16 ms (7-8), luminosité 0-100 (octet 9).
- Audio (id 0x0007) : octet 2 commande, RGB 3-5, période 7-8.
Les ids d'effets disponibles sont énumérés par appareil via GET_INFO (cluster → liste d'effets avec capacités et période par défaut) ; ne pas coder en dur.

### Feature 0x8081 (couleur par zone/touche, mode direct) : (O)
| Octet fn | Nom | Format |
|---|---|---|
| 0x00 | GET_INFO | |
| 0x10 | SET_INDIVIDUAL | jusqu'à **4 entrées** de 4 octets `[zone_id, R, G, B]` par paquet long |
| 0x20 | SET_CONSECUTIVE | |
| 0x30 / 0x40 | SET_DELTA_5BIT / 4BIT | |
| 0x50 | SET_RANGE | `[début, fin, R, G, B]` x 3 par paquet : même couleur sur une plage ; zones absentes ignorées |
| 0x60 | SET_SINGLE_VALUE | |
| 0x70 | FRAME_END | commit de la trame ; 16 octets nuls ; attendre l'ACK (sondage 10 ms, 3 ms en approche) |

Séquence d'OpenRGB pour un clavier "famille" : mettre chaque cluster en effet 0 (off, persist=1), envoyer une touche amorce par SET_INDIVIDUAL puis FRAME_END, ensuite SW_CONTROL ; puis boucle : SET_INDIVIDUAL x N, FRAME_END. Les zone_id sont les usages HID du clavier.

### Feature 0x8080 (par touche, ancienne génération G410/G810/G910) : (O)
- Adressage `(keyType u16 BE, keyId u8)` ; keyId = **usage HID** pour le type clavier. keyTypes (masque de bits, fn0 `typeFlags`) : 0x01 clavier, 0x02 multimédia, 0x04 touches G, 0x08 boutons, 0x10 logo, 0x40 autres/indicateurs.
- fn 0x00 GET_INFO, 0x10 GET_KEY_TYPE_INFO, 0x20 GET_KEY_COLORS, **0x30 SET_KEY_COLORS**, 0x40 SET_ALL_KEYS, **0x50 FLUSH_LEDS** (commit, corps vide).
- SET_KEY_COLORS sur rapport **très long 0x12** : payload `[keyType_hi, keyType_lo, count_hi, count_lo, (keyId,R,G,B) x count]`, **14 touches max par trame**.
- Particularités : G910 sur-déclare le bit multimédia ; G410 n'accepte que keyId 1 et 2 pour le type 0x40 ; G915 TKL doit rester en profil embarqué (O, quirks).

### Claviers G815 / G915 (protocole à rapports fixes sur usage 0x0602) : (O)
Format rapport long : `11 FF <feat> <cmd> ...` (device 0xFF).
- Commit : `11 FF 10 7F`.
- Init direct : `11 FF 08 3E`, `11 FF 08 1E`, `11 FF 0F 1E ... [0x10]=01`, `11 FF 0F 1E [4]=01 [0x10]=01` ; chaque envoi suivi d'une lecture.
- Couleur d'une touche (trame "little" `0x1F`) : `11 FF 10 1F <keycode R G B> x4` (16 octets de données, jusqu'à 4 touches) ; fin de liste par keycode 0xFF.
- Trame "big" `0x6F` : `11 FF 10 6F` suivi de `R G B` puis les keycodes ayant cette couleur, terminé par 0xFF (groupement par couleur unique).
- Mode matériel : `11 FF 0D 3D <zone> <mode> R G B ...` ; zones : 0 clavier, 1 logo, 2 multimédia, 3 touches G, 4 modificateurs ; modes : 0 off, 1 static, 2 breathing, 3 cycle, 4 wave, 5 direct. Breathing : vitesse BE16 aux octets 9-10 (x100), luminosité octet 12 ; cycle : vitesse 11-12, luminosité octet 13. Le commentaire OpenRGB signale un octet `0x3D` "à vérifier pour G815".

### Anciens claviers/zones (g810-led, rapports 0x11 FF) : (source lue partiellement)
Exemples observés dans g810-led `Keyboard.cpp` : commit `11 FF 0C 5A` (variante 0F 5D / 10 7F selon modèle), couleur de région `11 FF 0C 3A <région> 01 R G B`, luminosité/autres `11 FF 0C 0C <v>`, etc. Table complète par modèle : INCONNUE (non extraite).

### Souris Lightsync / zones (G203, G502...) : (O, `LogitechGLightsyncController.cpp`)
Rapport long : `11 <dev_idx> <feat_idx> <fn> <zone> <mode> R G B ...` ; mode static : octet 9 = 0x02 ; cycle : vitesse (BE16, x100) octets 11-12, luminosité octet 13 ; breathing : vitesse octets 9-10, luminosité octet 12. Mode direct : rapport court `10 <dev> <feat> 8A 01 01` active "sans sauvegarde en flash", `... 8A 00 00` rend la main. Index de feature et fn lus à l'init (OpenRGB les passe en paramètre).

### Casques Centurion (G522, etc.) : feature 0x0620 (O)
fn 0x00 GET_INFO, 0x10 GET_RGB_ZONE_INFO, 0x20 SET_INDIVIDUAL, 0x30 SET_CONSECUTIVE, 0x40 SET_RANGE, 0x50 SET_SINGLE_VALUE, 0x60 FRAME_END, 0x70/0x80 GET/SET_HOST_MODE_STATE. Même logique que 0x8081.

## Cadence
- Trames 0x8081 : un FRAME_END par image ; OpenRGB sonde l'ACK toutes les 10 ms (3 ms en approche) et réduit une fenêtre prédite par pas de 5 ms toutes les 32 images pour détecter un firmware plus rapide ; débit réel dépend du modèle (INCONNU, à mesurer).
- 0x8080 : 14 touches par trame 0x12 ; un clavier de 100+ touches = ~8 trames + FLUSH.
- Souris : une commande par zone ; protéger par un mutex quand un Powerplay et la souris appairée partagent le récepteur (observation OpenRGB).

## Ventilateurs / pompes
Sans objet (Logitech G n'a pas de refroidissement ; feature 0x8040 = luminosité).

## Accès bas niveau
HID uniquement. Windows : ouvrir la bonne collection (usage page 0xFF43 pour G815/G915 ; vendeur 0xFF00 pour HID++). Les récepteurs Lightspeed/Unifying exposent plusieurs appareils : sélectionner par **chemin HID**, pas par VID/PID. G HUB (service `lghub_agent`) maintient des handles ; l'accès concurrent est à prévoir.

## Ce qui est INCONNU
- Contenu réel de G HUB (non analysé) : il pourrait utiliser des commandes absentes d'OpenRGB.
- Table complète des ids d'effets 0x8071 par appareil (énumérés dynamiquement).
- Format exact des erreurs HID++ 2.0 et des timeouts à respecter (non revérifié).
- Keycodes complets G815/G915 (table `RGBController_LogitechG815.cpp`, non transcrite).
- Cadence maximale en mode direct par modèle.
- G600, X56, G213, G933 : protocoles spécifiques non détaillés ici.

## Estimation de difficulté
**Moyenne à élevée.** Le cadre HID++ est simple et uniforme, mais chaque famille (0x8070 / 0x8071 / 0x8080 / 0x8081 / G815 fixe / Centurion) a ses fonctions ; la découverte dynamique (IRoot) et les récepteurs ajoutent du travail. Le plus facile : G815/G915 (rapports fixes) puis 0x8081.

## Tests d'octets possibles sans matériel
1. Construire `11 FF 10 7F` + 16 zéros (20 octets) et comparer à la trame commit G815.
2. Construire une trame SET_INDIVIDUAL 0x8081 de 4 entrées : 4 + 16 = 20 octets, fn octet `0x10 | swId`.
3. Construire une trame 0x8080 SET_KEY_COLORS à 14 touches : 4 (en-tête HID++) + 4 + 14 x 4 = 64 octets.
4. Vérifier IRoot : `11 FF 00 0x|swId 80 71` (featureId 0x8071) → octet fn = `0x00 | swId`.
