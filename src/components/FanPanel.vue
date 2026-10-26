<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import type {
  CurveConfig,
  DeviceInfo,
  HardwareDiagnostics,
  Sensor,
  SensorDiag,
  Settings,
} from "../types";
import DeviceCard from "./DeviceCard.vue";
import { CURVE_PRESETS, presetPoints } from "../fanCurves";
import { groupFanDevices } from "../fanGroups";
import CardHandle from "./CardHandle.vue";
import { useCardOrder } from "../useCardOrder";

const props = defineProps<{ devices: DeviceInfo[]; settings: Settings | null }>();
const emit = defineEmits<{ toast: [msg: string]; saved: [] }>();

// duty local par "deviceId:channel"
const duties = reactive<Record<string, number>>({});
const sensors = ref<Sensor[]>([]);
const diag = ref<HardwareDiagnostics | null>(null);
// éditeurs de courbe ouverts, état local par clé "deviceId|channel"
const editors = reactive<Record<string, CurveConfig>>({});
let timer: ReturnType<typeof setInterval> | undefined;

const tempSensors = computed(() =>
  sensors.value.filter((s) => s.type === "Temperature"),
);
const mainTemps = computed(() =>
  tempSensors.value.filter((s) =>
    /cpu package|cpu core$|gpu core|gpu hot|liquid/i.test(s.name),
  ),
);

function emptyFanMessageFor(s: SensorDiag | null | undefined): string {
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
}

const emptyFanMessage = computed(() => emptyFanMessageFor(diag.value?.sensord));

const rawGroups = computed(() => groupFanDevices(props.devices));
// Disposition seulement : l'ordre des catégories est mémorisé, aucune logique de réglage n'est touchée.
const ord = useCardOrder("purergb.fans.cardOrder", computed(() => rawGroups.value.map((g) => g.id)));
const fanGroups = computed(() => ord.order.value.map((id) => rawGroups.value.find((g) => g.id === id)!));

function hasEditor(d: DeviceInfo): boolean {
  return d.fan_channels.some((fc) => editors[curveKey(d, fc.index)]);
}

function toggleEditor(d: DeviceInfo, ch: number) {
  if (editors[curveKey(d, ch)]) delete editors[curveKey(d, ch)];
  else openEditor(d, ch);
}

function key(d: DeviceInfo, ch: number) {
  return `${d.id}:${ch}`;
}

function curveKey(d: DeviceInfo, ch: number) {
  return `${d.id}|${ch}`;
}

function duty(d: DeviceInfo, ch: number): number {
  return duties[key(d, ch)] ?? 50;
}

function savedCurve(d: DeviceInfo, ch: number): CurveConfig | undefined {
  return props.settings?.curves?.[curveKey(d, ch)];
}

async function apply(d: DeviceInfo, ch: number) {
  const percent = duty(d, ch);
  try {
    await invoke("set_fan_duty", { deviceId: d.id, channel: ch, percent });
    emit("toast", `${d.name} — canal ${ch + 1} à ${percent}%`);
  } catch (e) {
    emit("toast", `Échec : ${e}`);
  }
}

function openEditor(d: DeviceInfo, ch: number) {
  const k = curveKey(d, ch);
  editors[k] = savedCurve(d, ch)
    ? JSON.parse(JSON.stringify(savedCurve(d, ch)))
    : {
        sensor_id: tempSensors.value[0]?.id ?? "",
        points: [
          { temp: 30, duty: 30 },
          { temp: 50, duty: 45 },
          { temp: 70, duty: 70 },
          { temp: 85, duty: 100 },
        ],
        enabled: true,
      };
}

async function saveCurve(d: DeviceInfo, ch: number) {
  const k = curveKey(d, ch);
  const cfg = editors[k];
  if (!cfg) return;
  try {
    await invoke("set_curve", { deviceId: d.id, channel: ch, config: cfg });
    delete editors[k];
    emit("toast", "Courbe enregistrée");
    emit("saved");
  } catch (e) {
    emit("toast", `Courbe : ${e}`);
  }
}

function applyPreset(d: DeviceInfo, ch: number, id: string) {
  const e = editors[curveKey(d, ch)];
  if (e && id) e.points = presetPoints(id);
}

/** Enregistre la courbe en cours d'édition sur tous les canaux de l'appareil (même capteur, mêmes points). */
async function saveCurveToAll(d: DeviceInfo, ch: number) {
  const cfg = editors[curveKey(d, ch)];
  if (!cfg) return;
  try {
    for (const fc of d.fan_channels) {
      await invoke("set_curve", { deviceId: d.id, channel: fc.index, config: JSON.parse(JSON.stringify(cfg)) });
      delete editors[curveKey(d, fc.index)];
    }
    emit("toast", `Courbe enregistrée sur ${d.fan_channels.length} canaux`);
    emit("saved");
  } catch (e) {
    emit("toast", `Courbe : ${e}`);
  }
}

