# PureRGB — Notification hotplug USB — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quand un appareil USB non reconnu apparaît en direct, PureRGB affiche une bannière in-app (+ notification OS best-effort) proposant d'ouvrir le diagnostic matériel déjà existant.

**Architecture:** Un thread `usb-hotplug` (spawné dans `.setup()`, même pattern que `conflict-guard` déjà existant) sonde `HidBackend::list_raw()` toutes les 5s, diffuse via une fonction pure `hotplug::diff_new_unrecognized` les nouveaux appareils non reconnus, émet un événement Tauri + une notification OS. Le frontend écoute l'événement, affiche une bannière avec bouton "Ouvrir le diagnostic" qui bascule vers l'onglet Réglages et déclenche le bouton diagnostic déjà existant.

**Tech Stack:** Rust (thread + `tauri-plugin-notification` v2), Vue3/TypeScript (`@tauri-apps/api/event` + `@tauri-apps/plugin-notification`).

**Spec source:** `docs/superpowers/specs/2026-07-22-usb-hotplug-notification-design.md`

---

### Task 1: Ajouter le plugin notification (backend + frontend + capacités)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs` (enregistrement du plugin)

- [ ] **Step 1: Dépendance Rust**

Dans `src-tauri/Cargo.toml`, après la ligne `tauri-plugin-dialog = "2"` :
```toml
tauri-plugin-dialog = "2"
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Dépendance JS**

Run: `npm install @tauri-apps/plugin-notification@^2`

Expected : `package.json` gagne `"@tauri-apps/plugin-notification": "^2.x.x"` dans `dependencies`.

- [ ] **Step 3: Capacité**

Dans `src-tauri/capabilities/default.json`, le tableau `permissions` actuel :
```json
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "core:tray:default",
    "opener:default",
    "dialog:default"
  ]
```
devient :
```json
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "core:tray:default",
    "opener:default",
    "dialog:default",
    "notification:default"
  ]
```

- [ ] **Step 4: Enregistrer le plugin dans `lib.rs`**

Trouver la ligne `.plugin(tauri_plugin_dialog::init())` (dans la chaîne `tauri::Builder::default()...`) et ajouter juste après :
```rust
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
```

- [ ] **Step 5: Vérifier**

Run: `cd src-tauri && cargo check`
Expected: succès.

Run: `npm run build`
Expected: succès.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json src-tauri/capabilities/default.json src-tauri/src/lib.rs
git commit -m "feat(hotplug): add tauri-plugin-notification"
```

---

### Task 2: Fonction pure de diff (TDD)

**Files:**
- Create: `src-tauri/src/hotplug.rs`
- Modify: `src-tauri/src/lib.rs:1-9` (déclaration du module)

- [ ] **Step 1: Déclarer le module**

Dans `src-tauri/src/lib.rs`, les déclarations de module actuelles :
```rust
mod backends;
mod conflicts;
mod core;
mod engine;
mod netdev;
mod sensors;
mod settings;
mod telemetry;
mod usbcapture;
```
deviennent (insertion alphabétique) :
```rust
mod backends;
mod conflicts;
mod core;
mod engine;
mod hotplug;
mod netdev;
mod sensors;
mod settings;
mod telemetry;
mod usbcapture;
```

- [ ] **Step 2: Écrire les tests d'abord**

Créer `src-tauri/src/hotplug.rs` :
```rust
//! Détection des nouveaux appareils USB non reconnus, par sondage périodique
//! de `HidBackend::list_raw()` (comparaison d'ensemble entre deux sondages).
//! Le pilotage réel reste inchangé — ce module ne fait que repérer les
//! nouveautés pour proposer un diagnostic, jamais pour piloter quoi que ce soit.

use crate::backends::hid::RawHidDevice;
use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;

    fn device(vid: &str, pid: &str, recognized: bool) -> RawHidDevice {
        RawHidDevice {
            vid: vid.into(),
            pid: pid.into(),
            manufacturer: "Test".into(),
            product: "Device".into(),
            recognized,
            has_native_driver: false,
        }
    }

    #[test]
    fn detecte_un_nouvel_appareil_non_reconnu() {
        let previous = HashSet::new();
        let current = vec![device("dead", "beef", false)];
        let result = diff_new_unrecognized(&previous, &current);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ignore_un_appareil_deja_vu() {
        let mut previous = HashSet::new();
        previous.insert(("dead".to_string(), "beef".to_string()));
        let current = vec![device("dead", "beef", false)];
        let result = diff_new_unrecognized(&previous, &current);
        assert!(result.is_empty());
    }

    #[test]
    fn ignore_un_nouvel_appareil_reconnu() {
        let previous = HashSet::new();
        let current = vec![device("1b1c", "0c0b", true)];
        let result = diff_new_unrecognized(&previous, &current);
        assert!(result.is_empty());
    }

    #[test]
    fn regroupe_plusieurs_nouveaux_appareils() {
        let previous = HashSet::new();
        let current = vec![
            device("dead", "beef", false),
            device("1b1c", "0c0b", true),
            device("cafe", "babe", false),
        ];
        let result = diff_new_unrecognized(&previous, &current);
        assert_eq!(result.len(), 2);
    }
}
```

