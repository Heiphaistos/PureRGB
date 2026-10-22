# NZXT CAM : Smart Device, RGB & Fan Controller, HUE, Kraken (LED + ventilateurs + pompe)

Date : 2026-10-22. **[V]** lu dans la source citée, **[S]** supposé, **[?]** INCONNU.
Déjà natif dans PureRGB (expérimental) : `nzxt_hue2.rs` (RGB HUE 2). Cette note ajoute les ventilateurs, la pompe, le Smart Device V1 et les Kraken.
Écrans LCD Kraken : voir `docs/screens-protocols/nzxt.md`.

## Source analysée

- **NZXT CAM** : seul le *bootstrapper* a été téléchargé : `https://nzxt-app.nzxt.com/NZXT-CAM-Setup.exe`, 1 496 448 o,
  SHA-256 `47c83ac3471b06f61aa938f66d26a578fe6996910bb2ecfb825823a8d92cadb7`, signé NZXT, Inc. (« NZXT CAM Installer 1.0 »). Il télécharge la charge utile
  depuis `nzxt-app.nzxt.com` (et `nzxt-app-staging.nzxt.com`) au lancement : **non analysé plus avant** (pas d'exécution ; la charge utile n'est pas dans l'exe).
  Aucun protocole HID n'a donc été relevé dans CAM lui-même.
- **Sources de protocole (faits seulement)** : liquidctl (GPL-3.0+) `driver/smart_device.py`, `kraken2.py`, `kraken3.py`, `util.py` (clone
  `D:\Projet\Analyses\rgb\cooling\liquidctl`) ; plugins SignalRGB `Nzxt/NZXT_Smart_Device_V2_Controller.js`, `Nzxt_Kraken_X3_AIO.js` (non relus en détail).
  Confiance **élevée** pour HUE2/Smart Device V2/Kraken X3 (support actif, kernel `nzxt-smart2`, `nzxt-kraken2/3`).

## Matériel couvert (VID 0x1E71) [V liquidctl]

| PID | Produit | Canaux | Pilote PureRGB |
|---|---|---|---|
| 1714 | Smart Device (V1) | 3 ventilos, 1 canal LED | aucun |
| 1711 | Grid+ V3 | 6 ventilos | aucun |
| 2006, 200D, 200F | Smart Device V2 | 3 ventilos, 2 canaux LED | natif (2006) |
| 2001, 2002 | HUE 2, HUE 2 Ambient | 4 canaux LED (HUE 2) | natif (2001) |
| 2009, 200E, 2010 | RGB & Fan Controller | 3 ventilos, 2 canaux LED | natif (2009, 200E) |
| 2011, 2019, 201F, 2020 | RGB & Fan Controller (3+6 canaux) | 3 ventilos, 6 canaux LED | natif (2019, 2020) |
| 2012, 2021 | 2023 RGB Controller | LED seulement (protocole voisin) | aucun |
| 2015 | H1 V2 (boîtier) | ventilos/pompe via SmartDevice2 (statut spécifique) | aucun |
| 170E | Kraken X42/X52/X62/X72 (gen. 3) | pompe+ventilo (cooling), LED logo+anneau | liquidctl |
| 1715 | Kraken M22 | LED seulement | liquidctl |
| 2007, 2014 | Kraken X53/X63/X73 | pompe, 3 canaux LED | liquidctl |
| 3008 | Kraken Z53/Z63/Z73 | pompe, ventilo, 1 canal LED externe, LCD | liquidctl |
| 300C | Kraken **2023 Elite** (liquidctl « broken ») | pompe, ventilo | liquidctl |
| 300E | Kraken **2023** standard | pompe, ventilo | liquidctl |
| 3012 | Kraken 2024 Elite RGB | idem + anneau | liquidctl |
| 3014 | Kraken 2024 Plus | idem | liquidctl |

**Écart `known.rs`** : il nomme 300C « Kraken 2023 » et 300E « Kraken 2023 Elite » ; liquidctl dit l'inverse (300C = Elite, 300E = standard, et `docs/screens-protocols/nzxt.md`
aussi). À corriger. `known.rs` donne aussi 3008 pour Z53/Z63/Z73 (correct) mais n'a pas 3012/3014/2012/2021/2015/2010/2011/201F/200D/200F.

## HID commun (famille HUE 2 : Smart Device V2, RGB & Fan Controller, Kraken X3/Z3)

[V : smart_device.py, kraken3.py]. Rapports de 64 o, sans ID (octet 0 = ID 0 pour hidapi), écriture complétée de zéros ; lecture 64 o.

### Init

