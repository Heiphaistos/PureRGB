# PureRGB — Alerte compatibilité RGB — Design

Sous-projet 3/4 de la refonte (suite [[2026-07-20-ui-redesign-design]], [[2026-07-20-effects-library-design]]).

## Recalibrage vs. demande initiale

Momo demandait une "matrice de compatibilité mode couleur × type périphérique" (ex. static rouge incompatible avec tel ventilo). Investigation du code : **cette matrice n'existe pas dans le matériel/protocole** — les effets logiciels écrivent des couleurs LED brutes, n'importe quel mode marche sur n'importe quel type d'appareil qui a des LED pilotables. La seule vraie incompatibilité, déjà présente dans le backend :

- `src-tauri/src/backends/mobo.rs:79-93` — ventilateurs carte mère : `led_count: 0`, `zones: []`, `set_colors` renvoie explicitement `Err("backend ventilation uniquement — RGB via OpenRGB")`. Pilotage PWM/vitesse uniquement, pas de LED.
- `src-tauri/src/backends/liquidctl/mod.rs:441-464` — AIO/pompes/hubs liquidctl : même schéma (`led_count: 0`, `zones: []`), RGB (si présent) détecté séparément par OpenRGB.
- **Bug trouvé** : `apply_effect` (`lib.rs:189`, cas global sans zone) n'a aucune garde sur `led_count==0`, contrairement à `apply_effect_all` (`lib.rs:346`, filtre déjà `d.led_count > 0`). Un effet appliqué sur un appareil sans LED est accepté silencieusement, sauvegardé dans `settings.json`, et ne s'affichera jamais nulle part — échec silencieux permanent.

Validé avec Momo : avertir sur la vraie incompatibilité (pas de LED pilotable), pas inventer une matrice fictive.

## Portée

- `src-tauri/src/lib.rs` — garde ajoutée dans `apply_effect` (cas global) : `led_count == 0` → `Err` explicite, même pattern que la garde zone existante.
- `src/types.ts` — helper `deviceHasRgb(d: DeviceInfo): boolean` = `d.led_count > 0 || d.zones.length > 0`.
- `src/components/DeviceCard.vue` — badge "Pas de RGB" (pill discret) si `!deviceHasRgb(device)`, visible en permanence sur la carte (toutes dispositions : grid/list/canvas/compact).
- `src/App.vue` — `onApplyEffect` vérifie `deviceHasRgb` avant d'invoquer `apply_effect` ; toast clair si bloqué, au lieu de laisser l'erreur backend brute remonter.

## Cas limites

- Zone hub/mobo redimensionnable mais actuellement à 0 LED (`leds_min !== leds_max`, différent du cas "pas de zones du tout") : déjà couvert par la bannière existante dans l'onglet Éclairage — pas de double avertissement, le badge "Pas de RGB" ne s'applique qu'aux appareils sans zones du tout (mobo/liquidctl PWM-only).
- Mode matériel natif (`ModeInfo`) : uniquement disponible pour appareils OpenRGB avec LED réelles (`modes` toujours vide côté mobo/liquidctl) — aucune garde supplémentaire nécessaire.

## Tests

Backend : vérifier que `apply_effect` sur un device `led_count==0` retourne une erreur (test unitaire Rust si pattern existant le permet, sinon vérification manuelle via `npm run tauri dev` + device mobo réel/factice).
Frontend : build vert, vérification visuelle badge sur device factice sans LED.

## Statut

Approuvé par Momo le 2026-07-20 (recalibrage accepté).
