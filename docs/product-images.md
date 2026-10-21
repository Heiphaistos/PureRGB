# Photos produit (catalogue `product_catalog.json`)

Fichier de donnees : `src-tauri/assets/product_catalog.json` (version 1).
Verificateur : `tools/product-catalog/verify.py`.

Le moteur affiche la photo d'un produit uniquement quand PureRGB l'identifie exactement.
L'image est **telechargee a l'execution** depuis `image.url` (jamais embarquee dans le
depot : droits d'auteur), decodee puis mise en cache. Le catalogue ne contient que des URL.

## Schema

```json
{"version":1,"products":[{
  "id":"thermalright-trofeo-vision-9-16",
  "kind":"screen|light|fan|cooler|aio|keyboard|mouse|ram|motherboard|gpu|other",
  "vendor":"Thermalright","name":"Trofeo Vision 9.16 LCD",
  "match":{"usb":["0416:5408"],"names":["trofeo vision 9.16"]},
  "image":{"url":"https://...","source_page":"https://..."}}]}
```

- `id` : `[a-z0-9-]`, unique, prefixe par le fabricant.
- `match.names` : sous-chaines en minuscules sans accents (>= 5 caracteres), jamais le
  fabricant seul. Normaliser le nom detecte de la meme facon avant comparaison.
- `match.usb` : `vid:pid` hexa minuscules. Present **seulement** si le couple est propre a
  ce produit et connu de nos pilotes / docs (`docs/screens-protocols/*.md`,
  `src-tauri/src/screens/drivers/*.rs`). Les VID:PID partages entre plusieurs produits
  (ex. `0416:5302`, `0416:5406`, `0402:3922`, `87ad:70db`, `87cd:70db`, `3633:0026`
  LM240/LM360, `1b1c:0c4e`) n'y figurent PAS : un couple partage ne designe pas un modele.
  `verify.py` refuse un `usb` utilise par deux entrees.
- `image.source_page` : page produit officielle du fabricant.

## Regle de resolution pour le moteur (IMPORTANT)

Plusieurs produits ont des noms imbriques (`kraken elite 360` dans `kraken elite 360 rgb`,
`th420 v2` dans `th420 v2 ultra ex`...). Le moteur doit :
1. essayer `usb` d'abord (exact) ;
2. sinon, parmi toutes les sous-chaines de `names` contenues dans le nom detecte
   (normalise), retenir **la plus longue** ; une egalite entre deux produits = pas de photo.

`verify.py` liste ces imbrications en AVERT pour memoire.

## Methode

1. Sources : uniquement le site officiel du fabricant ou son CDN officiel. Jamais un
   revendeur, une marketplace ou un hebergeur d'images tiers. Pas d'image officielle
   fiable = pas d'entree (on prefere l'icone a une mauvaise photo).
2. L'image est celle de la galerie / `og:image` / sitemap produit de la page officielle
   (Thermalright : premier visuel de la fiche ; NZXT, Thermaltake : sitemap Shopify ;
   Corsair : `og:image` redimensionnee `w_800` ; Lian Li, Cooler Master : galerie de la fiche).
3. Chaque image est regardee (planche-contact) : bon produit, pas de banniere, pas de
   badge de prix/recompense, pas d'ambiance sans produit.
4. `verify.py` revalide tout : schema, ids/usb uniques, `https`, HTTP 200, `Content-Type:
   image/*`, octets reellement image (PNG/JPEG/GIF/WebP), taille < 4 Mio, images
   identiques entre produits (AVERT : certains CDN renvoient la meme image pour un chemin
   bidon, DeepCool notamment).

## Hotes d'images (liste blanche du moteur)

| Hote | Fabricant |
|---|---|
| `www.thermalright.com` | Thermalright |
| `www.turzx.com` | Turing Smart Screen (TURZX) |
| `lian-li.com` | Lian Li |
| `nzxt.com` | NZXT (Shopify, `/cdn/shop/files/...?v=..&width=1024`) |
| `thermaltake.com` | Thermaltake (Shopify) |
| `assets.corsair.com`, `res.cloudinary.com` | Corsair (comptes Cloudinary `corsair-pwa`) |
| `cdn.deepcool.com` | DeepCool |
| `dlcdnwebimgs.asus.com` | ASUS / ROG |
| `www.coolermaster.com` | Cooler Master |
| `www.arctic.de` | Arctic |
| `assets3.razerzone.com` | Razer |

Redirections : ne suivre que vers un hote de cette liste. Envoyer un `User-Agent`
(certains CDN refusent l'absence d'UA). Formats : JPEG, PNG et **WebP** (DeepCool sert du
WebP sous une extension `.jpg` : decoder d'apres les octets, pas l'extension).
`res.cloudinary.com` est un hote partage : n'autoriser que le chemin `/corsair-pwa/`.

## Ajouter / verifier une entree

1. Trouver la page produit officielle ; reperer l'image (`og:image`, galerie, sitemap).
2. Ajouter l'objet au JSON (ids stables, `names` specifiques, `usb` seulement si prouve).
3. `python tools/product-catalog/verify.py` (code retour 0 attendu ; `--offline` = schema
   seul ; `--prune` retire les entrees dont l'image repond 404/non-image, jamais sur panne
   reseau).
4. Ouvrir l'image et verifier a l'oeil que c'est le bon produit.

Re-lancer `verify.py` regulierement (les CDN changent les chemins) et avant chaque release.

## Limites

- Les URL peuvent mourir (refonte du site) : le moteur doit tolerer 404/timeout et garder
  l'icone par defaut. Ne jamais bloquer l'UI sur ce telechargement.
- Couleur : une seule photo par modele (version noire ou premiere variante de la fiche).
- Debits : 74 Mo au total pour 133 images, mais le moteur n'en telecharge qu'une par produit
  detecte (max 3 Mo).
- Non couverts, faute d'image officielle accessible automatiquement : Noctua (HTTP 429),
  be quiet! (HTTP 403/523), MSI (403), Logitech G (seulement une image generique
  `og:image`), ASUS Strix Scope II (image > 4 Mio), Turing 5" / XuanFang / UsbPCMonitor /
  Kipye (pas de fiche officielle exploitable), Lian Li Galahad II Vision / Universal Screen
  8.8 sur ses ids USB, Corsair K70/K100 (absents du sitemap US), Thermaltake RC Pro / Tower 500.
