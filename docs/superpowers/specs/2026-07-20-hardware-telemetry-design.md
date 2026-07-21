# PureRGB — Télémétrie matériel opt-in — Design

## Contexte

Objectif de Momo : élargir la reconnaissance matérielle de PureRGB (LEDs, cartes mères, ventilateurs, ventirads, AIO, hubs, bandes LED, marques diverses dont certaines absentes du support OpenRGB — be quiet!, Noctua, Thermalright, Uphere, CoolMoon, Acer, Empire Gaming, Samsung, XFX). Vérification faite contre le code source OpenRGB (dossier `Controllers/`, 900+ répertoires) : la plupart des marques citées sont déjà supportées (MSI, ASUS, ASRock, Cooler Master, Corsair, Cougar, Thrustmaster, LG, PNY) car OpenRGB fait le vrai travail de pilotage, indépendamment de la table locale `known.rs` de PureRGB (qui ne sert qu'au panneau diagnostic + 2 drivers natifs maison).

Pour les marques absentes, deux cas : matériel branché sur un header ARGB carte mère (déjà piloté via le contrôleur de la carte mère, aucun gap réel) ou puce USB/SMBus propriétaire non reconnue (vrai gap, mais impossible à combler sans VID/PID + protocole d'un appareil réel). Écrire du code de protocole USB inventé sur du matériel non identifié est refusé (risque de mauvais comportement matériel, aucune garantie de fonctionnement).

Solution retenue : au lieu de collecter les VID/PID un par un manuellement, ajouter une télémétrie opt-in qui remonte le diagnostic matériel complet (déjà existant côté app, `HardwareDiagnostics`) vers un service VPS, avec un dashboard permettant de trier par fréquence et d'ajouter au catalogue de reconnaissance en un clic, propagé immédiatement à toutes les instances sans nouvelle release.

## Portée

### 1. App PureRGB (Tauri/Rust + Vue)

**Réglages** (`src-tauri/src/settings.rs`) :
- `telemetry_opt_in: bool` (défaut `false` — jamais silencieux).
- `telemetry_last_hash: Option<String>` — hash SHA-256 du dernier snapshot diagnostic envoyé, pour éviter de renvoyer un rapport identique à chaque lancement.

**UI Réglages** (`SettingsPanel.vue`) — nouvelle carte "Aide au diagnostic matériel" :
- Checkbox opt-in avec texte clair : "Envoyer les informations de diagnostic matériel (VID/PID détectés, état OpenRGB/liquidctl/sensord) pour aider à identifier le matériel non reconnu. Aucune donnée personnelle, désactivé par défaut."
- Pas de nouvelle carte si ça alourdit — réutilise le pattern `.inline` existant, cohérent avec les autres réglages.

**Panneau Diagnostic** (déjà existant, composant qui appelle `hardware_diagnostics`) :
- Bouton "Envoyer maintenant" (actif seulement si opt-in coché) — envoi manuel immédiat, indépendant du hash (permet de forcer un renvoi).

**Envoi automatique** :
- Au démarrage, si `telemetry_opt_in == true` : calcule le snapshot `HardwareDiagnostics` (déjà généré par la commande existante), hash-le, compare à `telemetry_last_hash` — si différent, POST vers le VPS puis met à jour le hash sauvegardé. Best-effort, timeout court (5s), échec silencieux (log seulement, ne bloque jamais le démarrage).

**Payload** (`POST /report`) :
```json
{
  "report_id": "<uuid v4 généré une fois, stocké dans settings.json>",
  "app_version": "0.13.0",
  "diagnostics": { /* struct HardwareDiagnostics sérialisée telle quelle */ }
}
```
`report_id` est un UUID local aléatoire, pas lié à une empreinte machine — sert uniquement à dédupliquer/regrouper les rapports successifs d'une même installation côté dashboard (compteur "vu N fois"), pas à identifier une personne.

**Table de reconnaissance dynamique** (`known.rs`) :
- Au démarrage, `GET /known-devices` (timeout 3s) sur le VPS. Succès → fusionne avec `KNOWN_DEVICES`/`KNOWN_VENDORS` compilés (le distant prime sur les VID/PID en commun), écrit en cache `%APPDATA%\PureRGB\known_devices_cache.json`. Échec/hors-ligne → utilise le cache local, sinon la table compilée seule. Aucune dépendance réseau bloquante.
- Cette fusion n'affecte que le panneau diagnostic + l'éligibilité aux 2 drivers natifs existants (Corsair Lighting Node, NZXT Hue2) — cohérent avec ce que `known.rs` fait déjà aujourd'hui. Ne prétend pas ajouter de pilotage réel pour du matériel qu'OpenRGB ne supporte pas.

### 2. Service VPS (nouveau)

Pattern identique aux micro-services existants (ForgeHook, PureRSS) : Hono + SQLite, PM2, port **3022** (confirmé libre — `ss -tlnp` VPS vérifié), bind 127.0.0.1, nginx devant, repo privé `Heiphaistos/PureRGB-Telemetry`, sous-domaine `telemetry-purergb.heiphaistos.org`.

**Endpoints** :
- `POST /report` — valide schéma (taille payload plafonnée ~64 Ko, champs attendus), rate-limit par IP (ex. 10/heure — un rapport par lancement d'app, pas un flux continu), stocke en SQLite (`reports` : id, report_id, ip_hash — pour rate-limit seulement, pas affiché —, app_version, diagnostics_json, created_at).
- `GET /known-devices` — public, sert la table courante `known_devices` (vid, pid, name, device_type, vendor) en JSON, lue par toutes les instances au démarrage.
- Dashboard `GET /dashboard` (protégé mot de passe, pattern VPSConnect — gate bcrypt) :
  - Liste les VID/PID non reconnus extraits des `hid_raw` de tous les rapports stockés, agrégés par (vid, pid) avec fréquence, manufacturer/product exemple, dernière vue.
  - Bouton "Ajouter" par ligne → formulaire nom + type d'appareil (`DeviceType` existant : Motherboard/Fan/Aio/Hub/LedStrip/Case/Accessory/…) → écrit dans `known_devices` (upsert par vid/pid) → immédiatement disponible via `GET /known-devices`.
  - Champs texte libres (manufacturer/product) échappés à l'affichage (anti-XSS).

### 3. Sécurité

- `POST /report` sans authentification (l'app doit pouvoir poster sans compte) — compensé par rate-limit IP + validation stricte de schéma + plafond de taille.
- Dashboard derrière mot de passe (bcrypt, cost ≥12, cohérent CLAUDE.md), jamais exposé sans auth.
- IP des rapporteurs hashée en stockage (pas conservée en clair), utilisée seulement pour le rate-limit, jamais affichée au dashboard.
- HTTPS forcé (certbot, cohérent avec le reste de l'infra), headers sécurité standards (CSP, X-Frame-Options).
- `.env` (mot de passe dashboard hashé) → `.gitignore`.

## Cas limites

- VPS injoignable au démarrage : app continue normalement, cache local des `known_devices` réutilisé, envoi diagnostic simplement ignoré ce lancement (retenté au suivant).
- Utilisateur désactive l'opt-in après l'avoir activé : plus aucun envoi automatique ni bouton actif ; ne supprime pas les rapports déjà envoyés côté VPS (hors scope — pas de demande de droit à l'oubli formulée, à ajouter si besoin plus tard).
- Rapport identique à chaque lancement (machine stable) : le hash évite le spam réseau/DB — un seul envoi tant que rien ne change matériellement.
- Ajout dashboard sur un VID/PID déjà connu localement (compilé) : le distant prime, permet de corriger une entrée existante sans nouvelle release.

## Tests

- Rust : `cargo check` + tests unitaires sur le hash/dédup et la fusion `known_devices` distant/compilé (priorité au distant sur collision).
- VPS : tests d'intégration sur `/report` (validation, rate-limit) et `/known-devices` (sérialisation), comme les autres micro-services du repo.
- Vérification manuelle : lancer l'app avec opt-in activé, confirmer le POST reçu côté VPS (log service), ajouter une entrée depuis le dashboard, relancer l'app et confirmer qu'elle apparaît en "reconnu" dans le diagnostic.

## Statut

Implémenté et déployé (v0.14.0). Service VPS `PureRGB-Telemetry` en production sur `telemetry-purergb.heiphaistos.org` (port 3022, nom réel — tiret, pas point comme initialement prévu). Compte admin dashboard créé. Vérifié en conditions réelles : healthcheck HTTPS public vert, endpoint `/known-devices` public répond, build backend (cargo check/test 31/31) et frontend (npm build) verts, `settings.json` réel confirme `telemetry_opt_in: false` par défaut. Chemins de binaires (`exe_path`) rédigés au nom de fichier seul avant tout envoi externe (contenaient le nom de compte Windows — trouvé en revue, corrigé). Vérification manuelle complète (cocher la case dans l'app, cliquer "Envoyer maintenant", voir la ligne apparaître au dashboard, l'ajouter, relancer l'app et confirmer la reconnaissance sans nouvelle release) reste à faire par Momo avec une fenêtre réelle devant lui.
