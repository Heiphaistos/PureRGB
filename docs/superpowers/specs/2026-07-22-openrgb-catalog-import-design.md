# PureRGB-Telemetry — Import du catalogue OpenRGB — Design

## Contexte

Sous-projet 2/5 (voir [[2026-07-22-openrgb-auto-update-design]] pour le 1/5, déjà livré). Objectif reformulé par Momo : "scraper openrgb.org/devices.html pour enrichir known.rs/dashboard télémétrie automatiquement".

**Note d'autonomie** : Momo a explicitement demandé de faire les 4 sous-projets restants sans repasser par un aller-retour de clarification à chaque décision ("fait tout le reste"). Ce document documente les décisions de conception prises de façon autonome (avec justification), au lieu du format habituel questions-une-à-une, pour rester auditable sans bloquer sur des échanges.

## Fait technique vérifié (pas supposé)

- `openrgb.org/devices.html` est une page JS (DataTables) qui charge ses données depuis `https://openrgb.org/data/supported_devices.csv` — un CSV statique directement récupérable, **pas besoin de scraper/rendre le HTML**. Vérifié par fetch direct : 2538 lignes, colonnes `Name,Category,Type,RGBController,VID,PID,SVID,SPID,Save,Direct,Effects,Comments`.
- Les lignes `Type=USB` portent un VID/PID hexadécimal réel (ex. `"Bloody B820R","Keyboard","USB","A4Tech Bloody B820R","09DA","FA10",...`). Les lignes SMBus/I2C (cartes mères, RAM) ont VID/PID vides — normal, pas des périphériques USB.
- Le champ `Category` est parfois multi-valeurs séparées par retour à la ligne (ex. `"RAM\nMotherboard\nGPU\nStorage"` pour un contrôleur SMBus générique ENE) — concerne surtout les lignes non-USB, donc peu d'impact sur les lignes qu'on importe réellement.
- Le champ `Name` est parfois vide (ex. Philips amBX) — `RGBController` sert de nom de repli.
- Le mécanisme `known_devices` (table SQLite du dashboard télémétrie) + `known_remote.rs` (PureRGB, déjà existant) est **exactement** la plomberie qu'il faut : PureRGB fetch déjà `GET /known-devices` et fusionne ces entrées pour l'étiquetage "reconnu" du panneau diagnostic (jamais pour le pilotage réel — voir doc-comment de `known_remote.rs`). Enrichir cette table côté serveur suffit : **aucun changement côté app PureRGB (Rust) n'est nécessaire** pour ce sous-projet.
- `DEVICE_TYPE_VALUES` (dashboard, `deviceTypes.ts`) est déjà volontairement large et aligné sur l'enum Rust `DeviceType` (`core/mod.rs`) — son commentaire dit explicitement "le but est de pouvoir cataloguer tout ce qui a un VID/PID, pas seulement ce qu'OpenRGB pilote aujourd'hui", ce qui correspond exactement à l'usage visé ici.
- `better-sqlite3` + WAL déjà configurés (`db/index.ts`). Migration additive de colonne déjà pratiquée pour `serial` (`ALTER TABLE ... ADD COLUMN` dans un `try/catch`) — même pattern à réutiliser.
- Aucune dépendance CSV existante dans `package.json` — un parseur CSV correct (gestion des guillemets/retours à la ligne intégrés) est nécessaire, pas un `split(',')` naïf qui casserait sur les commentaires multi-lignes.

## Décision : valeur ajoutée réelle (pourquoi ce n'est pas redondant avec OpenRGB)

