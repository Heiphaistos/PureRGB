# Journal des modifications

## 0.25.0

- Vidéos des écrans : si Windows ne sait pas lire un fichier (édition « N », extensions vidéo absentes), PureRGB propose d'installer un décodeur universel (ffmpeg 9.0.2, 115 Mo) puis relit la vidéo automatiquement. Sécurité : archive épinglée (taille + SHA-256 vérifiés avant extraction, redirections limitées à GitHub), seul `ffmpeg.exe` est extrait, dans `%ProgramFiles%\PureRGB\tools\` ; son empreinte est revérifiée avant usage. Media Foundation reste utilisé en premier.
- Réglages : ligne « Décodeur vidéo universel : installé / non installé ».

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
