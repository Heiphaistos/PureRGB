# PureRGB — Auto-update OpenRGB embarqué — Design

## Contexte

Sous-projet 1/5 d'une série d'améliorations détection/compatibilité demandées par Momo (les 4 autres : scraping du catalogue OpenRGB pour enrichir `known.rs`/dashboard télémétrie, notification hotplug USB, exploitation du champ `serial` déjà stocké côté télémétrie, auto-détection zones/LED-count). Chaque sous-projet a sa propre spec.

PureRGB embarque OpenRGB **1.0rc3**, figé au moment du build (`scripts/fetch-openrgb.ps1`, URL+SHA-256 codés en dur). Chaque nouvelle version upstream ajoute des dizaines d'appareils supportés sans qu'aucun code PureRGB ne change — mais aujourd'hui, en profiter demande une nouvelle release PureRGB à chaque fois. Objectif : que l'app se maintienne à jour elle-même.

## Fait technique vérifié (pas supposé)

- API Codeberg (Gitea) confirmée fonctionnelle cette session : `GET https://codeberg.org/api/v1/repos/OpenRGB/OpenRGB/releases?limit=1` retourne un JSON avec `tag_name` (ex. `release_candidate_1.0rc3`) et un tableau `assets[]` (chaque asset a `name` + `browser_download_url`).
- **Aucun fichier de checksums officiel** n'est publié avec les releases OpenRGB (pas de `SHA256SUMS`/`checksums.txt` dans les assets). Contrairement à l'installation actuelle (SHA-256 pinné en dur, vérifié avant extraction), une version auto-détectée à la compilation ne peut pas être vérifiée par hash a priori — seule garantie : HTTPS vers le domaine officiel `codeberg.org`. Décision validée avec Momo : HTTPS seul suffit (même modèle de confiance que les gestionnaires de paquets standards).
- Le nom de l'asset Windows change de hash de commit à chaque release (`OpenRGB_1.0rc3_Windows_64_6fbcf62.zip`, `OpenRGB_1.0rc2_Windows_64_0fca93e.zip`, etc.) — sélection par motif (`OpenRGB_*_Windows_64_*.zip`, exclut le `.msi`), pas par nom exact.
- `OpenRgbManager::locate()` (`src-tauri/src/backends/openrgb/manager.rs`) cherche d'abord `resource_dir` (bundle NSIS, sous Program Files si installé via le setup), puis `%APPDATA%\PureRGB\openrgb` (portable/fallback), puis les emplacements standards. Décision validée : l'auto-update écrase **les deux** premiers emplacements s'ils existent, indépendamment (chacun avec son propre backup/rollback) — sinon un utilisateur setup ne bénéficierait jamais de la mise à jour tant que PureRGB lui-même n'est pas mis à jour.
- `OpenRgbManager::ensure_running(host, port) -> Result<bool>` et `stop()` existent déjà et seront réutilisés pour valider qu'une version fraîchement basculée démarre réellement (port 6742 joignable) avant de committer le changement.

## Portée

### Nouveau module `src-tauri/src/backends/openrgb/updater.rs`