Si OpenRGB détecte déjà un appareil via son propre scan SDK, `known_devices` ne sert à rien pour cet appareil-là (il apparaît déjà nommé correctement dans l'UI normale). La valeur ajoutée de cet import concerne les cas où :
1. L'app tourne avec une version d'OpenRGB embarquée pas encore à jour (avant le sous-projet 1, ou entre deux vérifications) et un appareil du catalogue officiel n'est pas encore détecté par la version locale — le panneau diagnostic peut alors dire "modèle reconnu par OpenRGB (version plus récente), pas encore piloté par la vôtre" au lieu de "totalement inconnu".
2. Un appareil apparaît en HID brut (non piloté) mais correspond à un modèle du catalogue officiel — signal utile pour distinguer "OpenRGB sait faire, quelque chose bloque ici" de "vraiment jamais vu, capture USB utile" (sous-projet USB capture déjà livré plus tôt cette session).

## Portée

### `PureRGB-Telemetry/src/db/schema.sql` + migration

- Nouvelle colonne `source TEXT NOT NULL DEFAULT 'manual'` sur `known_devices`, migration additive dans `bootstrapDb()` (même pattern try/catch que `serial`).
- Valeurs possibles : `'manual'` (ajouté via le formulaire dashboard existant) ou `'openrgb_official'` (importé depuis le CSV). Sert uniquement à protéger les entrées manuelles d'un écrasement lors d'un ré-import — jamais exposé par l'API publique `GET /known-devices`.

### `PureRGB-Telemetry/src/routes/knownDevices.ts`

- La route `POST /` existante (ajout manuel) est modifiée pour écrire explicitement `source = 'manual'` dans son upsert (insertion et mise à jour) — garantit qu'un ajout humain n'est jamais considéré comme importé.
- Nouvelle route `POST /import-openrgb` (même middleware `requireDashboardAuth`, déclenchement manuel depuis le dashboard — **pas de cron**, cohérent avec la préférence déjà connue de Momo pour les vérifications à la demande plutôt qu'automatisées) :
  1. `fetch('https://openrgb.org/data/supported_devices.csv')` (fetch natif Node, déjà disponible, pas de nouvelle dépendance HTTP).
  2. Parse via `csv-parse` (nouvelle dépendance, `parse(text, { columns: true })`) — gère guillemets et retours à la ligne internes correctement, contrairement à un split naïf.
  3. Filtre les lignes `Type === 'USB'` avec VID et PID non vides et VID/PID de la forme exacte 4 caractères hexadécimaux (regex `^[0-9a-f]{4}$` insensible à la casse, VID/PID toujours en majuscules dans le CSV — normalisés en minuscules avant stockage, cohérent avec le schéma existant).
  4. Nom : colonne `Name`, repli sur `RGBController` si `Name` est vide.
  5. `device_type` : première valeur de `Category` (avant tout retour à la ligne), passée dans une table de correspondance CSV→`DEVICE_TYPE_VALUES` (`GPU→gpu, Motherboard→motherboard, RAM→dram, Mouse→mouse, Keyboard→keyboard, Mousemat→mousemat, Cooler→cooler, LEDStrip→led_strip, Headset→headset, Gamepad→gamepad, Accessory→accessory, Microphone→microphone, Speaker→speaker, Storage→storage, Case→case`), repli sur `unknown` si absent de la table.
  6. Upsert dans une transaction unique (`db.transaction(...)`, perf sur ~1500-2000 lignes) : `ON CONFLICT(vid, pid) DO UPDATE SET ... WHERE known_devices.source != 'manual'` — protège les entrées manuelles, mais rafraîchit les entrées `openrgb_official` d'un import précédent (nom/catégorie peuvent changer d'une release OpenRGB à l'autre).
  7. Redirige vers `/dashboard?import=ok&inserted=N&updated=N&skipped=N` en cas de succès (N = compteurs réels), `/dashboard?import=error` en cas d'échec réseau/parsing (loggé côté serveur avec `console.error`, jamais de détail d'erreur exposé au client — cohérent avec la règle "pas de stack trace").
- Logique d'import extraite dans une fonction pure testable séparée du handler HTTP : `importOpenRgbCsv(csvText: string, db: Database): { inserted: number; updated: number; skipped: number }` — le handler fait juste `fetch` + appelle cette fonction, permettant des tests unitaires sans mock réseau (on passe directement un texte CSV de test).

### Dashboard (`views.ts` / `dashboard.ts`)

- Un bouton "Importer depuis OpenRGB" (formulaire POST vers `/known-devices/import-openrgb`) ajouté sur la page dashboard, à côté du bouton déconnexion existant.
- Si `?import=ok&inserted=N&updated=N&skipped=N` présent dans l'URL, afficher un message "Import terminé : N ajoutés, N mis à jour, N ignorés (VID/PID invalides)". Si `?import=error`, afficher "Échec de l'import (voir logs serveur)".

### `PureRGB-Telemetry/package.json`

- Ajout de la dépendance `csv-parse` (version stable récente, à vérifier au moment de l'implémentation).

## Cas limites

- CSV injoignable (réseau, site down) : `fetch` échoue, catch, redirige avec `?import=error`, log serveur — jamais bloquant, jamais de crash.
- Ligne CSV avec VID/PID malformé (pas 4 hex chars) : comptée dans `skipped`, ignorée silencieusement (pas un cas d'erreur, juste une ligne non exploitable — SVID/SPID ne sont jamais utilisés, hors scope).
- Catégorie CSV inconnue de la table de correspondance : `device_type = 'unknown'`, jamais un échec.
- VID/PID déjà présent avec `source='manual'` : jamais écrasé par l'import (condition `WHERE ... != 'manual'` dans le `ON CONFLICT`).
- Ré-import répété (plusieurs clics) : idempotent, les lignes `openrgb_official` déjà à jour comptent comme `updated` (SQLite ne distingue pas "valeur identique" de "valeur changée" dans un `ON CONFLICT DO UPDATE` — compteur `updated` est donc une approximation "lignes qui ont matché un conflit", pas strictement "lignes dont une valeur a changé" ; acceptable, ce n'est qu'un résumé informatif affiché à l'admin, pas une donnée consommée ailleurs).

## Tests

- Unitaire (Node `--test`, comme l'existant) : `importOpenRgbCsv()` avec un texte CSV de test en dur (quelques lignes USB valides, une ligne SMBus sans VID/PID à ignorer, une ligne avec VID/PID malformé, une ligne `Name` vide utilisant `RGBController`, une catégorie multi-valeurs, une catégorie inconnue) → vérifie les compteurs et le contenu réel de `known_devices` après exécution sur une DB de test (`resetDbForTests`, pattern déjà utilisé par `knownDevices.test.ts`).
- Unitaire : protection `source='manual'` — insérer une entrée manuelle, lancer l'import avec un CSV contenant le même VID/PID sous un autre nom, vérifier que le nom manuel n'a pas changé.
- Le fetch réseau réel n'est pas testé unitairement (dépend du site OpenRGB) — vérifié manuellement en lançant l'import depuis le dashboard une fois déployé.

## Statut

Conception validée en autonomie (mandat "fait tout le reste"), prête pour plan d'implémentation.
