<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ref } from "vue";
import type { DeviceInfo } from "../types";

const props = defineProps<{ devices: DeviceInfo[] }>();
const emit = defineEmits<{ toast: [msg: string] }>();

const busy = ref(false);
const brightness = ref(60);

async function run(d: DeviceInfo, kind: string, arg?: string) {
  busy.value = true;
  try {
    await invoke("lcd_apply", { deviceId: d.id, kind, arg: arg ?? null });
    emit("toast", `LCD ${d.name} : ${kind} appliqué`);
  } catch (e) {
    emit("toast", `LCD : ${e}`);
  } finally {
    busy.value = false;
  }
}

async function pickAndApply(d: DeviceInfo, kind: "static" | "gif") {
  const filters =
    kind === "gif"
      ? [{ name: "GIF animé", extensions: ["gif"] }]
      : [{ name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "webp"] }];
  const path = await open({ multiple: false, filters });
  if (typeof path === "string") {
    await run(d, kind, path);
  }
}
</script>

<template>
  <section class="lcd-panel">
    <h2>Écran LCD</h2>
    <p class="sub">
      Kraken Z / Kraken 2023 via liquidctl : température du liquide, image fixe
      ou GIF animé, luminosité, orientation.
    </p>

    <p v-if="props.devices.length === 0" class="empty">
      Aucun écran LCD détecté (Kraken Z / 2023 requis, via liquidctl).
    </p>

    <div v-for="d in props.devices" :key="d.id" class="lcd-device">
      <h3>{{ d.name }}</h3>
      <div class="row">
        <button class="primary" :disabled="busy" @click="run(d, 'liquid')">
          Température du liquide
        </button>
        <button :disabled="busy" @click="pickAndApply(d, 'static')">
          Image fixe…
        </button>
        <button :disabled="busy" @click="pickAndApply(d, 'gif')">GIF animé…</button>
      </div>
      <div class="row">
        <label>
          Luminosité
          <input type="range" min="0" max="100" step="10" v-model.number="brightness" />
          {{ brightness }}%
        </label>
        <button :disabled="busy" @click="run(d, 'brightness', String(brightness))">
          Appliquer
        </button>
      </div>
      <div class="row">
        <span class="dim">Orientation :</span>
        <button
          v-for="a in ['0', '90', '180', '270']"
          :key="a"
          :disabled="busy"
          @click="run(d, 'orientation', a)"
        >
          {{ a }}°
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.lcd-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.lcd-panel h2 {
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

.lcd-device {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px 18px;
  margin-bottom: 14px;
  max-width: 620px;
}

.lcd-device h3 {
  font-size: 14px;
  margin-bottom: 12px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.row label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.dim {
  color: var(--text-dim);
  font-size: 13px;
}
</style>
