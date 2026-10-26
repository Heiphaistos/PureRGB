# HyperX NGENUITY : protocole USB des périphériques (analyse du logiciel officiel)

Document d'interopérabilité. Aucun code ni asset HyperX/HP copié ; seuls des faits de protocole (octets, identifiants). Marqueurs : **[V]** = lu dans le logiciel
officiel décompilé, **[S]** = déduit, **INCONNU** = non établi. Complète `hyperx.md` (OpenRGB, marqueur (O)).

## Source analysée

- **HyperX NGENUITY (Legacy / « NGENUITY 2 ») 5.38.0.0**, package Microsoft Store `33C30B79.HyperXNGenuity` (éditeur HP Inc.), installé le 2026-10-26 sur ce PC
  (`winget install 9P1TBXR6QDCX --source msstore`), avec l'accord de Momo. Aucun périphérique HyperX n'était branché : **aucun envoi USB réel**.
- Les fichiers du package ont été copiés vers `D:\Tools\analyse\hyperx\pkg` (hors dépôt) puis analysés :
  - `Assets\Native\NGenuity2Helper.exe` est un assemblage **.NET** (non obfusqué) qui contient toute la couche appareils (`NGenuity.Devices\*`, classes
    `CMD…`) : décompilé avec ILSpy (`ilspycmd`), 2630 fichiers C# lus dans `D:\Tools\analyse\hyperx\il`.
  - `NGenuity2.dll` (interface, .NET Native) : analysé avec Ghidra 12.1.4 (listes de chemins HID) ; la liste des appareils se retrouve aussi dans l'assemblage .NET.
  - `Assets\Firmware\*` (images de micrologiciel) : **non lus, non utilisés** (aucune écriture de micrologiciel dans PureRGB).
- Désinstallation après analyse : voir « Nettoyage » en fin de note.

## Cadre commun [V]

Les périphériques classiques utilisent des **feature reports** (`HidD_SetFeature`, octet 0 = identifiant de rapport) ou des **rapports de sortie** ; la
longueur est complétée avec des zéros jusqu'à la longueur des capacités HID de la collection. Les appareils sont repérés par un début de chemin
(`HID#VID_03F0&PID_xxxx&MI_yy[&Colzz]`). Les PID dont le nom NGENUITY contient `DFU` sont des **chargeurs d'amorçage** : ne jamais les revendiquer.

### Schéma « 0x81 » (claviers Alloy, tapis, Haste filaire) — implémenté (`native/hyperx.rs`)

1. Feature report 65 octets d'init : `[0]=00 [1]=04 [2]=F2 [9]=<init9>`.
2. Acquittement : `HidD_GetFeature` (id 0) ; valide si `[2]=F2` et `[4]=01`. NGENUITY abandonne l'image sinon (Haste : pas de lecture).
3. Paquets de LED (65 octets) : octet 0 = identifiant de rapport, puis 16 LED de 4 octets `81 R V B` aux octets `1+4i`. Emplacements non fournis = zéros.

| Modèle | VID:PID (interface) | init9 | LED | paquets | id des paquets de LED |
|---|---|---|---|---|---|
| Alloy Origins | 0951:16E5, 03F0:0591 (MI_03) | 9 | 144 | 9 | 0 |
| Alloy Elite II | 0951:1711, 03F0:058F (MI_03) | 9 | 144 | 9 | 0 |
| Alloy Origins 60 | 0951:1734, 03F0:0C8E (MI_03) | 5 | 80 | 5 | **7** |
| Alloy Origins 65 | 03F0:038F (MI_03) | 5 | 80 | 5 | **7** |
| Pulsefire Mat RGB | 0951:1741, 03F0:0F8D (MI_01) | 2 | 2 | 1 + 1 nul | **4** |
| FURY Ultra | 0951:1705, 03F0:0493 (MI_00) | 2 | 24 | 2 | **4** |
| Pulsefire Haste (filaire) | 0951:1727, 03F0:0F8F (MI_03) | 2 | 1 | 1 + 1 nul | 0 |

