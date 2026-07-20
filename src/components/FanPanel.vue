<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import type { CurveConfig, DeviceInfo, HardwareDiagnostics, Sensor, Settings } from "../types";

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
  try {
    diag.value = await invoke<HardwareDiagnostics>("hardware_diagnostics");
  } catch {
    diag.value = null;
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
    <h2>Ventilateurs &amp; courbes</h2>
    <p class="sub">
      Vitesse fixe ou courbe automatique selon un capteur de température
      (capteurs via LibreHardwareMonitor, AIO/hubs via liquidctl et drivers natifs).
    </p>

    <div v-if="mainTemps.length" class="sensor-strip">
      <span v-for="s in mainTemps" :key="s.id" class="sensor-chip">
        {{ s.hardware }} · {{ s.name }} : <strong>{{ s.value }} °C</strong>
      </span>
    </div>
    <p v-else class="note">
      Capteurs indisponibles (sidecar sensord absent) — courbes inactives.
    </p>

    <p v-if="devices.length === 0" class="empty">
      {{ emptyFanMessage }}
    </p>

    <div v-for="d in devices" :key="d.id" class="fan-device">
      <h3>{{ d.name }} <span class="backend-tag">{{ d.backend }}</span></h3>
      <div v-for="fc in d.fan_channels" :key="fc.index" class="fan-block">
        <div class="fan-row">
          <span class="fan-name">
            {{ fc.name }}<template v-if="fc.rpm != null"> · {{ fc.rpm }} tr/min</template>
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
          <button
            :class="{ active: savedCurve(d, fc.index)?.enabled }"
            :disabled="tempSensors.length === 0"
            @click="editors[curveKey(d, fc.index)] ? delete editors[curveKey(d, fc.index)] : openEditor(d, fc.index)"
          >
            {{ savedCurve(d, fc.index) ? "Courbe ✓" : "Courbe auto" }}
          </button>
        </div>

        <div v-if="editors[curveKey(d, fc.index)]" class="curve-editor">
          <label>
            Capteur
            <select v-model="editors[curveKey(d, fc.index)].sensor_id">
              <option v-for="s in tempSensors" :key="s.id" :value="s.id">
                {{ s.hardware }} — {{ s.name }} ({{ s.value }} °C)
              </option>
            </select>
          </label>
          <div
            v-for="(p, i) in editors[curveKey(d, fc.index)].points"
            :key="i"
            class="point-row"
          >
            <input type="number" v-model.number="p.temp" min="0" max="120" /> °C →
            <input type="number" v-model.number="p.duty" min="0" max="100" /> %
          </div>
          <label class="enable-row">
            <input type="checkbox" v-model="editors[curveKey(d, fc.index)].enabled" />
            Courbe active
          </label>
          <div class="editor-actions">
            <button class="primary" @click="saveCurve(d, fc.index)">Enregistrer</button>
            <button v-if="savedCurve(d, fc.index)" @click="deleteCurve(d, fc.index)">
              Supprimer
            </button>
          </div>
        </div>
      </div>
      <p class="note">Minimum 20 % en manuel pour éviter l'arrêt d'une pompe AIO.</p>
    </div>
  </section>
</template>

<style scoped>
.fan-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.fan-panel h2 {
  font-size: 19px;
}

.sub {
  color: var(--text-dim);
  font-size: 13px;
  margin: 6px 0 14px;
}

.sensor-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 18px;
}

.sensor-chip {
  font-size: 12px;
  color: var(--text-dim);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 4px 12px;
}

.empty {
  color: var(--text-dim);
  line-height: 1.7;
}

.fan-device {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px 18px;
  margin-bottom: 14px;
  max-width: 680px;
}

.fan-device h3 {
  font-size: 14px;
  margin-bottom: 12px;
}

.backend-tag {
  font-size: 11px;
  color: var(--text-dim);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 1px 8px;
  margin-left: 8px;
}

.fan-block {
  margin-bottom: 10px;
}

.fan-row {
  display: grid;
  grid-template-columns: minmax(120px, 180px) 1fr 44px auto auto;
  align-items: center;
  gap: 10px;
}

.fan-name {
  font-size: 13px;
  color: var(--text-dim);
}

.fan-val {
  font-size: 13px;
  text-align: right;
}

.fan-row button.active {
  border-color: var(--ok);
  color: var(--ok);
}

.curve-editor {
  margin: 10px 0 4px;
  padding: 12px 14px;
  border: 1px dashed var(--border);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.curve-editor select {
  margin-left: 8px;
  max-width: 380px;
}

.point-row input {
  width: 64px;
}

.enable-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.editor-actions {
  display: flex;
  gap: 8px;
}

.note {
  font-size: 12px;
  color: var(--text-dim);
  font-style: italic;
  margin-top: 6px;
}
</style>
