# Gigabyte AORUS (Gigabyte Control Center) - WATERFORCE / X II

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Sources : `FourGL/AorusWaterForce360-linux` @b58ddc1 (sans licence, Go), `aleksamagicka/gigabyte_waterforce-hwmon` (GPL-2.0, ventilos/pompe seulement), Phoronix <https://www.phoronix.com/news/Gigabyte-AORUS-Waterforce-Linux>.

## Logiciel officiel

- **GIGABYTE Control Center (GCC)** : package complet 26.08.03.01 (~858 Mo, 2026-08-03) sur la page support de chaque cooler, par ex. <https://www.gigabyte.com/CPU-Cooler/AORUS-WATERFORCE-X-II-360/support>. La mise a jour du firmware et de la "cooler library" se fait dans GCC (FAQ <https://www.gigabyte.com/Support/Consumer/FAQ/4338> ; SOP PDF X II 240/360 : <https://download.gigabyte.com/FileList/Manual/cooler_manual_AORUS-WATERFORCE-X-II_SOP.pdf>). Pas de telechargement ecran separe. [recherche web]

## Modeles

| Produit | Ecran | VID:PID |
|---|---|---|
| AORUS WATERFORCE X 240/280/360 (generation 1) | petit LCD de **texte** (°C, GHz, % CPU) [V: le daemon Go envoie 3 valeurs] | **1044:7A4D** ("Chu Yuen Enterprise Castor3") [V : `main.go` ligne 53, README] |
| AORUS WATERFORCE X II 240 / 360 | **LCD rond couleur "Edge View"**, video + texte (fiche Gigabyte) ; resolution [?] | [?] |
| AORUS WATERFORCE X II 280 | [?] | [?] |

## Protocole

- X (gen 1) : HID propre, ventilos/pompe documentes par le driver hwmon GPL-2.0 ; le daemon `AorusWaterForce360-linux` pousse seulement temperature / frequence / charge (3 chiffres) via `device.Write`. Pas de spec ecrite. Confiance **moyenne-faible** (un seul depot, sans licence).
- X II : **aucune documentation publique ni projet OSS trouve** (recherches web et GitHub). Aucun VID:PID connu dans mes sources : ne pas en inventer.

## A decouvrir par REA

1. Tout le X II : PID(s) (le meme controleur "Castor3" est probable [S], rien ne le prouve), resolution du LCD rond, transport, format de trame (video ? jpeg ? stockage interne).
2. X gen. 1 : format exact des paquets texte (octets du daemon Go) a verifier par capture.

Methode :
- Cible : **GCC** (`GIGABYTE Control Center`), modules "Cooler"/"AORUS WATERFORCE" ; chemin [S] `C:\Program Files\GIGABYTE\Control Center\`, a verifier. Chaines : `1044`, `7A4D`, `Castor3`, `Edge View`, `WATERFORCE`.
- Capture : USBPcap, filtre `usb.idVendor == 0x1044` ; scenarios : image fixe, video, texte personnalise, mise a jour du firmware LCD (separee dans GCC : ne PAS la rejouer sans controle).
- Ne jamais rejouer une trame de firmware.

## Effort / priorite

X II : effort **eleve** (inconnu), priorite **3** (marche de niche, aucun materiel de test). X gen.1 : priorite 3.