- [ ] **Step 3: Lancer les tests, vérifier qu'ils échouent**

Run: `cd src-tauri && cargo test --lib hotplug::`
Expected: FAIL — `diff_new_unrecognized` n'existe pas encore.

- [ ] **Step 4: Implémenter**

Insérer avant `#[cfg(test)]` :
```rust
/// Renvoie les appareils de `current` absents de `previous` (par VID/PID) et
/// non reconnus (`recognized == false`) — ceux qui méritent une notification.
pub fn diff_new_unrecognized(
    previous: &HashSet<(String, String)>,
    current: &[RawHidDevice],
) -> Vec<RawHidDevice> {
    current
        .iter()
        .filter(|d| !d.recognized && !previous.contains(&(d.vid.clone(), d.pid.clone())))
        .cloned()
        .collect()
}
```

- [ ] **Step 5: Lancer les tests, vérifier qu'ils passent**

Run: `cd src-tauri && cargo test --lib hotplug::`
Expected: PASS, 4/4 tests verts.

- [ ] **Step 6: Vérifier la compilation globale**

Run: `cd src-tauri && cargo check`
Expected: succès (avertissement "fonction jamais utilisée" acceptable — Task 3 la câble).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/hotplug.rs
git commit -m "feat(hotplug): add diff_new_unrecognized pure detection logic"
```

---

### Task 3: Thread `usb-hotplug`

**Files:**
- Modify: `src-tauri/src/lib.rs` (à l'intérieur de `.setup(|app| { ... })`)

- [ ] **Step 1: Repérer le point d'insertion**

Dans `lib.rs`, le bloc `conflict-guard` existant (à l'intérieur de `.setup(|app| { ... })`) ressemble à :
```rust
            // Garde anti-relance : certains logiciels (Corsair.Service) se
            // relancent seuls malgré service désactivé + tâche neutralisée.
            // Balayage périodique tant que l'app tourne, familles opt-in.
            {
                let app_handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("conflict-guard".into())
                    .spawn(move || loop {
                        std::thread::sleep(std::time::Duration::from_secs(12));
                        let state = app_handle.state::<AppState>();
                        let families: Vec<String> =
                            state.settings.lock().guarded_families.iter().cloned().collect();
                        drop(state);
                        for key in families {
                            if let Err(e) = conflicts::stop_family(&key, false) {
                                log::debug!("garde conflit {key}: {e:#}");
                            }
                        }
                    })?;
            }
```
Ajouter un bloc similaire juste après (avant la ligne `let start_minimized = ...`) :
```rust
            // Sondage périodique des nouveaux appareils USB non reconnus —
            // propose un diagnostic sans attendre que l'utilisateur remarque
            // lui-même qu'un appareil ne fonctionne pas. Premier sondage =
            // référence silencieuse (tout ce qui est déjà branché n'est pas
            // "nouveau"). Voir hotplug::diff_new_unrecognized.
            {
                let app_handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("usb-hotplug".into())
                    .spawn(move || {
                        let mut previous: std::collections::HashSet<(String, String)> =
                            std::collections::HashSet::new();
                        let mut first = true;
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                            let state = app_handle.state::<AppState>();
                            let current: Vec<crate::backends::hid::RawHidDevice> = {
                                let mut reg = state.registry.lock();
                                reg.backends_mut()
                                    .iter_mut()
                                    .find(|b| b.name() == "hid")
                                    .and_then(|b| b.as_any_mut().downcast_mut::<HidBackend>())
                                    .and_then(|hid| hid.list_raw().ok())
                                    .unwrap_or_default()
                            };
                            drop(state);
                            let current_keys: std::collections::HashSet<(String, String)> = current
                                .iter()
                                .map(|d| (d.vid.clone(), d.pid.clone()))
                                .collect();
                            if first {
                                previous = current_keys;
                                first = false;
                                continue;
                            }
                            let new_unrecognized = hotplug::diff_new_unrecognized(&previous, &current);
                            previous = current_keys;
                            if new_unrecognized.is_empty() {
                                continue;
                            }
                            log::info!(
                                "hotplug: {} nouvel(x) appareil(s) non reconnu(s)",
                                new_unrecognized.len()
                            );
                            if let Err(e) = app_handle.emit("unknown-device-detected", &new_unrecognized) {
                                log::debug!("émission événement hotplug: {e:#}");
                            }
                            let body = if new_unrecognized.len() == 1 {
                                format!(
                                    "{} {} ({}:{})",
                                    new_unrecognized[0].manufacturer,
                                    new_unrecognized[0].product,
                                    new_unrecognized[0].vid,
                                    new_unrecognized[0].pid
                                )
                            } else {
                                format!("{} nouveaux appareils non reconnus", new_unrecognized.len())
                            };
                            use tauri_plugin_notification::NotificationExt;
                            if let Err(e) =
                                app_handle.notification().builder().title("PureRGB").body(body).show()
                            {
                                log::debug!("notification OS hotplug: {e:#}");
                            }
                        }
                    })?;
            }
