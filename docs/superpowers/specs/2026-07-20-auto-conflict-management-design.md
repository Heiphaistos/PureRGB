# PureRGB — Gestion automatique des conflits — Design

Sous-projet 4/4 (dernier) de la refonte (suite [[2026-07-20-ui-redesign-design]], [[2026-07-20-effects-library-design]], [[2026-07-20-rgb-compatibility-design]]).

## Contexte technique découvert

`conflicts.rs` a déjà toutes les primitives nécessaires, testées en E2E lors des sessions précédentes :
- `stop_family(key, disable: bool)` : `disable=false` = arrêt réversible (Stop-Service + taskkill, aucune modification permanente) ; `disable=true` = en plus StartupType=Disabled + neutralise l'auto-restart Windows + désactive les tâches planifiées (mode "manuel durci" existant, resté inchangé).
- `restore_family(key, saved_modes)` : redémarre les services. Avec une map vide, relit le `start_mode` actuel du service (jamais touché puisque `disable=false` ne le modifie pas) et le redémarre à l'identique — fonctionne sans aucune donnée sauvegardée.
- Fermer la fenêtre ne quitte PAS l'app (minimise dans le tray, `on_window_event` l'empêche explicitement). Le seul vrai point de sortie est le menu tray "Quitter", qui fait déjà le ménage (arrêt moteur, sensord, OpenRGB, relâche headers PWM) avant `app.exit(0)`.

Conclusion : l'automatisation demandée (kill au lancement, relance à la fermeture) se branche sur `stop_family(_, false)` au démarrage et `restore_family(_, {})` dans le handler tray "Quitter" — aucune nouvelle primitive de contrôle service à écrire, juste l'orchestration.

## Portée

- `src-tauri/src/settings.rs` — nouveau champ `auto_manage_conflicts: bool` (défaut `true` — "actif auto accept" demandé explicitement, pas de confirmation).
- `src-tauri/src/lib.rs` :
  - `AppState` gagne `auto_stopped: Mutex<Vec<String>>` (clés de familles arrêtées automatiquement cette session, en mémoire seulement — pas besoin de survivre à un crash, la fermeture normale passe toujours par le handler tray).
  - Thread `hw-init` (démarrage) : si `auto_manage_conflicts`, scanne les conflits, `stop_family(key, false)` pour chaque famille `active`, avant le scan matériel (libère les handles HID pour OpenRGB) ; enregistre les clés dans `auto_stopped`.
  - Handler tray "Quitter" : avant `app.exit(0)`, si `auto_manage_conflicts`, `restore_family(key, {})` best-effort pour chaque famille dans `auto_stopped` (log seulement en cas d'échec, ne bloque jamais la fermeture).
- `src/components/SettingsPanel.vue` — case à cocher "Gérer automatiquement les conflits" dans la carte existante appropriée (pas de nouvelle carte, réutilise le pattern `.inline` existant).
- `src/types.ts` — `Settings.auto_manage_conflicts: boolean`.

## Cas limites

- Ne touche jamais les familles déjà en mode "désactivé" manuellement par l'utilisateur (`disabled_services` non vide pour cette famille) — pas de double gestion, l'auto-stop passe uniquement par les familles `active` au scan, indépendamment de leur état "disabled".
- Auto-stop échoue pour une famille (ex. droits insuffisants) : log, continue les autres, n'empêche pas le démarrage de l'app.
- Auto-restore échoue à la fermeture : log, l'app quitte quand même (jamais bloquant).
- Toggle "Garde" (opt-in existant, re-tue en boucle) reste indépendant et inchangé — l'auto-stop au lancement n'active pas la garde automatiquement.
- Famille dans `guarded_families` (garde active) : déjà re-tuée en continu par le thread `conflict-guard` existant, l'auto-stop au lancement ne fait rien de plus/différent pour elle.

## Tests

Rust : build/`cargo check` vert. Pas de test unitaire nouveau (orchestration d'I/O système déjà couverte par les tests existants de `conflicts.rs`).
Vérification manuelle : lancer l'app avec un processus factice nommé comme une famille connue tournant, confirmer l'arrêt auto au lancement (log), fermer via tray "Quitter", confirmer la tentative de redémarrage (log).

## Statut

Approuvé implicitement par la demande initiale de Momo ("actif auto accept pour les demandes").
