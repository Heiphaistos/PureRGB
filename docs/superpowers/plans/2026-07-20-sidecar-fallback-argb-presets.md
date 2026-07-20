# Sidecar Fallback + ARGB Presets + Fan Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the confirmed root cause of "0 ventilateur / 0 hub détecté" on portable installs (liquidctl/sensord have no download fallback, unlike OpenRGB), add a known-ARGB-model picker so users don't have to count LEDs by hand, and make the Ventilateurs panel explain *why* it's empty instead of a generic message.

**Architecture:** Mirror `OpenRgbManager`'s existing download-verify-extract pattern onto `LiquidctlBackend` and `SensorHub` (own binaries hosted as GitHub release assets, SHA-256 pinned). Add a frontend-only preset table feeding the existing `resize_zone` command (no new Rust surface for that part). Reuse the existing `hardware_diagnostics` command (already returns sensord/liquidctl state) in `FanPanel.vue` instead of adding a redundant command.

**Tech Stack:** Rust (Tauri v2 backend, anyhow, PowerShell subprocess for download), Vue 3 + TypeScript (frontend), GitHub CLI (`gh`) for release hosting.

**Spec:** `docs/superpowers/specs/2026-07-20-sidecar-fallback-argb-presets-design.md`

---

## Data-accuracy note (read before Task 6)

The spec asked for an "ultra complete" known-fan-model table. Fabricating dozens of LED counts from memory would risk shipping wrong numbers — which would reproduce the exact bug this plan fixes (fan stays dark because the configured LED count is wrong), just for a different user. Task 6 seeds the table with models whose LED count is verified against an official product page during implementation (one search pass, URL noted in a code comment per entry). The table is trivially extensible (one object literal per model) — more entries can be added later the same way, each backed by a source.

---

### Task 1: Verify build prerequisites

**Files:** none (verification only)

- [ ] **Step 1: Check Python, .NET SDK, and GitHub CLI are present**

Run: `python --version && dotnet --version && gh --version`
Expected: three version lines, no "command not found". (Already confirmed present on this machine: Python 3.14.6, .NET 8.0.422, gh 2.95.0 — re-run only to catch drift if this plan is executed later/elsewhere.)

- [ ] **Step 2: Check gh is authenticated against the right account**

Run: `gh auth status`
Expected: shows a logged-in account with access to `Heiphaistos/PureRGB` (private repo). If not authenticated, stop and ask the user to run `gh auth login` themselves — do not proceed with Task 3 without confirmed repo write access.

---

### Task 2: Build the sidecar binaries locally

**Files:**
- Produces (not committed, `.gitignore`d like existing sidecar builds): `src-tauri/resources/liquidctl/liquidctl.exe`, `src-tauri/resources/sensord/sensord.exe`

- [ ] **Step 1: Run the existing build script**

Run (from repo root, PowerShell): `powershell -ExecutionPolicy Bypass -File scripts/build-sidecars.ps1`
Expected output ends with:
```
liquidctl.exe pret: <repo>\src-tauri\resources\liquidctl
sensord.exe pret: <repo>\src-tauri\resources\sensord
```

- [ ] **Step 2: Verify both binaries exist**

Run: `powershell -Command "Test-Path src-tauri/resources/liquidctl/liquidctl.exe; Test-Path src-tauri/resources/sensord/sensord.exe"`
Expected: `True` printed twice.

No commit here — these are build artifacts, already covered by the existing `.gitignore` entry for `src-tauri/resources/` (same as the OpenRGB sidecar).

---

### Task 3: Publish sidecar binaries as a GitHub release + pin their hashes

**⚠️ Confirmation gate:** this creates a real GitHub release on `Heiphaistos/PureRGB` (private repo, but still a persistent, shared artifact). Confirm with Momo before running Step 1 if this task is being executed autonomously — do not create the release silently.

**Files:** none (hosting step; output feeds Tasks 4 and 5)

- [ ] **Step 1: Create the `sidecars-v1` release with both binaries attached**