```

- [ ] **Step 2: Import nécessaire**

Vérifier que `tauri::Emitter` est importé (nécessaire pour `.emit(...)` sur `AppHandle` en Tauri v2). Dans la liste d'imports en tête de `lib.rs` :
```rust
use tauri::{Manager, State};
```
devient :
```rust
use tauri::{Emitter, Manager, State};
```

- [ ] **Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: succès. Si erreur sur `NotificationExt`/`.notification()` introuvable, vérifier que `tauri_plugin_notification::init()` est bien enregistré (Task 1, Step 4) et que la dépendance est bien listée dans `Cargo.toml` (Task 1, Step 1).

- [ ] **Step 4: Build complet**

Run: `cd src-tauri && cargo build`
Expected: succès.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(hotplug): spawn usb-hotplug thread emitting event + OS notification"
```

---

### Task 4: Frontend — écoute de l'événement + bannière

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Imports**

En tête du `<script setup>`, la ligne :
```ts
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
```
devient :
```ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { computed, onMounted, ref } from "vue";
```

- [ ] **Step 2: Type + état réactif**

Juste après le bloc `type TabId = ...` (avant la déclaration `const TABS`), ajouter :
```ts
interface UnknownDeviceAlert {
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
}
```

Après la ligne existante `const toast = ref("");`, ajouter :
```ts
const pendingAlerts = ref<UnknownDeviceAlert[]>([]);
const diagnosticTrigger = ref(0);

function openDiagnosticFor(index: number) {
  pendingAlerts.value.splice(index, 1);
  tab.value = "settings";
  diagnosticTrigger.value++;
}

function dismissAlert(index: number) {
  pendingAlerts.value.splice(index, 1);
}
```

- [ ] **Step 3: Écoute de l'événement + demande de permission au montage**

Le bloc `onMounted` actuel :
```ts
onMounted(async () => {
  await loadSettings();
  await refresh();
  // L'init matériel en arrière-plan peut finir après le premier rendu :
  // re-scanner tant que le serveur n'est pas joignable (max ~30 s).
  for (let i = 0; i < 6 && !orgb.value.server_reachable; i++) {
    await new Promise((r) => setTimeout(r, 5000));
    await refresh();
  }
});
```
devient :
```ts
onMounted(async () => {
  await loadSettings();
  await refresh();
  // L'init matériel en arrière-plan peut finir après le premier rendu :
  // re-scanner tant que le serveur n'est pas joignable (max ~30 s).
  for (let i = 0; i < 6 && !orgb.value.server_reachable; i++) {
    await new Promise((r) => setTimeout(r, 5000));
    await refresh();
  }

  // Notification OS best-effort — l'événement in-app ci-dessous reste le
  // canal garanti même si la permission est refusée ou jamais accordée.
  try {
    if (!(await isPermissionGranted())) {
      await requestPermission();
    }
  } catch {
    /* plateforme sans notifications ou permission indisponible — ignoré */
  }

  await listen<UnknownDeviceAlert[]>("unknown-device-detected", (event) => {
    pendingAlerts.value.push(...event.payload);
  });
});
```

- [ ] **Step 4: Bannière dans le template**

