# REA-trcc-stockage : l'écran Thermalright LY (0416:5408) peut-il garder une image sans appli ?

Analyse statique, lecture seule (aucun matériel, aucun build). Sources : `D:\Projet\Analyses\TRCC-2.1.8\`
(`cs\usblcdnew\USBLCDNEW.decompiled.cs`, `cs\trcc\**`, `extract\TRCCCAP\*`) et `REA-trcc.md`. Complète `REA-trcc.md`.

## Verdict : INCONNU (aucune preuve de stockage interne ; aucune commande de stockage trouvée dans TRCC)

- Aucune commande « enregistrer image/vidéo dans l'écran » n'existe dans le logiciel officiel pour le PID 0x5408.
  Preuve forte, mais NÉGATIVE : le logiciel officiel ne l'utilise pas ; cela ne prouve pas que le firmware ne
  l'implémente pas.
- Le firmware connaît des commandes « logo », « OTA/mise à jour » : voir §2. Leur trame est INCONNUE : ne rien envoyer.
- Le logo affiché logiciels fermés est une image DU FIRMWARE (aucun octet de logo n'est envoyé par TRCC).

## 1. Ce que TRCC envoie réellement au LY (exhaustif pour `USBLCDNEW.dll`)

`DCReadWriteAsync.ThreadSendDeviceDataLY` (`USBLCDNEW.decompiled.cs` l.800-1018) ne contient QUE :

1. Poignée de main : écriture 2048 octets (`02 FF 00 00 00 00 00 00 01 00 00 00 00 00 00 00` + 2032 zéros), lecture 512.
2. Boucle de trames image : chunks 512 octets (`01 FF` + total LE32 + longueur chunk LE16 + drapeau 1 + nb chunks LE16 +
   index LE16 + 496 octets de JPEG), bourrage multiple de 4 chunks, écritures USB de 4096 (dernière 2048), lecture ACK 512.
3. À la fermeture (`powerOFF`) : le thread sort, `ReleaseInterface(0)`, `Close()`. RIEN n'est écrit sur l'USB.
   Le marqueur `AA BB CC DD` (Form1.cs l.787, l.1920) est écrit dans une mémoire partagée Windows (IPC TRCC -> USBLCDNEW),
   pas vers l'écran.

Aucun autre `Write`/`Read`/transfert de contrôle vers 0x5408 n'existe (`grep` : seulement les écritures l.966/975, le
handshake et l'ACK). `LY1` (0x5409, l.1020-1205) est identique en structure (drapeau 2, tampon entier). Aucune commande
de luminosité, rotation, veille, logo, enregistrement.

`TRCC.exe` ne parle pas USB directement pour le LY : il écrit dans la mémoire partagée `shareMemory_ImageRGB`
(`Form1.DelegateFormCZTV` cmd 1, `WriteShareMemoryRGB`). Seul `UCDevice` utilise `UsbHid` (anciens écrans HID, pas le LY).

## 2. Constantes `SSCRM_CMD_TYPE_*` (indice sur le firmware, NON utilisé)

`USBLCDNEW.decompiled.cs` l.54-70, déclarées `private const byte`, **jamais référencées ailleurs** (grep sur tout `cs\`) :

| Nom | Valeur |
|---|---|
| SSCRM_CMD_TYPE_DEV_INFO | 1 |
| SSCRM_CMD_TYPE_PICTURE | 2 |
| SSCRM_CMD_TYPE_LOGO | 3 |
| SSCRM_CMD_TYPE_OTA | 4 |
| SSCRM_CMD_TYPE_UPG_STATE | 5 |
| SSCRM_CMD_TYPE_ROTATE | 6 |
| SSCRM_CMD_TYPE_SCR_SET | 7 |
| SSCRM_CMD_TYPE_BKL_SET | 8 |
| SSCRM_CMD_TYPE_LOGO_STATE | 9 |

Lecture prudente :
- Ces noms (LOGO, LOGO_STATE, OTA, UPG_STATE, SCR_SET, BKL_SET) montrent que la famille de firmware prévoit un
  logo configurable/consultable, une mise à jour OTA, des réglages d'écran et de rétroéclairage. C'est un INDICE de
  capacité, pas une preuve pour le PID 0x5408.
- Les valeurs ne correspondent PAS aux octets observés : handshake octet 0 = `02`, trame image octet 0 = `01`,
  réponse octet 0 = `03`. Le mapping enum -> octets de trame est INCONNU (peut être une autre couche/un autre produit).
  Ne PAS supposer « LOGO = trame commençant par 03 ».
- Format de trame LOGO / LOGO_STATE / OTA / ROTATE / SCR_SET / BKL_SET : INCONNU. Taille, endpoint, séquence, ACK, durée : INCONNUS.

## 3. « Boot animation » dans TRCC (ne concerne pas le LY)

`FormCZTV.buttonFXTB_Click` (`FormCZTV.cs` l.2056-2116), libellé « 正在调整开机动画方向 » (réglage de l'orientation de
l'animation de démarrage) : pour 1920x462 (et toutes les résolutions >= 640x172 listées) la fonction fait `return`
immédiatement. Pour les petits écrans anciens (mode 1, 565), `GifTo565` envoie un en-tête `00 01 00 1E` puis des
images GIF en flux depuis l'hôte pendant ~30 s (120 x 250 ms) : c'est du streaming temps réel, pas un stockage prouvé.
`GifToJPG` (mode 2) est une méthode VIDE (l.1956). Conclusion : aucun envoi de boot animation vers un LY.

## 4. Vidéo / thèmes / fond

Tout passe par `ImageToJpg` puis JPEG une par une, cadence max 16 img/s (voir `REA-trcc.md` §4). Aucun téléversement de
fichier vidéo, aucun mode « lecture locale », aucun diaporama ou horloge autonome (grep : aucune occurrence de
flash/standalone/offline/persist/autonome/mémoire interne côté C#). L'horloge/les métriques de TRCC sont dessinées
par l'hôte dans l'image.

## 5. USBLCD.exe (natif x86, 2,7 Mo)

Contient l'outil « Xsail Programmer Tool … support SPI Nor/Nand Flash » (classes `FlashNor`, `FlashSpiNand`,
`FlashEmmc`, messages « erase nor flash », « Dump Flash Image »), un framework `usbd` « Xsail USB PRC System » et
`\\.\F:`. C'est un programmeur de flash pour des écrans anciens de type SCSI/disque (puces Allwinner Xsail), pas le
chemin du LY : l'exécutable ne contient aucune occurrence de 0x5408 (recherche d'octets `16 04 08 54`/`08 54` : 0),
et le LY est géré par `USBLCDNEW.dll`. Non désassemblé plus avant : ne pas en déduire quoi que ce soit pour 0x5408.

## 6. Explication probable du logo (inférence, non prouvée)

Sans hôte, l'écran affiche une image de son firmware. TRCC ne la pousse pas ; elle revient quand les trames cessent
et/ou que l'interface est libérée. Si l'écran garde la dernière image un instant ou revient au logo tout de suite :
INCONNU (à observer sur matériel, sans rien écrire en flash).

## 7. Risque matériel

- Écriture flash (logo, OTA, tout stockage) : usure + risque de brick (écran inutilisable si interrompu ou si firmware
  erroné). Nombre de cycles d'écriture du composant : INCONNU (la puce n'est pas identifiée ; une NOR/NAND SPI
  courante annonce 10^4 à 10^5 cycles, valeur générique NON mesurée sur cet écran).
- Aucune de ces commandes n'ayant un format connu, tout essai reviendrait à deviner des octets : INTERDIT.

## 8. Plan d'implémentation sûr

Ne rien implémenter pour l'instant : pas de commande connue. Pour trancher :
1. Analyse passive sur matériel : capturer l'USB (Wireshark/USBPcap) pendant que TRCC officiel démarre, affiche un
   thème, est fermé normalement, puis tué. Vérifier si des transferts autres que `02 FF` / `01 FF` apparaissent
   (notamment à la fermeture) et ce que fait l'écran ensuite.
2. Chercher dans le firmware (mise à jour OTA distribuée par Thermalright, si disponible) les handlers des commandes
   3, 4, 9 pour obtenir les trames réelles.
3. Seulement si une trame de stockage est prouvée : bouton dédié avec confirmation explicite (risque flash écrit), UN
   seul envoi, relecture/état (LOGO_STATE ?) si le protocole le permet, jamais en boucle, rien au démarrage automatique.

Conséquence pratique pour Momo (anti-cheat) : sans stockage prouvé, le moyen sûr est de fermer l'appli ; l'écran
retombe sur le logo du firmware. Une appli signée/lancée à la demande reste préférable à une commande devinée.
