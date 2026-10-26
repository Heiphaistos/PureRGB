# Marques groupe B : Redragon, Havit, Fantech, Tecware, Rapoo, Glorious, Xtrfy, Ducky, Akko, Royal Kludge, Ajazz, Epomaker, NuPhy, Motospeed, Zalman, Aerocool, Deepcool, Phanteks, Fractal, be quiet!, Antec, Evga, Zotac/Gainward/Palit

Document d'interopérabilité (2026-10-26). Aucun code ni asset copié ; faits seulement. Aucun logiciel fournisseur installé ni exécuté,
aucun envoi USB depuis ce PC : toutes les sources sont publiques (usb.ids, détecteurs OpenRGB, extraction SignalRGB déjà dans `signalrgb-devices.csv`).
Copies d'analyse : `D:\Tools\analyse\groupeB\` (usb.ids, clone OpenRGB). Aucun pilote ajouté, rien à désinstaller.
Niveaux : [S] = source secondaire (code OpenRGB/SignalRGB), [V] = vérifié par analyse du logiciel officiel (aucun ici), INCONNU.

## Identifiants ajoutés (`src-tauri/assets/rgb_usb_catalog_extra_b.json`, 28 appareils + 1 fabricant)
- Royal Kludge (VID 258A Sinowealth) : 18 PID RK61/RK84/RK98 [S SignalRGB, plugin `rk-keyboard-controller`].
- Glorious Model O/D Wireless : PID 2013, 2014, 2024, 2025, 2026 [S SignalRGB].
- Redragon M709 04D9:FC2A [usb.ids] ; EVGA X17 3842:240D, X15 3842:2415 [S SignalRGB] ; Rapoo VID 24AE (usb.ids) + V280 (4110), E9260 (1813).

## Par marque
| Marque | Identification | Protocole RGB |
|---|---|---|
| Redragon | souris 04D9:FCxx/FA7E (déjà au catalogue, OpenRGB `RedragonController`, usage page FFA0) ; M914 25A7:FA7B/FA7C (Areson, déjà) ; claviers = puces EVision 0C45/320F (déjà) ou Sinowealth | [S] souris Holtek/Areson et claviers EVision gérés par OpenRGB ; INCONNU côté logiciel officiel (non analysé) |
| Tecware, Ajazz | claviers sur puce EVision (0C45 / 320F), cités comme tels dans l'en-tête du pilote OpenRGB EVision | [S] même famille EVision ; pas de PID propre vérifiable |
| Glorious | Model O/D : 258A:0036/0033, sans fil 2011/2012/2013/2014/2022-2026 ; Model I 22D4:1503 (Laview, déjà) | [S] OpenRGB `SinowealthController` / SignalRGB ; INCONNU officiel (Glorious Core non analysé) |
| Royal Kludge | 258A (voir ci-dessus) | [S] SignalRGB `rk-keyboard-controller` |
| Akko / Epomaker | 3151:4003 (Akko B-series), 4010/4011 (Epomaker TH80 Pro), 5029 (Skyloong) déjà ; Ducky One 2 RGB 04D9:0348/0356 déjà | [S] OpenRGB `RoyuanKeyboardController`, `DuckyKeyboardController` |
| Zalman | 1C57:7ED0 Z Sync (déjà) | [S] OpenRGB `ZalmanZSyncController` |
| Rapoo | vendeur 24AE ajouté | INCONNU : aucun détecteur OpenRGB/SignalRGB, pas de PID RGB vérifiable |
| Havit, Fantech, Xtrfy, NuPhy, Motospeed | aucun VID/PID vérifiable en source publique (absents d'usb.ids, OpenRGB, SignalRGB) : non inventés. NuPhy tourne en QMK/VIA (NuPhy Console) : si le clavier expose l'interface Raw HID QMK il est couvert par le pilote QMK/VialRGB d'OpenRGB, à confirmer sur matériel | INCONNU |
| Aerocool, Antec, Phanteks, Fractal, be quiet!, Deepcool (ventilos/boîtiers/AIO ARGB) | composants SignalRGB de type `component:*` (nom + nombre de LED, sans VID/PID : se branchent sur un en-tête ARGB 5 V de la carte mère ou d'un contrôleur déjà géré). Voir `autres-refroidissement.md` | pas de protocole USB propre ; pilotés par le contrôleur hôte |
| Thermaltake | voir `thermaltake.md` | déjà documenté |
| Evga | clavier/souris 3842 au catalogue ; GPU : PCI sous-vendeur 3842, I2C (OpenRGB `EVGA*GPUController`, 60 entrées Ampere) | [S] |
| Zotac / Gainward / Palit | GPU : sous-vendeur PCI 19DA / 10B0 / 1569, contrôleur I2C 0x49 (0x4B Blackwell), familles Turing/V2/Blackwell (Zotac), `PalitGPUv1/v2` (Palit, Gainward, PNY) | [S] OpenRGB ; non USB, donc hors catalogue USB |
| Corsair | rien d'évident manquant ajouté | voir `corsair.md` |

## Preuves « sans RGB » (NO_RGB)
Aucune : pas de preuve qu'un appareil du groupe soit dépourvu de RGB ; `NO_RGB` inchangé.

## Code natif
Aucun pilote ajouté : aucun protocole [V] (pas de logiciel officiel analysé). Reste INCONNU = non implémenté.

## Tentative d'installation des logiciels officiels (2026-10-26)
- Glorious Core 2.1.23 (source officielle gloriousgaming.com, installeur Inno Setup 7.0.0.3) : `innoextract` ne gère pas cette version, 7-Zip ne sort qu'un blob. Lancé en silencieux (`/VERYSILENT /CURRENTUSER`), l'installeur reste bloqué sur l'élévation (clé HKLM, aucune réponse UAC possible) : arrêté, rien installé, temporaires nettoyés. Non contourné. Protocole Glorious : toujours [S] uniquement.
- Redragon (redragonzone.com injoignable), Rapoo (page de téléchargement 404), Epomaker (404), Akko (page sans lien direct), NuPhy (429) : aucun installeur obtenu depuis une source officielle ; non analysés.
- Havit, Fantech, Xtrfy, Motospeed, Ajazz, Tecware : non tentés. Aucun réglage non lumineux [V] relevé, donc aucun pilote natif.
