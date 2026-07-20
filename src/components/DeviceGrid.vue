<script setup lang="ts">
import { computed } from "vue";
import type { DeviceInfo, EffectConfig } from "../types";
import { DEVICE_TYPE_LABELS } from "../types";
import DeviceCard from "./DeviceCard.vue";

const props = defineProps<{
  devices: DeviceInfo[];
  selectedId: string | null;
  effects?: Record<string, EffectConfig>;
  dense?: boolean;
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
  <section class="device-grid">
    <p v-if="devices.length === 0" class="empty">
      Aucun appareil détecté.<br />
      Vérifiez qu'OpenRGB tourne (SDK Server activé) puis re-scannez.
    </p>
    <div v-for="[type, list] in grouped" :key="type" class="group">
      <h3>{{ type }}</h3>
      <div :class="dense ? 'list' : 'grid'">
        <DeviceCard
          v-for="d in list"
          :key="d.id"
          :device="d"
          :effect="effects?.[d.id]"
          :selected="d.id === selectedId"
          :dense="dense"
          @click="$emit('select', d.id)"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.device-grid {
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

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--space-3);
}

.list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.empty {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.6;
  padding: var(--space-3);
}
</style>
