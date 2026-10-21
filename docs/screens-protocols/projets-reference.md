# Projets de reference cross-constructeurs

Date : 2026-10-21. **[V]** = lu dans le depot clone / la page ; **[S]** = suppose.

| Projet | Licence | Ecrans pris en charge | Remarques / usage pour PureRGB |
|---|---|---|---|
| **liquidctl** <https://github.com/liquidctl/liquidctl> @292dc56 | GPL-3.0+ | LCD : NZXT Kraken Z53/63/73, 2023, 2023 Elite ("broken"), 2024 Elite RGB, 2024 Plus (images, GIF, mode liquide, luminosite, orientation) ; **MSI MPG Coreliquid K360** (320x240, experimental) ; **Lian Li GA II LCD** (v1.16.0, partiel : l'ecran attend un flux H.264 via HID). **Pas d'ecran** : ASUS Ryujin III ("The screen of the cooler is not yet supported") [V]. | Source de verite pour Kraken (deja utilise). Ne pas copier le code (GPL) ; lire les guides `docs/*.md` pour la doc. |
| **CoolerControl** <https://gitlab.com/coolercontrol/coolercontrol> | GPL-3.0+ | Reprend les pilotes liquidctl (via daemon Python) ; selon `damachine/coolerdash`, les ecrans Kraken 2023 / Z63 / Plus 240 sont pilotables par son API (>= 3.1.0) [V via coolerdash] | CoolerDash (MIT) montre le rendu externe (CPU/GPU/liquide). Pas d'API LCD generique documentee ici [S]. |
| **OpenLinkHub** <https://github.com/jurkovic-nikola/OpenLinkHub> @c8c1d2a | GPL-3.0+ | Corsair iCUE LINK AIO LCD (0C4E), XD5 LCD (0C43), Nautilus LCD cap (0C57/0C55), XC7 Elite LCD (0C42 : driver `xc7`) [V] | Meilleure source Corsair ; lecture seule, ne pas copier. |
| **lian-li-linux** <https://github.com/sgtaziz/lian-li-linux> @f55f3e5 | **MIT** | Voir `lian-li.md` (Galahad II LCD/Vision, HydroShift LCD et II, TL/SL LCD, Lancool 207, Universal Screen 8,8", Vision 9,2", OLED Curve) [V] | La plus riche et la seule en MIT : reutilisation possible (avec attribution) ; l'auteur precise que le protocole a ete analyse avec l'aide d'IA sur captures + logiciel constructeur : donc tout recouper. Fork `CrocolocoBear/lian-li-linux-strimmer` (MIT). |
| **turing-smart-screen-python** <https://github.com/mathoudebine/turing-smart-screen-python> | GPL-3.0 | Turing/TURZX/XuanFang/WeAct, UART + USB 1CBE | Voir `generiques-usb.md` ; autre agent. |
| **OpenRGB** <https://openrgb.org> | GPL-2.0 | Surtout RGB ; selon les pages wiki trouvees, ne pilote pas les LCD [S : non verifie en detail] | Pas de reference ecran. Utile pour l'etiquetage de VID:PID des controleurs RGB des memes coolers (ex. issue Galahad II Trinity <https://gitlab.com/CalcProgrammer1/OpenRGB/-/issues/3647>). |
| **SignalRGB** <https://signalrgb.com> | proprietaire | LCD : support partiel Galahad II LCD ("some support" selon staff), Kraken Elite sans controle LCD satisfaisant (forum : <https://forum.signalrgb.com/t/cant-disable-galahad-ii-lcd-screen-in-signal-rgb/8586>, <https://forum.signalrgb.com/t/support-and-or-workaround-to-allow-rgb-control-of-nzxt-kraken-elite-with-lcd-screen/8720>) | Concurrent, pas source de protocole. |
| **AIDA64** (FinalWire) | proprietaire | LCD externes : AX206, Turing/TURZX (2,1" a 8,8"), ASUS Ryujin (mode "display control" d'Armoury Crate a mettre sur Off) [V : forum AIDA64 + page ASUS en recherche] | Montre qu'un logiciel tiers peut piloter ces ecrans ; pas de protocole publie. |
| **HWiNFO** | proprietaire | Pas de pilote LCD generique a ma connaissance [S : non verifie] | Fournit les capteurs (memoire partagee) ; utile en source de donnees, pas en driver. |
| **lcd4linux** <https://github.com/ukoda/lcd4linux-ax206> | GPL-2.0 [S] | LCD caractere/graphiques ; AX206 (fork) | Historique, pas de constructeurs recents. |
| `zaclanzon/ryujin-iii-lcd-linux` | MIT | ASUS Ryujin III (0B05:1AA2 etc.) | Voir `asus.md` : doc de protocole complete. |
| `siam-parvez/opencreative` | MIT/Apache-2.0 | DeepCool LM-series (3633:0026) ; "NZXT, Lian Li, ASUS en route" (README) | Architecture Rust proche de la notre (crates protocol/device/render) ; voir `deepcool.md`. |
| `zazazaxky-m/deepcool-lcd` | MIT | DeepCool LM360 | Doc `PROTOCOL.md`. |
| `zorrobyte/icue-link-lcd-reactor` | MIT | Corsair iCUE LINK 0C4E | Doc dans README. |
| `pcmx1/thermaltake-lcd-linux` | MIT | Thermaltake AIO 2328 / RC Pro 232A | Voir `thermaltake.md`. |
| `messiahlap/th420-display` | non declaree | Thermaltake TH420 V2 Ultra EX 233C | Idem (ne rien copier). |
| `yesthatsjames/lightgarden` | MIT | ID-COOLING FX LCD 2000:3000 | Voir `id-cooling.md`. |
| `duckyman-ai/corenexd`, `FeikoWielsma/cm-pixel` | MIT | Cooler Master Core Nex Digital (segments), Atmos II Pixel | Voir `cooler-master.md`. |
| `andyjusa/hydroshift2-rs` | MIT | Lian Li HydroShift II LCD-C (1CBE:A021) | Bibliotheque Rust de protocole + tests ; recoupe `crypto.rs`. |

Toutes les URLs de depots GitHub ci-dessus ont ete consultees le 2026-10-21 (clone shallow ou page).
