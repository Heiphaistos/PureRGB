# Lian Li (L-Connect 3)

Date de la recherche : 2026-10-21. Legende : **[V]** = verifie dans la source citee, **[S]** = suppose / deduit, **[?]** = inconnu.
Clones lus pour cette fiche : `sgtaziz/lian-li-linux` @f55f3e5 (MIT), `liquidctl/liquidctl` @292dc56 (GPL-3.0+), `andyjusa/hydroshift2-rs` @5b5c95d (MIT).

## Logiciel officiel

- **L-Connect 3** (Windows 64 bits seulement). Page officielle : <https://lian-li.com/l-connect3/> (liens AWS/Aliyun) ; changelog <https://lian-li.com/l-connect3/l3-changelog/> ; beta <https://lian-li.com/l-connect3-beta-2/>. Le miroir `l.lian-li.com` est perime (v2.0.3). Versions vues dans la recherche : 2.1.14 sur l'en-tete, 2.1.29 au changelog. [V: recherche web, non telecharge]
- Les assets LCD personnalises se (re)exportent depuis Parametres > General avant mise a jour.

## Modeles d'ecran et VID:PID

Source principale : `crates/lianli-shared/src/device_id.rs` du depot `sgtaziz/lian-li-linux` [V] (tables `DeviceEntry`), recoupe avec `crates/lianli-devices/src/hydroshift_lcd/mod.rs`.

| Produit | Ecran | Transport | VID:PID | Statut dans le depot ref. |
|---|---|---|---|---|
| Galahad II LCD (Vision v1.4) | 480x480 rond | HID (3 types de rapports) | 0416:7391 | teste (liquidctl : 0416:7395 "LianLi-GA_II-LCD_v1.4") |
| Galahad II Vision | 480x480 | HID | 0416:7395 | teste |
| HydroShift LCD / LCD RGB / LCD TL | 480x480 | HID | 0416:7398 / 7399 / 739A | teste |
| HydroShift II LCD Circle | 480x480 | USB bulk "WinUSB" chiffre | 1CBE:A021 (mode bureau : 1A86:AD20) | teste |
| HydroShift II LCD Square | 480x480 | idem | 1CBE:A034 | teste |
| HydroShift II OLED Curve | 1080x2288 | idem (LCD) + HID LED | 1CBE:A068 (LED : 0416:8051) | teste |
| UNI FAN TL LCD (cable) | 400x400 | HID | 0416:7372 (hub TL) [V pour le hub] | teste |
| UNI FAN TL V2 / SL V3 LCD (sans fil) | 400x400 | bulk 1CBE:0006 (TL V2) / 1CBE:0005 (SL V3) + dongle | dongles 0416:8040/8041 et 1A86:E304/E305 | teste |
| TL Flex LCD | 400x400 | bulk | 1CBE:A018 | teste |
| SL Infinity Flex LCD | 400x400 | bulk | 1CBE:A019 | NON teste |
| Lancool 207 Digital | 1472x720 (720x1472 en portrait) | bulk | 1CBE:A065 (mode bureau 1A86:ACD1 / AD11) | teste |
| Universal Screen 8.8" | 1920x480 | bulk | 1CBE:A088 (mode bureau 1A86:ACE1 / AD21) ; anneau LED 0416:8050 | LCD non marque teste |
| Vision 9.2" | 1920x464 | bulk | 1CBE:A092 (mode bureau 1A86:AD26) | teste |

Notes [V] :
- Les ecrans "VID 1CBE" ont un second mode "bureau" (CH340, VID 1A86, PID dans 0xAD10..0xAD3F) ou ils se comportent comme un moniteur secondaire Windows (protocole "TURZX", voir plus bas). Le depot de reference sait basculer d'un mode a l'autre (`display_switcher.rs`, commande 0x96 vers desktop).
- Les modeles sans fil exigent aussi une liaison USB pour le LCD ; l'appairage radio initial se fait dans L-Connect 3.
- Pas d'ecran : UNI FAN SL-INF, Galahad II Trinity (RGB/pompe seulement).

## Protocole : etat de la documentation

**Documente publiquement, mais seulement sous forme de code source MIT** (pas de spec ecrite). Fichiers a lire (MIT, reutilisables en clean-room ou meme sous licence MIT avec attribution) :

