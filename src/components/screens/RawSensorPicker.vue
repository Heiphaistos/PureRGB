<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { errMsg, screenSensors } from "../../screens/api.ts";
import { sensorUnit } from "../../screens/sourceCatalog.ts";
import type { RawSensor } from "../../screens/types.ts";
import "./screens.css";

/** Liste filtrable des capteurs détectés, valeurs rafraîchies toutes les 2 s. Choisir = `sensor:<id>`. */
const props = defineProps<{ current: string }>();
const emit = defineEmits<{ pick: [key: string] }>();

const sensors = ref<RawSensor[]>([]);
const filter = ref("");
const error = ref("");
let timer: ReturnType<typeof setInterval> | undefined;

async function refresh(): Promise<void> {
  try {
    sensors.value = (await screenSensors()).sensors;
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

const shown = computed(() => {
  const words = filter.value.toLowerCase().split(/\s+/).filter(Boolean);
  return sensors.value.filter((s) => {
    const hay = `${s.hardware} ${s.name} ${s.type} ${s.id}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
});

const fmt = (s: RawSensor): string => `${Number.isInteger(s.value) ? s.value : s.value.toFixed(1)} ${sensorUnit(s.type)}`.trim();
</script>

<template>
  <div class="raw">
    <input v-model="filter" type="text" placeholder="Filtrer : cpu, temp, nvme, fan…" spellcheck="false" />
    <div v-if="error" class="sc-err">{{ error }}</div>
    <p v-else-if="!sensors.length" class="sc-sub">Aucun capteur détecté pour l'instant (sensord démarre en quelques secondes).</p>
    <ul v-else class="list">
      <li
        v-for="(s, i) in shown"
        :key="`${s.id}#${i}`"
        :class="{ on: props.current === `sensor:${s.id}` }"
        :title="s.id"
        @click="emit('pick', `sensor:${s.id}`)"
      >
        <span class="n">{{ s.name }}<small>{{ s.hardware }} · {{ s.type }}</small></span>
        <span class="v">{{ fmt(s) }}</span>
      </li>
      <li v-if="!shown.length" class="none">Aucun capteur ne correspond.</li>
    </ul>
  </div>
</template>

<style scoped>
.raw {
  margin: 0 0 10px;
}

.raw input {
  width: 100%;
  margin-bottom: 6px;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 220px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}

li {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding: 5px 8px;
  font-size: 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
}

li:hover {
  background: var(--bg-card);
}

li.on {
  background: var(--accent-soft);
  color: var(--accent);
}

li.none {
  cursor: default;
  color: var(--text-dim);
}

.n {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

small {
  color: var(--text-dim);
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.v {
  color: var(--ok);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
</style>
