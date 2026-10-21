# Implémentation ASUS Ryujin III + Thermaltake LCD (2026-10-21)

Branche `screens/asusthermal`. Fiches sources : `asus.md`, `thermaltake.md`. Tout est `verified: false` ; aucun matériel n'a été branché.
Dépôts lus hors du dépôt PureRGB (dossier temporaire), contenu traité comme DONNEES ; aucun code copié (zaclanzon MIT, pcmx1 MIT, messiahlap sans licence : octets seuls).

## ASUS ROG Ryujin III — `drivers/asus_ryujin.rs` (`RyujinProvider`)

PID 0B05:1AA2 / 1BCB / 1ADA / 1ADE, 320x240 (`resolution_confirmed: false`). Protocole : `Protocol::AsusRyujin`.

| Élément | État | Source |
|---|---|---|
| Détection (énumération HID interface 1, aucun handle ouvert) | implémenté | hidapi |
| Version du firmware `EC 82` -> `EC 02` | implémenté, testé sur faux transport | protocol.md « Command map » |
| Luminosité (`DC` Get, `5C` Set octet 2=1 et octet 7, Get de contrôle) | implémenté (`set_brightness` renvoie `Ok(true)`) | « Display status » |
| Stockage `71`/`F1` (libre, bitmap JPG) | implémenté | « GetDiskInfo reply » |
| Envoi d'un JPEG dans un emplacement libre (`72`, `73 01`, `7F 02`, bulk 4096 o + `EE 14`, `73 FF`) | implémenté, **hors `push_frame`** (méthode `upload_and_show_jpeg`) ; écriture flash, usage ponctuel | « Media upload » |
| Affichage du JPEG stocké `60 00 01 10 <slot>` + `51 1F` ; suppression `72`/`73 03` | implémenté | « Slideshow », « Media upload » |
| `push_frame` (images libres temps réel) | **NON** : erreur FR « Envoi d'image non validé… » | la source ne décrit aucun flux direct |

Non implémenté : page « hardware monitor » (`52/53`, texte seulement, pas des images), GIF, horloge, diaporama, veille ; trames brutes BGR sur le bulk évoquées par killryujin [S] ; Ryujin II (0B05:1988) ; couleurs/thèmes du moniteur ; tailles JPEG > 65535 o (champ `7F` vérifié sur 2 octets seulement) ; emplacement déjà occupé (écrasement non documenté : refusé).

Choix de sûreté : pas de nouvelle tentative en boucle (désynchronise la poignée de main, fiche) ; en cas d'échec après `73 01`, un seul `73 FF` au mieux. Les rapports `EE` arrivés avant d'être attendus sont mis en réserve ; les réponses hwmon (`19/1A/20/21`) sont ignorées. Toute réponse est validée (id `EC`, octet de commande, motifs `EE 13/14`, taille de bloc 0x1000, table de stockage).

A valider sur matériel :
1. Les rapports d'entrée sous Windows/hidapi : l'identifiant `EC`/`EE` est-il bien le 1er octet lu (supposé) ? Taille réelle lue (64) ?
2. Ouverture HID de l'interface 1 alors qu'Armoury Crate tourne ; interface 1 = `interface_number()` 1 (sinon filtre à revoir).
3. Bulk (interface 0, EP 0x01) sur périphérique composite : `NusbTransport::open` exige WinUSB sur le périphérique ; usbccgp (composite) fera probablement échouer l'ouverture, il faudra lier WinUSB à l'interface 0 (Zadig) ou ouvrir par interface.
4. Octet 7 = luminosité sur les variantes White/Extreme/EVA ; résolution réelle ; usure de la flash.
5. Flux temps réel : toujours à décoder (REA d'AacAIOFanHal / capture USBPcap, mode « display control »).

## Thermaltake LCD — `drivers/thermaltake_lcd.rs` (`ThermaltakeProvider`)

Protocole : `Protocol::ThermaltakeLcd`. Détection par énumération HID, **aucun handle à la détection** : l'appareil est ouvert au premier envoi (l'init du RC Pro doit être suivie immédiatement des morceaux), réouvert + ré-initialisé après toute erreur.

| PID | Modèle | Taille | Format |
|---|---|---|---|
| 264A:2328 | AIO LCD rond | 480x480 | « Legacy » |
| 264A:232A | RC Pro (Tower 500/900) | 480x128 | « Legacy » + `18` et `1D` |
| 264A:233C | TH420 V2 Ultra EX | 480x480 | « TH420 », 2 interfaces HID |

Legacy (pcmx1, MIT) : feature reports de 64 o, id `03` : `1A`, GET `07`, `0C`x3 (x4 RC Pro) avec `64 00 00 00 00 <w LE> <h LE> 04 05`, GET `0F`, (`1D` RC Pro) ; images suivantes : GET `0F` + `0C 64 FF FF EA FF FF FF`. JPEG en morceaux de 1016 o, en-tête `02 09 00 <fin> <taille LE16> <index LE16>`, dernier `fin=1` et taille = octets utiles. Paquet de 1024 o.
TH420 (messiahlap) : interface ctrl `:1.0` (440 o) : init `85 87 85 87 84 81` (`<op> 01 00 80`, réponse exigée), avant chaque image `12 01 00 80 64` (ACK optionnel) ; interface image `:1.1` : paquets de 1024 o `08 <n> 00 <80|00>` + 1020 o, premier paquet n = nombre total (255 max : la qualité JPEG est abaissée jusqu'à tenir), suivants n = index.

INCONNU / non implémenté : contenu des réponses (seuls : réponse non vide, id de feature report attendu), sens des champs `0x64`, `04 05`, `FF FF EA FF FF FF`, `1A`/`18`, luminosité, rotation matérielle (0 par défaut ; le 180° du dépôt source est propre à son montage), lecture de la température de liquide (octets 6-7 u16 BE en millidegrés : plafonne à 65 °C, douteux), Tower 500 (264A:233D, protocole proche du TH420 mais hors périmètre), autres écrans (Floe Ultra, MINECUBE…), recoupement des deux formats.

A valider sur matériel (capture USBPcap de TT RGB PLUS 2.0 recommandée) : tout. En particulier la correspondance PID <-> format (2328/232A = Legacy, 233C = TH420, déduite de deux dépôts différents), la tolérance aux feature reports sous Windows (taille 64 incl. id), l'ordre des interfaces `:1.0`/`:1.1` (`interface_number` 0/1), la cadence acceptée (répétition ~800 ms dans la source TH420), le message « fermez TT RGB PLUS ».

## Contrat

Variantes ajoutées dans `driver.rs::Protocol` : `AsusRyujin`, `ThermaltakeLcd` (additif ; `src/screens/types.ts` doit recevoir `"asus_ryujin" | "thermaltake_lcd"`). `ScreenProvider`s non branchés dans `manager.rs` (hors périmètre).
