<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import type { DeviceInfo, RamTempStatus, Sensor } from "../types";
import { formatValue, gaugePercent, groupByCategory, headlineTemp, levelOf, pushHistory, sparkPoints } from "../monitor";
import type { CategoryId } from "../monitor";

const props = defineProps<{ devices: DeviceInfo[] }>();

const sensors = ref<Sensor[]>([]);
const failed = ref(false);
const ramStatus = ref<RamTempStatus | null>(null);
const query = ref("");
const hist = reactive<Record<string, number[]>>({});
let timer: ReturnType<typeof setInterval> | undefined;

const cards = computed(() => groupByCategory(sensors.value, query.value));
const fanDevices = computed(() => props.devices.filter((d) => d.fan_channels.length > 0));
const hottest = computed(() =>
  sensors.value.filter((s) => s.type === "Temperature").reduce<Sensor | null>((m, s) => (!m || s.value > m.value ? s : m), null),
);
const tiles = computed(() =>
  ([["cpu", "Processeur"], ["gpu", "Carte graphique"], ["board", "Carte mère"], ["disk", "Disques"]] as [CategoryId, string][]).map(([id, label]) => ({
    id,
    label,
    sensor: headlineTemp(sensors.value, id),
  })),
);
const hasFanCard = computed(() => cards.value.some((c) => c.id === "fans"));
const showFanDevices = computed(() => fanDevices.value.length > 0 && (!query.value.trim() || hasFanCard.value));