Run:
```bash
gh release create sidecars-v1 \
  --repo Heiphaistos/PureRGB \
  --title "Sidecar binaries (liquidctl, sensord)" \
  --notes "Hosted binaries for PureRGB's runtime self-install fallback (liquidctl.exe, sensord.exe). Not an app release — do not expect a changelog here." \
  src-tauri/resources/liquidctl/liquidctl.exe \
  src-tauri/resources/sensord/sensord.exe
```
Expected: prints the release URL, e.g. `https://github.com/Heiphaistos/PureRGB/releases/tag/sidecars-v1`.

- [ ] **Step 2: Compute the SHA-256 of each uploaded binary**

Run:
```powershell
(Get-FileHash src-tauri/resources/liquidctl/liquidctl.exe -Algorithm SHA256).Hash.ToLower()
(Get-FileHash src-tauri/resources/sensord/sensord.exe -Algorithm SHA256).Hash.ToLower()
```
Expected: two 64-character lowercase hex strings. Keep both — they go verbatim into Tasks 4 and 5 as `LIQUIDCTL_SHA256` and `SENSORD_SHA256`.

- [ ] **Step 3: Sanity-check the download URL resolves**

Run: `powershell -Command "(Invoke-WebRequest -Uri 'https://github.com/Heiphaistos/PureRGB/releases/download/sidecars-v1/liquidctl.exe' -Method Head -UseBasicParsing).StatusCode"`
Expected: `200`.

---

### Task 4: `liquidctl` install fallback

**Files:**
- Modify: `src-tauri/src/backends/liquidctl/mod.rs`

- [ ] **Step 1: Add the pinned URL/SHA constants**

In `src-tauri/src/backends/liquidctl/mod.rs`, after the existing `const CREATE_NO_WINDOW: u32 = 0x0800_0000;` (currently line 26), add:

```rust
/// Filet de sécurité pour l'exe portable : NSIS copie resources/liquidctl/
/// à côté du binaire, mais le portable n'a pas de dossier resources/ du
/// tout (voir OpenRgbManager::install, même limitation déjà compensée pour
/// OpenRGB). Binaire hébergé sur nos propres releases GitHub (PyInstaller
/// onefile, aucune release Windows officielle liquidctl à épingler).
const LIQUIDCTL_URL: &str =
    "https://github.com/Heiphaistos/PureRGB/releases/download/sidecars-v1/liquidctl.exe";
const LIQUIDCTL_SHA256: &str = "REPLACE_WITH_HASH_FROM_TASK_3_STEP_2";
```

Replace `REPLACE_WITH_HASH_FROM_TASK_3_STEP_2` with the real hash captured in Task 3.

- [ ] **Step 2: Add `appdata_dir()` and `install()`**

Still in `src-tauri/src/backends/liquidctl/mod.rs`, add right after the `locate()` method (currently ends line 104):

```rust
    fn appdata_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB").join("liquidctl"))
    }

    /// Télécharge liquidctl.exe (SHA-256 pinné) vers %APPDATA%\PureRGB\liquidctl\.
    /// Onefile PyInstaller statique : pas de DLL runtime à copier séparément.
    fn install() -> Result<PathBuf> {
        let dir = Self::appdata_dir().context("APPDATA introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let exe = dir.join("liquidctl.exe");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{exe}' -UseBasicParsing; \
             $h = (Get-FileHash '{exe}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{exe}' -Force; throw \"hash mismatch: $h\" }}",
            url = LIQUIDCTL_URL,
            exe = exe.display(),
            sha = LIQUIDCTL_SHA256,
        );
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().context("téléchargement liquidctl")?;
        if !output.status.success() {
            bail!(
                "téléchargement liquidctl échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if !exe.is_file() {
            bail!("liquidctl.exe absent après téléchargement");
        }
        Ok(exe)
    }

    /// `locate()` puis, si absent, tentative de téléchargement — ne jamais
    /// laisser le backend mort silencieusement comme avant (portable sans
    /// resources/, cause confirmée d'un retour terrain "0 AIO/hub détecté").
    fn locate_or_install(&self) -> Option<PathBuf> {
        if let Some(p) = self.locate() {
            return Some(p);
        }
        match Self::install() {
            Ok(p) => Some(p),
            Err(e) => {
                log::warn!("installation liquidctl: {e:#}");
                None
            }
        }
    }
```

