# Thermaltake (TT RGB PLUS / TT PlayLink)

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Clones lus : `pcmx1/thermaltake-lcd-linux` @9f1423d (MIT), `messiahlap/th420-display` @6999fcc (sans licence declaree dans l'API GitHub, Rust ; a traiter comme "tous droits reserves" : ne rien copier).
Autres : `bekindpleaserewind/ttlcd` (GPL-3.0, Tower 200 Mini "LCD Panel Kit"), `AaronC-585/ttlcd-cpp` (GPL-3.0) : hors AIO.

## Logiciel officiel

- **TT RGB PLUS 2.0** pour la serie Ultra (TH420 V2 Ultra EX) : bouton de telechargement sur la page produit <https://www.thermaltake.com/th420-v2-ultra-ex-argb-sync-all-in-one-liquid-cooler.html>. TT RGB PLUS 2.0 et 1.0 s'ecrasent l'un l'autre. Telechargements generaux : <https://thermaltake.com/pages/downloads> (liste aussi TT RGB PLUS 3.0, "TT LCD Screen (MINECUBE)", "TT SEGMENT LCD" : rattachement a la TH420 non confirme). Envoi d'ecran aussi possible depuis l'app mobile **TT PlayLink**. [recherche web, non telecharge]

## Modeles d'ecran et VID:PID (VID Thermaltake **264A**)

| Produit | Ecran | PID | Source |
|---|---|---|---|
| TH420 V2 Ultra EX ARGB (LCD) | **2,1" rond 480x480** TFT | **233C** (2 interfaces HID :1.0 controle, :1.1 image) | `th420-display` [V : constantes `VID`/`PID`, README] |
| "Thermaltake AIO cooler LCD" (rond 480x480) | 480x480 | **2328** | `thermaltake-lcd-linux` `tt-lcd-aio.py` [V] |
| RC Pro (Tower 500 / 900), 3,9" | **480x128** rectangulaire | **232A** | `tt-lcd-rc-pro.py` [V] |
| Tower 500 | [?] | **233D** | `tt-lcd-tower500.py` [V : constantes] |
| Floe Ultra / autres | [?] | [?] | non trouves |

## Protocole : documente par deux projets recents (sources non officielles)

Deux formats observes, **JPEG sur HID** dans les deux cas :

- **AIO 264A:2328** et **RC Pro 264A:232A** [V, README `thermaltake-lcd-linux`] : init par feature reports (`SET 0x1A`, `GET 0x07`, `SET 0x0C` x3 (x4 sur RC Pro), `GET 0x0F`, + `SET 0x1D` RC Pro), puis JPEG decoupe en morceaux de 1016 o prefixes d'un en-tete de 8 o `02 09 00 <dernier?> <len LE16> <index LE16>` ; le flag `01` sur le dernier morceau declenche le rendu ; sur RC Pro les morceaux doivent suivre immediatement le `SET 0x1D` sur le meme fd.
- **TH420 V2 Ultra EX 264A:233C** [V, `src/device.rs`] : interface de controle :1.0 (rapports de 440 o, commandes `85/87/84/81/80 01 00 80`, lecture de la temperature de liquide dans la reponse), interface image :1.1 (paquets de 1024 o : `08 <n> 00 <80?> + 1020 o JPEG`, premier paquet = nombre total de morceaux, precede d'une commande `12 01 00 80 64`). Format different de l'AIO 2328 ci-dessus, d'ou la prudence : **deux familles**.

Confiance : **moyenne** (deux auteurs, tests sur leur materiel, depots jeunes, pas de recoupement independant pour 233C).

## A decouvrir par REA

1. Rapprocher les deux formats (est-ce la meme famille avec versions de firmware differentes ?), listes completes de commandes (luminosite, rotation, stockage).
2. Autres produits LCD Thermaltake (Floe Ultra, tours "View LCD", MINECUBE) : PID, resolution.
3. Le sens des champs `0x64` du `12 01 00 80 64` (cadence ? qualite ?).

Methode :
- Cibles : **TT RGB PLUS 2.0** (et 3.0 si la machine du dev la possede) ; chaines `264A`, `233C`, `2328`, `232A`, `LCD`, `JPEG`, `PlayLink`. Verifier Electron / .NET.
- Capture : USBPcap filtre `usb.idVendor == 0x264a` ; scenarios : image fixe, GIF, theme capteurs, rotation.

## Effort / priorite

Effort **faible-moyen** (JPEG sur HID, pas de chiffrement). Priorite **2** pour TH420 V2 Ultra EX et AIO LCD (modele grand public, 480x480 rond).
