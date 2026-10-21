# Autres constructeurs (peu ou pas de documentation publique)

Date : 2026-10-21. Methode : recherches web (WebSearch) et GitHub (`gh search repos`) par nom de constructeur/produit. **Aucun VID:PID n'est donne tant qu'il n'est pas lu dans un depot.** Absence de resultat != preuve d'absence : a refaire avant la phase REA.

## Resume

| Constructeur | Produits avec ecran (source) | Logiciel officiel | Protocole public ? | VID:PID |
|---|---|---|---|---|
| Arctic | Aucun AIO a ecran trouve (Liquid Freezer III Pro : pas d'ecran dans mes recherches) | - | N/A | - |
| Antec | rien trouve | [?] | non | [?] |
| Phanteks | **Glacier One 360M25-LCD**, ecran IPS 6" 1480x720 60 Hz, annonce fev. 2026 (<https://www.vortez.net/news_story/phanteks_introduces_glacier_one_360m25_lcd_aio_with_6_inch_ips_display.html>) | [?] | non | [?] |
| Montech | rien trouve | [?] | non | [?] |
| Jonsbo | ecran USB **960x376** (cle de boitier/ecran) ; `ZhouhaoJiang/JonsboCanvas` (Windows WinUI) vise `VID_345F&PID_9132&MI_03` [V : README + `msusbdisplay.inf`] ; le pilote embarque s'appelle "MSUSBDisplay" -> probablement un adaptateur video USB (type moniteur virtuel) et non un LCD a protocole proprietaire [S] | logiciel Jonsbo [?] | non decrit (le depot utilise un pilote Windows) | **345F:9132** (interface MI_03) [V] |
| Zalman | rien trouve | [?] | non | [?] |
| ID-COOLING | voir `id-cooling.md` | FX LCD Software | oui (communautaire) | 2000:3000 |
| be quiet! | **Light Loop IO LCD** (240/360), IPS 2,1", 500 nits, sortie aout 2026 (<https://www.club386.com/be-quiet-light-loop-lcd-aio-launch/>) | **IO Center** (reglages sauvegardes dans la memoire de l'AIO) | non | [?] |
| Fractal Design | aucun resultat pour "Celsius" a ecran | - | N/A | - |
| EK | **EK-Nucleus AIO "Vision"** (LCD couleur 24 bits, sources via recherche : <https://www.funkykit.com/news/pc-computers/ek-adds-a-new-player-to-the-closed-loop-liquid-cooling-game-the-ek-nucleus-aio/>) | [?] | non | [?] |
| ASRock | Taichi AQUA (capot 3,4" magnetique), Phantom Gaming 360 LCD 3,4" (<https://www.kitguru.net/page/38/!https:/www.kitguru.net/tech-news/featured-tech-reviews/matthew-wilson/ces-2026-asrock-showcases-new-aio-coolers-qd-oled-monitor-and-more/>) | [?] | non | [?] |
| Thermalright | couvert par un autre agent (TRCC) | - | - | - |

## Pistes de recherche suivantes

- Chercher par VID:PID reels une fois un produit en main (`lsusb`/Gestionnaire de peripheriques) : c'est la bonne methode pour ces constructeurs.
- be quiet! IO Center, Phanteks (logiciel "Phanteks Glacier"/Phanteks DRGB ?), EK : installer le logiciel officiel dans une VM et lister `USB\VID_xxxx` avec `pnputil /enum-devices /connected /class USB`.
- Beaucoup de ces produits sont des OEM de **Asetek** (Ryujin, Ryuo), **CoolIT**, **Hotspot (2000:3000)** ou **TURZX (1CBE)** : reperer le VID du controleur avant de chercher un protocole propre.

Methode REA generique : voir `README.md` section "Procedure commune".

## Priorite

3 pour tous (ni protocole ni materiel de test). be quiet! IO LCD et Phanteks 6" : a surveiller (nouveautes 2026).
