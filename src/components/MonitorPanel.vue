<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import type { DeviceInfo, RamTempStatus, Sensor } from "../types";
import { formatValue, groupSensors, levelOf, pushHistory, sparkPoints } from "../monitor";

const props = defineProps<{ devices: DeviceInfo[] }>();

const sensors = ref<Sensor[]>([]);
const failed = ref(false);
const ramStatus = ref<RamTempStatus | null>(null);
const query = ref("");
const hist = reactive<Record<string, number[]>>({});
let timer: ReturnType<typeof setInterval> | undefined;

const groups = computed(() => groupSensors(sensors.value, query.value));
const fanDevices = computed(() => props.devices.filter((d) => d.fan_channels.length > 0));
const hottest = computed(() =>
  sensors.value.filter((s) => s.type === "Temperature").reduce<Sensor | null>((m, s) => (!m || s.value > m.value ? s : m), null),
);

function range(id: string): string {
  const h = hist[id];
  return h && h.length > 1 ? `min ${Math.min(...h).toFixed(0)} · max ${Math.max(...h).toFixed(0)}` : "";
}

async function refresh() {
  try {
    sensors.value = await invoke<Sensor[]>("get_sensors");
    pushHistory(hist, sensors.value);
    ramStatus.value = await invoke<RamTempStatus>("get_ram_temp_status");
    failed.value = false;
  } catch {
    failed.value = true;
  }
}

onMounted(() => {
  refresh();
  timer = setInterval(refresh, 2000);
});
onUnmounted(() => clearInterval(timer));
</script>

<template>
  <section class="monitor">
    <h2>Surveillance</h2>
    <p class="sub">
      Tous les capteurs de la machine (processeur, carte graphique, carte mère, disques, ventilateurs, tensions, puissance), lecture seule,
      mis à jour toutes les 2 s. Les réglages de ventilation sont dans l'onglet Ventilateurs.
    </p>

    <div class="bar">
      <input v-model="query" type="search" placeholder="Filtrer (ex. CPU, GPU, NVMe, tension)" />
      <span v-if="hottest" class="hot-chip" :class="levelOf(hottest)">
        Plus chaud : {{ hottest.hardware }} · {{ hottest.name }} — {{ formatValue(hottest) }}
      </span>
    </div>

    <p v-if="failed || sensors.length === 0" class="note">
      Aucun capteur disponible : le module de lecture (sensord) n'est pas démarré ou la machine n'est pas prise en charge.
    </p>

    <p v-if="ramStatus && ramStatus.state !== 'ok'" class="note">{{ ramStatus.message }}</p>

    <div v-for="g in groups" :key="g.hardware" class="hw">
      <h3>{{ g.hardware }}</h3>
      <div v-for="t in g.types" :key="t.type" class="type">
        <h4>{{ t.label }}</h4>
        <div v-for="s in t.items" :key="s.id" class="row" :class="levelOf(s)">
          <span class="name">{{ s.name }}</span>
          <svg v-if="hist[s.id] && hist[s.id].length > 1" class="spark" viewBox="0 0 80 20" preserveAspectRatio="none">
            <polyline :points="sparkPoints(hist[s.id])" fill="none" stroke="currentColor" stroke-width="1.5" />
          </svg>
          <span v-else class="spark" />
          <span class="range">{{ range(s.id) }}</span>
          <strong class="val">{{ formatValue(s) }}</strong>
        </div>
      </div>
    </div>

    <div v-if="fanDevices.length" class="hw">
      <h3>Hubs, AIO et ventilateurs pilotés par PureRGB</h3>
      <div v-for="d in fanDevices" :key="d.id" class="type">
        <h4>{{ d.name }}</h4>
        <div v-for="fc in d.fan_channels" :key="fc.index" class="row">
          <span class="name">{{ fc.name }}</span>
          <span class="spark" />
          <span class="range">{{ fc.duty_percent != null ? `commande ${fc.duty_percent} %` : "" }}</span>
          <strong class="val">{{ fc.rpm != null ? `${fc.rpm} tr/min` : "—" }}</strong>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.monitor { flex: 1; overflow-y: auto; padding: 22px 26px; }
.monitor h2 { font-size: 19px; }
.sub { color: var(--text-dim); font-size: 13px; margin: 6px 0 14px; }
.bar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; margin-bottom: 16px; }
.bar input { min-width: 260px; }
.hot-chip { font-size: 12px; border: 1px solid var(--border); border-radius: 999px; padding: 4px 12px; color: var(--text-dim); }
.hot-chip.warn, .row.warn .val { color: #ffb020; }
.hot-chip.hot, .row.hot .val { color: #ff4d4d; }
.note { color: var(--text-dim); }
.hw { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 18px; margin-bottom: 14px; max-width: 760px; }
.hw h3 { font-size: 15px; margin-bottom: 8px; }
.type h4 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); margin: 10px 0 4px; }
.row { display: grid; grid-template-columns: 1fr 80px 120px 96px; align-items: center; gap: 10px; padding: 3px 0; font-size: 13px; }
.spark { width: 80px; height: 20px; color: var(--accent, #ff2bd6); opacity: 0.8; }
.range { font-size: 11px; color: var(--text-dim); }
.val { text-align: right; font-variant-numeric: tabular-nums; }
@media (max-width: 760px) {
  .row { grid-template-columns: 1fr 96px; }
  .spark, .range { display: none; }
}
</style>
