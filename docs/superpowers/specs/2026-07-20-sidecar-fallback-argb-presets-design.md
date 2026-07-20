# PureRGB — fallback sidecars portables + sélecteur ARGB connus + diagnostic ventilateurs

Date : 2026-07-20
Statut : approuvé par Momo (verbal, session brainstorming)

## Contexte / bugs confirmés

Retour terrain (pote de Momo, VM/tour perso, exe **portable**) : catégorie
Ventilateurs à 0 appareil, ventilos MF120 Prismatic (Cooler Master, 30 LEDs
chacun, branchés direct sur header ARGB carte mère — pas de boîtier USB,
confirmé absence de VID 0x2516 dans l'énumération USB brute du diagnostic)
ne s'allument pas.

Capture d'écran du **Diagnostic matériel** intégré (Réglages) :
- `liquidctl` : binaire introuvable (ni `resources/`, ni `%APPDATA%`)
- `sensord` : binaire introuvable, 0 capteur remonté, pas en cours
- `OpenRGB` : trouvé sous `%APPDATA%\PureRGB\openrgb\` (auto-installé) — fonctionnel

**Cause racine identifiée** (`src-tauri/tauri.conf.json:31` : `"targets": ["nsis"]`,
pas de cible portable Tauri dédiée) : le binaire "portable" distribué est le
`.exe` brut sans dossier `resources/` à côté. `OpenRgbManager` a un fallback
`install()` (téléchargement + SHA-256 pinné + extraction dans `%APPDATA%`,
`src-tauri/src/backends/openrgb/manager.rs:208-295`) qui compense — jamais
répliqué pour `liquidctl` (`backends/liquidctl/mod.rs:88-104`, `locate()`
seul, pas d'`install()`) ni `sensord` (`sensors.rs:71-87`, idem). Résultat :
sur un exe portable, `liquidctl.exe`/`sensord.exe` restent introuvables pour
toujours, silencieusement — d'où ventilateurs carte mère à 0 (backend `mobo`
dépend des capteurs Control de `sensord`) et AIO/hubs liquidctl invisibles.

Second problème, indépendant : zones ARGB sur header carte mère (fix v0.7.0,
`resize_zone`) nécessitent que l'utilisateur connaisse le nombre exact de
LEDs de son matériel pour redimensionner correctement la zone. Détection
matérielle automatique du nombre de LEDs est **physiquement impossible** sur
un header 3-pin passif (protocole WS281x-like unidirectionnel, aucune voie
de retour — limite hardware universelle, pas un manque OpenRGB/PureRGB).

Troisième problème : `FanPanel.vue:124-128` affiche un message d'erreur
générique qui ne mentionne pas le backend `mobo` et ne distingue pas
"sensord absent" de "sensord actif mais 0 canal Control sur ce chipset".

## Portée de ce cycle

1. Fallback téléchargement pour `liquidctl` et `sensord` (parité avec OpenRGB).
2. Sélecteur de ventilateurs/strips ARGB connus (calcul LED × quantité →
   `resize_zone`), avec entrée manuelle en repli.
3. Diagnostic ventilateurs carte mère : message différencié dans `FanPanel`.

Hors scope (specs séparées à venir, décomposition actée avec Momo) :
Scheduler, Hardware Sync (couleur ↔ temp/charge), E1.31 Receiver, Visual Map,
extras Effects Plugin (audio-viz, Ambilight, GIF, shaders GLSL).

## 1. Fallback sidecars portables

**Architecture** : répliquer exactement le pattern `OpenRgbManager` sur
`LiquidctlBackend` et `SensorHub` — constante `<NAME>_URL` + `<NAME>_SHA256`,
méthode `install()` (PowerShell : `Invoke-WebRequest` → vérif SHA-256 →
extraction dans `%APPDATA%\PureRGB\<name>\`), appelée en repli quand
`locate()` échoue, avant d'abandonner (actuellement `SensorHub::start()`
retourne `Ok(false)` silencieux, `LiquidctlBackend` ne tente jamais rien).

**Hébergement des binaires** : `liquidctl.exe` (PyInstaller onefile,
`build-sidecars.ps1`) et `sensord.exe` (self-contained .NET 8) sont des
artefacts de build produits par PureRGB lui-même (pas de release officielle
Windows tierce pour liquidctl, sensord est notre propre sidecar). Ils seront
attachés comme **assets de la release GitHub `Heiphaistos/PureRGB`** (tag
dédié, réutilisable entre versions applicatives pour éviter de re-signer un
SHA à chaque release mineure — même logique que le pin OpenRGB/PawnIO qui ne
suit pas la version de l'app). Bootstrapping (comme pour PawnIO en v0.4.0) :
builder les binaires → uploader sur un tag `sidecars-v1` → calculer le
SHA-256 réel → l'épingler dans le code → seulement alors release app.

**Modifications** :
- `backends/liquidctl/mod.rs` : `LIQUIDCTL_URL`/`LIQUIDCTL_SHA256`, méthode
  `install()` (mirror `OpenRgbManager::install()` sans la logique VC++ DLL —
  liquidctl PyInstaller onefile est statique). Les deux sites qui font
  `self.exe = self.locate()` (`mod.rs:155` et `mod.rs:272`, initialisation +
  `scan()`) appellent `install()` en repli si `locate()` retourne `None`.
- `sensors.rs` : `SENSORD_URL`/`SENSORD_SHA256`, méthode `install()` sur
  `SensorHub`. `start()` appelle `install()` si `locate()` échoue avant de
  retourner `Ok(false)`.
- `LiquidctlDiag`/`SensorDiag` (déjà exposés au diagnostic) : ajouter un
  champ ou message distinguant "introuvable, tentative de téléchargement en
  cours/échouée" de "introuvable, jamais tenté" pour ne pas complexifier le
  diagnostic (message texte suffit, pas de nouveau type).

**Risque / non-régression** : `ensure_running`/scan ne doivent pas bloquer
l'UI plusieurs secondes à chaque démarrage si le réseau est absent — même
comportement que l'installation OpenRGB existante (déjà async côté appelant,
thread `hw-init`), donc pas de changement d'architecture de threading requis.

## 2. Sélecteur ARGB connus

**Architecture** : 100% frontend, aucune modification Rust (réutilise la
commande `resize_zone` existante telle quelle). Nouveau fichier
`src/data/fanPresets.ts` : tableau `{ brand, model, ledsPerUnit }[]`, aussi
exhaustif que raisonnable au lancement (Cooler Master MF/SickleFlow/Halo/
Prismatic, Corsair QL/LL/ML, NZXT F/Aer, Lian Li SL/UNI/AL Infinity,
Thermaltake Riing/Pure/Toughfan, DeepCool, be quiet! Light Wings, Phanteks
D-RGB, EK, Alphacool, Arctic P/A-RGB, ID-Cooling, Montech, Cougar, SilverStone,
+ entrée générique "Strip WS2812B/SK6812 — LEDs/m × longueur"). Table éditable
facilement (une ligne = un modèle), pas de logique complexe.

UI : dans le panneau zones/Éclairage, à côté du champ de resize manuel
actuel, un sélecteur "Modèle connu" (marque → modèle) + un input "quantité
sur la chaîne" → calcule `ledsPerUnit × quantité`, pré-remplit le champ de
taille existant. L'utilisateur garde la main pour ajuster/valider avant
envoi (pas d'auto-apply sans confirmation — cohérent avec le pattern
resize existant). Entrée manuelle reste toujours disponible pour matériel
non listé.

**Testabilité** : fonction de calcul pure `ledsFor(preset, qty)` testable
unitairement côté TS (Vitest, déjà en place vu la stack Vue).

## 3. Diagnostic ventilateurs carte mère

**Architecture** : `FanPanel.vue` reçoit déjà `devices` (filtré
`fan_channels.length > 0`) mais aucune info sur *pourquoi* c'est vide.
Ajouter un appel `get_sensor_diag` (nouvelle commande Tauri triviale
wrapping `SensorHub::diag()`, déjà existant côté Rust — juste l'exposer) au
montage du panneau, à côté de `get_sensors`. Remplacer le message générique
statique (`FanPanel.vue:124-128`) par une logique conditionnelle :
- `sensord` introuvable → "capteurs indisponibles, tentative d'installation
  au prochain lancement" (ou "échec, vérifier connexion réseau" si le
  fallback #1 a été tenté et a échoué)
- `sensord` en cours, 0 capteur `Control` de type carte mère trouvé →
  "aucun header PWM piloté par LibreHardwareMonitor sur ce chipset — RGB
  toujours possible via OpenRGB, contrôle de vitesse non disponible sur
  cette carte mère"
- cas déjà couvert (liquidctl/hubs natifs) : message existant conservé

Bonus ciblé, faible risque : bump `LibreHardwareMonitorLib` 0.9.6 → dernière
version stable dans `sidecars/sensord/sensord.csproj` (meilleure couverture
Super I/O récents) — vérifier changelog avant bump, pas de breaking change
connu attendu vu l'API stable de la lib.

## Tests

- Unit Rust : `install()` de `LiquidctlBackend`/`SensorHub` — vérification
  SHA-256 (cas hash valide/invalide), pattern déjà couvert implicitement
  côté OpenRGB par la même logique PowerShell (pas de test unitaire dédié
  existant pour `OpenRgbManager::install()` non plus — cohérence : on ne
  sur-teste pas ce qui ne l'était pas avant, mais on garde les tests
  `rpm_pairing_by_number` etc. intacts dans `mobo.rs`).
- Unit TS : `ledsFor(preset, qty)` (calcul pur, plusieurs presets + edge
  cases quantité 0/négative rejetée par l'UI).
- E2E hôte (comme les cycles précédents) : diagnostic affiche fallback
  sidecar en action si `%APPDATA%\PureRGB\liquidctl` et `\sensord` absents
  au premier lancement, panneau Ventilateurs affiche le bon message selon
  état sensord simulé.
- Validation réelle limitée par le matériel disponible ici (pas de MF120
  Prismatic, pas de header ARGB carte mère physique sur la machine de dev) —
  comme les cycles précédents (v0.7/v0.8), validation finale sur le
  matériel de Momo ou de son pote.

## Hors scope explicite

Pas de duplication de l'écosystème complet de plugins OpenRGB dans ce
cycle — Scheduler / Hardware Sync / E1.31 Receiver / Visual Map / extras
Effects Plugin sont actés comme roadmap, un spec dédié chacun, dans cet
ordre (effort croissant).
