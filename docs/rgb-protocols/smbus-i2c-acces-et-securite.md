# SMBus / I2C pour la RAM RGB et les cartes graphiques : accès bas niveau et règles de sécurité

Document commun aux fiches `gskill-trident-z-ene.md`, `corsair-vengeance-dram.md`, `kingston-fury-ctrl.md`,
`ram-xpg-teamgroup-patriot-crucial.md`, `gpu-acces-i2c-nvapi-adl.md`, `gpu-controleurs-par-constructeur.md`.
Document d'interopérabilité : seuls des faits de protocole (adresses, registres, séquences) sont consignés, aucun code recopié.

## Source analysée et limites (à lire d'abord)

- **Source unique : le code source d'OpenRGB** (projet communautaire de rétro-ingénierie, licence GPL-2.0-or-later), copie locale
  `D:\Projet\Analyses\rgb\openrgb-src` (git `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, commit du 2026-10-21). Les références du type
  `Controllers/X/Y.cpp` sont relatives à ce dossier.
- **Les logiciels officiels (G.Skill Trident Z Lighting Control, iCUE, Kingston FURY CTRL, ASUS Armoury Crate/Aura, MSI Center/Mystic Light,
  Gigabyte RGB Fusion/Control Center, EVGA Precision X1, Zotac FireStorm, Sapphire TriXX, etc.) N'ONT PAS été téléchargés ni extraits** dans cette
  passe. Tout ce qui est écrit ici décrit donc ce que le matériel accepte (observé par OpenRGB, souvent par capture de trafic du logiciel
  officiel), pas la façon dont chaque logiciel officiel est structuré. Les pilotes noyau réellement chargés par les logiciels officiels
  sont marqués `INCONNU (non vérifié)`.
- Les affirmations qui ne viennent pas du code source (connaissance générale des bus I2C/SMBus, JEDEC) sont étiquetées `[général]`.
- Tout champ non confirmé reste `INCONNU`.

## 1. Les bus concernés

| Bus | Où | Matériel | Remarque |
|---|---|---|---|
| SMBus chipset (PCH Intel i801, AMD FCH/PIIX4) | carte mère | RAM RGB, contrôleurs RGB carte mère, SPD des barrettes, capteurs | **bus partagé** avec SPD, capteurs de température, horloge, VRM, EC |
| IMC Intel Skylake-X (index de slot) | CPU HEDT | RAM sur plateformes serveur/HEDT | module PawnIO dédié |
| SMBus du Super I/O Nuvoton NCT6793 | carte mère | quelques cartes mères | module PawnIO dédié |
| I2C GPU NVIDIA via NVAPI (port 1) | carte graphique | LED RGB de la carte | voir `gpu-acces-i2c-nvapi-adl.md` |
| I2C GPU AMD via ADL (ligne 1) | carte graphique | LED RGB de la carte | idem |

Ce qui est OpenRGB-spécifique et utile à imiter : chaque bus expose `pci_vendor / pci_device / pci_subsystem_vendor / pci_subsystem_device`
(ident PCI du contrôleur SMBus ou du GPU) et un `port_id`. La détection RAM n'est autorisée que sur les contrôleurs SMBus de la liste blanche.

### Contrôleurs SMBus autorisés pour la RAM (`pci_ids/pci_ids.h`, macro `IF_DRAM_SMBUS`)

| Vendeur | Device ID | Plateforme |
|---|---|---|
| AMD 0x1022 | 0x790B | FCH / PIIX4 (Zen) |
| Intel 0x8086 | 0x3A30 | ICH10 |
| Intel | 0xA123 | Sunrise Point-H |
| Intel | 0x2085 | Skylake-X PCU |
| Intel | 0xA2A3 | Kaby Lake-H |
| Intel | 0xA323 | Cannon Lake-H |
| Intel | 0x06A3 | Comet Lake-H |
| Intel | 0xA3A3 | Comet Lake-V |
| Intel | 0x43A3 | Tiger Lake |
| Intel | 0x7AA3 | Alder Lake |
| Intel | 0x7A23 | Raptor Lake |
| Intel | 0x7F23 | Arrow Lake |

La macro `IF_MOBO_SMBUS` ajoute les bus sans identité (vendor 0, ou 0xFFFF/0xFFFF) pour les contrôleurs de carte mère ; un contrôleur ENE de carte mère
n'est testé que si le sous-système PCI est ASUS (0x1043), 0 ou 0xFFFF (`ENESMBusControllerDetect.cpp`).

## 2. Pilotes requis sous Windows

- **PawnIO** (pilote noyau signé tiers + modules `.bin`) : c'est ce qu'OpenRGB 1.x utilise pour le SMBus (`i2c_smbus/Windows/i2c_smbus_pawnio.cpp`).
  Modules chargés : `SmbusI801.bin` (Intel), `SmbusPIIX4.bin` (AMD, ports 0 et 1 sélectionnés par l'ioctl `ioctl_piix4_port_sel`),
  `SmbusNCT6793.bin`, `SmbusIntelSkylakeIMC.bin` (slot choisi par `ioctl_smbus_index`), `LpcIO.bin` (Super I/O).
  API de la bibliothèque : `pawnio_version`, `pawnio_open`, `pawnio_load` (charge le blob), `pawnio_execute(handle, "ioctl_smbus_xfer", ...)`.
  Transfert : entrée 9 mots 64 bits `[adresse 7 bits, lecture/écriture, commande, taille, données SMBus...]`, sortie 5 mots.
  Ioctls nommés : `ioctl_identity` (renvoie 3 mots : nom du contrôleur sur 8 octets + IDs PCI vendeur/device/sous-système), `ioctl_smbus_xfer`,
  `ioctl_smbus_index`, `ioctl_piix4_port_sel`, `ioctl_set_sleep_mode`.
  Mode de veille du driver : 0 = occupé en permanence, 1 = occupé court, 2 = dort toujours (défaut OpenRGB) (`PAWNIO_SLEEPMODE_*`).
- **Anciennes voies** (non utilisées par OpenRGB récent) : WinRing0 (pilote noyau signé 2009, bloqué par Windows Defender / liste de pilotes vulnérables,
  à ne pas utiliser), pilotes constructeur (ASUS AsIO3, Corsair, etc.) `INCONNU (non vérifié)` pour les logiciels officiels.
- Accès administrateur obligatoire pour ouvrir PawnIO.
- **Mutex de coexistence** : `Global\Access_SMBUS.HTP.Method` pris autour de chaque transfert (et pour le changement de port PIIX4). Les autres logiciels qui
  touchent le même SMBus (monitoring, RGB officiels) doivent le prendre aussi, sinon deux transactions s'entrelacent sur le contrôleur.
  Ce nom est une convention de coexistence ; son usage par d'autres logiciels n'est pas vérifié ici.
- Un transfert SMBus Windows peut échouer ou se bloquer si ACPI/EC accède au bus en même temps : prévoir réessais et délai (voir fiches).

## 3. Anatomie d'une barrette (pourquoi c'est dangereux) `[général]` sauf mention

| Adresse 7 bits | Composant sur une barrette | Écrire dedans = |
|---|---|---|
| 0x50 à 0x57 | EEPROM SPD (DDR4) ou **hub SPD5118** (DDR5), un par slot | **corrompre le SPD** : profil JEDEC/XMP, numéro de série, la barrette peut ne plus démarrer |
| 0x36 / 0x37 | commandes de sélection de page SPD DDR4 (SPA0/SPA1) | changer la page vue par tous les lecteurs de SPD (volatil) |
| 0x18 à 0x1F | capteur de température DDR4 (TS3510 type), un par slot | modifier sa configuration |
| 0x30 à 0x37 | capteur de température DDR5 (via le hub) | idem |
| 0x48 à 0x4F | **PMIC DDR5** (alimentation de la barrette) | **changer les tensions de la barrette** (risque matériel) |
| autres (0x58-0x5F, 0x60-0x67, 0x70-0x77, 0x27, 0x39-0x3D...) | libres en JEDEC, utilisés par les contrôleurs RGB des constructeurs, mais aussi par d'autres puces de la carte mère | selon la puce |

Faits tirés de la source : lecture SPD DDR4 = `SPD_DDR4_EEPROM_LENGTH` 512 octets en 2 pages de 256, page choisie par une écriture sur 0x36+page ;
DDR5 = 2048 octets en pages de 128, registre de page virtuelle `0x0B` du hub (valeur lue `0x51` à l'offset 0 et `(octet1 & 0xEF) == 0x08` pour reconnaître un hub DDR5),
les lectures DDR5 utilisent l'offset `0x80 | (adresse & 0x7F)` (`SPDAccessor/DDR5DirectAccessor.cpp`).
Reconnaissance DDR4 : octet 0 de l'EEPROM vaut `0x23` après sélection de la page basse (`DDR4DirectAccessor.cpp`). Délai entre accès : 1 ms (`SPD_IO_DELAY`).

## 4. Séquence de détection sans risque (méthode OpenRGB, à reprendre)

1. **Bus** : ne considérer que les contrôleurs SMBus de la liste blanche du §1 (IDs PCI). Jamais « tous les bus ».
2. **Slots occupés** : pour chaque adresse 0x50..0x57, lire (jamais écrire) l'octet 0 du SPD pour savoir si une barrette DDR4 (`0x23`) ou DDR5 (hub `0x51`) est présente.
   Les anciens types (< DDR4) sont ignorés.
3. **Constructeur** : lire dans le SPD l'ID JEDEC du fabricant du module : DDR4 aux octets 0x140-0x141, DDR5 aux 0x200-0x201 ;
   formule : `(octet_haut << 8) + (octet_bas & 0x7F) - 1` (`SPDAccessor.cpp::jedec_id`). IDs utilisés par OpenRGB (`SPDAccessor/SPDCommon.h`) :
   Kingston 0x0117 / 0x300F / 0x3011, Corsair 0x021D, ADATA/XPG 0x044A, G.Skill 0x044C, TeamGroup 0x046E, Mushkin 0x8313, Gigabyte 0x8971,
   Thermaltake 0x8A41, Patriot 0x8501 (et 0xFF7E pour le Viper Steel). Le N° de pièce est à 0x149-0x15C (DDR4) / 0x209-0x226 (DDR5) ;
   les octets « fabricant » libres à 0x161.. (DDR4) / 0x22B.. (DDR5) servent à reconnaître certaines gammes (Kingston, Patriot).
4. **Ne tenter un contrôleur RGB que si le JEDEC du constructeur correspond ET que le type mémoire (DDR4/DDR5) correspond.** (Exception : Corsair DRAM moderne, ENE et Crucial sont
   détectés par adresse, pas par JEDEC : ce sont les cas les plus risqués, voir §5.)
5. **Sonder avec des lectures**, avec une signature vérifiable (voir fiches : motif `A0..AF`, chaîne « FURY », octets 0x43/0x44, magic 0x1589...). Une réponse « ack » seule n'est jamais une preuve.
6. Pause de 1 à 10 ms entre sondes (les contrôleurs RGB sont lents) et mutex SMBus global.
7. Mémoriser l'adresse trouvée ; ne plus sonder ensuite.

## 5. Ce qu'il ne faut JAMAIS écrire (règles pour PureRGB)

1. **Aucune écriture sur 0x50..0x57** (EEPROM SPD / hub) hors du mécanisme de page, ni sur 0x36/0x37 sauf sélection de page SPD, et **remettre la page 0** (`write 0x36`) après lecture de la page 1 `[général, recommandé]` pour ne pas perturber CPU-Z/HWiNFO.
2. **Aucune écriture sur 0x48..0x4F pour une barrette DDR5** (PMIC). Attention : `0x4F` figure dans la liste d'adresses de remappage ENE (`ene_ram_addresses`) ; OpenRGB ne l'utilise que si une lecture sur cette adresse **échoue** (adresse libre). À conserver, et sur DDR5 préférer exclure 0x48..0x4F de la liste.
3. **Pas d'écriture « sonde » sur une adresse non identifiée** : c'est le piège principal. Cas réels dans la source :
   - ENE / Crucial : le remappage écrit les registres `0x80F8/0x80F9` (ENE) ou `0x82EE/0x82EF/0x82F0` (Crucial) sur **0x77** / **0x27** dès que l'adresse répond à une lecture ;
     seul TForce Xtreem refait un test complet de signature avant d'écrire (commentaire « Full test to avoid conflicts with other ENE DRAMs »). PureRGB doit vérifier la signature (§ fiche ENE) AVANT toute écriture de remappage.
   - Patriot Viper, Corsair Vengeance V1, Gigabyte RGB Fusion2 DRAM, TForce utilisent `write_quick` (écriture de 0 octet) comme sonde de présence ; Gigabyte DRAM écrit en plus `0xE1 = 0x01` à l'adresse 0x67 après l'ack. `write_quick` n'est pas innocent sur toutes les puces : préférer une lecture.
   - Kingston FURY : la sonde écrit `0x53` dans le registre `0x08` (début de transaction) à `base + slot` ; protégé par le test JEDEC + variante SPD, c'est acceptable seulement dans ce cadre.
4. **Pas de sauvegarde en mémoire flash à chaque changement** : ENE `0x80A0 = 0xAA`, Gigabyte GPU `0xAA`, EVGA `0x90`, MSI GPU `0x3F` écrivent en flash/EEPROM du contrôleur LED : usure. Appliquer en mode direct/volatil par défaut, n'enregistrer que sur demande explicite de l'utilisateur.
5. **Pas d'écriture sur le bus du GPU qui n'est pas le bus RGB** : NVAPI port 1 (RGB) ; les ports DDC/écran (EDID 0x50, DDC 0x37) sont d'autres bus, les LCD des cartes AORUS (réponse `00 00 00 00`) ne sont pas des contrôleurs RGB.
6. **Pas de détection hors liste blanche PCI sur GPU** (vendeur + device + sous-vendeur + sous-device), voir `gpu-acces-i2c-nvapi-adl.md`.
7. Pas d'écriture pendant la veille / reprise : certains contrôleurs perdent leur état (EVGA Ampere ré-envoie son paquet d'initialisation avant chaque commande).
8. Remappage d'adresse (ENE, Crucial, Xtreem) : volatil dans le contrôleur LED (non confirmé par la source) ; ne jamais le répéter si les adresses cibles répondent déjà, ne jamais le faire sans avoir compté les barrettes SPD présentes.
9. Après une erreur d'I/O : attendre (backoff 3 x tentative x 10 ms chez Kingston), ne pas insister en boucle serrée.
10. Journaliser chaque écriture SMBus en mode diagnostic (adresse, registre, valeur) pour pouvoir expliquer un incident.

## 6. Estimation globale

Difficulté : **élevée**, non pas par le protocole (simple : registres à octets) mais par le risque et la coexistence (pilote noyau PawnIO, mutex partagé, détection fiable).
Tests d'octets sans matériel : voir chaque fiche (calculs de CRC8 Corsair, paquets Zotac/EVGA/Gigabyte, formule JEDEC, permutation R/B/G de l'ENE).
