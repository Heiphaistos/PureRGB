<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import type { Color, DeviceInfo, EffectConfig } from "../types";
import EffectPanel from "./EffectPanel.vue";

const props = defineProps<{
  open: boolean;
  device: DeviceInfo | null;
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
  close: [];
}>();

const panelRef = ref<HTMLElement | null>(null);

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && props.open) emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
watch(
  () => props.open,
  (v) => {
    if (v) requestAnimationFrame(() => panelRef.value?.focus());
  },
);

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
  <Transition name="drawer-backdrop">
    <div v-if="open" class="drawer-backdrop" @click="emit('close')"></div>
  </Transition>
  <Transition name="drawer-slide">
    <aside v-if="open" ref="panelRef" tabindex="-1" class="effect-drawer">
      <button type="button" class="drawer-close" aria-label="Fermer" @click="emit('close')">
        ✕
      </button>
      <EffectPanel
        :device="device"
        :saved-effects="savedEffects"
        @apply="onApply"
        @apply-all="onApplyAll"
        @apply-mode="onApplyMode"
        @toast="onToast"
        @refresh="onRefresh"
      />
    </aside>
  </Transition>
</template>

<style scoped>
.drawer-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
  z-index: 40;
}

.effect-drawer {
  position: fixed;
  top: 0;
  right: 0;
  height: 100vh;
  width: 420px;
  background: var(--bg-panel);
  border-left: 1px solid var(--border);
  box-shadow: var(--shadow-drawer);
  z-index: 41;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.effect-drawer:deep(.effect-panel) {
  width: 100%;
}

.drawer-close {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 1;
}

.drawer-slide-enter-active,
.drawer-slide-leave-active {
  transition: transform var(--transition-drawer);
}

.drawer-slide-enter-from,
.drawer-slide-leave-to {
  transform: translateX(100%);
}

.drawer-backdrop-enter-active,
.drawer-backdrop-leave-active {
  transition: opacity var(--transition-fast);
}

.drawer-backdrop-enter-from,
.drawer-backdrop-leave-to {
  opacity: 0;
}
</style>
