<script setup lang="ts">
import { computed, ref } from "vue";
import type { Color, DeviceInfo, EffectConfig } from "../types";
import { DEVICE_TYPE_LABELS } from "../types";
import DeviceCard from "./DeviceCard.vue";
import EffectPanel from "./EffectPanel.vue";

const props = defineProps<{
  devices: DeviceInfo[];
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

const expandedId = ref<string | null>(null);

function toggle(id: string) {
  expandedId.value = expandedId.value === id ? null : id;
}

const expandedDevice = computed(
  () => props.devices.find((d) => d.id === expandedId.value) ?? null,
);

const grouped = computed(() => {
  const map = new Map<string, DeviceInfo[]>();
  for (const d of props.devices) {
    const key = DEVICE_TYPE_LABELS[d.device_type] ?? d.device_type;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(d);
  }
  return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
});

function onApply(deviceId: string, config: EffectConfig, zone: number | null) {
  emit("apply", deviceId, config, zone);
}
function onApplyAll(config: EffectConfig) {
  emit("applyAll", config);
}
function onApplyMode(
  deviceId: string,
  modeIndex: number,
  speed: number | null,
  direction: number | null,
  colors: Color[] | null,
) {
  emit("applyMode", deviceId, modeIndex, speed, direction, colors);
}
function onToast(msg: string) {
  emit("toast", msg);
}
function onRefresh() {
  emit("refresh");
}
</script>

<template>
  <section class="device-canvas">
    <p v-if="devices.length === 0" class="empty">
      Aucun appareil détecté.<br />
      Vérifiez qu'OpenRGB tourne (SDK Server activé) puis re-scannez.
    </p>
    <div v-for="[type, list] in grouped" :key="type" class="group">
      <h3>{{ type }}</h3>
      <div class="canvas-grid">
        <div v-for="d in list" :key="d.id" class="tile" :class="{ expanded: expandedId === d.id }">
          <DeviceCard
            :device="d"
            :effect="savedEffects[d.id]"
            :selected="expandedId === d.id"
            @click="toggle(d.id)"
          />
          <div v-if="expandedId === d.id" class="tile-panel">
            <EffectPanel
              :device="expandedDevice"
              :saved-effects="savedEffects"
              @apply="onApply"
              @apply-all="onApplyAll"
              @apply-mode="onApplyMode"
              @toast="onToast"
              @refresh="onRefresh"
            />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.device-canvas {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
}

.group h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-dim);
  margin: var(--space-4) 0 var(--space-2);
}

.group:first-child h3 {
  margin-top: 0;
}

.canvas-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-3);
  align-items: start;
}

.tile {
  display: flex;
  flex-direction: column;
}

.tile.expanded {
  grid-column: 1 / -1;
}

.tile :deep(.device-card) {
  min-height: 120px;
}

.tile-panel {
  margin-top: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.tile-panel :deep(.effect-panel) {
  max-height: 60vh;
}

.empty {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.6;
  padding: var(--space-3);
}
</style>
