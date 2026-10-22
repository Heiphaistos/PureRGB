# images-boards.json - images de cartes meres (2026-10-22)

Couverture : 95 / 136 cartes (MSI 31/31, ASUS 27/33, Gigabyte 37/41, ASRock 0/24, Maxsun/Biostar/Colorful 0/7).
Chaque URL d'image a ete lue sur la page produit officielle (og:image) puis verifiee en GET :
statut 200, content-type image/png|webp, taille <= 4 Mo. Aucun fichier conserve.

## Hotes d'images (tous des sous-domaines du constructeur)
- storage-asset.msi.com (29, webp) et asset.msi.com (2, png) : MSI
- dlcdnwebimgs.asus.com (27, png) : CDN d'ASUS (hote different de asus.com, sous-domaine asus.com)
- static.gigabyte.com (37, png) : Gigabyte

## Cartes sans image
- ASRock (24) : asrock.com protege par Incapsula (challenge), inaccessible a curl, WebFetch, navigateur et Wayback.
- Maxsun (3), Biostar (3), Colorful (1) : hors des 4 constructeurs vises, non traites.
- ASUS : PRIME B550M-A (image 0 octet), PRIME B860M-A WIFI, TUF GAMING B550-PLUS, ROG STRIX B550-F GAMING,
  ROG STRIX B450-F GAMING, PRIME B550-PLUS (PNG > 4 Mo, rejetes).
- Gigabyte : B860M DS3H WIFI6E, X870E AORUS ELITE WIFI7, B850 AORUS ELITE WIFI7, B450M DS3H V2
  (content-type renvoye "image/Png" avec majuscule, rejete par prudence).

## Notes
- match.names = nom du modele en minuscules ; + code MSI "ms 7dXX" seulement s'il est unique dans boards.json.
- Chevauchements : un motif court peut etre contenu dans un modele plus long (ex. "mag b550 tomahawk" vs
  "mag b550 tomahawk max wifi", "b760m aorus elite" vs "... elite ax"). Le programme doit retenir le motif
  le plus long qui correspond.
- Pages sources MSI/Gigabyte lues via le navigateur (curl recoit 403) ; certaines pages ASUS/MSI via Wayback.
