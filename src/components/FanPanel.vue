<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { reactive } from "vue";
import type { DeviceInfo } from "../types";

defineProps<{ devices: DeviceInfo[] }>();
const emit = defineEmits<{ toast: [msg: string] }>();

// duty local par "deviceId:channel"
const duties = reactive<Record<string, number>>({});

function key(d: DeviceInfo, ch: number) {
  return `${d.id}:${ch}`;
}

function duty(d: DeviceInfo, ch: number): number {
  return duties[key(d, ch)] ?? 50;
}

async function apply(d: DeviceInfo, ch: number) {
  const percent = duty(d, ch);
  try {
    await invoke("set_fan_duty", { deviceId: d.id, channel: ch, percent });
    emit("toast", `${d.name} — canal ${ch + 1} à ${percent}%`);
  } catch (e) {
    emit("toast", `Échec : ${e}`);
  }
}
</script>

<template>
  <section class="fan-panel">
    <h2>Contrôle ventilateurs</h2>
    <p class="sub">
      Vitesse fixe par canal (PWM). Disponible sur les hubs avec driver natif
      activé (NZXT Smart Device V2 / RGB &amp; Fan Controller).
    </p>

    <p v-if="devices.length === 0" class="empty">
      Aucun hub ventilateur pilotable détecté.<br />
      Activez les drivers natifs dans Réglages si votre hub est branché.
    </p>

    <div v-for="d in devices" :key="d.id" class="fan-device">
      <h3>{{ d.name }}</h3>
      <div v-for="fc in d.fan_channels" :key="fc.index" class="fan-row">
        <span class="fan-name">{{ fc.name }}</span>
        <input
          type="range"
          min="20"
          max="100"
          step="5"
          :value="duty(d, fc.index)"
          @input="duties[key(d, fc.index)] = Number(($event.target as HTMLInputElement).value)"
        />
        <span class="fan-val">{{ duty(d, fc.index) }}%</span>
        <button @click="apply(d, fc.index)">Appliquer</button>
      </div>
      <p class="note">
        Minimum 20% pour éviter l'arrêt d'une pompe AIO par erreur.
      </p>
    </div>
  </section>
</template>

<style scoped>
.fan-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.fan-panel h2 {
  font-size: 19px;
}

.sub {
  color: var(--text-dim);
  font-size: 13px;
  margin: 6px 0 20px;
}

.empty {
  color: var(--text-dim);
  line-height: 1.7;
}

.fan-device {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px 18px;
  margin-bottom: 14px;
  max-width: 620px;
}

.fan-device h3 {
  font-size: 14px;
  margin-bottom: 12px;
}

.fan-row {
  display: grid;
  grid-template-columns: 130px 1fr 48px auto;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}

.fan-name {
  font-size: 13px;
  color: var(--text-dim);
}

.fan-val {
  font-size: 13px;
  text-align: right;
}

.note {
  font-size: 12px;
  color: var(--text-dim);
  font-style: italic;
  margin-top: 6px;
}
</style>
