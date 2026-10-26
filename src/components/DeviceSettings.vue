<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { reactive, ref } from "vue";
import type { DeviceInfo, DeviceSetting } from "../types";

const props = defineProps<{ device: DeviceInfo }>();

// Rien n'est envoyé à l'appareil tant que l'utilisateur n'a pas touché un réglage.
const values = reactive<Record<string, number>>({});
const msg = ref("");

const current = (s: DeviceSetting) => values[s.id] ?? s.default;

async function apply(s: DeviceSetting, v: number) {
  values[s.id] = v;
  try {
    await invoke("set_device_setting", { deviceId: props.device.id, setting: s.id, value: v });
    msg.value = "";
  } catch (e) {
    msg.value = String(e);
  }
}
</script>

<template>
  <div class="dset">
    <h3>Réglages de l'appareil</h3>
    <p class="sub">Appliqués à chaud, sans rien enregistrer dans l'appareil : ils reviennent à l'état d'origine à son extinction.</p>
    <div v-for="s in device.settings" :key="s.id" class="row">
      <label :for="`ds-${s.id}`">{{ s.label }}</label>
      <input
        v-if="s.kind === 'toggle'"
        :id="`ds-${s.id}`"
        type="checkbox"
        :checked="current(s) === 1"
        @change="apply(s, ($event.target as HTMLInputElement).checked ? 1 : 0)"
      />
      <template v-else-if="s.kind === 'slider'">
        <input
          :id="`ds-${s.id}`"
          type="range"
          :min="s.min"
          :max="s.max"
          :step="s.step"
          :value="current(s)"
          @change="apply(s, Number(($event.target as HTMLInputElement).value))"
        />
        <span class="val">{{ current(s) }} {{ s.unit }}</span>
      </template>
      <select v-else :id="`ds-${s.id}`" :value="current(s)" @change="apply(s, Number(($event.target as HTMLSelectElement).value))">
        <option v-for="(o, i) in s.options" :key="i" :value="i">{{ o }}</option>
      </select>
    </div>
    <p v-if="msg" class="warn-note">{{ msg }}</p>
  </div>
</template>

<style scoped>
.dset { margin-top: 16px; }
.dset h3 { font-size: 15px; }
.sub { color: var(--text-dim); font-size: 12px; margin: 4px 0 10px; }
.row { display: flex; align-items: center; gap: 12px; padding: 4px 0; font-size: 13px; }
.row label { min-width: 180px; }
.row input[type="range"] { flex: 1; max-width: 320px; }
.val { min-width: 80px; font-variant-numeric: tabular-nums; color: var(--text-dim); }
</style>
