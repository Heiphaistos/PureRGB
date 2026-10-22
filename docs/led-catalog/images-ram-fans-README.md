# images-ram-fans.json
73 entrées : 16 mémoires (ram-) et 57 ventilateurs/AIO/bandeaux (fan-). Toutes les URL d'images ont été lues sur une page officielle
puis vérifiées par `curl -sIL` (statut final 200, content-type jpeg/png/webp, <= 4 Mo quand Content-Length est donné).
Validation Python faite (id, https, <= 500 car., jeu de caractères, motifs, sans doublon).
## Hôtes d'images hors domaine du constructeur
- res.cloudinary.com (19 Corsair) : CDN officiel de corsair.com (og:image, largeur portée de w_96 à w_600 dans l'URL, vérifiée 200).
- darkflash-image-cloud.s3.us-west-2.amazonaws.com (Darkflash) : bucket S3 du site darkflash.com.
- assets3.razerzone.com (Razer) : CDN de razer.com.
- Montech : pas de Content-Length, taille non vérifiable.
- images.teamgroupinc.com, media.kingston.com, dlcdnwebimgs.asus.com : sous-domaines/CDN officiels.
## Trous RAM (sans image)
XPG Spectrix/Lancer (image principale non identifiable sans ambiguïté), Lexar ARES (og = collage marketing), HyperX, Patriot Viper,
Thermaltake TOUGHRAM (429), Crucial, Mushkin, Geil, PNY, Zadak, Silicon Power, OLOY, Klevv, Acer Predator, Kingston FURY Beast SE
(même visuel que Beast RGB DDR4, non dupliqué).
## Trous ventilateurs (sans image)
- 429 bloquant (NZXT, Thermaltake partiel) : NZXT F120 RGB/Core/Duo, Kraken Elite, Aer RGB 2, Kraken X, HUE 2 ; Thermaltake Riing Plus 12,
  Toughfan 12, Pure Plus 12, Floe Riing 360, CT120, SWAFAN 12.
- 403 : HYTE (Q60, LS30), MSI (CoreLiquid C360/V2), Gigabyte/AORUS (WaterForce).
- Pas d'image produit exploitable (og générique ou absent) : Antec (3), Fractal Design (4), Zalman, ID-Cooling, Thermalright (2),
  Cooler Master MF120 Halo/ML360 Illusion (404)/PL360 Flux, DeepCool (CF120, FD12, FD12 V2, LS720, LS720 SE, Assassin IV),
  Corsair iCUE LINK QX140, RX140, RX140 MAX ; LS100 en une seule entrée « ls100 ».
- Strix LC II 240 : couvert par l'entrée 360.
## Notes sur les motifs
- Pro / Pro SL (« vengeance rgb pro » préfixe de « vengeance rgb pro sl ») et Dominator Platinum DDR4/DDR5 : séparation par motif
  distinctif (« pro sl », « dominator platinum ddr5 »), mais un nom OpenRGB identique pour DDR4/DDR5 ne peut pas être départagé.
- Image Lian Li SL120 = visuel multi-ventilateurs (image principale du site).
