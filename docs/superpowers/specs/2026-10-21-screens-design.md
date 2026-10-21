# Module Écrans — conception (2026-10-21)

Objectif de Momo : une seule appli pour piloter **tous** les mini-écrans (AIO, ventirads, boîtiers, écrans USB-C internes) à la place de TRCC (Thermalright), L-Connect (Lian Li), etc. Températures/charges temps réel, vidéos/GIF/images redimensionnables, **beaucoup** de designs (le point faible des logiciels constructeur), éditeur de design complet.

## Architecture

```
sensord/sysinfo ──► bindings.rs ──► render/ (tiny-skia) ──► Frame RGBA ──► driver ──► USB
 (clés "cpu.temp")   ValueStore       Layout JSON + média      │              (RGB565/JPEG)
                                                                └──► PNG base64 ──► aperçu éditeur
```

- **Rendu 100 % Rust** : un seul moteur pour l'aperçu et l'écran réel ; fonctionne fenêtre fermée.
- **Contrat** : `screens/driver.rs` (`Frame`, `ScreenInfo`, `ScreenDriver`, `ScreenProvider`) et `screens/layout.rs` (JSON). Ne pas les casser sans prévenir : tout le reste en dépend.
- **Scheduler** (`screens/runner.rs`) : un thread par écran actif, cadence propre (1–30 FPS selon le design : horloge/capteurs = 1 FPS, vidéo = 15–30), n'envoie que si l'image a changé, reprise sur erreur (réouverture du périphérique, backoff), arrêt propre.
- **Persistance** : `%APPDATA%\PureRGB\screens.json` (écran → design actif, luminosité, rotation) ; designs utilisateur et médias dans `%APPDATA%\PureRGB\screens\{designs,media}`.

## Sources de données (`bindings.rs`)

Clés stables : `cpu.temp|load|clock|power|fan`, `gpu.temp|load|clock|power|vram_used_pct|fan`, `ram.used_pct|used_gb|total_gb`, `ssd.<n>.temp|used_pct`, `liquid.temp`, `pump.rpm`, `fan.<n>.rpm`, `net.up|down` (Mo/s), `time`, `date`, `uptime`, et `sensor:<id sensord>` pour n'importe quel capteur. Source : `SensorHub` existant (sensord, 2 s) + `sysinfo` pour RAM/réseau/disques.

## Pilotes — état des sources

| Famille | VID:PID | Source du protocole | Statut visé |
|---|---|---|---|
| Thermalright SCSI | 87CD:70DB, 0402:3922 | doc publique TRCC-Linux (`doc/REFERENCE_TECHNICAL.md`) | v1 |
| Thermalright HID type 2/3 | 0416:5302, 0416:5406, 0418:5303/5304 | idem | v1 |
| Thermalright Bulk | 87AD:70DB | idem | v1 |
| Thermalright LY | 0416:5408, 0416:5409 | idem | v1 |
| Turing Smart Screen / XuanFang / UsbPCMonitor | série USB (CH552) | wiki turing-smart-screen-python (« Hardware revisions ») | v1 |
| Kraken Z / 2023 | existant (liquidctl) | déjà dans PureRGB | à rattacher à l'éditeur |
| Lian Li (Galahad II LCD, SL-INF…), DeepCool, Corsair LCD, MSI, ASUS, Cooler Master, Arctic, Antec… | **à analyser avec REA** sur les logiciels constructeur | docs/screens-protocols/<vendor>.md | v2 |

Règle : un protocole n'entre dans le code que s'il est documenté publiquement **ou** observé (REA/capture USB) ; jamais deviné. Les écrans non vérifiés sur matériel réel sont marqués « expérimental » (`ScreenInfo.verified = false`).

## Designs (bibliothèque embarquée)

JSON `Layout` dans `screens/designs/*.json`, chargés via `include_str!` (liste dans `designs/mod.rs`). Objectif v1 : **40+ designs** répartis en catégories (Moniteur CPU/GPU, Minimal, Gaming, Rétro/Néon, Horloge, Aquarium/Média plein écran, Cyber, Pro/Sobre…), chacun conçu pour plusieurs ratios (carré 480², 320², 240², 1280×480, 1920×462, 640×172) en s'appuyant sur le rendu à l'échelle + variantes de mise en page. Le design de référence de Momo (photo à venir) : CPU temp+charge / RAM / heure à gauche, GPU temp+charge / SSD / date à droite.

## Éditeur (front, `src/components/screens/`)

Liste des écrans détectés → onglet par écran : aperçu temps réel (PNG du moteur), galerie de designs, éditeur de widgets (déplacer/redimensionner/pivoter à la souris, inspecteur : source de données, format, police, couleurs, rampe), médiathèque (images, GIF, vidéos importées, recadrage/échelle/ajustement cover-contain-stretch), luminosité/rotation, import/export d'un design (.json), « envoyer vers l'écran » + mode simulateur.

## Vidéo

Décodage via `ffmpeg.exe` sidecar épinglé SHA-256 (même mécanisme que liquidctl/sensord) : transcodage à l'import en séquence d'images JPEG à la résolution cible (cache disque), lecture en boucle par le scheduler. GIF : crate `image`.

## Livraison par phases

1. **v0.23.0** : contrat + rendu + 10 widgets + sim + Thermalright (4 protocoles) + Turing série + éditeur de base + 40 designs + médias image/GIF.
2. **v0.24.0** : vidéo (ffmpeg), rattachement Kraken LCD, éditeur avancé (calques, rampes, historique annuler/refaire).
3. **v0.25+** : constructeurs analysés par REA (Lian Li, DeepCool, Corsair, …), un fichier `docs/screens-protocols/<vendor>.md` puis un driver par vendeur.

## Hors périmètre / honnêteté

Pas de matériel d'écran sur le PC de dev : tout pilote réel est livré « expérimental » tant que Momo n'a pas validé sur sa tour. Les tests automatiques couvrent les trames (octets), le rendu (PNG de référence) et l'écran simulé.
