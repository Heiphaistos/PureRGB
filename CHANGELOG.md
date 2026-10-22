# Journal des modifications

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