async function deleteCurve(d: DeviceInfo, ch: number) {
  try {
    await invoke("set_curve", { deviceId: d.id, channel: ch, config: null });
    delete editors[curveKey(d, ch)];
    emit("toast", "Courbe supprimée");
    emit("saved");
  } catch (e) {
    emit("toast", `Courbe : ${e}`);
  }
}

async function refreshSensors() {
  try {
    sensors.value = await invoke<Sensor[]>("get_sensors");
  } catch {
    /* sidecar absent */
  }
  if (props.devices.length === 0) {
    try {
      diag.value = await invoke<HardwareDiagnostics>("hardware_diagnostics");
    } catch {
      diag.value = null;
    }
  }
}

onMounted(() => {
  refreshSensors();
  timer = setInterval(refreshSensors, 3000);
});
onUnmounted(() => clearInterval(timer));
</script>

<template>
  <section class="fan-panel">
    <header class="top">
      <div>
        <h2>Ventilateurs &amp; courbes</h2>
        <p class="sub">
          Vitesse fixe ou courbe automatique selon un capteur de température. Rien n'est réglé sans votre action : « Appliquer » ou
          une courbe activée.
        </p>
      </div>
      <div v-if="mainTemps.length" class="sensor-strip">
        <span v-for="s in mainTemps" :key="s.id" class="sensor-chip">
          {{ s.hardware }} · {{ s.name }} : <strong>{{ Math.round(s.value) }} °C</strong>
        </span>
      </div>
      <p v-else class="note">Capteurs indisponibles (sidecar sensord absent) — courbes inactives.</p>
    </header>

    <p v-if="devices.length === 0" class="empty">{{ emptyFanMessage }}</p>

    <p v-if="ord.customized.value" class="note"><button @click="ord.reset()">Réinitialiser l'ordre</button></p>
    <div class="cats">
    <section
      v-for="g in fanGroups"
      :key="g.id"
      class="cat"
      :class="{ wide: g.devices.some(hasEditor), dragging: ord.dragId.value === g.id, over: ord.overId.value === g.id && ord.dragId.value !== g.id }"
      @dragover="ord.onDragOver(g.id, $event)"
      @drop="ord.onDrop(g.id, $event)"
    >
      <h3>
        <CardHandle :label="g.label" @dragstart="ord.onDragStart(g.id, $event)" @dragend="ord.onDragEnd()" @move="ord.move(g.id, $event)" />
        {{ g.label }} <small>{{ g.hint }}</small>
      </h3>
      <div class="cards">
        <article v-for="d in g.devices" :key="d.id" class="fan-device" :class="{ wide: hasEditor(d) }">
          <div class="fan-device-head">
            <DeviceCard :device="d" compact />
            <span class="backend-tag">{{ d.backend }}</span>
          </div>
          <div class="fan-body">
            <div class="chan-list">
              <div v-for="fc in d.fan_channels" :key="fc.index" class="fan-row" :class="{ editing: editors[curveKey(d, fc.index)] }">
                <span class="fan-name">
                  {{ fc.name }}<template v-if="fc.rpm != null"> · <strong>{{ fc.rpm }} tr/min</strong></template>
                </span>
                <input
                  type="range"
                  min="20"
                  max="100"
                  step="5"
                  :value="duty(d, fc.index)"
                  @input="duties[key(d, fc.index)] = Number(($event.target as HTMLInputElement).value)"
                />
                <span class="fan-val">{{ duty(d, fc.index) }}%</span>
                <button @click="apply(d, fc.index)">Appliquer</button>
                <button :class="{ active: savedCurve(d, fc.index)?.enabled }" :disabled="tempSensors.length === 0" @click="toggleEditor(d, fc.index)">
                  {{ savedCurve(d, fc.index) ? "Courbe ✓" : "Courbe auto" }}
                </button>
              </div>
            </div>

            <div v-if="hasEditor(d)" class="editors">
              <template v-for="fc in d.fan_channels" :key="fc.index">
                <div v-if="editors[curveKey(d, fc.index)]" class="curve-editor">
                  <h4>Courbe : {{ fc.name }}</h4>
                  <label>
                    Capteur
                    <select v-model="editors[curveKey(d, fc.index)].sensor_id">
                      <option v-for="s in tempSensors" :key="s.id" :value="s.id">{{ s.hardware }} — {{ s.name }} ({{ s.value }} °C)</option>
                    </select>
                  </label>
                  <label>
                    Préréglage
                    <select @change="applyPreset(d, fc.index, ($event.target as HTMLSelectElement).value)">
                      <option value="">— choisir —</option>
                      <option v-for="p in CURVE_PRESETS" :key="p.id" :value="p.id">{{ p.label }}</option>
                    </select>
                  </label>
                  <div class="points">
                    <div v-for="(p, i) in editors[curveKey(d, fc.index)].points" :key="i" class="point-row">
                      <input type="number" v-model.number="p.temp" min="0" max="120" /> °C →
                      <input type="number" v-model.number="p.duty" min="0" max="100" /> %
                    </div>
                  </div>
                  <label class="enable-row">
                    <input type="checkbox" v-model="editors[curveKey(d, fc.index)].enabled" />
                    Courbe active
                  </label>
                  <div class="editor-actions">
                    <button class="primary" @click="saveCurve(d, fc.index)">Enregistrer</button>
                    <button v-if="d.fan_channels.length > 1" @click="saveCurveToAll(d, fc.index)">Enregistrer sur tous les canaux</button>
                    <button v-if="savedCurve(d, fc.index)" @click="deleteCurve(d, fc.index)">Supprimer</button>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </article>
      </div>
    </section>
    </div>

    <p v-if="devices.length" class="note foot">
      Minimum 20 % en manuel pour éviter l'arrêt d'une pompe AIO. PureRGB ne touche à un ventilateur que si vous appliquez
      un réglage ou activez une courbe. À la fermeture : cartes mères rendues au BIOS, Lian Li rendus à la synchro carte mère,
      NZXT restaurés à leur valeur d'avant ; les autres matériels (Thermaltake, Corsair Commander, liquidctl) gardent le dernier
      réglage appliqué.
    </p>
  </section>