1. `60 02 01 E8 <intervalle> 01 E8 <intervalle>` puis `60 03` : démarre le rapport de statut des ventilos ; `intervalle = 1 + round((secondes - 0.5)/0.25)` (0,5 s -> 1).
   (Kraken X3/Z3 : `70 02 01 B8 <intervalle>` puis `70 01`.)
2. `10 01` (firmware) -> réponse commençant par `11 01`, version aux octets 0x11,0x12,0x13.
3. `20 03` (info éclairage) -> réponse `21 03` : octet 14 = nombre de canaux ; à partir de l'octet 15, 6 emplacements (octets) par canal = ID d'accessoire (0 = vide).
   Accessoires : `01` bande HUE+, `02` AER RGB 1, `04/05/06` bandes HUE2 300/250/200 mm, `07` Cable Comb, `09/0A` Underglow 300/200, `0B/0C` AER RGB 2 120/140,
   `10` anneau Kraken X, `11` logo Kraken X, `13/14` F120/F140 RGB, `17` F140 RGB Core, `1B` F240 Core, `1D` F360 Core.

### Statut ventilos (Smart Device V2 / RGB & Fan Controller), rapport `67 02`

- Modes aux octets 16.. (`00` absent, `01` DC, `02` PWM) ; **RPM** u16 LE à partir de l'octet 24 (2 o/ventilo) ; **duty %** à partir de l'octet 40 ; bruit (dB) octet 56.

### Consigne de vitesse (V2) : `62 01 <1<<canal> 00 00 00` avec le pourcentage placé à l'octet 3+canal

Concrètement : `[62 01 (1<<c) 00 00 00]` et `msg[c+3] = pourcentage (0..100)`. Pas de courbe sur ces contrôleurs (courbe logicielle côté hôte).

## Smart Device V1 (1714) / Grid+ V3 (1711)

- Init : `01 5C` (détection) puis `01 5D` (début des rapports). Rapport de 21 o par ventilo (3 rapports) : RPM u16 BE aux octets 3..4, tension octets 7+8/100 V,
  courant 9+10/100 A, mode (octet 15 & 3 : 1 DC, 2 PWM), numéro de ventilo = (octet 15 >> 4) + 1, bruit octet 1. Firmware : octets 0x0B..0x0E ; LED : octet 0x11 = nombre d'accessoires,
  octet 0x10 >> 3 : 0 = bande HUE+ (10 LED), 1 = AER RGB (8 LED).
- Consigne : écriture 65 o `02 4D <canal> 00 <pourcent>`.
- LED : `02 4B <mode> <mod3+dir> <vitesse | seq<<5 | mod4>` + 57 octets de couleurs (ordre G,R,B), suite `03` + reste ; 40 LED max ; direction 0x10 pour l'arrière.

## Éclairage HUE 2 [V : smart_device.py]

Couleurs au format **G,R,B** (liquidctl convertit RGB -> GRB). Deux chemins :

- **Effet matériel** : `28 03 <canal-masque> 00 <mode> <vitesse> <moving> <backward> <taille/nb couleurs> <taille alt.> <couleurs…>` ; masque canal = `1<<n` (sync = tous).
  Modes (octet mode, variante) : fixed `00`, fading `01`, spectrum-wave `02`, marquee `03` (variante = taille 3..6 -> octet 0..3), covering-marquee `04`, alternating `05`, pulse `06`,
  breathing `07`, candle `08`, starry-night `09`, rainbow-flow `0B`, super-rainbow `0C`, rainbow-pulse `0D`. Vitesse : `00` plus lent .. `04` plus rapide.
