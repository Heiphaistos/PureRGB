# Éclairage clavier des portables OEM (Alienware/Dell G, HP OMEN, Lenovo Legion, Acer, MSI, Gigabyte/Aorus)

Date : 2026-10-26. Branche `oem`. Faits seulement, aucun code ni asset de fournisseur copié.

## Règles et limites de cette analyse

- Le PC de dev de Momo est un portable ACER (SUBSYS 1025) : ni BIOS, ni EC, ni ventilateurs, ni tensions, ni limites de puissance. Aucun logiciel OEM installé ni exécuté ;
  aucun envoi USB réel vers un clavier interne n'a été fait (tests = `MockHid` seulement).
- Légende : [V] = vérifié sur le logiciel du fabricant ; [S] = source secondaire (sources publiques OpenRGB, GPL-2, consultées 2026-10-26, copie locale
  `D:\Tools\analyse\groupeB\OpenRGB`) ; INCONNU = non établi.
- **Aucun chemin n'est [V]** : les applications des fabricants (AWCC, OMEN Gaming Hub, Legion Zone/Vantage, PredatorSense, NitroSense, MSI Center du Store) sont des
  paquets Microsoft Store non téléchargeables hors Store sans compte ; `MSI.MSICenter` (winget, CDN MSI) refuse : somme de contrôle différente de celle du manifeste
  (non contourné). Seul `Lenovo.LegionAccessoryCentral` 2.0.6 (CDN Lenovo, winget) a été téléchargé et extrait sans exécution (innoextract) dans
  `D:\Tools\analyse\oem-portables\lenovo\x` : il ne pilote que les périphériques Legion (modules `ldm_k300/k500/m300/m300s/m500/m600/m600s`, `hidapi.dll`), pas le clavier
  du portable ; non décompilé (hors périmètre éclairage portable). Les trames ci-dessous restent donc [S], d'où des pilotes expérimentaux désactivés par défaut.

## Pilotes natifs ajoutés (expérimentaux, `verified: false`)

| Famille | VID:PID | Chemin | Trame | Fichier |
|---|---|---|---|---|
| `lenovo_4zone` | 048D:C955, C965, C963, C975, C973, C985, C984 (ITE ; Legion 5 / IdeaPad 2020-2023) ; collection 0xFF89 usage 0xCC | HID direct [S] | feature report 33 o : `CC 16 01 01 <lum 1\|2> R0 G0 B0 … R3 G3 B3 00 00 00` | `lenovo_4zone.rs` |
| `msi_3zone` | 1770:FF00 (anciens portables MSI, 3 zones) | HID direct [S] | feature report 8 o : `01 02 40 <zone 1..3> R G B EC` | `msi_3zone.rs` |
| `dell_g` | 187C:0550, 187C:0551 (Dell G5/G15 ; AlienFX) | HID direct [S] | feature reports 34 o (id 0) : config `00 03 20 02` (plateforme octets 4-5, zones octet 6) puis couleur `00 03 27 R G B <n u16> <ids…>` | `dell_g.rs` |

- `lenovo_4zone` : réglage `bright` (luminosité 1 basse / 2 haute) via `DeviceSetting` ; persistance après redémarrage INCONNU. Effets 3/4/6 (respiration, vague, fondu) non utilisés.
- `msi_3zone` : la source envoie aussi les zones 4 à 7 (rôle INCONNU) : non envoyées. Persistance INCONNU.
- `dell_g` : plateformes connues pilotées : 0C01 (G5 SE 5505), 0E03 (G15 5511) zones 0..3 ; 0E07 (G15 5520), 0E0A (G15 5530) zones 0x10..0x13 ; noms Gauche / Milieu / Droite / Pavé numérique.
  Plateforme inconnue = refusée à l'ouverture, avant toute trame de couleur. 0A01 (G7 7500, 16 zones + barre) : table connue, non implémentée. JAMAIS envoyés : 0x21
  (animations, sauvegarde en flash), 0x22, 0x28 (reset), 0xFF (effacement). Délai 60 ms par trame ; la réponse est lue et une trame identique renvoyée = erreur.
- Aucun réglage non lumineux (disposition, touches de fonction, polling) n'est établi pour ces claviers de portable.

## Non implémenté (mécanisme seulement)

| Constructeur | Mécanisme | Décision |
|---|---|---|
| HP OMEN (4 zones) | WMI `ROOT\WMI` (appel BIOS), pas d'USB | passe par WMI/EC, non implémenté, interdit |
| Acer Nitro/Predator | HID 0CF2:5130 (page 0xFF5A usage 01), contrôleur ENE **dans l'EC** du portable, feature `A4 <id> <mode> <lum> <vitesse> <dir> R G B <masque zones> 00` [S] | passe par l'EC (le PC de Momo est un Acer) : non implémenté, interdit |
| Acer PredatorSense/NitroSense (ventilos, modes) | WMI/ACPI/EC | passe par WMI/EC, non implémenté, interdit |
| Lenovo Legion 7 (par touche) | ITE 048D:C935 (Y740), C956/C957 (Y750), C968/C967 (Y760), C977/C978 (Gen 7), C988/C987 (Gen 8), C997/C998 (Gen 9), C197 (Gen 10), C195 (Legion 5 Gen 10) ; collection 0xFF89/0xCC [S] | HID, trames par touche non relevées : INCONNU |
| Lenovo Vantage / Legion Zone (modes d'alimentation) | WMI/ACPI | passe par WMI/EC, non implémenté, interdit |
| MSI Raider / GE68 (clavier par touche + barre) | SteelSeries 1038:1122 (clavier Raider A18), 1038:1161 (barre Raider A18), 1038:113A (clavier GE68HX), 1038:114B (barre GE68HX) [S] | protocole non relevé : INCONNU. 1038 déjà revendiqué par la famille `steelseries` pour d'autres PID ; ces 4 PID ne le sont pas |
| MSI clavier MS-1562/1563/1564, 1565 | HID VID 1462 (MSI), usage 0x00FF/01 : PID 1562-1564 (Mystic Light 64 octets), 1601 (MS_1565) [S] | non relevé : INCONNU |
| Gigabyte Aorus 15/17 | HID VID 0414, interface 3, page 0xFF01 usage 01 : clavier 7A3F (17X) / 7A43 (15BKF), rétroéclairage 7A42 / 7A44 [S] ; mode direct = report `08 01 R G B <lum 0..0x32> 00 <0xFF - somme>` | persistance du mode direct non établie : non implémenté |
| Alienware AW410K / AW510K (périphériques, pas portables) | HID VID 04F2, PID 1968 / 1830, interface 2, page 0xFF0x [S] | hors portables, non relevé |
| Dell Alienware portables (AlienFX 4 zones historiques, 187C:0501/0527…) | non relevé [S] | INCONNU |
| Razer Blade | déjà couvert par la famille `razer` | non refait |

## PID à intégrer au catalogue (non modifié ici)

À ajouter à `assets/rgb_usb_catalog*.json` / `rgb_catalog.rs` par le chantier catalogue (non touchés dans cette branche) :
`048D:C955`, `C965`, `C963`, `C975`, `C973`, `C985`, `C984` (Lenovo 4 zones, famille `lenovo_4zone`) ; `1770:FF00` (MSI 3 zones, `msi_3zone`) ; `187C:0550`, `187C:0551` (Dell G, `dell_g`).
Reconnaissance sans pilote : les PID de la section précédente.