Le bloc existant :
```html
    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>
  </div>
</template>
```
devient :
```html
    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>

    <div v-if="pendingAlerts.length" class="hotplug-alerts">
      <div v-for="(alert, i) in pendingAlerts" :key="`${alert.vid}:${alert.pid}`" class="hotplug-alert">
        <span
          >Nouveau matériel non reconnu détecté : {{ alert.manufacturer }} {{ alert.product }} ({{ alert.vid }}:{{
            alert.pid
          }})</span
        >
        <div class="hotplug-alert-actions">
          <button @click="openDiagnosticFor(i)">Ouvrir le diagnostic</button>
          <button @click="dismissAlert(i)">Ignorer</button>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 5: Passer `diagnosticTrigger` à `SettingsPanel`**

Le bloc existant :
```html
      <SettingsPanel
        v-else
        :settings="settings"
        :layout="layoutMode"
        @saved="loadSettings(); refresh(); showToast('Réglages enregistrés')"
        @layout-change="setLayout"
      />
```
devient :
```html
      <SettingsPanel
        v-else
        :settings="settings"
        :layout="layoutMode"
        :diagnostic-trigger="diagnosticTrigger"
        @saved="loadSettings(); refresh(); showToast('Réglages enregistrés')"
        @layout-change="setLayout"
      />
```

- [ ] **Step 6: Style minimal pour la bannière**

Dans le bloc `<style scoped>` existant, ajouter à la fin (avant la fermeture `</style>`) :
```css
.hotplug-alerts {
  position: fixed;
  bottom: 16px;
  right: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 50;
  max-width: 360px;
}
.hotplug-alert {
  background: #1c1c1c;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 12px;
  color: #eee;
  font-size: 0.9em;
}
.hotplug-alert-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
```

- [ ] **Step 7: Vérifier**

Run: `npm run build`
Expected: succès (typecheck + build Vite).

- [ ] **Step 8: Commit**

```bash
git add src/App.vue
git commit -m "feat(hotplug): listen for unknown-device-detected event, show actionable banner"
```

---

### Task 5: `SettingsPanel.vue` — déclenchement auto du diagnostic

**Files:**
- Modify: `src/components/SettingsPanel.vue`

- [ ] **Step 1: Nouvelle prop**

La ligne actuelle :
```ts
const props = defineProps<{ settings: Settings | null; layout: LayoutMode }>();
```
devient :
```ts
const props = defineProps<{
  settings: Settings | null;
  layout: LayoutMode;
  diagnosticTrigger?: number;
}>();
```

- [ ] **Step 2: Watcher**

Juste après la fonction `runDiagnostics` existante (après son accolade fermante), ajouter :
```ts
watch(
  () => props.diagnosticTrigger,
  (v, old) => {
    if (v !== undefined && v !== old) runDiagnostics();
  },
);
```
(`watch` est déjà importé de `vue` en tête du fichier — `import { reactive, ref, watch } from "vue";` — rien à ajouter côté imports.)

- [ ] **Step 3: Vérifier**

Run: `npm run build`
Expected: succès.

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsPanel.vue
git commit -m "feat(settings): auto-run diagnostic when diagnosticTrigger prop changes"
```

---

### Task 6: Vérification manuelle + bump de version

**Files:** aucun changement de code — `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` pour le bump.

- [ ] **Step 1: Build complet**

Run: `cd src-tauri && cargo build` puis `npm run build`
Expected: les deux verts.

- [ ] **Step 2: Vérification manuelle (non exécutable ici sans session GUI + matériel réel)**

À faire par Momo : lancer `npm run tauri dev`, brancher un appareil USB dont le VID/PID n'est dans aucune table connue (ou débrancher/rebrancher un clavier générique), attendre jusqu'à 5s, confirmer :
1. La bannière apparaît en bas à droite avec le bon VID/PID.
2. Une notification Windows apparaît (si la permission a été accordée au premier lancement — sinon vérifier qu'aucune erreur ne bloque le reste, l'événement in-app doit fonctionner quand même).
3. Cliquer "Ouvrir le diagnostic" bascule vers l'onglet Réglages ET lance automatiquement le diagnostic (le panneau affiche un résultat sans clic supplémentaire).
4. Cliquer "Ignorer" fait disparaître la bannière sans naviguer.

- [ ] **Step 3: Bump 0.16.0 → 0.17.0**

`package.json` : `"version": "0.17.0",`
`src-tauri/Cargo.toml` : `version = "0.17.0"`
`src-tauri/tauri.conf.json` : `"version": "0.17.0",`

- [ ] **Step 4: Vérifier**

Run: `cd src-tauri && cargo check` puis `npm run build`
Expected: les deux verts.

- [ ] **Step 5: Commit**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "chore: bump version to 0.17.0"
```