- **Direct par LED** (« super-fixed » etc., jusqu'à 40 LED/canal) : `22 10 <canal> 00` + 60 octets (20 LED), `22 11 <canal> 00` + 60 octets (20 LED suivantes),
  puis `22 A0 <canal> 00 <mode> <mod3> 00 00 00 00 64 00 32 00 00 01` pour valider.
- Mode « wings » : séquence spéciale (8 paquets `22 20 <canal> <i> …` puis `22 03 <canal> 08`), non détaillée ici.

## Kraken X3 (2007/2014) et Z3 / 2023 (3008/300C/300E/3012/3014) [V : kraken3.py]

- **Statut** (rapport lu) : température liquide = octet 15 + octet 16/10 °C (si `FF FF` : défaut firmware) ; **pompe** RPM u16 = octet 18<<8 | octet 17, duty % octet 19 ;
  **ventilo** (Z3/2023) RPM = octet 24<<8 | octet 23, duty % octet 25.
- **Consigne** : la consigne est une **courbe liquide -> duty** de 40 points : paquet `72 <canal[3]>` + 40 octets de duty % pour liquide 20..59 °C
  (point 59 °C = critique) ; liquidctl émule le fixe par duty constant. Canaux : pompe `01 00 00` (X3/Z3), `01 01 00` (2023) ; ventilo `02 00 00` (Z3), `02 01 01` (2023).
  Plages : pompe 20..100 %, ventilo 0..100 % (« à ajuster à ce que le firmware impose »).
- **LED** :
  - X3 : 3 canaux (bitmask) : externe `001`, anneau `010`, logo `100`, sync `111` ; Z3 : externe seulement ; 2023 : aucun (anneau Elite : [?]).
  - Effets : opcode `2A 04 <canal> <canal> <mode> <vitesse 2 o> <couleurs 16x RGB> <dir> <nb couleurs> <mode_related> <static> <taille>` ; modes (octet) fixed `00`,
    fading `01`, spectrum-wave `02`, marquee `03`, covering `04`, alternating `05`, pulse `06`, breathing `07`, candle `08`, starry-night `09`, rainbow-flow `0B`, super-rainbow `0C`,
    rainbow-pulse `0D`, tai-chi `0E`, water-cooler `0F`, loading `10`. « static byte » : 40 (externe/sync), 8 (anneau), 1 (logo).
  - Direct : `22 10 <canal> 00` + couleurs, `22 11 <canal> 00`, `22 A0 <canal> 00 <mode> …`.
  - Table de vitesses (2 o LE par palier) fournie par liquidctl (5 paliers x 12 échelles).

## Kraken gen 3 / M22 (170E, 1715) [V : kraken2.py]

- Rapports 64 o lus (endpoint 0x81) / 65 o écrits (0x01). **Statut** : liquide = octet 1 + octet 2/10 °C ; ventilo RPM BE octets 3..4 ; pompe RPM BE octets 5..6.
- **Courbe** : un paquet par température : `02 4D <base+i> <temp> <duty>` ; bases `80` ventilo, `C0` pompe ; instantané `02 4D <base&0x70> 00 <duty>`.
  Profils de reset : ventilo (20,25) (30,50) (50,90) (60,100) ; pompe (20,50) (30,60) (40,90) (50,100). Critique 60 °C.
- **LED** : `02 4C <canal|mod2> <mode> <vitesse|seq<<5|mod4> <logo 3 o> <anneau 8 x 3 o>` ; canaux sync `0`, logo `1`, anneau `2`. Ordre des octets couleur : INCONNU avec certitude (liquidctl convertit en GRB
  puis permute les deux premiers octets pour le logo seulement : logo R,G,B ?, anneau G,R,B ? ; à vérifier sur matériel).

## Accès bas niveau

- USB HID uniquement. CAM (service/overlay) garde les HID ouverts ; coexistence non testée. Les Kraken exposent aussi un point WinUSB bulk pour l'écran (voir `screens-protocols/nzxt.md`).
- Sous Linux le noyau expose `nzxt-smart2`, `nzxt-kraken2/3` (hwmon) : preuve que les formats de statut ci-dessus sont stables.

## INCONNU

- Tout contenu de CAM (non analysé, bootstrapper seul) : modes d'éclairage propriétaires des anneaux Elite 2023/2024, profils « noyau », télémétrie, mise à jour firmware.
- Anneau LED des Kraken 2023/2024 Elite RGB ; format exact de la courbe de pompe/ventilo pour 2023 (liquidctl « broken » sur 300C) ; H1 V2 ; 2023 RGB Controller (2012/2021) : parseur de statut.
- Contrôle de ventilos par courbe matérielle sur Smart Device V2 (non vu).

## Difficulté de réimplémentation

HUE2 + ventilos : **faible** (RPM/duty lisibles, consigne 6 octets). Kraken X3/Z3 pompe/ventilo : **faible-moyenne** (courbe 40 points). Kraken 2023 : **moyenne** (incertitudes liquidctl).
Smart Device V1 : **faible**. Effets matériels complets : **moyenne** (nombreux paliers de vitesse).

## Tests d'octets possibles sans matériel

- `60 02 01 E8 01 01 E8 01`, `60 03`, `10 01`, `20 03` : longueur 64, zéros de fin.
- `62 01 01 00 00 00` + duty à l'octet 3 pour ventilo 1 ; `62 01 04 ...` + duty à l'octet 5 pour ventilo 3.
- Décodage d'une réponse `21 03` fabriquée : octet 14 = 2, octets 15..20 = `0B 0B 00 00 00 00`, 21..26 = `04 00 ...` -> deux canaux avec accessoires.
- Kraken : calculer 40 duty pour 20..59 °C et vérifier `72 01 00 00` + 40 octets.