- [ ] **Step 3: Wire the two `locate()` call sites to use the fallback**

In the same file, `diagnose()` (currently line 155) — change:
```rust
        if self.exe.is_none() {
            self.exe = self.locate();
        }
```
to:
```rust
        if self.exe.is_none() {
            self.exe = self.locate_or_install();
        }
```

And in `Backend::scan()` (currently line 272), change:
```rust
        if self.exe.is_none() {
            self.exe = self.locate();
        }
```
to:
```rust
        if self.exe.is_none() {
            self.exe = self.locate_or_install();
        }
```

- [ ] **Step 4: Build to verify it compiles**

Run: `cd src-tauri && cargo build 2>&1 | tail -40`
Expected: `Compiling purergb_lib...` then success, no errors. (`log` crate must already be a dependency — it's used elsewhere in this file's sibling backends; if `cargo build` reports `log` unresolved, add `log = { workspace = true }` — or whatever form the other backends use — to `src-tauri/Cargo.toml` matching the existing pattern for `sensors.rs`/`mobo.rs`.)

- [ ] **Step 5: Run existing test suite to check no regression**

Run: `cd src-tauri && cargo test 2>&1 | tail -40`
Expected: all existing tests still pass (this task adds no new automated test — `OpenRgbManager::install()` has none either; the download path is exercised for real at Task 3 and validated end-to-end in Task 9).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/backends/liquidctl/mod.rs
git commit -m "fix: liquidctl portable-exe install fallback (mirrors OpenRGB)

Portable exe ships without resources/ (only NSIS installer copies sidecars
next to the binary), so liquidctl.exe was permanently unreachable there —
confirmed root cause of 0 AIO/hub devices on a user's portable install."
```

---

### Task 5: `sensord` install fallback

**Files:**
- Modify: `src-tauri/src/sensors.rs`

- [ ] **Step 1: Add `bail` to the anyhow import and add the pinned constants**

In `src-tauri/src/sensors.rs`, change line 5:
```rust
use anyhow::{Context, Result};
```
to:
```rust
use anyhow::{bail, Context, Result};
```

Then, after `const CREATE_NO_WINDOW: u32 = 0x0800_0000;` (currently line 13), add:

```rust
/// Même limitation que liquidctl (voir backends/liquidctl/mod.rs) : exe
/// portable sans resources/. sensord est notre propre build .NET self-contained.
const SENSORD_URL: &str =
    "https://github.com/Heiphaistos/PureRGB/releases/download/sidecars-v1/sensord.exe";