Corrections par rapport à `hyperx.md` (OpenRGB) : Origins 60/65 = **80 LED, 5 paquets, init9 = 5** (et non 144/9/9) ; le tapis Pulsefire Mat a **2 LED** (le
schéma OpenRGB à 32 LED reste cohérent avec le FURY Ultra, 24 LED chez NGENUITY) ; l'octet 9 de l'Elite II vaut 9 ; NGENUITY ne met pas de marqueur `81` sur les
emplacements inutilisés. Les identifiants de rapport 4 et 7 sont ceux de NGENUITY (l'init part avec 0) : **[V] dans le code, non éprouvé sur matériel**.
Cadence NGENUITY : 20 images/s (Origins/Elite II/Haste), 24 (Pulsefire Mat). Le code de la boucle du FURY Ultra écrit au-delà de 16 LED par paquet (défaut de NGENUITY) :
PureRGB s'en tient à 16 LED par paquet comme les autres modèles.

Interface : collections repérées par `MI_03` (claviers, Haste), `MI_01` (tapis), `MI_00` (FURY Ultra) ; les pages d'usage ne figurent pas dans NGENUITY (celles des versions HP,
0xFF90, viennent de `hyperx.md`, **[S]**).

### Protocole « Universal » (Pulsefire Haste 2 et famille) — implémenté (`native/hyperx_universal.rs`)

