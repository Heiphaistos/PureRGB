# PureRGB — Assistant de détection du nombre de LEDs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un bouton "Assistant de détection" à côté du redimensionnement manuel de chaque zone ARGB lance une recherche dichotomique (Oui/Non, ~log2(N) étapes) qui détermine le nombre exact de LEDs physiques, au lieu de forcer l'utilisateur à deviner un nombre.

**Architecture:** 100% frontend (`EffectPanel.vue`) — orchestration des commandes Tauri déjà existantes `resize_zone` et `apply_effect` (aucun nouveau code Rust). État du wizard dans des refs séparées de `zoneSizeEdits` pour ne pas être écrasées par le watcher existant sur `props.device`.

**Tech Stack:** Vue3/TypeScript, commandes Tauri déjà existantes.

**Spec source:** `docs/superpowers/specs/2026-07-22-zone-led-detection-wizard-design.md`

---

### Task 1: État et logique du wizard

**Files:**
- Modify: `src/components/EffectPanel.vue`

- [ ] **Step 1: Nouvel état réactif**

Après la ligne existante `const resizingZone = ref<number | null>(null);` (dans le bloc "Zones ARGB redimensionnables"), ajouter :

```ts
const wizardZone = ref<number | null>(null);
const wizardLow = ref(0);
const wizardHigh = ref(0);
const wizardMid = ref(0);
const wizardOriginalSize = ref<number | null>(null);
const wizardBusy = ref(false);
```

- [ ] **Step 2: Fonction de test d'un candidat**

Après la fonction `applyZoneSize` existante (juste avant `const targetZone = ref<number | null>(null);`), ajouter :

```ts
async function testCandidate(zoneIdx: number, n: number) {
  if (!props.device) return;
  await invoke("resize_zone", { deviceId: props.device.id, zone: zoneIdx, newSize: n });
  await invoke("apply_effect", {
    deviceId: props.device.id,
    config: { kind: "static", colors: [{ r: 255, g: 255, b: 255 }], speed: 1, brightness: 1, reverse: false },
    zone: zoneIdx,
  });
  wizardMid.value = n;
}

async function startWizard(zoneIdx: number) {
  if (!props.device) return;
  const z = props.device.zones[zoneIdx];
  wizardOriginalSize.value = z.led_count;
  wizardLow.value = z.leds_min;
  wizardHigh.value = z.leds_max;
  wizardZone.value = zoneIdx;
  wizardBusy.value = true;
  try {
    // Nettoyage : tout éteindre à la taille maximale avant de commencer, pour
    // qu'une frontière blanc/noir nette apparaisse à chaque test (sinon des
    // LEDs au-delà du candidat testé pourraient garder une ancienne couleur).
    await invoke("resize_zone", { deviceId: props.device.id, zone: zoneIdx, newSize: z.leds_max });
    await invoke("apply_effect", {
      deviceId: props.device.id,
      config: { kind: "off", colors: [], speed: 1, brightness: 1, reverse: false },
      zone: zoneIdx,
    });
    await testCandidate(zoneIdx, Math.ceil((wizardLow.value + wizardHigh.value) / 2));
  } catch (e) {
    emit("toast", `Assistant de détection : ${e}`);
    wizardZone.value = null;
  } finally {
    wizardBusy.value = false;
  }
}

async function confirmWizard(allLit: boolean) {
  if (wizardZone.value === null || !props.device) return;
  const zoneIdx = wizardZone.value;
  if (allLit) {
    wizardLow.value = wizardMid.value;
  } else {
    wizardHigh.value = wizardMid.value - 1;
  }

  if (wizardLow.value >= wizardHigh.value) {
    // Recherche terminée. Le dernier test affiché correspondait à wizardMid,
    // qui peut différer de wizardLow (réponse "Non" décale la borne haute
    // sans re-tester) — s'assurer que la zone est bien à la taille finale.
    wizardBusy.value = true;
    try {
      if (wizardMid.value !== wizardLow.value) {
        await invoke("resize_zone", { deviceId: props.device.id, zone: zoneIdx, newSize: wizardLow.value });
      }
      emit("toast", `Zone « ${props.device.zones[zoneIdx]?.name} » : ${wizardLow.value} LED détectées`);
      emit("refresh");
    } catch (e) {
      emit("toast", `Assistant de détection : ${e}`);
    } finally {
      wizardZone.value = null;
      wizardBusy.value = false;
    }
    return;
  }

  wizardBusy.value = true;
  try {
    await testCandidate(zoneIdx, Math.ceil((wizardLow.value + wizardHigh.value) / 2));
  } catch (e) {
    emit("toast", `Assistant de détection : ${e}`);
  } finally {
    wizardBusy.value = false;
  }
}

async function cancelWizard() {
  if (wizardZone.value === null || wizardOriginalSize.value === null || !props.device) {
    wizardZone.value = null;
    return;
  }
  const zoneIdx = wizardZone.value;
  const original = wizardOriginalSize.value;
  wizardZone.value = null;
  try {
    await invoke("resize_zone", { deviceId: props.device.id, zone: zoneIdx, newSize: original });
    emit("refresh");
  } catch (e) {
    emit("toast", `Annulation : ${e}`);
  }
}
```

