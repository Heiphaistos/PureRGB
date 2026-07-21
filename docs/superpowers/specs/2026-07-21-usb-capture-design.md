# PureRGB — Capture USB intégrée au panneau Diagnostic — Design

## Contexte

Suite à la télémétrie matériel opt-in (v0.14.0) : Momo a rempli le catalogue distant avec 5 appareils réels. Vérification faite contre le code source OpenRGB (`Controllers/*/`) : 2 sont déjà pilotables (Cougar 700K EVO clavier, écran LG UltraGear — même VID/PID que le modèle officiellement supporté), 3 ne le sont pas du tout (souris Cougar DualBlader, souris/clavier Empire Gaming RF-903/RF-800 — VID partagé avec d'autres fabricants mais PID absent de tout driver OpenRGB existant).

Pour ces 3 appareils, la seule voie pour ajouter un vrai support (pas juste l'étiquette "reconnu") est de capturer le trafic USB réel pendant que le logiciel officiel du fabricant change une couleur, puis d'écrire un driver natif à partir des octets observés — même méthode que Corsair Lighting Node / NZXT HUE2 dans ce projet. Faire ça manuellement (Wireshark + USBPcap installés à la main par Momo) marche mais demande de guider Momo pas à pas à chaque nouvel appareil. Cette spec intègre la capture directement dans PureRGB pour fluidifier ce futur usage récurrent.

**Important, à ne jamais perdre de vue** : cette fonctionnalité automatise la collecte de données (installer le driver, lancer/arrêter la capture, sauvegarder, uploader) — **pas** l'analyse. Écrire le driver reste un travail manuel après coup, capture par capture.

## Faits techniques vérifiés (pas supposés)

- **USBPcap 1.5.4.0**, release GitHub officielle : `https://github.com/desowin/usbpcap/releases/download/1.5.4.0/USBPcapSetup-1.5.4.0.exe`. Téléchargé et hashé cette session — **SHA-256 réel** : `87a7edf9bbbcf07b5f4373d9a192a6770d2ff3add7aa1e276e82e38582ccb622`.
- Licences (README officiel) : `USBPcapDriver` (le filtre noyau) en **GPLv2** — traité comme OpenRGB dans ce projet : binaire tiers non modifié, mention licence, jamais lié statiquement à PureRGB. `USBPcapCMD` (l'outil CLI) en **BSD 2-Clause**.
- Release signée avec un vrai certificat commercial (dossier `certificates/` du repo) — **pas besoin d'activer `TESTSIGNING`** sur la machine de l'utilisateur (contrairement à une build depuis les sources), donc pas de dégradation de la posture de sécurité Windows.
- **Limite confirmée** (issue GitHub desowin/usbpcap#64 + doc officielle) : `USBPcapCMD.exe` filtre uniquement par **hub racine** (`\\.\USBPcapN`), jamais par appareil précis. Plusieurs appareils sur le même hub apparaissent mélangés dans une même capture.
- Syntaxe CLI confirmée (tour officiel) : `USBPcapCMD.exe -d \\.\USBPcap2 -o sortie.pcap` démarre une capture sur le hub racine N vers un fichier ; **Ctrl+C** (ou tuer le process) l'arrête proprement.

## Décision de conception : capturer tous les hubs racine, filtrer après coup

Plutôt que de corréler précisément quel hub racine correspond au VID/PID visé (nécessiterait de parcourir l'arbre PnP Windows, fragile), l'app capture **tous les hubs racine détectés en parallèle** pendant la fenêtre. Comme le VID/PID cible est déjà connu (ligne sélectionnée dans le panneau Diagnostic), le filtrage se fait après coup côté analyse (`tshark`/Wireshark, filtre `usb.idVendor==X && usb.idProduct==Y`) — pas besoin de corrélation complexe côté app. Contrepartie acceptée : légèrement plus de données par capture (toujours petit pour une session de quelques dizaines de secondes), un seul process supplémentaire par hub racine présent (typiquement 2 à 8 sur un PC de bureau).

## Portée

### 1. Module Rust `src-tauri/src/usbcapture.rs` (nouveau)

- **Installation à la demande** (même schéma que `OpenRgbManager::pawnio_install()`) : télécharge `USBPcapSetup-1.5.4.0.exe` via PowerShell (`Invoke-WebRequest`), vérifie le SHA-256 pinné ci-dessus, lance `-install -silent` (mode silencieux documenté du setup NSIS). Ne réinstalle pas si déjà présent (détection : présence de `HKLM\SYSTEM\CurrentControlSet\Services\USBPcap` ou tentative d'ouverture de `\\.\USBPcap1`).
- **Énumération des hubs racine** : tente d'ouvrir `\\.\USBPcap1` à `\\.\USBPcap8` (bornes larges, un PC de bureau dépasse rarement 8 contrôleurs hôte USB) ; ceux qui s'ouvrent sont retenus.
- **Démarrage capture** (`start_capture(target_vid: String, target_pid: String) -> Result<CaptureSession>`) : spawn un process `USBPcapCMD.exe -d \\.\USBPcapN -o <dossier>\usbcapture_<N>.pcap` par hub racine détecté ; chaque process tourne en arrière-plan.
- **Arrêt capture** (`stop_capture(session: CaptureSession) -> Vec<PathBuf>`) : termine chaque process proprement (équivalent Ctrl+C — `taskkill` ciblé par PID, pas par nom, pour ne tuer que les process lancés par cette session), attend la fermeture des fichiers, retourne la liste des chemins `.pcap` produits (non vides seulement — un hub racine sans trafic produit un fichier vide/quasi-vide, filtré).
- **Nettoyage** : dossier `%APPDATA%\PureRGB\captures\<horodatage>\`, jamais purgé automatiquement (Momo peut vouloir les regarder après coup), mais listé/géré depuis l'UI (suppression manuelle possible).
- Durée de capture bornée : arrêt forcé automatique après **5 minutes** même sans clic "Arrêter" (anti-oubli, évite un fichier qui grossit indéfiniment si Momo part sans arrêter).

### 2. UI — Panneau Diagnostic (`SettingsPanel.vue`)

Par ligne d'appareil **non reconnu** dans le tableau HID brut existant (`hidRows()`), nouveau bouton **"Capturer le protocole USB"**.

Flux :
1. Clic → écran d'avertissement modal (pas de case à cocher pré-existante, consentement explicite à **chaque** capture, indépendant du toggle télémétrie général) :
   > *"Cette capture enregistre TOUT le trafic USB de cet ordinateur pendant la fenêtre, pas seulement cet appareil — d'autres périphériques branchés sur le même port apparaîtront aussi. Si un clavier est branché, vos frappes peuvent être incluses dans la capture. Ne tapez rien de sensible (mots de passe, etc.) pendant que la capture est active. Le fichier reste en local — vous choisirez ensuite de l'envoyer ou non."*
2. Bouton "Démarrer" → installe USBPcap si absent (barre de progression), lance la capture sur tous les hubs racine, affiche un chronomètre + instruction : *"Ouvrez maintenant le logiciel officiel de cet appareil et changez une couleur ou un effet."*
3. Bouton "Arrêter" (ou arrêt auto à 5 min) → sauvegarde, écran récapitulatif : liste des fichiers produits + taille + hub racine, avec deux actions :
   - **"Envoyer pour analyse"** — upload vers le nouvel endpoint télémétrie (consentement de l'envoi, distinct du consentement de capture déjà donné à l'étape 1 — un clic supplémentaire explicite, jamais automatique).
   - **"Garder en local seulement"** — ferme l'écran, fichiers restent dans `%APPDATA%\PureRGB\captures\`.

### 3. Service VPS (`PureRGB-Telemetry`)

- Nouvelle table `capture_uploads` (id, filename, size_bytes, uploaded_at) — pas de contenu binaire en SQLite, fichiers sur disque (`/app/data/captures/`).
- `POST /capture-upload` : authentification **par token statique** différente du login dashboard (l'app ne peut pas se connecter avec un mot de passe humain). Contrairement à `JWT_SECRET`/`IP_HASH_PEPPER` (générés par le VPS, jamais transmis nulle part), ce token doit être **identique des deux côtés** puisque c'est un secret partagé app↔serveur — il ne peut donc pas être généré indépendamment par chacun. Généré **une seule fois à l'implémentation** (valeur aléatoire forte), compilé en dur dans le binaire PureRGB (même traitement que `TELEMETRY_BASE_URL`, déjà une constante fixe puisque l'app ne parle qu'à ce seul service VPS contrôlé par Momo) et posé à l'identique dans le `.env` du VPS (`CAPTURE_UPLOAD_TOKEN`). Vérifié par comparaison à temps constant. Taille plafonnée (**50 Mo** par requête), extension forcée `.pcap`, nom de fichier assaini (horodatage + UUID généré serveur, jamais le nom fourni par le client).
- Dashboard : nouvelle section "Captures USB reçues" (liste + bouton téléchargement individuel), même protection mot de passe que l'existant.
- Rate-limit sur `/capture-upload` (ex. 20/heure par IP hashée — cohérent avec `/report`, capture volontaire et rare, pas besoin de plus permissif).

### 4. Sécurité

- Token d'upload : valeur aléatoire forte fixée à l'implémentation (jamais une valeur par défaut/devinable), compilée dans le binaire Rust — jamais exposée via une commande Tauri accessible au frontend (aucun `invoke()`, même depuis devtools, ne peut le lire ; seul le code Rust qui construit la requête HTTP y a accès).
- `USBPcapCMD.exe`/`USBPcapSetup-1.5.4.0.exe` : SHA-256 vérifié avant exécution, comme PawnIO.
- Nécessite déjà les droits administrateur (PureRGB les demande au lancement pour PawnIO/OpenRGB — cohérent, USBPcap aussi requiert un driver noyau).
- Fichiers de capture jamais exposés publiquement côté VPS — dashboard authentifié uniquement, comme le reste du catalogue.
- Capture jamais silencieuse : consentement explicite à 2 reprises (démarrage capture, puis envoi) — jamais lié au toggle télémétrie général.

## Cas limites

- USBPcap déjà installé (utilisateur avancé, Wireshark existant) : détection réutilise l'install existante, pas de réinstallation.
- Aucun hub racine ne s'ouvre (droits insuffisants, driver mal installé) : message clair, pas de crash, capture annulée proprement.
- Process `USBPcapCMD.exe` meurt en cours de route (device débranché, erreur) : détecté au moment de l'arrêt (fichier absent/vide pour ce hub), simplement omis de la liste finale, pas d'échec global.
- Upload échoue (réseau, VPS injoignable) : fichier reste en local, message d'erreur clair, retry manuel possible (pas de tentative automatique en arrière-plan — c'est une action explicite de l'utilisateur, pas un envoi best-effort comme la télémétrie diagnostic).
- Capture dépasse 5 minutes sans clic "Arrêter" : arrêt automatique, écran récapitulatif affiché quand même (pas perdu).

## Tests

- Rust : tests unitaires sur l'énumération des hubs racine (mock des chemins `\\.\USBPcapN`), sur la construction des commandes `USBPcapCMD.exe` (arguments exacts), sur le filtrage des fichiers vides.
- VPS : tests d'intégration sur `/capture-upload` (token valide/invalide, taille limite, rate-limit), comme les endpoints existants.
- Vérification manuelle (nécessaire, pas automatisable) : capture réelle sur une machine avec USBPcap fraîchement installé, confirmer qu'au moins un des 3 appareils cibles (DualBlader, RF-903, RF-800) produit des paquets visibles avec le bon VID/PID une fois filtré dans Wireshark.

## Statut

Design validé section par section avec Momo (gestion du risque clavier, destination des captures via dashboard, consentement séparé à chaque capture confirmé deux fois après un bug de sélection, déclenchement manuel démarrer/arrêter, capture multi-hub plutôt que corrélation précise). URL et SHA-256 USBPcap vérifiés en direct cette session (téléchargement réel + hash calculé, pas une valeur supposée). Prêt pour plan d'implémentation.
