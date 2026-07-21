# PureRGB — Notification hotplug USB — Design

## Contexte

Sous-projet 3/5 (voir [[2026-07-22-openrgb-auto-update-design]] pour le 1/5 et le repo PureRGB-Telemetry pour le 2/5, tous deux livrés). Objectif : "notification hotplug USB (détecter branchement en direct, proposer diagnostic)".

**Note d'autonomie** (mandat "fait tout le reste") : décisions de conception prises en autonomie, documentées ici pour audit, sans aller-retour de clarification.

## Fait vérifié dans le code existant

- `HidBackend::list_raw()` (`src-tauri/src/backends/hid/mod.rs:81`) énumère déjà tous les appareils HID via `hidapi::device_list()` et calcule déjà un champ `recognized: bool` par appareil (vrai si présent dans `KNOWN_DEVICES`/`KNOWN_VENDORS` locaux OU dans le registre distant `known_remote` fetché du dashboard télémétrie). C'est exactement la primitive nécessaire — **aucune nouvelle logique de reconnaissance à écrire**, seulement une boucle de sondage qui diffuse ce qui change.
- Le protocole OpenRGB a son propre paquet `DeviceListUpdated` (`RGBCONTROLLER`... `mod.rs:196` du backend openrgb) mais il n'est utilisé aujourd'hui que pour être ignoré pendant une lecture synchrone (`recv_expect`) — l'exploiter demanderait un client TCP asynchrone séparé, bien plus complexe qu'un sondage HID périodique, et ne couvrirait de toute façon que les appareils qu'OpenRGB gère déjà (donc jamais les VID/PID vraiment inconnus, exactement le cas qu'on veut détecter). **Décision : sondage HID périodique, pas le canal de notification OpenRGB.**
- Aucun plugin notification/événement Tauri n'est utilisé nulle part dans le code actuel (`grep` vide sur `app.emit`/`listen(` côté JS, aucun `tauri-plugin-notification` en dépendance) — ce sous-projet introduit le premier usage du système d'événements Tauri dans cette app. Pattern raisonnable : les plugins `dialog`/`opener`/`single-instance` existants suivent déjà exactement ce schéma d'ajout (Cargo.toml + capabilities/default.json + package.json JS binding).
- Panneau diagnostic existant : `SettingsPanel.vue` a une section "Diagnostic matériel" (ligne ~384) avec un bouton "Lancer le diagnostic" (`runDiagnostics()`, invoke `hardware_diagnostics`). `App.vue` gère la navigation via `tab = ref<TabId>("rgb")` (`TabId` inclut `"settings"`). Ces deux éléments donnent le point d'entrée naturel pour "proposer diagnostic" — pas besoin de nouveau composant lourd, juste router vers l'onglet existant et déclencher son bouton.

## Portée

### Backend Rust — nouveau thread `usb-hotplug`

