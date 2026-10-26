# Empire Gaming (clavier + souris 2.4G) et USB 1EA7:0064 / 1EA7:0066 — 2026-10-26

Conclusion : ces deux PID sont des **récepteurs 2,4 GHz HID génériques sans éclairage pilotable par le PC**. Classés `NO_RGB` (`rgb_catalog.rs`). Aucun pilote natif, aucune trame envoyée.

## Faits
- [V] usb.ids : `1ea7` = SHARKOON Technologies GmbH ; `0064` = « 2.4GHz Wireless rechargeable vertical mouse [More&Better] » ; `0066` = « [Mediatrack Edge Mini Keyboard] » (dongle d'un clavier-touchpad plexgear). Même couple de PID, produits sans rapport entre eux : c'est une puce OEM réutilisée par de nombreuses marques. Linux/OpenBSD affichent « 2.4G Mouse » pour 0066 (fil Arch Linux 281179 : clavier+souris+consumer control, `usbhid` suffit).
- [V] Le catalogue officiel (empiregaming.eu/en/telechargements/) ne contient ni « RF900 » ni « Reflecting ». Modèles proches : ARMOR RF358 / RF800 (packs clavier+souris sur un seul récepteur nano rangé sous la souris), MK800, RF903 (souris 2.4G, guide PDF seul).
- [V] Guide officiel ARMOR RF358 (GF.20240918) : « Le pack ne propose aucun logiciel. » Rétroéclairage réglé **aux touches** : FN+RGB MODE (Breathing), FN+RGB ON / OFF, FN+PageUp/PageDown (luminosité, effet Statique), FN+flèches (vitesse, Breathing). Appairage récepteur par ECHAP+K (clavier) et clic droit+molette (souris) : protocole radio propriétaire, interne au couple émetteur/récepteur.
- [V] Guide officiel MK800 : RGB fixe / respiration par FN, souris « RGB dynamique », 125 Hz, 4 DPI (800/1200/1600/2400), aucune mention de logiciel. Fiche produit RF800 : modes Breathing/Static du clavier, aucune mention de logiciel.
- [V] Seuls logiciels Empire Gaming publiés pour des souris : RF906, RF908, W905, X-BLADES, HELLHOUNDS, HELLFIRE (zip officiels). RF906_MOUSE.zip téléchargé et extrait sans exécution (innoextract) : application Qt5 + `hidapi`/`HidDevice.dll` (feature reports `HidD_SetFeature/GetFeature`, profils, DPI, `LedMode`, `7 COLOR BREATHING`), détection par expression `HID#VID_xxxx&PID_xxxx&col01/col04/MI_00`. **Aucune occurrence de 1EA7** (ni exe, ni DLL, ni Config.ini) : famille différente, ne concerne pas ces PID. Ghidra non lancé : rien à y chercher pour ces PID.

## INCONNU
- Modèle exact « RF900 » et clavier « Reflecting » de Momo (aucune source officielle). Hypothèse [S] : pack de la gamme ARMOR/RF8xx, même règle FN. À confirmer par Momo en lisant l'étiquette ou le guide fourni.
- Trames d'éclairage éventuelles du firmware : jamais publiées, non cherchées par envoi USB (règle : aucun envoi sur le PC de dev).
- Le guide PDF RF800/RF903 est une image sans texte (non lu) ; RF903 « Plug-and-Play » d'après les fiches revendeur [S].

## Code
`NO_RGB` : `1EA7:0064` et `1EA7:0066` (inventaire = « sans éclairage RGB », et `recognized` côté HID). Test `known_without_rgb_list_is_exact` adapté.
