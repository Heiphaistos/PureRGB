# Journal des modifications

## 0.51.1

- **Inventaire** : à partir du rapport reçu de la tour, les concentrateurs USB et 15 alimentations Corsair (dont l'AX1600i, `1B1C:1C11`, identifiée dans iCUE) sont classés « sans éclairage RGB » au lieu de « hors catalogue ». 20 contrôleurs Corsair ajoutés au catalogue.
- **Boîtier Cooler Master** : le `2516:01C1` est l'écran LCD Iris du HAF 700 EVO (ADB/TCP, pas un contrôleur LED) ; il est nommé et jamais adopté par le pilote natif. Noms de six autres identifiants Cooler Master corrigés d'après MasterPlus+.
- **Documentation** : analyses MasterPlus+, iCUE, G HUB complétées. Casque Logitech PRO X 2 et souris Sharkoon : toujours INCONNU, aucun pilote ajouté.

## 0.51.0

- **Surveillance** : les cartes (Processeur, Carte graphique, Carte mère, Disques, Ventilateurs, Puissance…) se déplacent par glisser-déposer (ou flèches du clavier sur la poignée), l'ordre est mémorisé, et elles se répartissent en maçonnerie sur 1 à 4 colonnes sans trou. Même chose pour les catégories de l'onglet Ventilateurs (disposition seulement, aucune règle d'écriture modifiée).
- **Mode jeu** : base d'anti-cheats étendue de 8 à 19 familles (~75 exécutables, source citée pour chacun), panneau repliable avec recherche. Limite : détection par nom de processus (les pilotes noyau sans processus ne sont pas interrogés).
- **Photos des produits** : nouvelle correspondance tolérante (carte MSI Z790 GAMING PRO WIFI, clavier Cougar 700K EVO, casque Logitech, manettes Xbox/Sony/Nintendo/8BitDo, ~24 manettes) ; les appareils connus seulement du fabricant affichent le nom produit HID ; photo des écrans en grand.
- **Manettes** : photo et nom propre de chaque manette détectée.
- **Diagnostic** : « Capturer le protocole USB » disponible aussi pour les appareils reconnus (écran Thermalright) ; l'analyse « Inventaire RGB et ventilateurs » est envoyée à la télémétrie (si l'envoi de diagnostic est coché) avec un bouton « Envoyer l'analyse ».
- Signature Authenticode préparée dans la CI (inactive sans certificat dans les secrets du dépôt).

### Mode arrière-plan

- **Mode arrière-plan** : au « Quitter » du menu de la zone de notification, PureRGB peut laisser tourner, sans fenêtre ni WebView2, seulement ce que vous cochez (Réglages > Arrière-plan, ou page Écrans) : **écrans** (design/thème/vidéo), **éclairage appliqué**, **ventilateurs** (uniquement les courbes/réglages manuels déjà actifs, jamais de réglage automatique), **mode manette**. Ne sont lancés que les modules cochés ET déjà en service ; « Quitter complètement » arrête tout. C'est le même `purergb.exe` (`--background=écrans,…`) : aucun service Windows, aucune élévation, aucun pilote ajouté, aucune injection.
- Le mode jeu s'applique dans ce processus comme dans l'application complète (même code). Ce n'est pas un moyen de contourner un anti-cheat : le bénéfice est zéro fenêtre et peu de mémoire.
- Rouvrir PureRGB (ou « Ouvrir PureRGB » dans le menu du processus d'arrière-plan) arrête proprement ce processus, rend les ventilateurs au BIOS comme à la fermeture normale, puis reprend la main. Les logiciels constructeur arrêtés au lancement ne sont relancés qu'au « Quitter complètement ».
- « Lancer au démarrage de Windows » est inchangé : l'application complète démarre minimisée, aucune seconde entrée n'est créée.

## 0.50.2

- Correction de la mise à jour automatique : l'application se fermait sans jamais se relancer. Cause : le « job » de fermeture propre tuait l'installeur et la relance au moment où PureRGB quittait. Il est maintenant relâché juste avant (après restauration du matériel et arrêt des moteurs).
- Version portable : la mise à jour remplace maintenant le fichier `.exe` lui-même (téléchargé depuis la release, signature vérifiée avec la clé de l'application avant tout remplacement), puis relance. Avant, c'était l'installeur qui était lancé, inadapté à la version portable.
- La release publie `PureRGB-portable.exe.sig`.

## 0.50.1

- Même contenu que 0.50.0 (la 0.50.0 n'a pas été publiée : un test de temporisation échouait sur la machine de build, rendu déterministe).

## 0.50.0

- Interface refaite en pleine largeur (2 à 4 colonnes selon la fenêtre) : **Surveillance** (bandeau de synthèse + cartes par composant avec jauges et mini-courbes), **Ventilateurs** (catégories Carte mère/BIOS, AIO, ventirad, hubs, boîtier ; éditeur de courbe à côté de la liste ; aucune règle de sécurité modifiée), **Manettes** (testeur et remappage en cartes), **Réglages** (catégories claires, Mode jeu et anti-cheat réécrit : état, ce qui est suspendu, anti-cheats reconnus), liste du test de compatibilité groupée par catégorie avec les appareils non pilotables repliés.
- **Diagnostic matériel** pousse : nouveau panneau en cartes avec pastille d'état, système/BIOS, pilotes natifs, SMBus, ViGEmBus/HidHide, ventilateurs (tr/min, commande, pilotable), logiciels concurrents, mode jeu, dernières erreurs ; boutons « Copier le rapport » et « Enregistrer le rapport » (lecture seule, aucune donnée de plus envoyée).
- Corrections : faux avertissements « reconnu mais non détecté » pour un appareil déjà piloté par OpenRGB (clavier Cougar, carte MSI) ; journal du rapport limité aux dernières 24 h.

## 0.49.0

- Pilotes **embarqués** dans l'application : installeurs officiels ViGEmBus 1.22.0 et HidHide 1.5.230 (signés Nefarius), fonctionnent hors ligne. Chaque installeur est copié dans un dossier protégé, son SHA-256 épinglé est vérifié, puis il est lancé : rien ne s'installe sans confirmation, jamais pendant le mode jeu. Bouton « Installer HidHide » dans Manettes ; PureRGB ne configure pas HidHide (vous choisissez les périphériques à masquer). Pas de pilote Flydigi : l'application officielle n'en utilise pas et son protocole n'est pas documenté.

## 0.48.0

- Nouvel onglet **Surveillance** : tous les capteurs de la machine (processeur, carte graphique, carte mère, disques, tensions, fréquences, puissance, ventilateurs) regroupés par composant, avec filtre, mini-courbes, min/max et alerte de température, ainsi que la vitesse des hubs et AIO pilotés par PureRGB.
- Températures de la RAM (DDR4 TSE2004, DDR5 SPD5118) en lecture seule via SMBus : uniquement avec le réglage des pilotes natifs, refus si un logiciel constructeur tient le bus, aucune écriture sur le bus. Jamais essayé sur du vrai matériel.
- Nouvel onglet **Manettes** : testeur en direct (boutons, gâchettes, sticks, zone morte, dérive, vibration) et **remappage** par manette Xbox 360 virtuelle (touches, gâchettes, zones mortes radiale/axiale, courbes, anti-dérive, profils). Désactivé par défaut, suspendu par le mode jeu pour les jeux à anti-cheat, aucune installation de pilote sans confirmation (ViGEmBus 1.22.0, téléchargement épinglé SHA-256). Les jeux voient la manette physique ET la virtuelle sans HidHide (non installé par PureRGB). Nouvelle dépendance : vigem-client 0.1.4.
- Ventilateurs : préréglages de courbe (Silencieux, Équilibré, Performance) et « Enregistrer sur tous les canaux ».
- Flydigi : analyse du pilote filtre et des dongles (docs), détection des dongles 2,4 GHz ; pilote natif refusé faute de transport sûr et de preuve que l'éclairage est volatil.
- Catalogue de photos : +75 références (Zotac, EVGA, Founders Edition, Sapphire, XFX, Inno3D, Gainward, ASRock, Jonsbo, Montech, Zalman, Govee, NZXT Kraken, Flydigi...), 1 295 au total.

## 0.47.0

- Catalogue de photos produit : 969 -> 1 220 références (MSI cartes mères, cartes graphiques et AIO ; Gigabyte Aorus ; Noctua ; be quiet! ; Phanteks ; HYTE ; Sharkoon ; Roccat ; Wooting ; Ducky ; Turtle Beach ; manettes Sony, Xbox, 8BitDo, Flydigi). Images téléchargées et regardées, puis téléchargement réel complet par PureRGB (3 lenteurs ponctuelles sur 1 221, 1 image défaillante retirée).
- Nettoyage : les fonctions d'écrans prêtes et testées mais non branchées (rotation, firmware, bascule d'affichage) sont marquées explicitement, 12 avertissements de code mort en moins.

## 0.46.0

- Catalogue de photos produit : 330 -> 969 références (périphériques Corsair, Razer, Logitech G, SteelSeries, HyperX, Glorious, Keychron ; cartes graphiques ASUS, Gigabyte, PowerColor, ASRock, Palit, PNY ; cartes mères ASUS, MSI, NZXT ; RAM Corsair et TeamGroup ; boîtiers, AIO et ventirads ; ventilateurs, hubs et bandeaux). Chaque image a été téléchargée et regardée, le catalogue passe le schéma, l'unicité des motifs et un téléchargement réel complet par le code de PureRGB (curl, liste blanche d'hôtes, décodage). Les entrées sans image vérifiable ont été écartées plutôt que devinées.
- Correction : motif `dr12` trop court (Darkflash DR12) rendait le catalogue inutilisable pour le validateur.

## 0.45.0

- Nouveau pilote natif NZXT (HUE 2, Smart Device V2, « Motherboard Device », RGB & Fan Controller 3 et 6 canaux, B650E, RGB & Fan Controller 2024 : PID 2001, 2002, 2005, 2006, 200B, 200D, 200F, 2009, 200E, 2010, 2011, 2019, 201F, 2020, 201B, 2022), expérimental, jamais validé sur du vrai matériel, désactivé par défaut. Il remplace l'ancien pilote HUE 2 : lecture des accessoires branchés (`20 03`/`21 03`) pour connaître le nombre de LED de chaque canal (canal vide ou accessoire inconnu : ignoré, rien n'est écrit), couleur directe par LED (ordre G,R,B, 40 LED par canal au plus), cadence limitée à 50 images par seconde. Ventilateurs : consigne bornée à 20-100 % ; à la remise en état d'origine, les ventilateurs réglés par PureRGB retrouvent le duty lu sur l'appareil avant le premier réglage (statut `67 02`), ou ne reçoivent rien si ce duty n'a pas pu être lu. Aucun effet matériel, aucun firmware ; les Kraken, les « 2023 RGB Controller » (2012, 2021) et les PID 200C, 2016, 2023, 2029, 3016 ne sont pas pris en charge. Voir docs/rgb-protocols/nzxt.md.
- Ventilateurs, règle stricte « ne rien toucher sans réglage manuel » (suite à des ventilateurs bruyants « à fond » signalés) :
  - liquidctl : `initialize all` n'est plus jamais lancé au scan. Les pilotes liquidctl dont `initialize` ou chaque consigne réécrit tous les canaux (non réglés = 100 %, mode de pompe forcé, profils remis à zéro : Hydro Platinum / Pro XT / Pro, H110i GT, Kraken X2 / M22, Asetek 690LC legacy, MSI CoreLiquid, Lian Li Uni) ne sont plus initialisés et passent en lecture seule (réglage manuel refusé, note explicite) ; les autres sont initialisés un par un. Le `status` de l'Asetek 690LC legacy, qui réécrit pompe et ventilateur, n'est plus appelé.
  - Pilote NZXT : plus de « 100 % » à la fermeture ; le duty d'origine est lu à la première consigne et restauré, sinon rien n'est écrit.
  - Carte mère (sensord/LibreHardwareMonitor) : seuls les canaux réglés par PureRGB sont rendus au BIOS (fermeture, mode jeu) ; une courbe supprimée ou désactivée rend aussitôt son canal au lieu de le laisser figé à la dernière consigne.
  - Hubs natifs : à la fermeture, Lian Li Uni (synchro carte mère) et NZXT (duty d'origine) rendent leurs ventilateurs, même après un re-scan ; Thermaltake et Corsair Commander Pro n'ont pas d'état lisible : ils gardent le dernier réglage et la note de l'appareil le dit.
  - Tests : ouverture/scan/couleurs/fermeture sans réglage = zéro écriture de vitesse (NZXT, Lian Li, Thermaltake, Commander Pro, carte mère), restauration de la valeur capturée, aucune valeur à 100 % générée par le logiciel.
- Pilote natif iCUE LINK System Hub (Corsair 0C3F), expérimental : énumération de la chaîne en lecture seule puis couleurs directes. Une zone « Chaîne iCUE LINK » redimensionnable (1 à 476 LED, dans l'ordre des canaux) car la chaîne n'est connue qu'après ouverture. Appareil inconnu ou nombre de LED contradictoire entre sources (ORIGIN OA, XG3 Hybrid) : ouverture refusée sans rien écrire. Aucune consigne de pompe/ventilateur, aucune écriture des ressources 1D/1E ni changement de mode matériel `01 03`. Pas de keep-alive autonome ; réduction de luminosité d'iCUE non appliquée. Non testé sur matériel.
- Pilote natif casque SteelSeries Arctis 5 (1250, 12AA), source unique (OpenRGB) : une couleur par oreille (6 écritures de 37 octets par zone). Zone inchangée non réécrite, changements à moins de 1 s refusés (persistance de l'application INCONNUE). Nova 3 non implémenté (sauvegarde flash possible). Non testé sur matériel.

## 0.44.0

- Mode jeu (activé par défaut, réglable dans Réglages > Général) : quand un jeu protégé par un anti-cheat est détecté (liste versionnée `src-tauri/data/gamemode_anticheat.json` : EasyAntiCheat, BattlEye, Vanguard, FACEIT, EA Javelin, GameGuard, Call of Duty, plus vos propres exécutables), PureRGB arrête l'OpenRGB qu'il a lancé, sensord (capteurs, ventilateurs carte mère rendus au BIOS), l'accès SMBus/RAM (poignée PawnIO fermée), l'envoi aux mini-écrans USB et les pilotes HID natifs (option « garder l'éclairage USB »). Bandeau et notification en français ; reprise automatique 10 s après la fermeture du jeu. Le pilote PawnIO n'est jamais déchargé ni désinstallé.
- Réglages : texte d'honnêteté (le mode jeu réduit le risque sans garantir l'absence de bannissement, seule la fermeture de PureRGB le supprime) et bouton « Quitter PureRGB proprement ».
- Catalogue communautaire : bouton « Signaler / confirmer » sur chaque périphérique USB du diagnostic et dans le panneau d'un appareil. L'utilisateur choisit « Ça marche », « Marche en partie » ou « Ça ne marche pas » et peut ajouter une note de 280 caractères au plus. Un aperçu affiche le contenu EXACT envoyé (VID:PID, noms USB, famille de pilote, version de l'app, résultat, note) et l'envoi exige une case de consentement cochée à chaque fois. Ni nom d'utilisateur, ni chemin, ni numéro de série, ni IP, ni capture USB.
- Nouvel écran « Retours de la communauté » (Réglages, diagnostic) : comptes anonymes par appareil (« confirmé fonctionnel par N utilisateurs »), en lecture seule, mis en cache pour fonctionner hors-ligne. Les réponses aberrantes du serveur sont ignorées.
- Serveur de télémétrie : routes POST/GET `/device-feedback` (schéma strict, limitation de débit par IP hachée, un vote par IP hachée + appareil + version, table `device_feedback`).

## 0.43.0

- OpenRGB : un OpenRGB.exe résiduel de NOTRE installation (sous %ProgramFiles%\PureRGB) qui tient le port du serveur sans répondre au protocole est maintenant terminé puis relancé. Un OpenRGB de l'utilisateur ou tout autre programme n'est jamais touché. Le diagnostic nomme le processus qui tient le port et son ancienneté.
- Carte mère MSI (Mystic Light) : le nombre de LEDs de chaque connecteur ARGB (JPIPE 1/2, JRAINBOW 1/2, JCORSAIR) est réglable (1 à 75), enregistré dans les réglages et réappliqué au démarrage. Limite du protocole par zone : une seule couleur par connecteur (la 1re LED), appliquée à tout le connecteur, hub inclus ; pas de dégradé spatial. Les zones des autres drivers natifs redimensionnables (ASUS, Gigabyte, ASRock, Thermaltake) deviennent réglables aussi.
- Écrans USB : l'erreur « Accès refusé » indique maintenant les pilotes/services Windows liés au périphérique (WinUSB, usbccgp, HidUsb, pilote tiers) et ce qu'il faut en faire ; le texte part aussi dans la télémétrie `screens.scan_errors`.
- Diagnostic : les casques Logitech PRO X (046d:0aba, 046d:0af7), sans éclairage, ne sont plus signalés « hors catalogue, signalez-le » ni « non reconnu ».

- Manettes : toutes les manettes vues en HID (dont celles en XInput) sont listées avec le type « Manette », même sans RGB, avec la raison (Xbox, Nintendo, autres). La Flydigi Vader 4 (qui emprunte l'ID Xbox 360 045E:028E) est reconnue par sa chaîne produit, sans confondre les vraies manettes Xbox ; son éclairage n'est pas pris en charge pour l'instant (aucun protocole public fiable, voir docs/rgb-protocols/flydigi.md).
- Nouveaux pilotes intégrés (expérimentaux, jamais validés sur du vrai matériel, désactivés par défaut) : Sony DualSense / DualSense Edge (barre lumineuse + 5 LED de joueur) et DualShock 4 (barre lumineuse), USB et Bluetooth (CRC32), couleur directe sans réglage persistant, d'après le driver Linux hid-playstation (voir docs/rgb-protocols/manettes.md). Les palettes et effets existants s'appliquent.

## 0.42.0

- Éditeur d'écran : nouveau panneau « Couleurs du design » (aucun widget sélectionné). Changer une couleur la change partout dans le design (rectangles, jauges, barres, texte, fond), ce qui permet de recolorer les thèmes intégrés (par exemple les rectangles roses).

## 0.41.0

- Nouveaux pilotes intégrés (expérimentaux, jamais validés sur du vrai matériel, désactivés par défaut), d'après les notes d'analyse : souris Logitech filaires G203, G303, G403, G Pro / G Pro Hero (mode direct sans écriture en mémoire flash) et G203 Lightsync, une seule zone pilotée ; SteelSeries Rival 600 (six LED latérales).
- Refusés faute de trames établies dans les notes (aucun protocole deviné) : Rival 100/300, Rival 600 molette et logo, casques Arctis / Siberia, casques et souris Logitech sans fil.

## 0.40.2

- Correctif de la version 0.40.1 dont la construction automatique a échoué : un test de chronométrage de l'envoi vers un écran lent était trop serré pour les machines chargées (marge élargie, la détection d'un retard cumulé reste valable). Aucun changement pour les utilisateurs par rapport à 0.40.1.

## 0.40.1

- Télémétrie (si activée) : le rapport liste maintenant les logiciels constructeur en cours d'exécution qui peuvent verrouiller un écran ou un appareil (TRCC, L-Connect, iCUE…), pour trouver ce qui bloque l'écran Thermalright.
- Nettoyage interne : avertissements du compilateur supprimés, identifiant du pilote intégré repris dans les notes d'appareil.

## 0.40.0

- Cadre des pilotes intégrés : un modèle peut demander une seconde collection HID du même appareil (`Model::aux`, `Hid::aux`), pour les protocoles qui séparent contrôle et LEDs.
- Nouveaux pilotes intégrés (expérimentaux, jamais validés sur du vrai matériel, désactivés par défaut), d'après les notes issues de l'analyse des logiciels constructeur : Cooler Master ARGB Gen 2 A1 (PID 0173, 01C9, 01CB : 3 canaux de 60 LEDs, mode temps réel) et ARGB Gen 1 en mode direct par LED ; Lian Li Uni Hub SL / AL / SL-Infinity / SL V2 / AL V2 / Redragon (couleurs R,B,G, LEDs par ventilateur corrigées, vitesse en pourcentage avec plancher à 10 %) ; claviers Roccat Vulcan 100 / 120 Aimo et Magma / Magma Mini (deux interfaces HID, refus propre si la seconde est absente).
- Qualité : `lib.rs` (1557 lignes) découpé en six modules de commandes (`src-tauri/src/commands/`), sans changement de comportement ; README mis à jour (liste unifiée, test de sécurité, pilotes intégrés).
- Notes d'analyse ajoutées dans `docs/rgb-protocols/` : Cooler Master MasterPlus+, NZXT CAM, Lian Li L-Connect 3.

## 0.39.0

- Appareil vu par deux pilotes : le pilote qui offre le plus de LEDs pilotables devient le principal (à égalité, le pilote intégré), l'autre reste en repli automatique avec des couleurs ré-échantillonnées. Corrige une perte de fonctions constatée sur une carte MSI Z790 : le pilote intégré n'offre qu'1 LED par connecteur ARGB alors qu'OpenRGB en pilote 60. Une carte affiche « Pilote intégré » ou « Repli OpenRGB ». La ligne « mémoire détectée en lecture seule » disparaît quand OpenRGB pilote cette mémoire.
- Détection du serveur OpenRGB plus tolérante (délai 1,5 s au lieu de 0,4 s) : un serveur chargé n'est plus déclaré injoignable.
- Pilotes intégrés (expérimentaux, jamais testés sur du vrai matériel, désactivés par défaut), d'après les notes issues de l'analyse des logiciels constructeur : Razer 82 modèles (claviers BlackWidow V3/V4, Huntsman, Ornata, DeathStalker, Cynosa ; tapis Firefly/Goliathus/Strider ; Base Station, Core X, kits d'éclairage), Logitech G815 / G915 / G915 TKL (couleur par touche) et G512 / G513 / G610, SteelSeries Apex Pro / Pro TKL / 7 / 7 TKL / 5 / 9 TKL / 9 Mini avec la table de 114 touches confirmée, Rival 3 / 5, Aerox 3 / 5 / 9 filaires, tapis QCK, Roccat Kone Pro / Burst Pro / Burst Pro Air.
- Notes d'analyse ajoutées dans `docs/rgb-protocols/` (Corsair iCUE, Logitech G HUB, Razer Synapse, SteelSeries GG, Roccat Swarm) : les logiciels qui ne téléchargent leur contenu qu'à l'installation n'ont pas pu être lus ; les faits viennent alors de projets tiers, signalés comme tels. Roccat Vulcan / Pyro / Magma attendent un cadre natif à deux interfaces HID.

## 0.38.0

- Une seule liste d'appareils, un seul pilotage : un appareil vu à la fois par un pilote intégré et par OpenRGB n'apparaît qu'une fois. Le pilote intégré travaille en premier ; après 3 échecs de suite, OpenRGB prend le relais automatiquement (note « repli automatique sur OpenRGB » sur la carte) et le pilote intégré n'écrit plus jamais en même temps, donc aucun conflit. Un nouveau scan redonne sa chance au pilote intégré.
- Réglages réorganisés en 5 sections (Général, Appareils et pilotes, Photos et vidéo, Test de sécurité, Diagnostic). Les anciennes cartes « Serveur OpenRGB » et « Drivers natifs » forment une seule carte « Pilotage des appareils » (le serveur OpenRGB passe sous « Avancé »).
- Test de compatibilité et de sécurité (Réglages) : revue en lecture seule de chaque composant (pilote, LEDs, logiciels concurrents, accès au bus), puis essai d'écriture à la demande, appareil par appareil, après confirmation : une couleur très faible et une extinction (2 écritures), puis remise de l'état d'origine. La mémoire (risque élevé) reste bloquée tant que l'écriture RAM n'est pas autorisée ; une écriture lente (bus saturé) est signalée.
- Nouveaux pilotes intégrés (expérimentaux, jamais testés sur du vrai matériel, désactivés par défaut) : Corsair Commander Core / ST / Core XT (RGB), Cooler Master ARGB Controller (2516:1011, 4 ports), Logitech G410 / G810 / G910 (couleur par touche), SteelSeries Apex Pro TKL 2023 et Aerox 3 (5 modèles), Roccat Kone Pro, et 24 souris Razer de plus (31 modèles Razer au total). Tout ce que les notes de protocole ne permettent pas d'écrire sans deviner est refusé : ventilateurs Corsair, iCUE LINK, Hydro, claviers Razer, G815/G915, souris Logitech, Apex 3/5/7, Rival.

## 0.37.0

- Sécurité du matériel : l'écriture sur la mémoire RAM (bus SMBus) devient un réglage à part, désactivé par défaut (« Autoriser l'écriture sur la mémoire RAM », sous Drivers natifs). Sans lui, « Drivers natifs » ne fait que DÉTECTER les barrettes en lecture seule (aucune écriture, pas même les sélecteurs d'identité) et les affiche « détectée, non pilotable ». Testé : zéro écriture sur le bus, et refus de toute couleur.

## 0.36.0

- Photos de produits : le catalogue passe de 133 à 330 produits (95 cartes mères MSI/Gigabyte/ASUS, mémoires Corsair/G.Skill/Kingston/TeamGroup, ventilateurs, hubs et contrôleurs, boîtiers, cartes graphiques). Photos prises sur le site du constructeur, redimensionnées et vérifiées ; chaque produit renvoie vers sa page. Non couverts : cartes ASRock (site bloqué aux robots) et quelques marques sans image exploitable. `tools/led-catalog/merge_images.py` régénère le catalogue.
- Mémoire native (très expérimental, désactivé par défaut) : première couche SMBus sans OpenRGB (accès PawnIO) avec le pilote Corsair Vengeance RGB Pro / Dominator / Vengeance RGB DDR5. Détection en lecture quasi seule (liste blanche d'écritures, jamais de zone SPD/PMIC), protégée par les tests d'octets sur faux bus ; AUCUN essai sur vrai matériel à ce jour. Activer « Drivers natifs » dans Réglages, et fermer iCUE.
- Test d'écran rendu indépendant de la charge de la machine (il échouait parfois quand l'ordinateur était occupé).

## 0.35.0

- Délais dépassés quand on change un réglage : l'envoi des couleurs à OpenRGB n'attendait jamais de réponse, donc un contrôleur lent (RAM en SMBus, carte mère) accumulait des images en file et chaque réglage suivant expirait derrière. Chaque envoi attend maintenant que le serveur l'ait traité (test sur un faux serveur lent). Les erreurs d'envoi sont écrites dans le journal (une par appareil et par minute).
- Port 6742 occupé par un autre programme : PureRGB démarre OpenRGB sur le premier port libre suivant, l'enregistre dans les réglages et reconnecte le client, sans intervention.
- Appareils affichés deux fois : avec les drivers natifs activés, la copie OpenRGB d'une carte déjà pilotée en natif est masquée ; une ligne « vu en USB » disparaît dès qu'un autre backend pilote l'appareil.
- Écran Thermalright « accès refusé » : le message ne réclamait plus à tort WinUSB quand le pilote est bon ; il nomme les processus suspects (TRCC, L-Connect, iCUE…) avec leur PID. TRCC rejoint la liste des logiciels gérés (Stopper dans le panneau, ou arrêt automatique).

## 0.34.0

- Ventilateurs, pompes et bandeaux : le sélecteur « Modèle connu » (Éclairage, zone ARGB) propose maintenant 71 références de 14 marques (Corsair, Lian Li, NZXT, Thermaltake, Cooler Master, be quiet!, Arctic, DeepCool, Phanteks, Antec, Fractal, Montech, HYTE, Razer) avec leur nombre de LEDs relevé sur la documentation du constructeur ; quantité × LEDs/unité, plus rien à compter. Catalogue généré depuis `docs/led-catalog/fans.json` (`tools/led-catalog/gen_fans.py`).
- Cartes mères : l'inventaire affiche les connecteurs ARGB/RGB du modèle (136 cartes ASUS, MSI, Gigabyte, ASRock… au catalogue) et la limite de LEDs documentée par le constructeur (MSI 75, ASRock 80, etc.). Un connecteur ne pouvant pas détecter ce qui y est branché, le nombre de LEDs d'un ventilateur reste à choisir par son modèle.
- Télémétrie (opt-in, désactivée par défaut) : le rapport contient désormais carte mère, mémoire, GPU, USB RGB vus par Windows avec leur modèle au catalogue ou « hors catalogue », constats de l'inventaire, appareils pilotés et zones, logiciels en conflit actifs, erreurs de détection des écrans, dernières erreurs du journal (sans nom de compte ni chemin). Premier envoi 90 s après le démarrage, puis toutes les 30 min si quelque chose a changé.
- Télémétrie : un rapport refusé par le serveur (schéma, taille) était compté comme envoyé ; c'est maintenant une erreur visible. Le rapport est réduit sous la limite de 64 Ko du serveur.
- Catalogues de recherche (docs) : `docs/led-catalog/` regroupe les ventilateurs, cartes mères et mémoires (DDR4/DDR5) avec source et niveau de confiance ; les nombres introuvables restent `null`, rien n'est deviné.

## 0.33.0

- Mini-écrans : la détection n'avait lieu qu'une fois au démarrage. Un écran branché mais sans pilote actif (ouverture ratée, branchement tardif, démarrage USB lent) est maintenant ré-essayé automatiquement (au changement d'état, sinon chaque minute) et la liste se met à jour toute seule.
- Mini-écrans : quand rien n'est détecté, la page affiche la raison exacte de l'échec de chaque famille de pilotes, au lieu d'un conseil générique.

## 0.32.0

- Nombre de LEDs automatique : un connecteur ARGB de carte mère (ou canal de hub) qui n'annonce aucune LED reçoit 60 LEDs d'office, plus rien à compter ; la valeur reste modifiable à la main (un 0 choisi est respecté). Un connecteur ne peut pas dire ce qui y est branché, donc c'est une taille courante, pas une mesure.
- Logiciels constructeur : seuls ceux qui TOURNENT sont signalés comme gênants (inventaire, panneau). Un logiciel installé mais arrêté affiche « aucun conflit ». Quand OpenRGB est arrêté, les appareils USB indiquent « en attente d'OpenRGB » au lieu de réclamer de fermer un logiciel.
- Inventaire : la cause de l'échec d'OpenRGB est affichée sur le constat « Serveur OpenRGB injoignable ».
- OpenRGB à initialisation lente : s'il finit par démarrer après l'échec, PureRGB re-détecte tout seul. La re-détection au branchement n'a plus lieu pendant l'initialisation d'OpenRGB, et un appareil qui disparaît puis revient n'est plus compté comme nouveau (cause probable des relances en boucle de la 0.29).

## 0.31.0

- Port du serveur OpenRGB (6742) occupé par un autre programme : PureRGB vérifie que ce qui répond parle bien le protocole OpenRGB ; sinon il nomme le programme (et son PID) dans la bannière au lieu de croire que tout va bien.

## 0.30.0

- OpenRGB qui ne démarre pas : la bannière affiche maintenant la vraie raison du dernier échec (code de sortie, autre OpenRGB ouvert, logiciel RGB constructeur actif, port 6742 pris par un autre programme) au lieu d'un simple bouton.
- Réparation automatique : si des fichiers de l'OpenRGB embarqué manquent (antivirus, suppression), il est retéléchargé (SHA-256 vérifié) au lancement. Si le processus s'arrête seul, l'échec est signalé tout de suite au lieu d'attendre 20 s.
- Journal de diagnostic dans `%APPDATA%\PureRGB\purergb.log` (l'exécutable n'a pas de console), remis à zéro au-delà de 1 Mo.
- Re-détection au branchement : au plus une fois toutes les 2 minutes, pour qu'un appareil qui se réénumère en boucle ne relance jamais OpenRGB sans cesse.

## 0.29.0

- Drivers natifs USB (expérimentaux, sous « Drivers natifs » dans Réglages, désactivés par défaut) : nouveau cadre `backends/hid/native/` (une famille de protocole par fichier, tests d'octets sur faux périphérique). Familles livrées : ASUS Aura (5 PID), ASRock Polychrome (2), MSI Mystic Light 185 octets (mode statique par zone), Gigabyte RGB Fusion 2 (3 PID, 2 connecteurs ARGB), Razer (7 modèles, matrice moderne), Wooting V2/V3 (5), HyperX Alloy Origins et tapis (8), Thermaltake Riing Plus (18 PID, ventilateurs + RGB), Corsair Commander Pro (ventilateurs), Lian Li Uni Hub SL (RGB). Rien n'a encore tourné sur du vrai matériel ; OpenRGB reste le repli pour tout le reste.
- Catalogue de 1 087 identifiants USB de contrôleurs RGB (85 fabricants) : l'inventaire reconnaît le modèle exact et dit pourquoi un appareil connu n'est pas piloté.
- Corrections d'identifiants : Lian Li A100 = Uni Hub SL (A101 = AL), Kraken 2023 (300C = Elite, 300E = standard).
- Documentation de protocoles dans `docs/rgb-protocols/` (SignalRGB, cartes mères, périphériques, refroidissement, RAM/GPU).

## 0.28.0

- Détection automatique : quand un périphérique d'un fabricant d'éclairage (Corsair, Razer, NZXT, ASUS, MSI, Gigabyte/ITE, SteelSeries, Lian Li, Cooler Master, Thermaltake, HyperX, Logitech, Roccat) est branché, PureRGB relance l'OpenRGB qu'il gère, re-détecte et met la liste à jour, sans clic (avant : seulement les appareils inconnus étaient signalés).
- Réglages > « Inventaire RGB et ventilateurs » : compare ce que Windows voit (carte mère, barrettes de mémoire, cartes graphiques, USB) à ce que PureRGB pilote, et explique chaque écart : pilote PawnIO absent pour la mémoire, logiciel constructeur en conflit, connecteurs ARGB à 0 LED, ventilateurs mesurés mais non pilotables, capteurs indisponibles. Bouton « Redémarrer OpenRGB » dans le panneau.
- Interne : le réglage du décodeur vidéo devient son propre composant (SettingsPanel repasse sous 800 lignes).

## 0.27.0

- Écrans : l'aperçu n'est plus pivoté par la rotation. Elle compense le montage de l'écran dans le boîtier ; l'aperçu montre le design tel qu'on le voit sur l'écran, à l'endroit (corrige l'aperçu « à l'envers » avec une rotation de 180°).
- 12 nouveaux thèmes vidéo « Forge » (18 au total) : cartes à gauche, bandeau haut, colonnes, variantes ambre / cyan / rose / vert, formats 1920×462 et 1280×480. Tous avec CPU, GPU, RAM, SSD 1 et SSD 2 (masqué si absent), heure et date.
- Réglages et options d'écran : cartes réparties sur 2 à 3 colonnes selon la largeur au lieu d'une pile étroite.

## 0.26.0

- Photos des produits : quand PureRGB identifie exactement un appareil (mini-écran, AIO, ventilateur, clavier…), il affiche la vraie photo du produit, avec le nom exact du modèle, dans les onglets Écrans, Éclairage et Ventilateurs. 133 modèles au catalogue (Thermalright dont le Trofeo Vision 9.16, Lian Li, Corsair, NZXT, DeepCool, ASUS, Thermaltake, Cooler Master, Turing, Arctic, Razer…). Les photos sont téléchargées depuis le site du fabricant, vérifiées (décodage réel, 640 px max) et mises en cache ; jamais incluses dans l'application. Réglage pour les désactiver (aucune requête réseau alors).
- Identification précise : correspondance par identifiant USB puis par nom complet, jamais par le seul fabricant ; aucune photo « approchante » d'un autre modèle.

## 0.25.0

- Vidéos des écrans : si Windows ne sait pas lire un fichier (édition « N », extensions vidéo absentes), PureRGB propose d'installer un décodeur universel (ffmpeg 9.0.2, 115 Mo) puis relit la vidéo automatiquement. Sécurité : archive épinglée (taille + SHA-256 vérifiés avant extraction, redirections limitées à GitHub), seul `ffmpeg.exe` est extrait, dans `%ProgramFiles%\PureRGB\tools\` ; son empreinte est revérifiée avant usage. Media Foundation reste utilisé en premier.
- Réglages : ligne « Décodeur vidéo universel : installé / non installé ».
- Designs : bibliothèque refaite, 660 designs répartis en 11 catégories de 60 (moniteur complet, barres et courbes, jauges et anneaux, rond et AIO, gros chiffres et minimal, horloges, heure et calendrier, gaming et HUD, rétro et néon, verre et blueprint, ultra-large avec fond libre). Règle commune : chaque design affiche au minimum CPU (température et charge), GPU (température et charge), RAM, SSD 1 et SSD 2 (masqué automatiquement s'il n'existe pas), puis des extras selon la catégorie (heure, jour, mois, année, disques, carte mère, réseau, ventilateurs, AIO, puissance…).
- Thèmes vidéo intégrés « Forge » (6 formats : ultra-large 1920x462, 1280x480, carré) avec la vidéo fournie dans l'application, utilisable comme fond de n'importe quel design.
- Éditeur : nouvelle propriété « Visible si » pour n'afficher un widget que si une donnée existe sur le PC (ex. second SSD).
- Les anciens designs faits main (sans le socle complet) sont remplacés par les nouveaux.

## 0.24.0

- Fermeture corrigée : plus de « ne répond plus » en quittant, et plus aucun processus ne reste en arrière-plan (OpenRGB, sensord, liquidctl, WebView2 sont fermés avec l'application, même après un plantage ou un arrêt forcé).
- Plus de 700 designs d'écran (génération par familles : capteurs détaillés, styles cyberpunk/synthwave/CRT/LCD/verre/néon/blueprint…, horloges, formats ultra-larges 1920x462 avec centre libre pour une vidéo de fond, ronds, rubans) en plus des 47 faits main.
- Vidéo et GIF plus fluides : rendu, encodage et envoi USB en parallèle, cadence planifiée précisément, qualité JPEG adaptative, plafond de 16 images/s pour les écrans Thermalright LY (comme TRCC). Cadence réelle affichée dans les options de l'écran.
- Capteurs : détection du CPU et du GPU fiabilisée (Intel, AMD, NVIDIA…), nouvelles sources (disques par lettre et SSD, carte mère, chipset, VRM, ventilateurs, débit disque, réseau, puissance…), sélecteur de capteur brut et panneau « Capteurs détectés » pour diagnostiquer.
- Éditeur : bouton de rotation rapide (90° et 180°), 19 blocs prêts à l'emploi, sélecteur de média pour le fond, galerie filtrable pour de grandes bibliothèques.
- Thèmes avec médias : export/import d'un thème `.purergb-theme` (design + vidéo/GIF/images), validé (taille, chemins, SHA-256).
- Pilotes Thermalright : constantes du protocole LY vérifiées sur l'analyse de TRCC 2.1.8 (rotation selon le modèle, qualité JPEG, résolution 1920x462).

## 0.23.0

- Nouveau : onglet **Écrans** pour piloter les mini-écrans LCD (AIO, ventirads, boîtiers, écrans USB-C internes) depuis PureRGB, à la place des logiciels constructeur.
- Moteur de rendu intégré (jauges, textes, courbes d'historique, barres, horloge analogique, images et GIF) alimenté en temps réel par les capteurs ; ce qui est vu dans l'aperçu est ce qui est envoyé à l'écran, application fermée dans la zone de notification comprise.
- Éditeur de designs : déplacer, redimensionner, pivoter, calques, annuler/refaire, import/export JSON, médiathèque d'images et de GIF.
- Bibliothèque de designs fournie (moniteur, gaming, minimal, horloges, vitrine) dont la disposition CPU/RAM/heure + GPU/SSD/date.
- Pilotes (expérimentaux, non validés sur matériel réel) : Thermalright (SCSI, HID, Bulk, LY), Turing Smart Screen / XuanFang / UsbPCMonitor / Kipye (USB série). Écran simulé pour essayer sans matériel.
- Vidéos (mp4, m4v, mov, avi, wmv, mkv, webm selon les codecs de Windows) en fond ou en widget, décodées par Windows (Media Foundation) : aucun logiciel supplémentaire à installer. Cadence relevée automatiquement (15 i/s min.) pour les GIF et vidéos.
- AIO sans flux direct connu (ASUS ROG Ryujin III) : bouton « Envoyer comme image fixe » (image stockée dans l'appareil).
- Documentation des protocoles des autres constructeurs dans `docs/screens-protocols/` (Lian Li, Corsair, DeepCool, ASUS, Thermaltake…), pilotes prévus ensuite.

## 0.22.0

- Sécurité : PureRGB, OpenRGB, liquidctl, sensord et le pilote PawnIO ne sont plus lancés avec les droits administrateur que depuis `%ProgramFiles%` (jamais depuis `%APPDATA%`/`%LOCALAPPDATA%`). Les binaires gérés sont installés dans `%ProgramFiles%\PureRGB\` après vérification SHA-256 ; le démarrage automatique (tâche planifiée) est refusé si l'exe n'est pas dans Program Files.
- L'installeur passe en mode « par machine » (Program Files).
- OpenRGB : version et SHA-256 épinglés, URL en liste blanche, téléchargement sans PowerShell ; la mise à jour automatique « dernière version Codeberg » est supprimée (OpenRGB évolue avec les versions de PureRGB). Le serveur SDK n'écoute plus que sur 127.0.0.1.
- Envoi de capture USB : plus de jeton embarqué dans le binaire (envoi anonyme).

## 0.21.0

- Mise à jour automatique de l'application : vérification silencieuse au lancement, bouton « Rechercher une mise à jour » dans les réglages, signature minisign vérifiée, installation silencieuse (NSIS passive) puis relance.
- Nouveau réglage « Vérifier les mises à jour automatiquement » (actif par défaut), distinct du démarrage automatique d'OpenRGB.
- Publication : la CI joint l'installeur, sa signature `.sig` et `latest.json` à la release.

## 0.20.0

- Control Flow Guard activé sur les builds Windows.
