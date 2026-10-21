# MSI (MSI Center) - MPG/MAG CORELIQUID LCD

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Clone lu : `liquidctl/liquidctl` @292dc56 (GPL-3.0+) `docs/msi-mpg-coreliquid-guide.md`, `liquidctl/driver/msi.py` ; `william-drakemond/msi-coreliquid-control` (MIT, GUI sur liquidctl, pas de protocole propre) ; `sarzeaud/coreliquid_driver` (GPL-3.0, MEG CoreLiquid S360, ventilos).

## Logiciel officiel

- **MSI Center** (EZ Display tab pour les ecrans ; MSI recommande 2.0.65.0+ pour la serie P13, firmware P13_20251204v01). Blog : <https://www.msi.com/blog/how-to-master-your-mpg-coreliquid-p13-with-msi-center-complete-lcd-screen-customization-guide> ; pages produit par modele (ex. <https://www.msi.com/Liquid-Cooling/MPG-CORELIQUID-K360-V2>, <https://www.msi.com/Liquid-Cooling/MPG-CORELIQUID-D360>, <https://www.msi.com/Liquid-Cooling/MPG-CORELIQUID-P13>), le telechargement se fait depuis la page "support" de chaque produit. [recherche web ; lien direct MSI Center non verifie]
- Les premiers K240/K360 (2020/2021) utilisaient "Dragon Center" / MSI Center [S].

## Modeles et VID:PID

| Produit | Ecran | VID:PID | Source |
|---|---|---|---|
| MPG CORELIQUID K360 | **320x240** [V : guide liquidctl] | **0DB0:B130** [V `msi.py`] | liquidctl, "partiellement" |
| "Suspected MSI MPG Coreliquid" | [?] | 0DB0:CA00, 0DB0:CA02 [V : drapeau `_unsafe: experimental_coreliquid_cooler`] | liquidctl |
| MPG CORELIQUID K240 V2 / K360 V2 | LCD 2,4" [fiche MSI] | [?] | pages MSI |
| MPG CORELIQUID D240 / D360 | 2,4" IPS (GIF, MP4, photos, meteo) [fiche MSI] | [?] | pages MSI |
| MPG CORELIQUID P13 360 | **2,1" IPS**, modes : Hardware Monitor, Video, Horloge, Extended Display [fiche MSI] | [?] | blog MSI |
| MAG CORELIQUID LCD (C/A/M... certains avec ecran) | [?] | [?] | non verifie, la tache cite "MAG CORELIQUID LCD" |

Resolution/PID des modeles V2, D, P13 : **inconnus**. "Extended Display" (P13) suggere un mode ou l'ecran se comporte comme un moniteur : [S].

## Protocole : partiellement documente (K360 uniquement)

Le guide liquidctl decrit des fonctions cote utilisateur (taille 320x240, images redimensionnees, orientation, luminosite 0-100, canal "lcd", commandes `settings`, `hardware`, `banner`, `disable`, indices d'images "experimental"). Le protocole octet par octet n'existe qu'en code GPL (`liquidctl/driver/msi.py`) : **a ne pas copier** ; si on s'y refere, relire pour comprendre puis valider par capture. Pas d'autre projet OSS trouve couvrant le LCD MSI.

Confiance : **moyenne-faible** pour K360 (experimental dans liquidctl) ; **nulle** pour les autres.

## A decouvrir par REA

1. Tous les modeles hors K360 : PID, resolution, mode de transport (HID ? bulk ? moniteur ?).
2. Format de trame pour "Video" et "Extended Display" (P13).
3. Stockage interne vs flux hote.

Methode :
- Cible : **MSI Center** (modules "EZ Display" / pilote cooler ; chemin typique `C:\Program Files (x86)\MSI\MSI Center\` [S]) ; chercher les DLL/Exe de "Liquid Cooling", "EZ Display", "CoreLiquid".
- Chaines : `0DB0`, `B130`, `CA00`, `CA02`, `CORELIQUID`, `EZ Display`, `GIF`, `320`.
- Capture : USBPcap filtre `usb.idVendor == 0x0db0` ; scenarios : image fixe, GIF, hardware monitor, video, extended display. Comparer a `msi.py` pour verifier la validite de ses constantes.

## Effort / priorite

K360 : effort moyen (a partir de captures), priorite **3** (modele ancien, faible visibilite). V2/D/P13 : effort eleve (tout est inconnu), priorite **2** (marche en forte croissance, MSI Center repandu).
