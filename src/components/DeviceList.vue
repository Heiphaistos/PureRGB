<script setup lang="ts">
import { computed } from "vue";
import type { DeviceInfo } from "../types";
import { DEVICE_TYPE_LABELS } from "../types";

const props = defineProps<{
  devices: DeviceInfo[];
  selectedId: string | null;
}>();

defineEmits<{ select: [id: string] }>();

const grouped = computed(() => {
  const map = new Map<string, DeviceInfo[]>();
  for (const d of props.devices) {
    const key = DEVICE_TYPE_LABELS[d.device_type] ?? d.device_type;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(d);
  }
  return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
});
</script>

<template>
  <aside class="device-list">
    <p v-if="devices.length === 0" class="empty">
      Aucun appareil détecté.<br />
      Vérifiez qu'OpenRGB tourne (SDK Server activé) puis re-scannez.
    </p>
    <div v-for="[type, list] in grouped" :key="type" class="group">
      <h3>{{ type }}</h3>
      <button
        v-for="d in list"
        :key="d.id"
        class="device"
        :class="{ selected: d.id === selectedId, dim: !d.controllable }"
        @click="$emit('select', d.id)"
      >
        <span class="dev-name">{{ d.name }}</span>
        <span class="dev-meta">
          {{ d.led_count }} LED<template v-if="d.fan_channels.length">
            · {{ d.fan_channels.length }} ventilo</template
          >
        </span>
        <span class="dev-note">{{ d.note }}</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.device-list {
  width: 320px;
  min-width: 320px;
  overflow-y: auto;
  border-right: 1px solid var(--border);
  background: var(--bg-panel);
  padding: 14px;
}

.empty {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.6;
  padding: 12px;
}

.group h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-dim);
  margin: 14px 4px 8px;
}

.group:first-child h3 {
  margin-top: 0;
}

.device {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  text-align: left;
  margin-bottom: 6px;
  padding: 10px 12px;
}

.device.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.device.dim {
  opacity: 0.6;
}

.dev-name {
  font-weight: 600;
  font-size: 13px;
}

.dev-meta {
  font-size: 12px;
  color: var(--text-dim);
}

.dev-note {
  font-size: 11px;
  color: var(--text-dim);
  font-style: italic;
}
</style>