- **Correction importante après lecture du code réel** : `hw-init`/`telemetry-init` sont spawnés tôt dans `run()`, *avant* que Tauri n'ait construit son `AppHandle` (ils n'ont donc pas accès à `app.emit(...)`). Le thread `conflict-guard` existant (`lib.rs:1057-1076`, spawné *à l'intérieur* de `.setup(|app| { ... })`, capture `app.handle().clone()`, boucle avec `sleep` + accède à `app_handle.state::<AppState>()`) est le pattern exact à reproduire. `usb-hotplug` est donc spawné dans `.setup()`, pas dans le corps principal de `run()`.
- Boucle : toutes les 5 secondes, `state.registry.lock()` → backend `hid` → `list_raw()` (même appel que le diagnostic manuel).
- Garde un `HashSet<(String, String)>` en mémoire (vid, pid) des clés déjà vues. **Premier sondage après lancement : établit la base sans notifier** (tout ce qui est déjà branché au démarrage n'est "pas nouveau"). Sondages suivants : toute clé présente maintenant mais absente de l'ensemble précédent = nouvel appareil.
- Parmi les nouveaux appareils d'un même cycle, ne notifier que ceux avec `recognized == false` — un appareil reconnu n'a pas besoin d'attirer l'attention (il fonctionne déjà ou son statut est déjà clair).
- Debounce/lot naturel : tous les nouveaux appareils non reconnus d'un même cycle de 5s sont regroupés dans UNE seule notification (évite le spam si plusieurs appareils apparaissent d'un coup, ex. un hub USB rebranché).
- Émission : `app_handle.emit("unknown-device-detected", payload)` (payload = liste des `{vid, pid, manufacturer, product}` nouveaux) pour le frontend, **et** une notification OS best-effort via `tauri-plugin-notification` (titre "PureRGB", corps résumant le(s) appareil(s)) — best-effort explicite : si la permission notification n'a jamais été accordée, échec silencieux (log), l'événement frontend reste le canal fiable garanti (fonctionne même sans permission OS, tant que l'app est au premier plan ou visible en arrière-plan via le tray).
- Aucune persistance entre lancements (le `HashSet` est réinitialisé à chaque démarrage) — un appareil débranché puis rebranché dans la même session ne re-notifie que s'il a disparu de l'ensemble entre-temps (comportement naturel, pas un objectif explicite mais acceptable).

### Nouvelles dépendances

- `src-tauri/Cargo.toml` : `tauri-plugin-notification = "2"`.
- `package.json` : `@tauri-apps/plugin-notification` (même version majeure que les autres plugins `@tauri-apps/plugin-*` déjà présents).
- `src-tauri/capabilities/default.json` : ajouter `"notification:default"` à la liste `permissions`.
- `src-tauri/src/lib.rs` : enregistrer le plugin (`tauri::Builder::default().plugin(tauri_plugin_notification::init())`, même pattern que `tauri_plugin_dialog::init()` déjà présent).

### Frontend

- `App.vue` : au montage, `listen<UnknownDevicePayload[]>("unknown-device-detected", (event) => { ... })` (import `listen` de `@tauri-apps/api/event`) — pousse dans un tableau réactif `pendingAlerts`. Bannière simple inline dans le template (pas de nouveau fichier composant — la logique est courte, cohérent avec le style existant du fichier) : "Nouveau matériel non reconnu détecté : {manufacturer} {product} ({vid}:{pid})" + bouton "Ouvrir le diagnostic" (bascule `tab.value = 'settings'`, incrémente un compteur `diagnosticTrigger` passé en prop à `SettingsPanel`) + bouton "Ignorer" (retire l'entrée de `pendingAlerts`).
- `SettingsPanel.vue` : nouvelle prop optionnelle `diagnosticTrigger?: number` ; un `watch()` sur cette prop appelle `runDiagnostics()` quand elle change (permet de redéclencher même si le panneau est déjà monté — un simple `onMounted` ne suffirait pas pour les triggers ultérieurs).
- Première utilisation de l'API notification côté frontend : demander la permission une fois au montage de `App.vue` (`isPermissionGranted()` puis `requestPermission()` si nécessaire, pattern standard documenté du plugin) — best-effort, ignoré si refusée (l'événement in-app reste le canal principal).

## Cas limites

- Permission notification OS refusée ou jamais accordée : notification OS silencieusement absente, l'événement Tauri (bannière in-app) fonctionne quand même — c'est le canal garanti, la notification OS est un bonus.
- Beaucoup d'appareils déjà branchés au lancement (ex. clavier+souris+hub) : aucune notification au premier sondage (référence de base), comportement voulu.
- Rafale de branchements simultanés (hub USB) : un seul événement/une seule notification par cycle de 5s, listant tous les nouveaux appareils du cycle.
- Thread `usb-hotplug` ne doit jamais bloquer/ralentir le reste de l'app : sondage HID est déjà utilisé ailleurs (diagnostic manuel) sans latence perceptible signalée ; boucle indépendante, aucun verrou partagé tenu longtemps (lock `registry` seulement le temps de `list_raw()`).
- App fermée/minimisée dans le tray : le thread continue de tourner (déjà le cas pour tous les threads de fond existants), la notification OS reste le seul canal utile dans ce cas (la fenêtre n'étant pas visible, la bannière in-app n'a d'effet que si l'utilisateur rouvre la fenêtre — c'est acceptable, l'événement Tauri est simplement mis en file/perdu si personne n'écoute, sans conséquence négative).

## Tests

- Rust : la logique de diff (base établie au premier cycle, détection de nouvelles clés, filtrage sur `recognized == false`, regroupement par cycle) est extraite en fonction pure testable : `fn diff_new_unrecognized(previous: &HashSet<(String,String)>, current: &[RawHidDevice]) -> Vec<RawHidDevice>` — testable sans hidapi réel (on construit des `RawHidDevice` à la main).
- Le sondage réel (thread, timer 5s, hidapi réel) n'est pas testable unitairement — vérifié manuellement (brancher un appareil de test/USB inconnu, confirmer bannière + notification).
- Frontend : pas de suite de tests JS existante dans ce repo (vérifié — aucun fichier `*.test.ts`/`*.spec.ts` sous `src/`) ; vérification manuelle uniquement, cohérent avec le reste du frontend de cette app.

## Statut

Conception validée en autonomie, prête pour plan d'implémentation.