function range(id: string): string {
  const h = hist[id];
  return h && h.length > 1 ? `${Math.min(...h).toFixed(0)}–${Math.max(...h).toFixed(0)}` : "";
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
    <header class="top">
      <div>
        <h2>Surveillance</h2>
        <p class="sub">Tous les capteurs de la machine, en lecture seule, mis à jour toutes les 2 s. Les réglages sont dans l'onglet Ventilateurs.</p>
      </div>
      <input v-model="query" type="search" placeholder="Filtrer (ex. CPU, GPU, NVMe, tension)" />
    </header>

    <p v-if="failed || sensors.length === 0" class="note">
      Aucun capteur disponible : le module de lecture (sensord) n'est pas démarré ou la machine n'est pas prise en charge.
    </p>
    <p v-if="ramStatus && ramStatus.state !== 'ok'" class="note">{{ ramStatus.message }}</p>

    <div v-if="sensors.length" class="summary">
      <div v-if="hottest" class="tile hottest" :class="levelOf(hottest)">
        <span class="t-label">Plus chaud</span>
        <strong class="t-val">{{ formatValue(hottest) }}</strong>
        <span class="t-sub">{{ hottest.hardware }} · {{ hottest.name }}</span>
      </div>
      <div v-for="t in tiles" :key="t.id" class="tile" :class="t.sensor ? levelOf(t.sensor) : ''">
        <span class="t-label">{{ t.label }}</span>
        <template v-if="t.sensor">
          <strong class="t-val">{{ formatValue(t.sensor) }}</strong>
          <span class="t-sub">{{ t.sensor.name }}</span>
          <svg v-if="hist[t.sensor.id] && hist[t.sensor.id].length > 1" class="t-spark" viewBox="0 0 80 20" preserveAspectRatio="none">
            <polyline :points="sparkPoints(hist[t.sensor.id])" fill="none" stroke="currentColor" stroke-width="1.5" vector-effect="non-scaling-stroke" />
          </svg>
        </template>
        <strong v-else class="t-val none">—</strong>
      </div>
    </div>

    <div class="cards">
      <article v-for="c in cards" :key="c.id" class="card">
        <h3>{{ c.label }} <small>{{ c.count }}</small></h3>
        <template v-for="sec in c.sections" :key="sec.title">
          <h4>{{ sec.title }}</h4>
          <div v-for="s in sec.items" :key="s.id" class="row" :class="levelOf(s)">
            <span class="name" :title="`${s.hardware} · ${s.name}`">{{ s.name }}</span>
            <svg v-if="hist[s.id] && hist[s.id].length > 1" class="spark" viewBox="0 0 80 20" preserveAspectRatio="none">
              <polyline :points="sparkPoints(hist[s.id])" fill="none" stroke="currentColor" stroke-width="1.5" vector-effect="non-scaling-stroke" />
            </svg>
            <span v-else class="spark" />
            <span class="range">{{ range(s.id) }}</span>
            <strong class="val">{{ formatValue(s) }}</strong>
            <span v-if="gaugePercent(s) != null" class="gauge"><i :style="{ width: gaugePercent(s) + '%' }" /></span>
          </div>
        </template>
        <template v-if="c.id === 'fans' && showFanDevices">
          <template v-for="d in fanDevices" :key="d.id">
            <h4>{{ d.name }} (piloté par PureRGB)</h4>
            <div v-for="fc in d.fan_channels" :key="fc.index" class="row">
              <span class="name">{{ fc.name }}</span>
              <span class="spark" />
              <span class="range">{{ fc.duty_percent != null ? `${fc.duty_percent} %` : "" }}</span>
              <strong class="val">{{ fc.rpm != null ? `${fc.rpm} tr/min` : "—" }}</strong>
            </div>
          </template>
        </template>
      </article>

      <article v-if="showFanDevices && !hasFanCard" class="card">
        <h3>Ventilateurs <small>{{ fanDevices.length }}</small></h3>
        <template v-for="d in fanDevices" :key="d.id">
          <h4>{{ d.name }} (piloté par PureRGB)</h4>
          <div v-for="fc in d.fan_channels" :key="fc.index" class="row">
            <span class="name">{{ fc.name }}</span>
            <span class="spark" />
            <span class="range">{{ fc.duty_percent != null ? `${fc.duty_percent} %` : "" }}</span>
            <strong class="val">{{ fc.rpm != null ? `${fc.rpm} tr/min` : "—" }}</strong>
          </div>
        </template>
      </article>
    </div>
  </section>
</template>

<style scoped>
.monitor { flex: 1; min-width: 0; overflow-y: auto; padding: 22px 26px; }
.top { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: flex-start; gap: var(--space-3); margin-bottom: var(--space-4); }
.monitor h2 { font-size: 19px; }
.sub { color: var(--text-dim); font-size: 13px; margin-top: 6px; }
.top input { min-width: 280px; }
.note { color: var(--text-dim); margin-bottom: var(--space-3); }
.summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-5); }
.tile { position: relative; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 12px 14px; display: flex; flex-direction: column; gap: 2px; min-width: 0; overflow: hidden; color: var(--ok); }
.tile.warn { color: var(--warn); border-color: var(--warn); }
.tile.hot { color: var(--err); border-color: var(--err); }
.tile.hottest { background: var(--accent-soft); border-color: var(--accent); }
.t-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); }
.t-val { font-size: 24px; font-variant-numeric: tabular-nums; }
.t-val.none { color: var(--text-dim); }
.t-sub { font-size: 12px; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.t-spark { width: 100%; height: 26px; margin-top: 6px; opacity: 0.85; }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(420px, 1fr)); gap: var(--space-3); align-items: start; }
.card { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 16px; min-width: 0; }
.card h3 { font-size: 15px; margin-bottom: 4px; }
.card h3 small { font-weight: 400; font-size: 11px; color: var(--text-dim); margin-left: 6px; }
.card h4 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); margin: 12px 0 4px; }
.row { display: grid; grid-template-columns: minmax(0, 1fr) 64px 76px 84px; grid-template-areas: "n s r v" "g g g g"; align-items: center; gap: 0 8px; padding: 3px 0; font-size: 13px; }
.name { grid-area: n; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spark { grid-area: s; width: 64px; height: 18px; color: var(--accent); opacity: 0.8; }
.range { grid-area: r; white-space: nowrap; font-size: 11px; color: var(--text-dim); text-align: right; }
.val { grid-area: v; text-align: right; font-variant-numeric: tabular-nums; }
.gauge { grid-area: g; height: 3px; background: var(--border); border-radius: 2px; margin-top: 2px; overflow: hidden; }
.gauge i { display: block; height: 100%; background: var(--ok); transition: width 0.4s; }
.row.warn .val { color: var(--warn); }
.row.hot .val { color: var(--err); }
.row.warn .gauge i { background: var(--warn); }
.row.hot .gauge i { background: var(--err); }
@media (max-width: 760px) {
  .cards { grid-template-columns: minmax(0, 1fr); }
  .range { display: none; }
}
</style>
