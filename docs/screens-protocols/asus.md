# ASUS ROG (Armoury Crate) - Ryujin III / Ryujin II

Date : 2026-10-21. **[V]** verifie dans la source, **[S]** suppose, **[?]** inconnu.
Clones lus : `zaclanzon/ryujin-iii-lcd-linux` @d52214e (MIT, `docs/protocol.md` + `capture/`), `kuroame-dev/killryujin` @892be8a (MIT), `liquidctl/liquidctl` @292dc56 (GPL-3.0+, `docs/asus-ryujin3-guide.md`).

## Logiciel officiel

- **Armoury Crate** (Windows). Page : <https://rog.asus.com/content/armoury-crate/> (installeur "framework" ~2 Mo auto-detection, ou package complet ~3 Go) ; ASUS fournit aussi un outil de desinstallation. Le pilote d'ecran passe par le composant `AacAIOFanHal` (version 1.5.10.0 vue dans la capture de reference). ASUS indique de mettre le mode de controle d'affichage d'Armoury Crate sur Off pour utiliser AIDA64 sur l'ecran. [recherche web + capture du depot de ref.]
- Le cooler doit etre branche a un header USB 2.0 interne de la carte mere.

## Modeles et VID:PID (VID **0B05**) [V : `liquidctl/driver/asus_ryujin.py`, killryujin]

| Produit | PID | Ecran |
|---|---|---|
| ROG Ryujin II 360 | 1988 | 3,5" LCD [?] (liquidctl : ecran non gere) |
| ROG Ryujin III 360 | 1AA2 | **320x240** (3,5" selon le depot de ref.) |
| ROG Ryujin III Extreme | 1BCB | idem [S : meme famille, killryujin le liste] |
| ROG Ryujin III White Edition | 1ADA | idem (teste par killryujin) |
| ROG Ryujin III EVA | 1ADE | idem |
| ROG Ryuo I 240 | 1887 | OLED 1,8", ventilos seulement dans liquidctl [?] |

Le 360 a ete decrit par une capture USBPcap d'Armoury Crate : firmware `AURJ2-S750-0108`. Le README du depot ref. parle de "3.5 inch 320x240".

## Protocole : DOCUMENTE publiquement (ecrit, avec captures)

Fiche principale : <https://github.com/zaclanzon/ryujin-iii-lcd-linux> -> `docs/protocol.md` (13 Ko, MIT) et `capture/` (timeline HID parsee, charges bulk, pcapng, runbook de capture). Resume [V d'apres ce document, verifie par l'auteur sur materiel] :

- Peripherique composite : interface 0 = vendeur 0xFF, bulk OUT 0x01 / IN 0x81 (donnees de fichier, ecritures de 4096 o) ; interface 1 = HID, interrupt OUT 0x02 / IN 0x82, 64 o (toutes les commandes).
- Commande = rapport de sortie de 65 o : ID `0xEC` + 64 o. Reponse : rapport d'entree `EC <cmd> ...` (certains "get" repondent avec un autre ID : 82->02, 99->19...). Evenements asynchrones : ID `0xEE` pendant les operations fichier (`ee13` effacement/slot, `ee14` ACK de chunk).
- Fonctions verifiees : luminosite (octet 7 de la commande `5C`), veille, page "hardware monitor" (3 lignes label/valeur, glyphes unites en points de code prives), envoi et suppression de GIF/JPEG dans le stockage de l'AIO (16 slots, Armoury Crate en utilise 10, ~32 Mo, 8 Ko par fichier), lecture d'un GIF stocke, JPEG de fond + 6 lignes de texte, horloge, lectures firmware/etat/table de stockage.
- Commandes cles : `51` playMedia, `52/53` hardware monitor, `5C` statut ecran, `5D` slideshow, `60` fond + texte, `71/72/73/7F` gestion de fichiers.
- **Mise a jour "en direct"** : l'appareil garde le dernier contenu choisi sans hote ; pour un affichage dynamique le depot de ref. envoie seulement les commandes `53` (lignes) ; killryujin evoque en plus des **trames brutes 320x240 BGR sur l'endpoint bulk** (non verifie ici : README de killryujin uniquement [S]). Une video 30 fps temps reel n'est PAS decrite.
- Risques [V] : ecrire en flash pendant qu'Armoury Crate tient le HID desynchronise la poignee de main (`ee13 1001`, mi-effacement) ; ne pas reessayer en boucle, arret total de l'alimentation ~30 s si bloque (killryujin).
- Coexistence : le pilote hwmon Linux `asus_rog_ryujin` partage l'interface HID ; sous Windows fermer Armoury Crate / Aura / OpenRGB (ils lisent la meme interface et volent les paquets `0xEE`).

Confiance : **elevee** pour le Ryujin III 360 (0B05:1AA2, firmware AURJ2-S750-0108) ; **moyenne** pour White/Extreme/EVA (meme famille, firmware different possible) ; **faible** pour Ryujin II (pas d'ecran decrit).

Licence : MIT -> reimplementation d'apres la doc OK ; liquidctl (GPL) ne decrit pas l'ecran.

## A decouvrir par REA

1. **Streaming temps reel** sur le bulk (trame BGR 320x240 ? framing, cadence, mode "display control" ON) : c'est la cle pour un vrai "ecran pilote par PureRGB" et non un simple envoi de GIFs stockes.
2. Variantes Extreme / EVA / White : resolution, firmware, differences de commandes.
3. Ryujin II 360 : ecran jamais decrit.
4. Persistance : limite d'usure flash, marche a suivre en cas d'echec.

Methode :
- Cibles : service/DLL **AacAIOFanHal** d'Armoury Crate (dossier `C:\Program Files (x86)\LightingService\` ou `\ASUS\ARMOURY CRATE Service\` [S], a verifier), ainsi que les modules "AIO" de l'app. Chaines : `EC`, `AURJ`, `0B05`, `1AA2`, `setHWmonitorLayout`, `enableFileStreamTransfer`, `WinUSB`, `bulk`.
- Capture : USBPcap sur le bus de l'AIO (filtre `usb.idVendor == 0x0b05`), scenarios : (a) mode "monitoring" avec valeurs changeantes, (b) GIF long, (c) mode "extended display"/video s'il existe ; comparer au runbook `capture/` du depot de ref.
- REA : `inspect_pcap_capture`, puis `trace_feature` sur la DLL pour la couche bulk.

## Effort / priorite

Effort **moyen** (HID 65 o + bulk 4096 o, gestion d'etat des fichiers, fragile). Priorite **2** pour les affichages "capteurs" (simple), **3** pour des vraies animations tant que le streaming n'est pas decode.
