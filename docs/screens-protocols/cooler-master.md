# Cooler Master (MasterCTRL)

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Clones lus : `duckyman-ai/corenexd` @9a57116 (MIT), `FeikoWielsma/cm-pixel` @47f543a (MIT, `PROTOCOL.md`).

## Logiciel officiel

- **MasterCTRL** (Windows). <https://www.coolermaster.com/en-global/masterctrl.html> ; notes de version <https://www.coolermaster.com/en-global/masterctrl-release-notes/> (v1.5.2.244 au 2026-10-06). Eviter le sous-domaine `legacy.` (plus ancien). [recherche web, non telecharge]
- Compagnon `MasterHUB` pour d'autres produits : <https://www.coolermaster.com/en-us/masterhub-download/>.

## Modeles d'ecran (fiches constructeur via recherche web)

| Produit | Ecran | VID:PID | Etat |
|---|---|---|---|
| MasterLiquid 360 Ion (+ White) | LCD 2,1" | [?] | rien en OSS |
| MasterLiquid Core LCD 6" (240/360) | 6", 960x480 | [?] | rien en OSS ; probablement un ecran "type moniteur" [S] |
| MasterLiquid Core Nex Digital | **ecran a segments** (pas pixel) | **2516:0234** [V corenexd] | protocole HID 64 o, voir ci-dessous |
| MasterLiquid Atmos II LCD / Atmos V2 Pixel | matrice hexagonale **32x32, 556 LED** RGB888 | **2516:021C** [V cm-pixel] | decode, MIT |
| MasterFrame 360 Stage LCD (boitier) | [?] | [?] | cite par MasterCTRL, non documente |

## Protocoles documentes

### Atmos II "Pixel" (2516:021C) [V `PROTOCOL.md`]
- HID composite 3 interfaces : iface 0 (usage page 0xFFFF) commandes/keepalive ; iface 1 (usage page 0xFF01) **canal d'image** ; iface 2 souris factice a ignorer.
- Une trame = **28 rapports HID de 64 o** vers l'interface 1 (prefixe `0x80 0xDD <idx BE16>` + 60 o de charge, rapport ID 0 donc 65 o cote hidapi). Rafraichissement continu ~8-15 fps.
- **Aucune init ni keepalive requis** (confirme en live) ; luminosite et rotation sont faites cote hote (mise a l'echelle des valeurs RGB). Peu pertinent pour PureRGB (ecran LED hexagonal, pas un ecran de pixels carre).

### Core Nex Digital (2516:0234) [V `frame.go`]
- Rapport ID 0x10, 64 o ; champs : mode (0x01 normal), charge CPU %, frequence MHz, icone CPU/GPU, temperature, unite, mini-barre, RPM, anneau %. Ce n'est pas un ecran graphique : hors module Ecrans (au mieux "afficheur de capteurs").

## Autres LCD (Ion, Core LCD 6", Stage LCD)

**Aucune documentation publique ni projet OSS** (recherche GitHub "MasterLiquid", "MasterCTRL" : aucun resultat pour LCD pixel). Ne pas supposer de VID:PID (VID Cooler Master 2516 connu seulement pour les deux produits ci-dessus).

## A decouvrir par REA

1. **MasterLiquid 360 Ion** (2,1") : PID, transport, format d'image.
2. **Core LCD 6"** (960x480) : mode moniteur ou protocole ? 
3. **Stage LCD** (boitier).

Methode :
- Cibles : **MasterCTRL** et son agent **CMAgent.exe** (cite par cm-pixel : `pixel_caps.json` "full device capability descriptor from CMAgent.exe" -> contient une table de capacites utile pour reperer les autres peripheriques). Chaines : `2516`, `ION`, `LCD`, `ScreenSaver`, `ssr`, `pixel`, `PID_`.
- Capture : USBPcap, filtre `usb.idVendor == 0x2516` ; scenarios : image, GIF, video, theme capteurs ; a repeter par produit.
- Verifier avec `Get-PnpDevice` si le Core LCD 6" apparait comme moniteur.

## Effort / priorite

Ion / Core LCD 6" : effort **eleve** (inconnu), priorite **2** (Cooler Master a un fort parc). Atmos II / Nex Digital : priorite **3** (non carres / non pixels).
