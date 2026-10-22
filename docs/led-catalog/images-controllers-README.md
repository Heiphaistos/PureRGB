# images-controllers.json - hubs/controleurs, boitiers, cartes graphiques

Date 2026-10-22. 46 entrees : 11 hub-, 25 case-, 10 gpu-. 49 couples VID:PID relies, tous presents dans rgb_usb_catalog.json.
Validation : id/kind/noms/usb/URL (caracteres, longueur, https, hote) controles par script ; toutes les images : statut final 200, image/png|jpeg|webp, <= 4 Mo ; pages source 200 (sauf 429 ci-dessous).

## Couverture
- Corsair : Lighting Node PRO/CORE, Commander PRO, iCUE LINK System Hub, boitiers 4000X/5000X/7000X/3500X RGB/3500X ARGB.
- Lian Li : UNI HUB (a100-a105), Edge Hub V2, L-Wireless Controller, O11 Dynamic Razer (1532:0f13), O11 EVO/EVO RGB/EVO XL/Mini V2/Vision, Lancool 207 Digital/216/II Mesh RGB.
- Cooler Master : ARGB Gen 2 Controller A1 (2516:0173, 01c9, 01cb) ; TD500 ARGB, HAF 500/700, Cosmos C700M.
- Thermaltake : Riing Plus (264a:1fa5-1fb5) et Riing Quad (264a:2260-2270, 232b) ; photo = pack ventilateurs + controleur.
- NZXT : Control Hub, H5/H6/H7/H9 Flow RGB. Phanteks : NV5, NV7, Evolv X2.
- ASUS GPU : ROG Astral 5090/5080, ROG Strix 5070 Ti/5070, TUF Gaming 5090/5080/5070/4090/4070 Ti/3080 (noms seuls : les GPU n'ont pas de VID:PID USB).

## Trous
- Corsair Commander CORE / CORE XT (1b1c:0c1a..0c3e) : page CL-9011112 en maintenance (503) ; Commander CORE seul non vendu.
- NZXT RGB & Fan Controller / Hue 2 / Smart Device V2 : plus de page produit officielle.
- Razer Chroma Addressable RGB Controller (1532:0f1f) : page introuvable (404). Phanteks hub D-RGB : seulement page legacy sans image exploitable.
- MSI, Gigabyte/Aorus, Zotac : 403/468 pour curl et WebFetch. PNY, Palit, EVGA : pas de page exploitable. Fractal : og:image generique (ecarte). HYTE : Cloudflare 403. DeepCool : URLs 404.
- Anciennes ROG Strix (30/40xx) : pages absentes du site ROG.
- Thermaltake Riing Trio, Poseidon, SRGBmods, JGINYUE, GaiZhongGai : non traites.

## Hotes d'images
- assets.corsair.com, lian-li.com, www.coolermaster.com, phanteks.com, thermaltake.com, dlcdnwebimgs.asus.com (sous-domaine asus.com) : hotes constructeur.
- nzxt.com/cdn/shop/files/... : URL lue sur cdn.shopify.com via products.json de nzxt.com, puis servie par nzxt.com (200 verifie).
- Corsair : virgules des transformations Cloudinary encodees en %2C (200 verifie).
- Pages source nzxt.com et thermaltake.com repondent 429 a curl (anti-bot) ; URLs lues via products.json / WebFetch.
- Photos NZXT = boitier monte (image principale de la fiche).
