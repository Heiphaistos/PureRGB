<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { errMsg, screenSensors, screenValues } from "../../screens/api.ts";
import { formatValue } from "../../screens/format.ts";
import type { LiveValues, SensorsReport } from "../../screens/types.ts";
import { SOURCES } from "../../screens/widgets.ts";
import "./screens.css";

/** Diagnostic : sensord tourne-t-il, combien de capteurs, quelle clé lit quel capteur. */
const report = ref<SensorsReport | null>(null);
const values = ref<LiveValues>({});
const error = ref("");
let timer: ReturnType<typeof setInterval> | undefined;

async function refresh(): Promise<void> {
  try {
    const [r, v] = await Promise.all([screenSensors(), screenValues()]);
    report.value = r;
    values.value = v;
    error.value = "";
  } catch (e) {
    error.value = errMsg(e);
  }
}

onMounted(() => {
  void refresh();
  timer = setInterval(() => void refresh(), 2000);
});
onUnmounted(() => clearInterval(timer));

const unitOf = (key: string): string => SOURCES.find((s) => s.key === key)?.unit ?? "";
const rows = computed(() =>
  Object.entries(report.value?.used ?? {})
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, u]) => ({
      key,
      sensor: u.id ? `${u.name} — ${u.hardware}` : u.name,
      value: values.value[key] === undefined ? "--" : `${formatValue("{:.1}", values.value[key])} ${unitOf(key)}`.trim(),
    })),
);
// Clés matérielles du catalogue sans capteur sur cette machine.
const SYSTEM_GROUPS = new Set(["Mémoire", "Réseau", "Heure"]);
const missing = computed(() =>
  SOURCES.filter((s) => !SYSTEM_GROUPS.has(s.group) && !s.key.startsWith("disk.") && values.value[s.key] === undefined).map((s) => s.key),
);
</script>

<template>
  <div class="sc-card box">
    <div class="sc-row head">
      <div class="sc-grow">
        <strong>Capteurs détectés</strong>
        <p class="sc-sub">Ce que PureRGB lit sur votre PC, et quel capteur alimente chaque source des widgets.</p>
      </div>
      <span v-if="report" class="sc-badge" :class="report.sensord_running ? 'ok' : 'err'">
        {{ report.sensord_running ? `sensord démarré · ${report.sensor_count} capteurs` : "sensord arrêté" }}
      </span>
    </div>
    <div v-if="error" class="sc-err">{{ error }}</div>
    <template v-if="report">
      <p v-if="!report.sensord_running" class="sc-sub">
        Températures, ventilateurs et fréquences viennent de sensord (LibreHardwareMonitor), lancé avec PureRGB en administrateur.
        {{ report.exe_path ? "" : "Le fichier sensord.exe est introuvable ou n'a pas pu être téléchargé." }}
        Seuls la mémoire, les disques, le réseau et l'heure restent disponibles.
      </p>
      <p v-else-if="!report.sensor_count" class="sc-sub">sensord tourne mais n'a encore rien remonté : patientez quelques secondes.</p>
      <table v-if="rows.length" class="map">
        <thead>
          <tr><th>Source</th><th>Capteur utilisé</th><th>Valeur</th></tr>
        </thead>
        <tbody>
          <tr v-for="r in rows" :key="r.key">
            <td><code>{{ r.key }}</code></td>
            <td>{{ r.sensor }}</td>
            <td class="v">{{ r.value }}</td>
          </tr>
        </tbody>
      </table>
      <p v-if="report.sensor_count && missing.length" class="sc-sub">
        Sans capteur sur ce PC : <code>{{ missing.join(" · ") }}</code>. Utilisez « Capteur brut… » dans un widget pour cibler n'importe quel capteur de la liste.
      </p>
    </template>
  </div>
</template>

<style scoped>
.box {
  padding: 16px 18px;
}

.head {
  margin-bottom: 8px;
}

.map {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  margin-top: 8px;
}

th {
  text-align: left;
  color: var(--text-dim);
  font-weight: 500;
  padding: 3px 6px;
}

td {
  padding: 3px 6px;
  border-top: 1px solid var(--border);
}

.v {
  text-align: right;
  color: var(--ok);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
</style>