- `check_latest_version() -> Result<(String, String)>` — interroge l'API Codeberg via `crate::netdev::curl` (réutilisé, pas de nouvelle dépendance HTTP), retourne `(tag_name, download_url)` de l'asset Windows 64-bit.
- `update_if_needed(mgr: &OpenRgbManager, saved_version: &Option<String>) -> Option<String>` — orchestration complète :
  1. Compare `tag_name` reçu à `saved_version`. Identique → ne rien faire, retourne `None`.
  2. Télécharge le zip vers `%APPDATA%\PureRGB\openrgb_update_staging\`, extrait.
  3. Pour chaque emplacement existant (`resource_dir` si présent, `%APPDATA%\PureRGB\openrgb`) : renomme le dossier courant en `<nom>_backup`, déplace la version fraîchement extraite à sa place, copie les DLL VC++ runtime (même logique que `fetch-openrgb.ps1`).
  4. Arrête le serveur courant (`mgr.stop()`), relance (`mgr.ensure_running(...)`), attend jusqu'à 10s que le port réponde.
  5. **Succès** (port joignable) → supprime le(s) dossier(s) `_backup`, retourne `Some(tag_name)` (à sauvegarder dans `settings.json`).
  6. **Échec** → restaure le(s) `_backup` à la place de la nouvelle version, log l'incident (`log::warn!`), retourne `None` (garde l'ancienne version enregistrée, réessaiera au prochain lancement).
  7. Nettoie `openrgb_update_staging` dans tous les cas.

### `settings.rs`

- Nouveau champ `pub openrgb_version: Option<String>` (défaut `None` — première exécution après cette mise à jour de PureRGB traite toute version comme "à vérifier").

### `lib.rs` — thread `hw-init`

- **Ordonnancement critique** : le check+bascule doit s'exécuter **avant** le premier `ensure_running` du flux normal (qui démarre le serveur "pour de vrai" pour cette session), et donc avant `scan_with_zone_sizes`/`restore_saved_state`. Si l'update avait lieu après, le scan et la restauration des effets auraient déjà eu lieu contre l'**ancienne** version — les basculer ensuite rendrait ce travail obsolète (nombre de LED, zones, appareils potentiellement différents sous la nouvelle version). Insertion : tout en haut du thread `hw-init`, avant même la logique `auto_manage_conflicts` existante.
- Le démarrage/arrêt que `update_if_needed` effectue en interne (étape 4 de son propre flux, pour valider que la nouvelle version répond sur le port) est une bascule de **validation**, indépendante et antérieure au `ensure_running` "réel" de la suite du thread — ne pas les confondre. Après le retour d'`update_if_needed` (que la mise à jour ait eu lieu ou non), le flux existant continue normalement : `ensure_running` démarre le serveur (version courante, éventuellement fraîchement basculée) pour la session, puis `scan_with_zone_sizes`/`restore_saved_state` s'exécutent contre cette version définitive.
- Si `Some(new_version)` retourné, met à jour `settings.openrgb_version` et sauvegarde (`settings::save`) avant de continuer.
- Comme pour la télémétrie, ce check ne doit **jamais bloquer indéfiniment** le chemin critique de restauration des effets RGB — mais contrairement à la télémétrie (déplacée dans un thread séparé car son travail est indépendant du reste), ce check doit rester **séquentiel et bloquant** dans `hw-init` puisque son résultat (quelle version démarrer) conditionne directement les étapes suivantes du même thread. Un timeout raisonnable (ex. 15s au total pour tout le flux update, API+téléchargement+validation) borne le pire cas.

## Cas limites

- API Codeberg injoignable (réseau, service down) : échec silencieux, log, app démarre normalement avec la version déjà installée — jamais bloquant.
- Espace disque insuffisant pour le téléchargement/backup : échec de l'écriture détecté, rollback vers l'état d'origine (rien n'a été supprimé avant confirmation du succès), log clair.
- Deux mises à jour lancées en parallèle (peu probable, un seul processus PureRGB à la fois par design existant) : hors scope, pas de verrou dédié nécessaire.
- Aucun `resource_dir` ET aucune copie `%APPDATA%` (aucune install OpenRGB gérée par PureRGB détectée, seulement une install standard utilisateur trouvée par `locate()`) : ne rien toucher — l'auto-update ne touche que les copies que PureRGB gère lui-même, jamais une installation OpenRGB indépendante de l'utilisateur.

## Tests

- Rust : tests unitaires sur la sélection d'asset par motif de nom (plusieurs cas : hash de commit variable, présence du `.msi` à exclure), sur la comparaison de version (identique → no-op).
- Le flux réseau/téléchargement/bascule réel n'est pas testable unitairement (dépend d'un vrai serveur OpenRGB et de vrais fichiers) — vérifié manuellement : lancer l'app avec `openrgb_version` délibérément vide/périmé dans `settings.json`, confirmer le téléchargement, la bascule, et le redémarrage réussi du serveur.

## Statut

Design validé section par section avec Momo (auto-update silencieux confirmé, HTTPS seul accepté en absence de checksum officiel, check à chaque lancement, rollback automatique en cas d'échec de démarrage, mise à jour des deux emplacements setup+portable). Prêt pour plan d'implémentation.
