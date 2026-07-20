<script setup lang="ts">
import { computed } from "vue";
import type { DeviceInfo, EffectConfig } from "../types";
import { DEVICE_TYPE_LABELS } from "../types";
import { brandChip } from "../assets/brands";
import DeviceIcon from "./DeviceIcon.vue";

const props = defineProps<{
  device: DeviceInfo;
  effect?: EffectConfig;
  selected?: boolean;
  compact?: boolean;
  dense?: boolean;
}>();

const typeLabel = computed(
  () => DEVICE_TYPE_LABELS[props.device.device_type] ?? props.device.device_type,
);
const chip = computed(() => brandChip(props.device.vendor, props.device.name));
const isOff = computed(() => !props.effect || props.effect.kind === "off");
const swatch = computed(() => {
  const c = props.effect?.colors[0];
  return c ? `rgb(${c.r}, ${c.g}, ${c.b})` : "transparent";
});
</script>

<template>
  <button
    type="button"
    class="device-card"
    :class="{ selected, compact, dense, dim: !device.controllable }"
  >
    <span class="dc-icon-box">
      <DeviceIcon :type="device.device_type" />
      <span class="dc-chip" :style="{ background: chip.color }">{{ chip.text }}</span>
    </span>
    <span class="dc-body">
      <span class="dc-name">{{ device.name }}</span>
      <span class="dc-type">{{ typeLabel }}</span>
    </span>
    <span
      v-if="!compact"
      class="dc-swatch"
      :class="{ off: isOff }"
      :style="isOff ? {} : { background: swatch }"
    ></span>
    <span class="dc-status" :class="{ active: device.controllable }"></span>
  </button>
</template>

<style scoped>
.device-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-2);
  width: 100%;
  text-align: left;
  padding: var(--space-3);
  border: 1px solid var(--border);
  background: var(--bg-card);
  border-radius: var(--radius);
  transition: border-color var(--transition-fast), background var(--transition-fast), box-shadow var(--transition-fast);
}

.device-card:hover {
  border-color: var(--accent);
  box-shadow: var(--shadow-md);
}

.device-card.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.device-card.dim {
  opacity: 0.6;
}

.dc-icon-box {
  position: relative;
  width: 34px;
  height: 34px;
  color: var(--text-dim);
}

.dc-chip {
  position: absolute;
  right: -6px;
  bottom: -6px;
  min-width: 16px;
  height: 16px;
  padding: 0 3px;
  border-radius: var(--radius-sm);
  font-size: 9px;
  font-weight: 700;
  line-height: 16px;
  text-align: center;
  color: #fff;
  border: 1px solid var(--bg-card);
}

.dc-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}

.dc-name {
  font-weight: 600;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.dc-type {
  font-size: 11px;
  color: var(--text-dim);
}

.dc-swatch {
  width: 100%;
  height: 6px;
  border-radius: 999px;
  background: var(--border);
}

.dc-status {
  position: absolute;
  top: var(--space-3);
  right: var(--space-3);
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-dim);
}

.dc-status.active {
  background: var(--ok);
  box-shadow: 0 0 8px var(--ok);
}

/* Variante compacte : utilisée en ligne dans FanPanel/LcdPanel, pas de swatch,
   pas de marge externe — chaque consommateur gère son propre wrapper. */
.device-card.compact {
  flex-direction: row;
  align-items: center;
  border: none;
  background: none;
  padding: 0;
  gap: var(--space-2);
}

.device-card.compact:hover {
  box-shadow: none;
  border-color: transparent;
}

.device-card.compact .dc-icon-box {
  width: 22px;
  height: 22px;
}

.device-card.compact .dc-status {
  position: static;
  margin-left: var(--space-2);
}

/* Variante dense : ligne pleine largeur pour la disposition Liste, garde
   le chrome carte (bordure/fond) mais tout sur une seule ligne. */
.device-card.dense {
  flex-direction: row;
  align-items: center;
  padding: var(--space-2) var(--space-3);
  gap: var(--space-3);
}

.device-card.dense .dc-icon-box {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
}

.device-card.dense .dc-body {
  flex-direction: row;
  align-items: baseline;
  gap: var(--space-2);
  width: auto;
  flex: 1;
  min-width: 0;
}

.device-card.dense .dc-type {
  flex-shrink: 0;
}

.device-card.dense .dc-swatch {
  width: 40px;
  flex-shrink: 0;
}

.device-card.dense .dc-status {
  position: static;
}
</style>