| Famille | Fichiers (depot `sgtaziz/lian-li-linux`, branche par defaut) | Contenu [V] |
|---|---|---|
| Galahad II LCD / Vision, HydroShift LCD | `crates/lianli-devices/src/hydroshift_lcd/{protocol,initialization,controller}.rs` | 3 types de rapports HID : A (ID 1, 64 o : handshake 0x81, pompe 0x8A, ventilo 0x8B, firmware 0x86), B (ID 2, 1024 o, entete 11 o : LCD control 0x0C, H264 0x0D, JPEG 0x0E), C (ID 3, 512 o, firmware >= 1.2). Entete B/C : `[id][cmd][taille totale BE32][n paquet BE24][len BE16]` puis donnees. 480x480, 24 fps. |
| 1CBE:* (HydroShift II, TL/SL LCD, Lancool 207, Universal Screen, Vision 9.2) | `crates/lianli-devices/src/crypto.rs`, `winusb/lcd/` | Entete 500 o chiffre DES-CBC (cle ASCII publique dans le code `slv3tuzx`), enveloppe 512 o finie par `A1 1A`, commandes 0x65 JPG, 0x66 PNG, 0x0D rotation, 0x0E luminosite, 0x33 horloge, 0x79/0x7B lecture H264, 0xFB pompe/ventilo (CRC16-CCITT), 0xFC anneau LED (TinyUZ). Confirme independamment par `andyjusa/hydroshift2-rs` (README, section "Protocol notes"). |
| Mode "bureau" 1A86:AD10..AD3F | `crates/lianli-devices/src/turzx/` | Protocole TURZX decode depuis `LIANLI_display_driver.dll` : magic 0xAF, flux A/B (0x6C/0x6D, 0x68/0x69), commit 0x66, formats MJPEG 0x0111 / H264 0x0112. Le depot mentionne `target/TURZX.md` (absent du clone). |

**Recoupement important [V]** : le VID 1CBE est celui des ecrans **Turing / TURZX USB** (`mathoudebine/turing-smart-screen-python` @2b33ab4, GPL-3.0, `library/lcd/lcd_comm_turing_usb.py` : VID 0x1CBE, PID 0x0028 = 480x480 rond, 0x0046, 0x0050, 0x0080, 0x0088 = 480x1920, 0x0092 = 462x1920, 0x0123 ; meme cle DES `slv3tuzx`, memes IDs de commande 101/102 = 0x65/0x66, H264 17/121/122). Donc **les ecrans Lian Li "1CBE" sont l'OEM TURZX** et un seul driver PureRGB (celui que l'agent Turing/XuanFang peut deja ecrire pour le mode USB) couvrirait aussi HydroShift II, Lancool 207, Universal Screen 8.8", Vision 9.2", TL/SL LCD. A coordonner avec le fichier Turing. (Ne pas copier le code GPL du projet Turing : decrire le protocole depuis la doc et nos captures.)

Confiance : **elevee** pour Galahad II LCD/Vision et HydroShift LCD (HID, JPEG, 480x480, teste sur materiel par l'auteur ; liquidctl 1.16 le confirme cote HID et note que l'ecran attend un flux H.264 continu via HID, `docs/lian-li-ga-ii-lcd-guide.md` section Screen). **Moyenne** pour les 1CBE (teste par l'auteur, mais un seul depot, protocole chiffre DES avec cle statique).

Note de licence : liquidctl (GPL-3.0+) ne doit pas etre copie ; `lian-li-linux` est MIT. Le projet affirme avoir utilise l'IA pour analyser captures USB et logiciel constructeur : a recouper par nos propres captures avant de s'y fier pour les champs non decrits.

## A decouvrir par analyse du logiciel constructeur (REA)

1. **Galahad II LCD / HydroShift LCD (priorite 1)** : confirmer par capture les trames JPEG (type B/C), rechercher le meilleur debit (JPEG vs H264, 24 fps), les commandes non utilisees (`_CMD_GET_FANLIGHT_INFO` 0x84, serie 0x88/0x89, reset 0x8E), les regles de bascule "mode LCD / mode bureau", le comportement quand L-Connect tourne en meme temps (conflit d'acces HID).
2. **1CBE (HydroShift II, TL Flex, Lancool 207, Universal Screen, Vision 9.2)** : verifier que la cle DES est bien constante par modele ; formats de luminosite/rotation ; resolution reelle par PID ; SL Infinity Flex (non teste).
3. **Hors couverture publique** : Lancool 217 Wireless, V150 Wireless, UNI FAN CL/RL120 (non testes), protocole radio des dongles (pas necessaire pour l'ecran si USB direct).

Methode proposee :
- Installer L-Connect 3 dans une VM Windows (le depot de ref. dit que ca marche avec passthrough USB) ; repertoire d'installation a analyser (a confirmer apres installation, [?]). Cibles probables : l'exe principal Electron/.NET de L-Connect et la DLL `LIANLI_display_driver.dll` (nom cite par le depot de ref. pour le mode bureau) ; verifier avec `rea inspect_pe_resources`/`binary_overview` s'il s'agit de .NET (decompilation managee) ou d'Electron (`analyze_javascript_application`).
- Chaines a chercher : `7391`, `7395`, `7398`, `1CBE`, `slv3tuzx`, `DES`, `LCD`, `Hydro`, `JPG`, `H264`, `A11A`, `0x0E`.
- Capture : USBPcap sur le bus du controleur de l'AIO (filtre `usb.idVendor == 0x0416`), puis `inspect_pcap_capture` (REA) ; scenarios : apparition d'une image fixe, changement de luminosite, rotation, lecture d'une video, passage du mode LCD a bureau.
- Validation : rejouer les trames sur materiel ; tests d'octets construits a la main d'apres la capture.

## Effort / priorite

Voir `README.md`. Resume : HID Galahad II/HydroShift = effort **moyen**, priorite **1** (le plus documente et le plus repandu). Ecrans 1CBE = effort **moyen-eleve** (chiffrement DES + enveloppes), priorite **2**.
