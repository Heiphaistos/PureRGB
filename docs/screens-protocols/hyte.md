# HYTE (Nexus) - Y70 Touch / Y70 Touch Infinite / THICC Q60

Date : 2026-10-21. **[V]** verifie, **[S]** suppose, **[?]** inconnu.
Sources : fiches HYTE et revendeurs (recherche web) ; depots tiers `AppSprout-dev/vitrine` (GPL-3.0, Tauri/Linux), `YanissAmz/hyte-y70-touch-dashboard` (MIT), `ktmsft/hyte-panel`.

## Logiciel officiel

- **HYTE Nexus** : <https://hyte.com/nexus> (la page renvoie vers hellonexus.com pour Nexus 3.0, Windows/macOS/Linux) ; versions : <https://hyte.com/nexus/releases>, beta : <https://hyte.com/nexus/beta>. Guide : <https://support.hyte.com/hc/en-us/articles/27798413283220-Nexus-Software-Guide>. Compte facultatif. [recherche web, non telecharge]

## Modeles

| Produit | Ecran | Liaison |
|---|---|---|
| Y70 Touch / Touch Infinite | **14,5" 688x2560 (2,5K), 60 Hz, tactile 5 points** (Infinite) ; Y70 Touch d'origine 14,1" | **DisplayPort/HDMI + USB 2.0 (tactile) + SATA power** : c'est un **moniteur secondaire**, pas un LCD USB [V : fiches revendeurs] |
| THICC Q60 (AIO avec ecran) | [?] | [?] : rien trouve |

## Protocole

- Y70 Touch : **aucun protocole d'image a ecrire** : l'ecran est un moniteur ordinaire. Les depots tiers (Vitrine, hyte-y70-touch-dashboard) ouvrent simplement une fenetre plein ecran sur la sortie correspondante. Le tactile = HID standard (souris/tactile) [S]. VID:PID du controleur tactile : [?], non necessaire.
- Nexus 2/3 expose des widgets/overlays ; son API interne n'est pas documentee.
- THICC Q60 : **inconnu**, ne pas supposer.

## A decouvrir par REA

1. THICC Q60 (si un ecran existe) : transport, PID.
2. Nexus : comment il identifie l'ecran (EDID / nom de moniteur) pour une auto-detection fiable cote PureRGB.

Methode : `Get-PnpDevice -Class Monitor` + EDID (REA non necessaire), puis Nexus pour le Q60 : chaines `Q60`, `THICC`, `HYTE`.

## Effort / priorite

Y70 Touch : effort **faible** (fenetre moniteur + detection EDID), mais **hors module driver** ; priorite **2** (produit populaire, ecran immense a remplir de designs).