</template>

<style scoped>
.fan-panel { flex: 1; min-width: 0; overflow-y: auto; padding: 22px 26px; }
.top { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: flex-start; gap: var(--space-3); margin-bottom: var(--space-4); }
.fan-panel h2 { font-size: 19px; }
.sub { color: var(--text-dim); font-size: 13px; margin: 6px 0 0; }
.sensor-strip { display: flex; flex-wrap: wrap; gap: 8px; justify-content: flex-end; }
.sensor-chip { font-size: 12px; color: var(--text-dim); border: 1px solid var(--border); border-radius: 999px; padding: 4px 12px; }
.empty { color: var(--text-dim); line-height: 1.7; }
.cats { display: grid; grid-auto-flow: dense; grid-template-columns: repeat(auto-fit, minmax(520px, 1fr)); gap: var(--space-5) var(--space-4); align-items: start; margin-bottom: var(--space-4); }
.cat { min-width: 0; }
.cat.dragging { opacity: 0.45; }
.cat.over { box-shadow: inset 0 3px 0 var(--accent); }
.cat.wide { grid-column: 1 / -1; }
.cat h3 { display: flex; align-items: center; font-size: 14px; margin-bottom: var(--space-3); padding-bottom: 6px; border-bottom: 1px solid var(--border); }
.cat h3 small { font-weight: 400; font-size: 12px; color: var(--text-dim); margin-left: 10px; }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(480px, 1fr)); gap: var(--space-3); align-items: start; }
.fan-device { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 16px; min-width: 0; }
.fan-device.wide { grid-column: 1 / -1; }
.fan-device-head { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-3); }
.backend-tag { font-size: 11px; color: var(--text-dim); border: 1px solid var(--border); border-radius: 999px; padding: 1px 8px; margin-left: 8px; }
.fan-body { display: grid; grid-template-columns: minmax(0, 1fr); gap: var(--space-4); }
.wide .fan-body { grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr); }
.chan-list { display: flex; flex-direction: column; gap: 10px; }
.fan-row { display: grid; grid-template-columns: 1fr 44px auto auto; grid-template-areas: "name name name name" "slider val ap cv"; align-items: center; gap: 4px 10px; padding: 6px 8px; border-radius: var(--radius-sm); }
.fan-row.editing { background: var(--accent-soft); }
.fan-name { grid-area: name; font-size: 13px; color: var(--text-dim); }
.fan-name strong { color: var(--text); font-weight: 600; }
.fan-row input[type="range"] { grid-area: slider; }
.fan-val { grid-area: val; font-size: 13px; text-align: right; }
.fan-row button { padding: 5px 10px; }
.fan-row button.active { border-color: var(--ok); color: var(--ok); }
.editors { display: flex; flex-direction: column; gap: var(--space-3); min-width: 0; }
.curve-editor { padding: 12px 14px; border: 1px dashed var(--border); border-radius: 8px; display: flex; flex-direction: column; gap: 8px; font-size: 13px; }
.curve-editor h4 { font-size: 12px; color: var(--accent); text-transform: uppercase; letter-spacing: 0.05em; }
.curve-editor select { margin-left: 8px; max-width: 100%; }
.points { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 6px; }
.point-row input { width: 64px; }
.enable-row { display: flex; align-items: center; gap: 8px; }
.editor-actions { display: flex; flex-wrap: wrap; gap: 8px; }
.note { font-size: 12px; color: var(--text-dim); font-style: italic; }
.note.foot { margin-top: var(--space-2); }
@media (max-width: 900px) { .wide .fan-body { grid-template-columns: minmax(0, 1fr); } }
</style>
