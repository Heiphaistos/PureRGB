# PureRGB — Bibliothèque d'effets étendue — Design

Sous-projet 2/4 de la refonte demandée par Momo (suite à [[2026-07-20-ui-redesign-design]]).

## Portée

100% frontend, zéro changement backend Rust. Les 9 primitives d'effet (`off/static/breathing/rainbow_cycle/rainbow_wave/color_wave/comet/blink/gradient`, `types.ts`) restent inchangées — les thèmes ne sont que des combinaisons pré-configurées de ces primitives, appliquées à tous les appareils via `apply_effect_all`.

## Fichiers

- `src/themes.ts` — `Theme` gagne un champ `category: ThemeCategory`. 24 thèmes existants recatégorisés (aucune config couleur modifiée), 36 nouveaux ajoutés → 60 total.
- `src/components/ThemePanel.vue` — chips de filtre catégorie au-dessus de la grille, "Tous" par défaut, sélection persistée en mémoire locale (pas besoin de backend).

## Catégories (8)

Néon/Cyberpunk, Nature, Rétro/Synthwave, Fêtes/Saisonnier, Gaming/Compétitif, Pastel/Doux, Sobre/Pro, Effets dynamiques.

## Cas limites

- Filtre "Tous" toujours présent en premier, compte affiché par chip.
- Recatégorisation des 24 existants n'altère aucune `config` (couleurs/vitesse/luminosité identiques) — seul le tri/groupement change.

## Tests

Aucun test backend nécessaire. Vérification visuelle : `npm run tauri dev`, onglet Thèmes, filtre par catégorie fonctionne, 60 thèmes présents, un thème par catégorie appliqué réellement (toast + `apply_effect_all` retourne N appareils).

## Statut

Approuvé par Momo le 2026-07-20.
