# Catalogue eclairage cartes meres (boards.json)

Contenu : `vendor_rules` (regles par fabricant) + `boards` (136 cartes).
Principe : donnee absente = null / confidence "unknown". Rien d'extrapole.

## Methode
- MSI : manuels PDF officiels (download-2.msi.com) car msi.com renvoie 403. Noms JRAINBOW/JARGB_V2_n/JRGB, MS-xxxx lus dans les manuels.
- ASUS : pages Tech Specs asus.com / rog.asus.com. Noms recopies de la source.
- Gigabyte : pages /sp gigabyte.com. Ces pages donnent le NOMBRE de headers, pas leurs noms : les listes contiennent des libelles generiques repetes N fois (D_LED1/LED_C1 non confirmes).
- ASRock : manuels PDF officiels (tableau Onboard Headers) : ADDR_LED1..3, RGB_LED1..2.
- Biostar/Colorful/Maxsun : fiches revendeur / revues, confidence "retailer".

## Couverture (cartes)
MSI 31, ASUS 33, Gigabyte 41, ASRock 24, Biostar 3, Maxsun 3, Colorful 1. 129 "official", 7 "retailer".
Chipsets : LGA1700, LGA1851, AM5, AM4 (B550/X570/B450/X470/A520).

## Trous
- board_id : renseigne seulement pour MSI (sauf 3 cartes). null ailleurs (non publie).
- onboard_leds : null partout (aucune source ne donne un nombre de zones LED).
- Regles Gigabyte : chiffres issus d'un manuel (via fiche tierce) = "retailer" ; tension D_LED commutable selon carte, a verifier par carte.
- Regles ASUS : nombre de broches non indique sur la FAQ (null). Regles MSI JRGB : courant non publie.
- Biostar : 3 A d'apres Geizhals ; Maxsun/Colorful : max_leds et courant null.
- Cartes demandees mais absentes (page/manuel introuvable ou 403) :
  MSI PRO Z790-P WIFI, PRO B660M-A, PRO B550-A, PRO Z890-P, PRO B860-P, MAG Z690 TOMAHAWK, B350/X370 ;
  ASUS ROG STRIX Z890-E, X870-F, PRIME A520M-K, B450M-A, TUF X570-PLUS, pas de X870E/B850 ROG ;
  Gigabyte Z890 AORUS ELITE/MASTER, B860 AORUS ELITE, B860M DS3H ;
  ASRock B760M Pro RS/D4, H610M-HDV/M.2, A620M-HDV/M.2, B650M-HDV/M.2 ; Biostar B550MH, B760MX2-E.
- Peu ou pas de couverture : H610/B660/Z690 (surtout MSI/ASRock), A620, B350/X370.
- Cartes jumelles partageant un board_id (Z790 Tomahawk/Edge, B550 Tomahawk/MAX WIFI) : tel que publie.
