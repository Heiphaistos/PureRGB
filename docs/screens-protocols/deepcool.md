# DeepCool (DeepCreative)

Date : 2026-10-21. **[V]** verifie dans la source, **[S]** suppose, **[?]** inconnu.
Clones lus : `Nortank12/deepcool-digital-linux` @8ab8338 (GPL-3.0), `zazazaxky-m/deepcool-lcd` @f9c7400 (MIT), `siam-parvez/opencreative` @3e24f04 (MIT/Apache-2.0). Ces deux derniers sont tres recents (octobre 2026) : fiabilite a confirmer par nos propres captures.

## Logiciel officiel

- **DeepCreative** (Windows 10/11). <https://downloads.deepcool.com/> et centre de telechargement <https://www.deepcool.com/downloadpage/> (liens trouves par recherche web, non telecharge). Les anciennes versions sont sous "Previous Releases" sur la meme page. L'ancien logiciel "DeepCool Digital" gere les afficheurs numeriques.

## Deux familles tres differentes

### A. Afficheurs "DIGITAL" (segments / matrice, PAS des LCD pixel)

AK400/500/620 Digital (+SE/PRO/G2 NYX), AG, CH170/270/360/560/690 Digital, LD240/360, LS520/720 SE, LP240/360, LQ240/360 (dashboard), CH510, Morpheus, Assassin IV VC Vision, etc. VID **0x3633** (13875), PID 1..44 selon le modele ; CH510 = VID **0x34D3** (13523) PID **0x1100** (4352). [V : `src/devices/mod.rs` et `device-list/README.md` du depot GPL ; les PID y sont en decimal.] Ce sont des paquets HID courts (cadres de segments) : **hors perimetre du module Ecrans**. Tables d'octets deja publiees (`device-list/tables/*.md`, GPL : a relire, ne pas copier).

### B. Vrais LCD

| Produit | Ecran | VID:PID | Source |
|---|---|---|---|
| **LM240 / LM360** ("LM-Series", 6e gen., mini LCD) | 320x240, RGB565 LE | **3633:0026** [V] | `deepcool-lcd/docs/PROTOCOL.md` + `opencreative/docs/protocol/lm-series.md` |
| **MYSTIQUE 240 / 360** (5e gen.) | 2,8" 640x480 (fiche constructeur vue en recherche web) | PID decimal **9** sous VID 0x3633 dans la liste du depot GPL [V] ; soit 0x0009 [S : conversion] | table d'octets VIDE ("?"), `device-list/tables/mystique-series.md` |

## Protocole LM-Series : documente (communaute, MIT)

Transport [V] : interface 0 classe vendeur 0xFF, endpoint bulk OUT 0x01 (512 o), IN 0x81 non decode. Paquet : `0xAA | len u16 LE | payload | CRC-16/XMODEM u16 LE` (poly 0x1021, init 0). Commandes confirmees sur 5 paquets captures du logiciel constructeur (depot `daedlock/deepcool-lm`, MIT, cite par OpenCreative) :

| Commande | Payload |
|---|---|
| Init / requete | `09` -> paquet `aa 01 00 09 29 91` |
| Luminosite | `06 03` + niveau LE16 (captures : 97 et 29 ; deepcool-lcd envoie 10..100) |
| Bascule ecran on/off ("zen") | `03 00 00 00` |
| En-tete de trame | `00 01` + taille LE32 + `2c 01` (signification de `2c 01` = 300 : INCONNUE) puis 153 600 octets en un seul write bulk |

Confiance : **moyenne-elevee** pour LM360 (teste par l'auteur de deepcool-lcd) ; LM240 "devrait marcher" [S]. Pas de reponses IN decodees, pas de video native decrite (les projets rejouent des images). Rotation par gyroscope de la Mystique : [?].

## A decouvrir par REA

1. **Mystique 240/360** (priorite 2) : tout. Possiblement voisin de LM ([S], rien ne le prouve) mais 640x480 + gyroscope. Discussion citee par le depot GPL : <https://github.com/Nortank12/deepcool-digital-linux/discussions/18>.
2. **LM-Series** : champ `2c 01`, commandes hors captures (fps, orientation, stockage interne GIF ?), reponses IN, differences LM240 / LM360.
3. Afficheurs DIGITAL : hors sujet ici.

Methode :
- Cible : exe **DeepCreative** (chemin d'installation [?] a relever apres installation). Typer avec REA `binary_overview` ; si Electron, `analyze_javascript_application` / `extract_artifact` (asar).
- Chaines : `3633`, `0026`, `MYSTIQUE`, `LM-Series`, `0xAA`, `crc`, `rgb565`, `LCD`, `gyro`.
- Capture : USBPcap, filtre `usb.idVendor == 0x3633`, scenarios : changer de theme, luminosite, retourner la pompe (orientation), video, gif ; `inspect_pcap_capture`.
- Les 5 paquets de `daedlock/deepcool-lm` servent de controle croise (notre capture doit les contenir).

## Effort / priorite

LM-Series : effort **faible** (paquets simples, 1 write bulk par trame, RGB565), priorite **1**. Mystique : effort **eleve** (inconnu), priorite **2**.