Rapports de sortie de 64 octets (`[0]` = id de rapport, `[1]` = sous-rapport ; les 2 bits de poids fort du sous-rapport désignent le périphérique enfant
d'un récepteur 3-en-1, 0 = appareil principal). Identifiants de requête : `10` infos appareil, `14` profil courant, `20` touches, `22` macros, `30` mode jeu,
`32` DPI, `36` capteur, `40` luminosité, `42` effet préréglé, `44` lumière directe, `46` effets complémentaires, `50` batterie, `52` connexion,
`54` réglages sans fil (veille), `60/62` audio, `64` jauge vocale, `66` égaliseur matériel, `70` récepteur 3-en-1, `F0` chargeur d'amorçage. Réponses : `FF` acquittement
(`[1]` = sous-rapport, résultat), `11, 15, 41, 45, 47, 51, 53, 55, 61, 63, 65, 67, 71`, notifications `FB`. Un masque de profils (u16 petit-boutiste, octets 2-3) dit
dans quels profils enregistrés la valeur est écrite ; **masque 0 = « préréglage applicatif »** (NGENUITY l'emploie en direct, `isSaveToDevice: false`,
et passe de vrais numéros de profil seulement pour « synchroniser vers l'appareil »).

| Trame | Octets [V] |
|---|---|
| Lumière directe, début | `44 01 <n>` n = nombre de rapports de LED |
| Lumière directe, données | `44 02 <i> 00` puis RVB par LED à partir de l'octet 4 (20 LED par rapport, i = numéro du rapport) |
| Luminosité | `40 01 <masque L> <masque H> <255 x % / 100>` |
| Seuil de batterie faible | `50 01 <seuil %> <0 = ne pas enregistrer>` (NGENUITY : 5..25, défaut 10) |
| Batterie (lecture) | requête `50 02` ; réponse id `51` (format non relevé) |
| Veille sans fil | `54 01 <masque L> <masque H> <secondes sur 3 octets petit-boutiste>` (casques ; souris : non utilisé) |
| DPI | `32 01 <masque> <polling> <masque paliers> <palier actif> {u16 (dpi/50-1), R, V, B} x paliers` : table complète, état courant non relu |
| Capteur | `36 01 <masque> <snap> <réglage angle> <angle actif> <distance de décollage>` |
| Mode jeu (clavier) | `30 01 <masque> <actif> <bits : Win=1, Alt+Tab=2, Alt+F4=4, Maj+Tab=8, Ctrl+Échap=16>` |
| Temps de fondu rétroéclairage (clavier) | `46 <SetLEDSleepMode> <masque> <actif> <durée u16>` |

Mappage des modèles (HyperXCenter) : Haste 2 `03F0:0B97` (MI_02) ; Haste 2 Mini Wireless `0BA0` / récepteur `0DA0` (MI_02) ; Haste 2 S `0AB8` / `0BB8` (MI_02) ;
Haste 2 Pro souris `0ABD` (MI_02) / récepteur `0CBD` (MI_03) ; Haste 2 Core Wireless récepteur `0AB5` (MI_02). Toutes : **1 LED**. Haste 2 Wireless
(`0D97`, `0F98`) : deux générations selon la version du micrologiciel (trame de LED sans le rapport de début `44 01` en première génération) : **non revendiqué**.
Alloy Rise / Rise 75 (`0FA0`, `02A1`, `01B9`, `0EB8`) : 130 / 103 LED mais nombre de rapports dépendant de la disposition clavier lue sur l'appareil : non revendiqué.

### Réglages exposés dans PureRGB (RAM, réversibles, désactivés par défaut)

| Appareil | Réglage | Trame | Statut |
|---|---|---|---|
| Haste 2 (tous) | Luminosité de la LED 0-100 % | `40 01 00 00 <255 x % / 100>` | trame [V] ; non persistant [V] par usage NGENUITY (masque 0), pas de preuve matérielle |
| Haste 2 sans fil (Mini, S, Pro, Core) | Seuil d'alerte batterie 5-25 % | `50 01 <seuil> 00` | [V] (octet « enregistrer » à 0) |

### Réglages relevés mais NON implémentés (et pourquoi)

- **DPI, fréquence d'interrogation (125 Hz à 8 kHz), distance de décollage, angle** : le DPI est une table complète (paliers + couleurs) dont l'état courant n'est pas relu ; une valeur
  devinée écraserait les paliers de l'utilisateur. Souris classiques (Surge, Raid, Core, FPS Pro, Dart, Haste sans fil) : commandes « appliquer le profil »/« enregistrer »
  = profil de l'appareil, interdit.
- **Casques** (Cloud Alpha Wireless, Cloud II Wireless, Cloud 2 Core, Cloud 3, Stinger, Ralphie, Flight S…) : sidetone (marche/arrêt, volume), coupure micro, invite vocale,
  extinction automatique, batterie. Exemple Cloud Alpha Wireless [V] : rapport de sortie `21 BB <cmd> <val>` avec cmd `10` sidetone marche/arrêt (val 0/1), `13` invite vocale, `15` micro coupé,
  `12` extinction automatique (val = durée) ; lectures : `05` sidetone, `06` volume du sidetone (jamais écrit par NGENUITY : la commande `11` n'a pas d'octet de valeur dans le code), `07` extinction,
  `0A` micro coupé, `0B` batterie. **Non implémenté** : toutes les collections de casques sont de la forme `MI_03&Colzz` (plusieurs collections sur l'interface) et NGENUITY ne donne pas la
  page d'usage, donc impossible de choisir la bonne collection sans risque d'écrire sur une autre ; longueur du rapport de sortie inconnue. Mêmes raisons pour Cloud 2 Core (`0995`), Cloud 3 (`089D`), etc.
- **Micros** : DuoCast (gain `66 08 <v>` avec v dans {37, 39, 41}, retour micro `66 09`, mute `66 06`, filtre passe-haut `66 07`, diagramme polaire `66 05` 1 = cardioïde, 2 = omnidirectionnel ; rapport de
  sortie 65 octets sur la collection audio `03F0:0A8C MI_03 Col02`) et QuadCast 2/2S (rapports `77 05..07` sur une seconde interface du codec) : collections `Col` sans page d'usage connue : non implémenté.
  Lumière QuadCast S / 2 / 2S / DuoCast : feature `04 F2 .. [9]=1` puis `81 R V B` x N sur la collection secondaire (2 ou 14 LED) : [V] mais collection secondaire à `Col` : non implémenté.
- **Alloy Origins Core** (`0951:16E6`, `03F0:098F`) : rapports de sortie `A2 <n>` de 64 octets, 7 paquets de 60 octets de couleurs avec lecture de la réponse à chaque paquet (disposition par touche
  dans un tableau de 380 octets) : non implémenté. **Alloy Elite / FPS, souris classiques** : le mode « application » s'entre par « appliquer le profil » (persistance non établie) : non implémenté.
  Surge (feature 264 octets, `07 14 00 A0` + plans R/V/B), Core/FPS Pro (`07 0A R V B .. [8]=FF/A0`), Raid (`07 0A` roue + logo `[8]=A0`) : formats lus [V], même réserve.
- Dart / FPS Pro CD / Haste Wireless : rapports de sortie 65 octets `D2 <0 logo|10 roue> 00 08 R V B R V B <luminosité>` (Dart, Haste Wireless) et `D8 .. [4]=06 roue(5-7) logo(8-10) luminosité[11]` (FPS Pro CD),
  chacun suivi d'une lecture de rapport d'entrée : lecture non reproductible avec le cadre actuel (`HidD_GetInputReport`) : non implémenté.

## Liste complète des périphériques (PID à ajouter au catalogue)

Noms exacts utilisés par NGENUITY (`HyperXDeviceModel` / descriptions) : Alloy Elite RGB, Alloy FPS RGB, Alloy Elite 2, Alloy Origins, Alloy Origins Core, Alloy Origins 60,
Alloy Origins 65, Alloy MKW 100, Alloy Rise, Alloy Rise 75, Alloy Rise 75 Wireless, Pulsefire Surge, Pulsefire Raid, Pulsefire FPS Pro, Pulsefire FPS Pro CD, Pulsefire Core, Pulsefire Dart,
Pulsefire Haste, Pulsefire Haste Wireless, Pulsefire Haste 2, Haste 2 Wireless, Haste 2 Mini Wireless, Haste 2 Core, Haste 2 Core Wireless, Haste 2 S, Haste 2 Pro, Pulsefire Fuse Wireless,
Pulsefire Saga, Pulsefire Saga Pro, FURY Ultra, Pulsefire Mat RGB, Cloud Flight S, Cloud Stinger Core Wired / Wireless, Cloud Alpha S, Cloud Alpha Wireless, Cloud Stinger S, Cloud II Wireless
(et DTS), Cloud MIX Buds / Mix Buds 2, Cloud Mix 2, Cloud Stinger 2 Wireless, Cloud Flight Wireless / Flight 2, Cloud II Core Wireless, Cloud III / III Wireless / III S Wireless, QuadCast S,
QuadCast 2, QuadCast 2 S, DuoCast, SoloCast, Vision S, Wireless Embedded Receiver. VID `0951` = Kingston (anciens), `03F0` = HP (actuels). Table complète des chemins HID relevés dans
`HyperXDeviceUtils` (suffixes `DFU` = amorçage, à ne **jamais** revendiquer ; `DONGLE` = récepteur radio). PID revendiqués par PureRGB : voir les deux tableaux ci-dessus ; **à ajouter au catalogue
(`assets/rgb_usb_catalog*.json`, non touché ici)** : tous les PID de cette liste qui n'y figurent pas encore, avec le nom de la colonne « Constante » mis en forme.

| VID:PID | Constante NGENUITY | Interface / collection |
|---|---|---|
| 0951:16BE | KB_ALLOY_ELITE | MI_02&Col05 |
| 0951:16BE | KBN_ALLOY_ELITE | MI_02&Col07 |
| 0951:16BE | KB_ALLOY_ELITE_DFU | MI_01&Col05 |
| 0951:16DC | KB_ALLOY_FPS | MI_02&Col05 |
| 0951:16DC | KBN_ALLOY_FPS | MI_02&Col07 |
| 0951:16E5 | KB_ALLOY_ORIGINS | MI_03 |
| 0951:16E5 | KBN_ALLOY_ORIGINS | MI_01&Col05 |
| 0951:16F5 | KB_ALLOY_ORIGINS_DFU | - |
| 03F0:0591 | KB_ALLOY_ORIGINS_HP | MI_03 |
| 03F0:0591 | KBN_ALLOY_ORIGINS_HP | MI_01&Col05 |
| 03F0:0691 | KB_ALLOY_ORIGINS_DFU_HP | - |
| 0951:16E6 | KB_ALLOY_ORIGINS_CORE | MI_02 |
| 0951:16F6 | KB_ALLOY_ORIGINS_CORE_DFU | - |
| 03F0:098F | KB_ALLOY_ORIGINS_CORE_HP | MI_02 |
| 03F0:0A8F | KB_ALLOY_ORIGINS_CORE_DFU_HP | - |
| 0951:1711 | KB_ALLOY_ELITE_II | MI_03 |
| 0951:1711 | KBN_ALLOY_ELITE_II | MI_01&Col05 |
| 0951:1712 | KB_ALLOY_ELITE_II_DFU | - |
| 03F0:058F | KB_ALLOY_ELITE_II_HP | MI_03 |
| 03F0:058F | KBN_ALLOY_ELITE_II_HP | MI_01&Col05 |
| 03F0:068F | KB_ALLOY_ELITE_II_DFU_HP | - |
| 0951:1734 | KB_ALLOY_ORIGINS_60 | MI_03 |
| 0951:1734 | KBN_ALLOY_ORIGINS_60 | MI_01&Col05 |
| 0951:1735 | KB_ALLOY_ORIGINS_60_DFU | - |
| 03F0:0C8E | KB_ALLOY_ORIGINS_60_HP | MI_03 |
| 03F0:0C8E | KBN_ALLOY_ORIGINS_60_HP | MI_01&Col05 |
| 03F0:0D8E | KB_ALLOY_ORIGINS_60_DFU_HP | - |
| 03F0:038F | KB_ALLOY_ORIGINS_65 | MI_03 |
| 03F0:038F | KBN_ALLOY_ORIGINS_65 | MI_01&Col05 |
| 03F0:048F | KB_ALLOY_ORIGINS_65_DFU | - |
| 0951:1756 | KB_ALLOY_MKW100 | MI_00 |
| 03F0:0B8F | KB_ALLOY_MKW100_HP | MI_00 |
| 03F0:0C8F | KB_ALLOY_MKW100_HP_DFU | MI_00 |
| 0951:16D3 | MS_PULSEFIRE_SURGE | MI_01&Col05 |
| 0951:16D3 | MSN_PULSEFIRE_SURGE | MI_02 |
| 03F0:0490 | MS_PULSEFIRE_SURGE_HP | MI_01&Col05 |
| 03F0:0490 | MSN_PULSEFIRE_SURGE_HP | MI_02 |
| 0951:16E4 | MS_PULSEFIRE_RAID | MI_01&Col05 |
| 0951:16E4 | MSN_PULSEFIRE_RAID | MI_02 |
| 0951:16F9 | MS_PULSEFIRE_RAID_DFU | MI_01&Col05 |
| 03F0:0290 | MS_PULSEFIRE_RAID_HP | MI_01&Col05 |
| 03F0:0290 | MSN_PULSEFIRE_RAID_HP | MI_02 |
| 03F0:0390 | MS_PULSEFIRE_RAID_DFU_HP | MI_01&Col05 |
| 0951:16E2 | MS_PULSEFIRE_DART_MOUSE | MI_01 |
| 0951:16F8 | MS_PULSEFIRE_DART_MOUSE_DFU | MI_01 |
| 0951:16E1 | MS_PULSEFIRE_DART_DONGLE | MI_02 |
| 0951:16F7 | MS_PULSEFIRE_DART_DONGLE_DFU2 | MI_01 |
| 03F0:088E | MS_PULSEFIRE_DART_MOUSE_HP | MI_01 |
| 03F0:098E | MS_PULSEFIRE_DART_MOUSE_DFU_HP | MI_01 |
| 03F0:068E | MS_PULSEFIRE_DART_DONGLE_HP | MI_02 |
| 03F0:078E | MS_PULSEFIRE_DART_DONGLE_DFU_HP | MI_01 |
| 0951:16D7 | MS_PULSEFIRE_FPS_PRO | MI_01&Col05 |
| 0951:16D7 | MSN_PULSEFIRE_FPS_PRO | MI_02 |
| 03F0:0E8F | MS_PULSEFIRE_FPS_PRO_HP | MI_01&Col05 |
| 03F0:0E8F | MSN_PULSEFIRE_FPS_PRO_HP | MI_02 |
| 0951:1729 | MS_PULSEFIRE_FPS_PRO_CD | MI_02 |
| 0951:172A | MS_PULSEFIRE_FPS_PRO_CD_DFU | - |
| 03F0:0693 | MS_PULSEFIRE_FPS_PRO_CD_HP | MI_02 |
| 03F0:0793 | MS_PULSEFIRE_FPS_PRO_CD_DFU_HP | - |
| 0951:16DE | MS_PULSEFIRE_CORE | MI_01&Col05 |
| 0951:16DE | MSN_PULSEFIRE_CORE | MI_02 |
| 03F0:0D8F | MS_PULSEFIRE_CORE_HP | MI_01&Col05 |
| 03F0:0D8F | MSN_PULSEFIRE_CORE_HP | MI_02 |
| 0951:1727 | MS_PULSEFIRE_HASTE | MI_03 |
| 0951:1728 | MS_PULSEFIRE_HASTE_DFU | - |
| 0951:1727 | MSN_PULSEFIRE_HASTE | MI_01&Col04 |
| 03F0:0F8F | MS_PULSEFIRE_HASTE_HP | MI_03 |
| 03F0:0190 | MS_PULSEFIRE_HASTE_DFU_HP | - |
| 03F0:0F8F | MSN_PULSEFIRE_HASTE_HP | MI_01&Col04 |
| 03F0:048E | MS_PULSEFIRE_HASTE_WIRELESS_MOUSE | MI_02 |
| 03F0:058E | MS_PULSEFIRE_HASTE_WIRELESS_MOUSE_DFU | MI_01 |
| 03F0:028E | MS_PULSEFIRE_HASTE_WIRELESS_DONGLE | MI_02 |
| 03F0:038E | MS_PULSEFIRE_HASTE_WIRELESS_DONGLE_DFU | MI_01 |
| 0951:16EA | HS_CLOUD_FLIGHT_S | MI_05&Col03 |
| 0951:16ED | HS_CLOUD_ALPHA_S | MI_05&Col03 |
| 0951:16ED | HS_CLOUD_ALPHA_S_DFU | MI_05&Col01 |
| 03F0:0C91 | HS_CLOUD_ALPHA_S_2 | MI_05&Col03 |
| 0951:1719 | HS_CLOUD_II_WIRELESS | Col02 |
| 0951:1718 | HS_CLOUD_II_WIRELESS_DONGLE | MI_03&Col03 |
| 03F0:0C8A | HS_CLOUD_II_WIRELESS_DONGLE2 | MI_03&Col03 |
| 0951:1743 | HS_CLOUD_ALPHA_WIRELESS_DONGLE | MI_03&Col01 |
| 0951:1765 | HS_CLOUD_ALPHA_WIRELESS_DONGLE2 | MI_03&Col01 |
| 03F0:098D | HS_CLOUD_ALPHA_WIRELESS_DONGLE3 | MI_03&Col01 |
| 03F0:058B | HS_CLOUD_II_WIRELESS_DTS_HEADSET_ONSEMI | Col02 |
| 03F0:018B | HS_CLOUD_II_WIRELESS_DTS_DONGLE_ONSEMI | MI_03&Col03 |
| 03F0:038B | HS_CLOUD_II_WIRELESS_DTS_HEADSET_TI | Col02 |
| 03F0:0696 | HS_CLOUD_II_WIRELESS_DTS_DONGLE_TI | MI_03&Col03 |
| 03F0:089D | HS_CLOUD_III | MI_03&Col02 |
| 0951:1709 | HS_STINGER_CORE_WIRED | MI_03&Col02 |
| 0951:170B | HS_STINGER_CORE_WIRELESS | MI_03&Col03 |
| 0951:1715 | HS_STINGER_S | MI_03&Col02 |
| 03F0:078A | HS_CLOUD_MIX_BUDS | mi_03&col04 |
| 03F0:078A | HS_CLOUD_MIX_BUDS_NOTIFICATION | mi_03&col05 |
| 03F0:0D93 | HS_CLOUD_STINGER2_WIRELESS | mi_03&col03 |
| 03F0:0995 | HS_CLOUD_2_CORE_WIRELESS_DONGLE | MI_03&Col02 |
| 03F0:069F | HS_CLOUD_2_CORE_WIRELESS_TREAD_DONGLE | MI_03&Col02 |
| 0951:16C4 | HS_CLOUD_FLIGHT_WIRELESS | mi_03&col03 |
| 0951:1723 | HS_CLOUD_FLIGHT_WIRELESS_CD | mi_03&col03 |
| 03F0:0E90 | HS_CLOUD_FLIGHT_WIRELESS_CD_HP | mi_03&col03 |
| 03F0:099C | HS_RALPHIE_ULL_1 | Col03 |
| 03F0:0EBC | HS_RALPHIE_ULL_2 | Col03 |
| 03F0:0AA0 | HS_RALPHIE_ULL_TREAD | MI_03&Col03 |
| 03F0:089F | HS_RALPHIE_ULL_TREAD_EML | MI_03&Col03 |
| 03F0:0A9F | HS_RALPHIE_ULL_TREAD_EPW | MI_03&Col03 |
| 03F0:08B7 | HS_RALPHIE_ULL_ATLAS | MI_03&Col03 |
| 03F0:0C9D | HS_CLOUD_3_WIRELESS_DONGLE | MI_03&Col01 |
| 03F0:05B7 | HS_CLOUD_3_WIRELESS_ATLAS_DONGLE | MI_03&Col01 |
| 03F0:0E9D | HS_CLOUD_3_WIRELESS_HEADSET | - |
| 03F0:06B7 | HS_CLOUD_3_WIRELESS_ATLAS_HEADSET | - |
| 03F0:039E | HS_CLOUD_MIX_BUDS_2_DONGLE | MI_03&Col05 |
| 03F0:0FAE | HS_CLOUD_MIX_2_DONGLE | MI_03&Col06 |
| 03F0:06BE | HS_CLOUD_3_S_WIRELESS_DONGLE | MI_03&Col05 |
| 0951:171F | MIC_QUADCAST_S | MI_00 |
| 0951:171F | MICN_QUADCAST_S | MI_01&Col02 |
| 03F0:048C | MIC_HP_QUADCAST_S_WHITE | MI_00 |
| 03F0:048C | MICN_HP_QUADCAST_S_WHITE | MI_01&Col02 |
| 03F0:068C | MIC_HP_QUADCAST_S_2S_WHITE | MI_00 |
| 03F0:068C | MICN_HP_QUADCAST_S_2S_WHITE | MI_01&Col02 |
| 03F0:0F8B | MIC_HP_QUADCAST_S_BLACK | MI_00 |
| 03F0:0F8B | MICN_HP_QUADCAST_S_BLACK | MI_01&Col02 |
| 03F0:028C | MIC_HP_QUADCAST_S_2S_BLACK | MI_00 |
| 03F0:028C | MICN_HP_QUADCAST_S_2S_BLACK | MI_01&Col02 |
| 0951:171D | CMEDIA_QUADCAST_S | mi_03&col01 |
| 03F0:0D8B | CMEDIA_HP_QUADCAST_S | mi_03&col01 |
| 03F0:0294 | CMEDIA_HP_QUADCAST_S_2S | mi_03&col01 |
| 0951:1720 | MIC_QUADCAST_S_DFU | - |
| 0951:1774 | MIC_QUADCAST_S_2S_WB_DFU | - |
| 03F0:09AF | MIC_HP_QUADCAST_2_CONTROLLER | MI_00 |
| 03F0:0AAF | MIC_HP_QUADCAST_2_CONTROLLER_DFU | - |
| 03F0:07B4 | MIC_HP_QUADCAST_2_CODEC_UNPLUGGED | MI_02&Col04 |
| 03F0:07AF | MIC_HP_QUADCAST_2_CODEC_PLUGGED | MI_03&Col04 |
| 03F0:02B5 | MIC_HP_QUADCAST_2S_CONTROLLER | MI_01&Col02 |
| 03F0:03B5 | MIC_HP_QUADCAST_2S_CONTROLLER_DFU | - |
| 03F0:0D84 | MIC_HP_QUADCAST_2S_CODEC | MI_03&Col06 |
| 03F0:0EB4 | MIC_HP_QUADCAST_2S_CODEC_DFU | - |
| 03F0:0A8C | MIC_DUOCAST_AUDIO | MI_03 |
| 03F0:098C | MIC_DUOCAST_CONTROLLER | MI_00 |
| 03F0:088C | MIC_DUOCAST_CONTROLLER_DFU_HP | - |
| 0951:175F | MIC_DUOCAST_CONTROLLER_DFU_KINGSTON | - |
| 0951:170F | MIC_KINGSTON_SOLOCAST_BLACK | MI_02&Col01 |
| 03F0:078B | MIC_HP_SOLOCAST_BLACK | MI_02&Col01 |
| 03F0:0592 | MIC_HP_SOLOCAST_WHITE | MI_02&Col01 |
| 03F0:098B | MIC_HP_SOLOCAST_BLACK_2ND_SOURCE | MI_02 |
| 03F0:0992 | MIC_HP_SOLOCAST_WHITE_3RD_SOURCE | MI_02&Col02 |
| 03F0:0B8B | MIC_HP_SOLOCAST_BLACK_3RD_SOURCE | MI_02&Col02 |
| 0951:1705 | MP_FURY_ULTRA | MI_00 |
| 0951:1706 | MP_FURY_ULTRA_DFU | - |
| 0951:1705 | MPN_FURY_ULTRA | MI_01&Col02 |
| 03F0:0493 | MP_FURY_ULTRA_HP | MI_00 |
| 03F0:0593 | MP_FURY_ULTRA_DFU_HP | - |
| 03F0:0493 | MPN_FURY_ULTRA_HP | MI_01&Col02 |
| 0951:1741 | MP_PULSEFIRE_MAT_RGB | MI_01 |
| 0951:1742 | MP_PULSEFIRE_MAT_RGB_DFU1 | - |
| 03F0:0F8D | MP_PULSEFIRE_MAT_RGB_HP | MI_01 |
| 03F0:018E | MP_PULSEFIRE_MAT_DFU_HP | - |
| 03F0:0B97 | MS_PULSEFIRE_HASTE2 | MI_02 |
| 03F0:0C97 | MS_PULSEFIRE_HASTE2_DFU | - |
| 03F0:0F98 | MS_PULSEFIRE_HASTE2_WIRELESS_DONGLE | MI_02 |
| 03F0:0D97 | MS_PULSEFIRE_HASTE2_WIRELESS_MOUSE | MI_02 |
| 03F0:0199 | MS_PULSEFIRE_HASTE2_WIRELESS_DONGLE_DFU | MI_01 |
| 03F0:0E97 | MS_PULSEFIRE_HASTE2_WIRELESS_MOUSE_DFU | MI_01 |
| 03F0:0CBD | MS_PULSEFIRE_HASTE2_PRO_DONGLE | MI_03 |
| 03F0:0ABD | MS_PULSEFIRE_HASTE2_PRO_MOUSE | MI_02 |
| 03F0:0DBD | MS_PULSEFIRE_HASTE2_PRO_DONGLE_DFU | MI_01 |
| 03F0:0BBD | MS_PULSEFIRE_HASTE2_PRO_MOUSE_DFU | - |
| 03F0:0BA0 | MS_PULSEFIRE_HASTE2_MINI_WIRELESS_MOUSE | MI_02 |
| 03F0:0CA0 | MS_PULSEFIRE_HASTE2_MINI_WIRELESS_MOUSE_DFU | - |
| 03F0:0DA0 | MS_PULSEFIRE_HASTE2_MINI_WIRELESS_DONGLE | MI_02 |
| 03F0:0EA0 | MS_PULSEFIRE_HASTE2_MINI_WIRELESS_DONGLE_DFU | - |
| 03F0:04B5 | MS_PULSEFIRE_HASTE2_CORE | MI_02 |
| 03F0:05B5 | MS_PULSEFIRE_HASTE2_CORE_DFU | - |
| 03F0:0FA0 | KB_ALLOY_RISE | MI_02 |
| 03F0:01A1 | KB_ALLOY_RISE_DFU | MI_02 |
| 03F0:02A1 | KB_ALLOY_RISE_75 | MI_02 |
| 03F0:03A1 | KB_ALLOY_RISE_75_DFU | MI_02 |
| 03F0:01B9 | KB_ALLOY_RISE_75_WIRELESS_DONGLE | MI_02 |
| 03F0:02B9 | KB_ALLOY_RISE_75_WIRELESS_DONGLE_DFU | - |
| 03F0:0EB8 | KB_ALLOY_RISE_75_WIRELESS_KEYBOARD | MI_02 |
| 03F0:0FB8 | KB_ALLOY_RISE_75_WIRELESS_KEYBOARD_DFU | - |
| 03F0:0AB5 | MS_PULSEFIRE_HASTE2_CORE_WIRELESS_DONGLE | MI_02 |
| 03F0:0BB5 | MS_PULSEFIRE_HASTE2_CORE_WIRELESS_DONGLE_DFU | MI_01 |
| 03F0:0AB8 | MS_PULSEFIRE_HASTE_2S_WIRELESS_MOUSE | MI_02 |
| 03F0:0CB8 | MS_PULSEFIRE_HASTE_2S_WIRELESS_MOUSE_DFU | - |
| 03F0:0BB8 | MS_PULSEFIRE_HASTE_2S_WIRELESS_DONGLE | MI_02 |
| 03F0:0DB8 | MS_PULSEFIRE_HASTE_2S_WIRELESS_DONGLE_DFU | - |
| 03F0:0FBD | MS_PULSEFIRE_FUSE_WIRELESS | MI_02 |
| 03F0:0FBD | MS_PULSEFIRE_FUSE_WIRELESS_DFU | MI_01&COL02 |
| 03F0:0EBE | MS_PULSEFIRE_SAGA | MI_02 |
| 03F0:0FBE | MS_PULSEFIRE_SAGA_DFU | MI_01 |
| 03F0:04BF | MS_PULSEFIRE_SAGA_PRO_MOUSE | MI_02 |
| 03F0:05BF | MS_PULSEFIRE_SAGA_PRO_MOUSE_DFU | - |
| 03F0:06BF | MS_PULSEFIRE_SAGA_PRO_DONGLE | MI_03 |
| 03F0:07BF | MS_PULSEFIRE_SAGA_PRO_DONGLE_DFU | MI_01 |
| 03F0:01BF | TIO_EMBLEMS_PAIRED | MI_05&Col02 |
| 03F0:01BF | TIO_EMBLEMS_PAIRED_ONE_2_ONE | MI_04&Col02 |
| 03F0:01C5 | TIO_EMBLEMS_NOT_PAIRED | Col05 |
| 03F0:0AC1 | HS_CLOUD_FLIGHT_2_WIRELESS_DONGLE | MI_03&Col04 |
| 03F0:0BC1 | HS_CLOUD_FLIGHT_2_WIRELESS_HEADSET | MI_03&Col04 |

## Nettoyage (fait le 2026-10-26)

Paquet `33C30B79.HyperXNGenuity` désinstallé (`Remove-AppxPackage`). Vérifié ensuite : aucun processus, aucun service, aucune tâche planifiée, aucun pilote HyperX
(`pnputil /enum-drivers`), dossier des paquets utilisateur `33C30B79*` absent. Le package n'installe ni service ni pilote tant que l'application n'est pas lancée (elle ne l'a jamais été) :
aucun `pnputil /delete-driver` nécessaire. Les dépendances UWP (NET.Native, VCLibs) étaient déjà présentes sur ce PC (janvier 2026) et sont conservées.
Copie d'analyse hors dépôt : `D:/Tools/analyse/hyperx`.
