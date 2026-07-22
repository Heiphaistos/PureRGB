<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, reactive, ref, watch } from "vue";
import type { Color, DeviceInfo, EffectConfig, EffectKind, ModeInfo } from "../types";
import { colorToHex, EFFECT_LABELS, hexToColor, zoneResizable } from "../types";
import { FAN_PRESETS, ledsFor, type FanPreset } from "../data/fanPresets";

const props = defineProps<{
  device: DeviceInfo | null;
  savedEffects: Record<string, EffectConfig>;
}>();

const emit = defineEmits<{
  apply: [deviceId: string, config: EffectConfig, zone: number | null];
  applyAll: [config: EffectConfig];
  applyMode: [
    deviceId: string,
    modeIndex: number,
    speed: number | null,
    direction: number | null,
    colors: Color[] | null,
  ];
  toast: [msg: string];
  refresh: [];
}>();

// --- Zones ARGB redimensionnables (ventilos/bandeaux sur connecteur carte
// mère ou canal de hub : OpenRGB ne peut pas deviner le nombre de LEDs). ---
const resizableZones = computed(() =>
  (props.device?.zones ?? [])
    .map((z, i) => ({ z, i }))
    .filter(({ z }) => zoneResizable(z)),
);
const emptyResizable = computed(() =>
  resizableZones.value.filter(({ z }) => z.led_count === 0),
);
const zoneSizeEdits = ref<Record<number, number>>({});
const resizingZone = ref<number | null>(null);
const wizardZone = ref<number | null>(null);
const wizardLow = ref(0);
const wizardHigh = ref(0);
const wizardMid = ref(0);
const wizardOriginalSize = ref<number | null>(null);
const wizardBusy = ref(false);

// Sélecteur de modèle connu — calcule le nombre de LEDs à la place de
// l'utilisateur (détection matérielle automatique impossible sur un
// header 3-pin passif, voir fanPresets.ts).
const presetChoice = ref<Record<number, FanPreset>>({});
const presetQty = ref<Record<number, number>>({});

function applyPresetCalc(zoneIdx: number) {
  const preset = presetChoice.value[zoneIdx];
  const qty = presetQty.value[zoneIdx];
  if (!preset || !qty || qty <= 0) return;
  zoneSizeEdits.value[zoneIdx] = ledsFor(preset, qty);
}

async function applyZoneSize(zoneIdx: number) {
  if (!props.device) return;
  const wanted = zoneSizeEdits.value[zoneIdx];
  if (wanted === undefined) return;
  resizingZone.value = zoneIdx;
  try {
    await invoke("resize_zone", {
      deviceId: props.device.id,
      zone: zoneIdx,
      newSize: wanted,
    });
    emit("toast", `Zone « ${props.device.zones[zoneIdx]?.name} » : ${wanted} LED`);
    emit("refresh");
  } catch (e) {
    emit("toast", `Redimensionnement : ${e}`);
  } finally {
    resizingZone.value = null;
  }
}

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
    emit("refresh");
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
      zoneSizeEdits.value[zoneIdx] = wizardLow.value;
      emit("toast", `Nombre détecté : ${wizardLow.value} LED — bascule ré-appliquée manuellement (${e})`);
      emit("refresh");
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
    emit("refresh");
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
    emit("refresh");
  }
}

// Cible : null = appareil entier, sinon index de zone.
const targetZone = ref<number | null>(null);

// --- Modes matériels (OpenRGB) ---
const selectedMode = ref<number | null>(null);
const modeSpeed = ref(0);
const modeDirection = ref(0);
const modeColors = ref<Color[]>([]);

const currentMode = computed<ModeInfo | null>(() => {
  if (selectedMode.value === null) return null;
  return props.device?.modes?.[selectedMode.value] ?? null;
});
const modeHasSpeed = computed(() => ((currentMode.value?.flags ?? 0) & 1) !== 0);
const modeHasDirection = computed(() => ((currentMode.value?.flags ?? 0) & 0b1110) !== 0);
const modeHasColors = computed(
  () => ((currentMode.value?.flags ?? 0) & (1 << 6)) !== 0 && (currentMode.value?.colors_max ?? 0) > 0,
);
const modeSpeedMin = computed(() =>
  Math.min(currentMode.value?.speed_min ?? 0, currentMode.value?.speed_max ?? 0),
);
const modeSpeedMax = computed(() =>
  Math.max(currentMode.value?.speed_min ?? 0, currentMode.value?.speed_max ?? 0),
);

function selectMode(i: number) {
  selectedMode.value = i;
  const m = props.device?.modes?.[i];
  if (!m) return;
  modeSpeed.value = m.speed;
  modeDirection.value = m.direction;
  modeColors.value = m.colors.length
    ? m.colors.map((c) => ({ ...c }))
    : [{ r: 255, g: 80, b: 0 }];
}

function setModeColor(i: number, hex: string) {
  modeColors.value[i] = hexToColor(hex);
}

