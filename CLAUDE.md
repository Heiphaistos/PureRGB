# PureRGB — instructions projet

Tauri v2 (Rust) + Vue 3 + TypeScript. Windows 10/11 uniquement. Interface 100 % française. Dépôt `Heiphaistos/PureRGB` (branche `main`).

## Règles de code (non négociables)

- **Max 800 lignes par fichier**, visé 300. Un fichier = une responsabilité. Jamais de fichier fourre-tout (`lib.rs` ne reçoit plus que le câblage des commandes ; la logique va dans son module).
- Zéro `unwrap()`/`expect()` hors tests, zéro `any`, zéro code mort, zéro `try/catch` vide. Toute entrée externe est validée (octets USB, JSON de layout, chemins de fichiers, taille d'image).
- Un protocole matériel = **un fichier driver** + des **tests d'octets** (trames attendues construites à la main d'après la doc/capture, pas d'après le code testé).
- Aucun protocole inventé. Chaque driver cite sa source (doc publique, capture, analyse REA) en tête de fichier ; un champ inconnu reste marqué `// INCONNU` et n'est jamais deviné.
- Licences : on réimplémente d'après la **documentation du protocole**, jamais de code GPL copié (PureRGB = MIT). Pas d'assets (thèmes, polices, images) copiés d'un logiciel constructeur.
- Version : incrémenter à chaque modif (`package.json`, `Cargo.toml`, `tauri.conf.json`) + `CHANGELOG.md`.
- Commits atomiques, messages impératifs en anglais. Avant chaque release : `cargo test`, `npm run build`, `npm ci`.
- Sécurité : exécutable élevé → binaires gérés uniquement depuis `%ProgramFiles%\PureRGB\`, téléchargements épinglés SHA-256 + URL en liste blanche (voir `trust.rs`).

## Module Écrans (mini-écrans LCD des boîtiers/AIO/ventirads)

Spec : `docs/superpowers/specs/2026-10-21-screens-design.md`. Arborescence Rust : `src-tauri/src/screens/` (`driver.rs` contrat, `drivers/` un fichier par protocole, `render/` moteur de rendu, `layout.rs` modèle JSON, `bindings.rs` sources de données, `media.rs` image/GIF/vidéo, `designs/` bibliothèque embarquée). Front : `src/components/screens/`.

Principes : le rendu est fait **côté Rust** (même moteur pour l'aperçu de l'éditeur et pour l'écran → ce qu'on voit est ce qu'on envoie, et ça tourne fenêtre fermée dans le tray). Les drivers ne connaissent que `Frame` (RGBA) → leur format de fil. Un écran simulé (`drivers/sim.rs`) permet de tout tester sans matériel.

## Tests

- `cargo test --manifest-path src-tauri/Cargo.toml` ; `npm run build` (vue-tsc + vite) ; `npm test`.
- Tester pour de vrai (résultat observé) : rendu PNG d'un design vérifié à l'œil, trame USB comparée octet par octet, jamais « ça compile donc ça marche ». Dire clairement ce qui n'est PAS testé faute de matériel.
