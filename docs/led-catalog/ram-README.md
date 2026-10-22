# Catalogue LEDs barrettes RAM RGB (ram.json)

Methode : recherche web (WebSearch + WebFetch), 2026-10-22. Une ligne par serie/generation.
confidence : official = page/communique/article du constructeur ; retailer = fiche revendeur ou presse secondaire ; unknown = led_count null.
Aucune valeur extrapolee. Le champ control vaut "inconnu" partout : aucune source consultee ne donne bus/adresse/puce
(SMBus/I2C non confirme par un document officiel). Ne pas le deduire sans reverse (OpenRGB, etc.).

Verifie (official) : Corsair Vengeance RGB Pro 10, RGB Pro SL 10, Vengeance RGB DDR5 10, Dominator Platinum RGB DDR4 12,
Dominator Platinum DDR5 12, Dominator Titanium 11 ; Kingston Beast RGB DDR4 Special Edition 10, Renegade DDR5 RGB Limited Edition 12.
Retailer : Corsair Vengeance RGB RS 6, Thermaltake TOUGHRAM RGB D5 8 (XG = 16 selon revendeurs), Lexar ARES RGB DDR5 7 (source unique).

Trous (null) : G.Skill (toutes series), Kingston Beast standard et Renegade DDR4, HyperX, ADATA XPG, TeamGroup, Patriot,
Lexar DDR4 ; non recherches : Crucial, Mushkin, Geil, PNY, Zadak, Silicon Power, OLOY, Klevv, Acer Predator.
part_prefix : Corsair DDR5 Dominator "CMP" non verifie ; autres prefixes indicatifs, a confirmer.
Les pages kingston.com ont renvoye 403 au fetch direct ; chiffres Kingston issus des extraits de recherche.
A faire : ouvrir les fiches G.Skill/ADATA/TeamGroup/Patriot par SKU, ou compter via photos/demontage.