function applyHardwareMode() {
  if (!props.device || selectedMode.value === null) return;
  emit(
    "applyMode",
    props.device.id,
    selectedMode.value,
    modeHasSpeed.value ? modeSpeed.value : null,
    modeHasDirection.value ? modeDirection.value : null,
    modeHasColors.value ? modeColors.value.map((c) => ({ ...c })) : null,
  );
}

const state = reactive<EffectConfig>({
  kind: "static",
  colors: [{ r: 255, g: 80, b: 0 }],
  speed: 1.0,
  brightness: 1.0,
  reverse: false,
});

// Nb de couleurs éditables selon l'effet.
const colorCount = computed(() => {
  switch (state.kind) {
    case "off":
    case "rainbow_cycle":
    case "rainbow_wave":
      return 0;
    case "color_wave":
    case "gradient":
      return 2;
    default:
      return 1;
  }
});

const hasMotion = computed(
  () => !["off", "static", "gradient"].includes(state.kind),
);

const hasDirection = computed(() =>
  ["rainbow_wave", "color_wave", "comet", "gradient"].includes(state.kind),
);

watch(
  () => props.device?.id,
  (id) => {
    targetZone.value = null;
    selectedMode.value = null;
    zoneSizeEdits.value = Object.fromEntries(
      (props.device?.zones ?? [])
        .map((z, i) => [i, Math.max(z.led_count, z.leds_min)])
        .filter((_, i) => zoneResizable(props.device!.zones[i])),
    );
    presetChoice.value = {};
    presetQty.value = {};
    if (!id) return;
    const saved = props.savedEffects[id];
    if (saved) {
      state.kind = saved.kind;
      state.colors = saved.colors.map((c) => ({ ...c }));
      state.speed = saved.speed;
      state.brightness = saved.brightness;
      state.reverse = saved.reverse;
    }
  },
  { immediate: true },
);

function ensureColors() {
  while (state.colors.length < colorCount.value) {
    state.colors.push({ r: 0, g: 144, b: 255 });
  }
}

watch(colorCount, ensureColors, { immediate: true });

function setColor(i: number, hex: string) {
  state.colors[i] = hexToColor(hex);
}

function snapshot(): EffectConfig {
  return {
    kind: state.kind,
    colors: state.colors.map((c: Color) => ({ ...c })),
    speed: state.speed,
    brightness: state.brightness,
    reverse: state.reverse,
  };
}
</script>

