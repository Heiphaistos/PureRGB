# fans.json - LEDs adressables par unite (ventilateurs, pompes, strips)

110 lignes : official 72, retailer 0, unknown 38 (led_count null).

## Methode
- 4 passes de recherche (WebSearch limite aux domaines constructeur + WebFetch), un nombre n'est retenu que s'il est ecrit.
- led_count = par UNITE (ventilateur, pompe/cap, bande) ; `leds_note` precise ce qui est compte.
- Aucune deduction : nombre absent = null + unknown.

## Limites de fiabilite (a reverifier avant usage critique)
- Beaucoup de nombres sont lus dans le resume de recherche d'une page constructeur, sans relecture de la page (pages NZXT support en 403, Noctua en 429, HYTE en 403). Marques "official" mais source non relue : NZXT Aer RGB 2 et Hue 2 Underglow, Fractal Prisma AL-12, Antec Prizm X, Montech RX120, tout le groupe be quiet!/Thermaltake/Cooler Master/Arctic/DeepCool/Phanteks (resumes de recherche).
- Corsair QX140/RX120/RX140 : tires d'un article comparatif Corsair, pas de la fiche de chaque modele.
- Noctua NF-A12x25 : 0 LED (gamme sans RGB), page non relue.
- Entrees ambigues : Thermaltake Riing Plus (12 ou 12 paires), Arctic Liquid Freezer III (12, perimetre flou), be quiet! Light Loop 360 (64 au total).
- Champ `connector` : Corsair/Lian Li/NZXT mis a "proprietaire" sans verification manuel par manuel (NZXT F120 RGB fonctionne aussi sur header ARGB 3 broches d'apres sa page).

## Non trouve (unknown)
- Aucune donnee officielle : ASUS (ROG Ryujin III, Strix LC II), MSI (MAG Coreliquid), Gigabyte (Aorus Waterforce), Thermalright, ID-Cooling, Zalman, Enermax, Darkflash.
- Partiel : Lian Li SL120 V2 / AL120 V2 / Galahad II, NZXT Kraken Elite/X, DeepCool FC120/LS720 pompe, Phanteks T30/Neon.
- Non traites : Phanteks Glacier One, Cooler Master Atmos/ML240L, Thermaltake Floe RC, DeepCool FL12/LT/LD, Arctic LF III 240/280, be quiet! Silent Wings Pro 4/Pure Wings 3 ARGB, Enermax, Montech Metal, Thermalright TL-K12/TL-C12, ID-Cooling, Antec Symphony, HYTE Flow. Objectif ~120 quasi atteint (110), mais 72 seulement avec nombre.
