# Journal des modifications

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