<template>
  <section class="effect-panel">
    <template v-if="device">
      <div class="head">
        <div>
          <h2>{{ device.name }}</h2>
          <p class="sub">
            {{ device.vendor || device.backend }} · {{ device.led_count }} LED ·
            {{ device.zones.map((z) => z.name).join(", ") || "zone unique" }}
          </p>
        </div>
      </div>

      <div v-if="device.zones.length > 1" class="zone-row">
        <label>Cible</label>
        <select v-model="targetZone">
          <option :value="null">Appareil entier</option>
          <option v-for="(z, i) in device.zones" :key="i" :value="i">
            {{ z.name }} ({{ z.led_count }} LED)
          </option>
        </select>
      </div>

      <div v-if="resizableZones.length" class="argb-box">
        <p v-if="emptyResizable.length" class="argb-alert">
          🌀 Des ventilateurs ou bandeaux ARGB branchés sur
          {{ emptyResizable.map(({ z }) => `« ${z.name} »`).join(", ") }} ?
          Ils sont invisibles tant que le nombre de LEDs est à 0 — indiquez-le
          ci-dessous pour les allumer.
        </p>
        <details :open="emptyResizable.length > 0">
          <summary>Connecteurs ARGB — nombre de LEDs branchées</summary>
          <p class="argb-hint">
            1 ventilateur ARGB ≈ 8 à 16 LED (voir sa fiche). Plusieurs
            ventilateurs chaînés sur le même connecteur : additionnez.
          </p>
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
        </details>
      </div>

      <div class="effects-grid">
        <button
          v-for="(label, kind) in EFFECT_LABELS"
          :key="kind"
          class="effect-tile"
          :class="{ active: state.kind === kind }"
          @click="state.kind = kind as EffectKind"
        >
          {{ label }}
        </button>
      </div>

      <div class="controls">
        <div v-if="colorCount > 0" class="row">
          <label>Couleur{{ colorCount > 1 ? "s" : "" }}</label>
          <div class="colors">
            <input
              v-for="i in colorCount"
              :key="i"
              type="color"
              :value="colorToHex(state.colors[i - 1] ?? { r: 255, g: 80, b: 0 })"
              @input="setColor(i - 1, ($event.target as HTMLInputElement).value)"
            />
          </div>
        </div>

        <div class="row">
          <label>Luminosité — {{ Math.round(state.brightness * 100) }}%</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            v-model.number="state.brightness"
          />
        </div>

        <div v-if="hasMotion" class="row">
          <label>Vitesse — ×{{ state.speed.toFixed(1) }}</label>
          <input
            type="range"
            min="0.1"
            max="5"
            step="0.1"
            v-model.number="state.speed"
          />
        </div>

        <div v-if="hasDirection" class="row inline">
          <input id="reverse" type="checkbox" v-model="state.reverse" />
          <label for="reverse">Sens inversé</label>
        </div>
      </div>

      <div class="actions">
        <button
          class="primary"
          :disabled="!device.controllable"
          @click="emit('apply', device.id, snapshot(), targetZone)"
        >
          {{ targetZone === null ? "Appliquer à cet appareil" : `Appliquer à « ${device.zones[targetZone]?.name} »` }}
        </button>
        <button @click="emit('applyAll', snapshot())">Appliquer à tout</button>
      </div>
      <p v-if="!device.controllable" class="warn-note">
        Appareil détecté mais non pilotable directement — {{ device.note }}.
      </p>

      <div v-if="device.modes.length > 0" class="hw-modes">
        <h3>Modes matériels natifs</h3>
        <p class="sub">
          Animés par le firmware de l'appareil — persistent même app fermée.
          Tous les paramètres exposés par le matériel sont réglables.
        </p>
        <div class="modes-grid">
          <button
            v-for="(m, i) in device.modes"
            :key="i"
            class="effect-tile"
            :class="{ active: selectedMode === i, current: device.active_mode === i }"
            :title="device.active_mode === i ? 'mode actif' : ''"
            @click="selectMode(i)"
          >
            {{ m.name }}
          </button>
        </div>
        <div v-if="currentMode" class="controls">
          <div v-if="modeHasSpeed" class="row">
            <label>Vitesse matérielle — {{ modeSpeed }}</label>
            <input
              type="range"
              :min="modeSpeedMin"
              :max="modeSpeedMax"
              step="1"
              v-model.number="modeSpeed"
            />
          </div>
          <div v-if="modeHasDirection" class="row">
            <label>Direction</label>
            <select v-model.number="modeDirection">
              <option :value="0">Gauche</option>
              <option :value="1">Droite</option>
              <option :value="2">Haut</option>
              <option :value="3">Bas</option>
              <option :value="4">Horizontal</option>
              <option :value="5">Vertical</option>
            </select>
          </div>
          <div v-if="modeHasColors" class="row">
            <label>
              Couleurs ({{ currentMode.colors_min }}–{{ currentMode.colors_max }})
            </label>
            <div class="colors">
              <input
                v-for="(c, i) in modeColors"
                :key="i"
                type="color"
                :value="colorToHex(c)"
                @input="setModeColor(i, ($event.target as HTMLInputElement).value)"
              />
              <button
                v-if="modeColors.length < currentMode.colors_max"
                @click="modeColors.push({ r: 0, g: 144, b: 255 })"
              >
                +
              </button>
              <button
                v-if="modeColors.length > Math.max(1, currentMode.colors_min)"
                @click="modeColors.pop()"
              >
                −
              </button>
            </div>
          </div>
          <div class="actions">
            <button class="primary" @click="applyHardwareMode">
              Appliquer le mode matériel
            </button>
          </div>
        </div>
      </div>
    </template>
    <p v-else class="empty">Sélectionnez un appareil à gauche.</p>
  </section>
</template>

<style scoped>
.effect-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.head h2 {
  font-size: 19px;
}

.sub {
  color: var(--text-dim);
  font-size: 13px;
  margin-top: 3px;
}

.effects-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
  margin: 20px 0;
}

.effect-tile {
  padding: 16px 10px;
  font-size: 13px;
}

.effect-tile.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.controls {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 460px;
}

.row label {
  display: block;
  font-size: 13px;
  color: var(--text-dim);
  margin-bottom: 6px;
}

.row.inline {
  display: flex;
  align-items: center;
  gap: 8px;
}

.row.inline label {
  margin: 0;
}

.colors {
  display: flex;
  gap: 10px;
}

.actions {
  display: flex;
  gap: 10px;
  margin-top: 24px;
}

.warn-note {
  margin-top: 12px;
  color: var(--warn);
  font-size: 13px;
}

.zone-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 14px;
}

.argb-box {
  margin-top: 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
  background: var(--bg-card);
}

.argb-alert {
  color: var(--warn);
  font-size: 13px;
  margin-bottom: 8px;
}

.argb-box summary {
  cursor: pointer;
  font-size: 13px;
  color: var(--text-dim);
}

.argb-hint {
  font-size: 12px;
  color: var(--text-dim);
  margin: 8px 0;
}

.argb-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
  font-size: 13px;
}

.argb-name {
  flex: 1;
}

.argb-name em {
  color: var(--text-dim);
  font-style: normal;
}

.argb-row input {
  width: 90px;
}

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

.zone-row label {
  font-size: 13px;
  color: var(--text-dim);
}

.hw-modes {
  margin-top: 30px;
  border-top: 1px solid var(--border);
  padding-top: 18px;
}

.hw-modes h3 {
  font-size: 15px;
}

.modes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 8px;
  margin: 14px 0;
}

.effect-tile.current {
  outline: 1px dashed var(--ok);
}

.empty {
  color: var(--text-dim);
  padding: 40px;
  text-align: center;
  width: 100%;
}

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
</style>