- [ ] **Step 3: Vérifier**

Run: `npm run build`
Expected: succès (les fonctions ne sont pas encore appelées depuis le template — Task 2 les câble ; TypeScript ne signale pas de fonction non-utilisée pour du code de script Vue, seulement pour des imports, donc pas d'erreur attendue ici).

- [ ] **Step 4: Commit**

```bash
git add src/components/EffectPanel.vue
git commit -m "feat(zones): add binary-search LED-count detection wizard logic"
```

---

### Task 2: UI du wizard dans le template

**Files:**
- Modify: `src/components/EffectPanel.vue`

- [ ] **Step 1: Repérer le bloc existant**

Le template contient actuellement (dans la boucle `v-for="{ z, i } in resizableZones"`) :
```html
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
(la dernière `</div>` ferme `.argb-zone`, pas `.argb-row`.)

- [ ] **Step 2: Insérer l'UI du wizard**

Remplacer par :
```html
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
            <div v-if="wizardZone === i" class="wizard-box">
              <p>
                Test en cours : <strong>{{ wizardMid }}</strong> LED allumées en blanc.<br />
                Est-ce que TOUTES les LEDs de la bande sont allumées, y compris la toute dernière ?
              </p>
              <div class="wizard-actions">
                <button :disabled="wizardBusy" @click="confirmWizard(true)">Oui, toutes allumées</button>
                <button :disabled="wizardBusy" @click="confirmWizard(false)">Non, ça s'arrête avant</button>
                <button :disabled="wizardBusy" @click="cancelWizard">Annuler</button>
              </div>
            </div>
            <button v-else :disabled="wizardZone !== null" class="wizard-start" @click="startWizard(i)">
              Assistant de détection
            </button>
          </div>
```

- [ ] **Step 3: Style minimal**

Dans le bloc `<style scoped>` existant (rechercher les classes `.argb-row`/`.argb-zone` déjà présentes pour rester cohérent visuellement), ajouter à la fin :
```css
.wizard-box {
  margin-top: 6px;
  padding: 8px;
  border: 1px solid #444;
  border-radius: 6px;
  font-size: 0.85em;
}
.wizard-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
  flex-wrap: wrap;
}
.wizard-start {
  margin-top: 4px;
}
```

- [ ] **Step 4: Vérifier**

Run: `npm run build`
Expected: succès (typecheck + build Vite).

- [ ] **Step 5: Commit**

```bash
git add src/components/EffectPanel.vue
git commit -m "feat(zones): wire detection wizard UI into ARGB zone panel"
```

---

### Task 3: Vérification manuelle + bump de version

**Files:** `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` pour le bump.

- [ ] **Step 1: Build complet**

Run: `cd src-tauri && cargo build` (aucun changement Rust dans ce sous-projet, mais on vérifie que rien n'est cassé) puis `npm run build`
Expected: les deux verts.

- [ ] **Step 2: Vérification manuelle (non exécutable ici — nécessite du matériel ARGB réel)**

À faire par Momo : ouvrir le panneau d'une zone ARGB redimensionnable connue (nombre de LEDs déjà su à l'avance pour vérifier l'exactitude), cliquer "Assistant de détection", répondre honnêtement aux questions Oui/Non à chaque étape, confirmer que :
1. Le nombre final détecté correspond au nombre réel de LEDs physiques.
2. Le nombre d'étapes est proche de log2(leds_max - leds_min) (ex. ~8-9 étapes pour une plage 1-300).
3. "Annuler" en cours de route restaure bien la taille de zone d'origine.
4. Une erreur réseau/OpenRGB pendant le wizard affiche un toast clair et laisse l'assistant dans un état récupérable (pas de crash, pas de blocage des boutons).

- [ ] **Step 3: Bump de version**

`package.json` : `"version": "0.18.0",`
`src-tauri/Cargo.toml` : `version = "0.18.0"`
`src-tauri/tauri.conf.json` : `"version": "0.18.0",`

(Vérifier la version actuelle avant de bump — elle a pu changer depuis l'écriture de ce plan si d'autres sous-projets ont déjà incrémenté ; utiliser la version courante + 1 en mineur, pas nécessairement "0.18.0" littéralement si la réalité est différente au moment de l'exécution.)

- [ ] **Step 4: Vérifier**

Run: `cd src-tauri && cargo check` puis `npm run build`
Expected: les deux verts.

- [ ] **Step 5: Commit**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "chore: bump version"
```