const SENSORD_SHA256: &str = "REPLACE_WITH_HASH_FROM_TASK_3_STEP_2";
```

Replace `REPLACE_WITH_HASH_FROM_TASK_3_STEP_2` with the real sensord hash from Task 3.

- [ ] **Step 2: Add `appdata_dir()` and `install()` on `SensorHub`**

In the same file, inside `impl SensorHub`, right after `fn locate(&self) -> Option<PathBuf> {` block (currently ends line 87), add:

```rust
    fn appdata_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("PureRGB").join("sensord"))
    }

    /// Télécharge sensord.exe (SHA-256 pinné) vers %APPDATA%\PureRGB\sensord\.
    /// Publish self-contained .NET 8 : runtime déjà inclus, pas de DLL à part.
    fn install() -> Result<PathBuf> {
        let dir = Self::appdata_dir().context("APPDATA introuvable")?;
        std::fs::create_dir_all(&dir)?;
        let exe = dir.join("sensord.exe");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             Invoke-WebRequest -Uri '{url}' -OutFile '{exe}' -UseBasicParsing; \
             $h = (Get-FileHash '{exe}' -Algorithm SHA256).Hash.ToLower(); \
             if ($h -ne '{sha}') {{ Remove-Item '{exe}' -Force; throw \"hash mismatch: $h\" }}",
            url = SENSORD_URL,
            exe = exe.display(),
            sha = SENSORD_SHA256,
        );
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().context("téléchargement sensord")?;
        if !output.status.success() {
            bail!(
                "téléchargement sensord échoué: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if !exe.is_file() {
            bail!("sensord.exe absent après téléchargement");
        }
        Ok(exe)
    }
```

- [ ] **Step 3: Wire `start()` to fall back to `install()`**

In the same file, `pub fn start(self: &Arc<Self>) -> Result<bool>` (currently lines 90-127), change:
```rust
        let exe = match self.locate() {
            Some(e) => e,
            None => return Ok(false), // sidecar absent : capteurs indisponibles
        };
```
to:
```rust
        let exe = match self.locate() {
            Some(e) => e,
            None => match Self::install() {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("installation sensord: {e:#}");
                    return Ok(false);
                }
            },
        };
```

- [ ] **Step 4: Build to verify it compiles**

Run: `cd src-tauri && cargo build 2>&1 | tail -40`
Expected: success, no errors.

- [ ] **Step 5: Run existing test suite**

Run: `cd src-tauri && cargo test 2>&1 | tail -40`
Expected: all existing tests pass, including `mobo.rs`'s `rpm_pairing_by_number` and `control_id_prefix_matches_lhm_identifier_scheme` (unaffected by this change, listed here only as a regression tripwire since `mobo.rs` depends on `sensors.rs`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/sensors.rs
git commit -m "fix: sensord portable-exe install fallback (mirrors OpenRGB)

Same gap as liquidctl — sensord.exe unreachable on portable installs meant
the mobo fan backend and all temperature/RPM sensors were silently dead."
```

---

### Task 6: ARGB fan/strip preset table

**Files:**
- Create: `src/data/fanPresets.ts`

- [ ] **Step 1: Verify each seed model's LED count against an official product page**

Before writing the file, confirm (web search, one pass) the per-unit LED count for each entry below. `Cooler Master MF120 Prismatic` is already verified (24 side LEDs + 6 extra = 30, cross-checked against two independent product pages during the design phase). Verify the rest the same way; if a number can't be confirmed from an official source, drop that row rather than guess — the table can grow later.

- [ ] **Step 2: Write the preset table and the pure calculation function**

Create `src/data/fanPresets.ts`:

```typescript
// Nombre de LEDs adressables par unité (1 ventilateur, ou 1m de bandeau pour
// les entrées génériques) — sourcé depuis la fiche produit officielle.
// Détection matérielle automatique impossible sur un header 3-pin passif
// (WS281x-like unidirectionnel, aucune voie de retour) : ceci est
// l'équivalent pratique — l'utilisateur choisit son modèle au lieu de
// compter les LEDs à la main.
export interface FanPreset {
  brand: string;
  model: string;
  ledsPerUnit: number;
}

export const FAN_PRESETS: FanPreset[] = [
  // Cooler Master — https://www.coolermaster.com/en-global/products/masterfan-mf120-prismatic.html
  { brand: "Cooler Master", model: "MF120 Prismatic (tri-loop)", ledsPerUnit: 30 },
  // Corsair — fiches produit officielles corsair.com
  { brand: "Corsair", model: "QL120", ledsPerUnit: 34 },
  { brand: "Corsair", model: "LL120", ledsPerUnit: 16 },
  { brand: "Corsair", model: "ML120 RGB", ledsPerUnit: 4 },
  // Entrée générique pour tout le reste (bandeau, ventilo non listé) :
  // l'utilisateur renseigne lui-même les LEDs/unité depuis la fiche produit.
  { brand: "Générique", model: "Autre (LEDs/unité personnalisé)", ledsPerUnit: 1 },
];

export function ledsFor(preset: FanPreset, qty: number): number {
  if (!Number.isFinite(qty) || qty <= 0) return 0;
  return Math.round(preset.ledsPerUnit * qty);
}
```

Note: the "Générique" row with `ledsPerUnit: 1` makes the quantity field double as a direct LED-count input when nothing matches — pairs with the manual entry that already exists in `EffectPanel.vue`, not a replacement for it.

- [ ] **Step 3: Quick manual verification of `ledsFor` (no test framework in this repo yet — matches existing project convention of Rust-only automated tests)**

Run: `node -e "const {ledsFor}=require('./src/data/fanPresets.ts')" 2>&1 || echo "expected: TS via node needs a loader — verify instead with:"`
Run: `npx tsx -e "import {FAN_PRESETS, ledsFor} from './src/data/fanPresets'; console.log(ledsFor(FAN_PRESETS[0], 3)); console.log(ledsFor(FAN_PRESETS[0], 0)); console.log(ledsFor(FAN_PRESETS[0], -1));"`
Expected: `90`, `0`, `0` (MF120 Prismatic × 3 = 90; zero/negative quantity clamped to 0). If `tsx` isn't installed, run `npm exec -- vite-node src/data/fanPresets.ts` instead, or add a temporary `console.log` at the bottom of the file, run `npx vite build --mode development` isn't needed — simplest is opening the app itself after Task 7 wiring and checking the computed value in the UI.

- [ ] **Step 4: Commit**

```bash
git add src/data/fanPresets.ts
git commit -m "feat: known ARGB fan/strip preset table (LED count lookup)"
```

---

### Task 7: Wire the preset picker into the zone-resize UI

**Files:**
- Modify: `src/components/EffectPanel.vue`

- [ ] **Step 1: Import the preset data**

In `src/components/EffectPanel.vue`, change the import block (currently lines 1-5):
```typescript
import { invoke } from "@tauri-apps/api/core";
import { computed, reactive, ref, watch } from "vue";
import type { Color, DeviceInfo, EffectConfig, EffectKind, ModeInfo } from "../types";
import { colorToHex, EFFECT_LABELS, hexToColor, zoneResizable } from "../types";
```
to:
```typescript
import { invoke } from "@tauri-apps/api/core";
import { computed, reactive, ref, watch } from "vue";
import type { Color, DeviceInfo, EffectConfig, EffectKind, ModeInfo } from "../types";
import { colorToHex, EFFECT_LABELS, hexToColor, zoneResizable } from "../types";
import { FAN_PRESETS, ledsFor, type FanPreset } from "../data/fanPresets";
```

- [ ] **Step 2: Add picker state next to the existing zone-resize state**

After `const resizingZone = ref<number | null>(null);` (currently line 37), add:

```typescript
// Sélecteur de modèle connu — calcule le nombre de LEDs à la place de
// l'utilisateur (détection matérielle automatique impossible sur un
// header 3-pin passif, voir fanPresets.ts).
const presetChoice = ref<Record<number, FanPreset>>({});
const presetQty = ref<Record<number, number>>({});

function applyPresetCalc(zoneIdx: number) {
  const preset = presetChoice.value[zoneIdx];
  const qty = presetQty.value[zoneIdx];
  if (!preset || !qty) return;
  zoneSizeEdits.value[zoneIdx] = ledsFor(preset, qty);
}
```

- [ ] **Step 3: Add the picker UI above the existing manual input row**

In the template, inside the `v-for="{ z, i } in resizableZones"` block (currently lines 224-238), the row starts with:
```html
          <div v-for="{ z, i } in resizableZones" :key="i" class="argb-row">
            <span class="argb-name">{{ z.name }} <em>({{ z.led_count }} LED)</em></span>
            <input
              type="number"
              :min="z.leds_min"
              :max="z.leds_max"
              v-model.number="zoneSizeEdits[i]"
            />
            <button
              :disabled="resizingZone !== null || zoneSizeEdits[i] === z.led_count"
              @click="applyZoneSize(i)"
            >
              {{ resizingZone === i ? "…" : "Appliquer" }}
            </button>
          </div>
```
Replace it with (adds a preset row above the existing manual row, same `argb-row` per zone):
```html
          <div v-for="{ z, i } in resizableZones" :key="i" class="argb-zone">
            <span class="argb-name">{{ z.name }} <em>({{ z.led_count }} LED)</em></span>
            <div class="argb-preset-row">
              <select v-model="presetChoice[i]" @change="applyPresetCalc(i)">
                <option :value="undefined" disabled selected>Modèle connu…</option>
                <option v-for="p in FAN_PRESETS" :key="p.model" :value="p">
                  {{ p.brand }} — {{ p.model }} ({{ p.ledsPerUnit }} LED/unité)
                </option>
              </select>
              <input
                type="number"
                min="1"
                placeholder="qté"
                v-model.number="presetQty[i]"
                @input="applyPresetCalc(i)"
              />
            </div>
            <div class="argb-row">
              <input
                type="number"
                :min="z.leds_min"
                :max="z.leds_max"
                v-model.number="zoneSizeEdits[i]"
              />
              <button
                :disabled="resizingZone !== null || zoneSizeEdits[i] === z.led_count"
                @click="applyZoneSize(i)"
              >
                {{ resizingZone === i ? "…" : "Appliquer" }}
              </button>
            </div>
          </div>
```

- [ ] **Step 4: Add matching styles**

In `<style scoped>`, after the existing `.argb-row` rules (currently ending around line 517 with `.argb-row input { width: 90px; }`), add:

```css
.argb-zone {
  margin-top: 10px;
}

.argb-preset-row {
  display: flex;
  gap: 10px;
  margin: 6px 0;
}

.argb-preset-row select {
  flex: 1;
  font-size: 12px;
}

.argb-preset-row input {
  width: 70px;
}
```

- [ ] **Step 5: Build the frontend to catch type errors**

Run: `npm run build 2>&1 | tail -60`
Expected: Vite/TS build succeeds, no type errors on `FanPreset`, `ledsFor`, or the new refs.

- [ ] **Step 6: Manual UI check**

Run: `npm run tauri dev` (or the project's existing dev command if different — check `package.json` `scripts` if `tauri dev` isn't present), open the Éclairage tab on any device with a resizable zone (or a device with 0 zones is fine to at least confirm the panel renders without a resizable zone), pick a preset + quantity, confirm the manual LED-count input below updates to the computed value, then confirm the existing "Appliquer" button still fires `resize_zone` unchanged.
Expected: no console errors, computed value matches `ledsPerUnit × qty`.

- [ ] **Step 7: Commit**

```bash
git add src/components/EffectPanel.vue
git commit -m "feat: known-model picker for ARGB zone LED count

Practical equivalent to hardware auto-detect, which is physically
impossible on a passive 3-pin ARGB header (no return data line)."
```

---

### Task 8: Differentiated empty state in the Ventilateurs panel

**Files:**
- Modify: `src/components/FanPanel.vue`

- [ ] **Step 1: Import the diagnostics type and fetch it**

Change the import line (currently line 4):
```typescript
import type { CurveConfig, DeviceInfo, Sensor, Settings } from "../types";
```
to:
```typescript
import type { CurveConfig, DeviceInfo, HardwareDiagnostics, Sensor, Settings } from "../types";
```

After `const sensors = ref<Sensor[]>([]);` (currently line 11), add:
```typescript
const diag = ref<HardwareDiagnostics | null>(null);
```

- [ ] **Step 2: Fetch diagnostics alongside sensors**

Change `refreshSensors` (currently lines 92-98):
```typescript
async function refreshSensors() {
  try {
    sensors.value = await invoke<Sensor[]>("get_sensors");
  } catch {
    /* sidecar absent */
  }
}
```
to:
```typescript
async function refreshSensors() {
  try {
    sensors.value = await invoke<Sensor[]>("get_sensors");
  } catch {
    /* sidecar absent */
  }
  try {
    diag.value = await invoke<HardwareDiagnostics>("hardware_diagnostics");
  } catch {
    diag.value = null;
  }
}
```

- [ ] **Step 3: Add a computed empty-state message**

After the `mainTemps` computed (currently lines 19-23), add:
```typescript
const emptyFanMessage = computed(() => {
  const s = diag.value?.sensord;
  if (!s || !s.exe_path) {
    return "Capteurs carte mère indisponibles (sensord introuvable — l'app tente de le réinstaller automatiquement au prochain lancement, une connexion réseau est nécessaire).";
  }
  if (!s.running) {
    return "sensord trouvé mais pas démarré — redémarrez l'application.";
  }
  if (s.sensor_count === 0) {
    return "sensord tourne mais ne remonte aucun capteur sur cette machine — matériel non supporté par LibreHardwareMonitor.";
  }
  return "Capteurs détectés, mais aucun header ventilateur pilotable trouvé sur cette carte mère (RGB reste possible via OpenRGB, seul le contrôle de vitesse est indisponible ici). AIO/hubs NZXT & Corsair : détectés via liquidctl au scan. Hubs en driver natif : activer dans Réglages.";
});
```

- [ ] **Step 4: Replace the static empty-state paragraph**

Change (currently lines 124-128):
```html
    <p v-if="devices.length === 0" class="empty">
      Aucun appareil à ventilateurs pilotables détecté.<br />
      AIO/hubs NZXT &amp; Corsair : détectés via liquidctl au scan. Hubs en
      driver natif : activer dans Réglages.
    </p>
```
to:
```html
    <p v-if="devices.length === 0" class="empty">
      {{ emptyFanMessage }}
    </p>
```

- [ ] **Step 5: Build the frontend**

Run: `npm run build 2>&1 | tail -60`
Expected: success, no type errors.

- [ ] **Step 6: Manual UI check**

Run the dev build, open the Ventilateurs tab. With `sensord` genuinely absent (rename `%APPDATA%\PureRGB\sensord\sensord.exe` temporarily if present on this dev machine) confirm the "introuvable" message shows; restore the file afterwards.
Expected: message text changes based on the simulated state, no crash.

- [ ] **Step 7: Commit**

```bash
git add src/components/FanPanel.vue
git commit -m "fix: differentiate empty Ventilateurs state (sensord absent vs no controllable header)

Previous message never mentioned the mobo backend at all — user reporting
'0 fans detected' had no way to tell sensord was simply missing."
```

---

### Note: LibreHardwareMonitorLib bump — not needed

The spec's bonus item ("bump LHM 0.9.6 → dernière version stable") was
checked against NuGet on 2026-07-20: `0.9.6` (already pinned in
`sidecars/sensord/sensord.csproj:18`) **is** the latest stable release —
every version above it (`0.9.7-pre687` through `0.9.7-pre708`) is a
prerelease. No change made; re-check NuGet if this plan is executed much
later than its write date.

---

### Task 9: Version bump + full verification + push

**Files:**
- Modify: `package.json:4`, `src-tauri/tauri.conf.json:4`, `src-tauri/Cargo.toml:3`

- [ ] **Step 1: Bump version 0.8.1 → 0.9.0 in all three files**

`package.json:4`: `"version": "0.8.1",` → `"version": "0.9.0",`
`src-tauri/tauri.conf.json:4`: `"version": "0.8.1",` → `"version": "0.9.0",`
`src-tauri/Cargo.toml:3`: `version = "0.8.1"` → `version = "0.9.0"`

- [ ] **Step 2: Full backend test + build**

Run: `cd src-tauri && cargo test 2>&1 | tail -60 && cargo build --release 2>&1 | tail -20`
Expected: all tests pass, release build succeeds.

- [ ] **Step 3: Full frontend build**

Run: `npm run build 2>&1 | tail -60`
Expected: succeeds.

- [ ] **Step 4: Commit the version bump**

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "chore: bump version to 0.9.0"
```

- [ ] **Step 5: Push**

Run: `git push origin main`
Expected: pushes all commits from this plan (Tasks 4, 5, 6, 7, 8, 9) to `origin/main`.

**Note:** this task does not build/publish a new PureRGB app release (`.exe`/setup) — that's a separate, explicit step Momo asked about before ("release portable et setup republié"). Building and publishing v0.9.0's actual release artifacts is a follow-up action to confirm with Momo once this plan's commits are reviewed, same as prior version cycles (v0.7.0, v0.8.0, v0.8.1 releases were each their own explicit publish step).

---

## Deferred (separate specs, in this order)

Scheduler → Hardware Sync (color reacts to temp/load) → E1.31 Receiver → Visual Map → Effects Plugin extras (audio-viz/Ambilight/GIF/shaders). Not started in this plan — see the design doc's "Hors scope explicite" section.
