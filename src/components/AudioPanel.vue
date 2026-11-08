<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import type { DeviceInfo } from "../types";

interface Endpoint {
  id: string;
  name: string;
  flow: "sortie" | "entree";
  volume: number;
  muted: boolean;
  is_default: boolean;
  score: number;
}

const props = defineProps<{ device: DeviceInfo }>();
const all = ref<Endpoint[]>([]);
const showAll = ref(false);
const msg = ref("");

const mine = computed(() => all.value.filter((e) => e.score >= 2));
const shown = computed(() => (showAll.value || mine.value.length === 0 ? all.value : mine.value));

async function refresh() {
  try {
    all.value = await invoke<Endpoint[]>("audio_list", { deviceName: props.device.name });
    msg.value = "";
  } catch (e) {
    msg.value = String(e);
  }
}

async function apply(e: Endpoint, change: { volume?: number; muted?: boolean }) {
  try {
    await invoke("audio_set", { id: e.id, ...change });
    if (change.volume !== undefined) e.volume = change.volume;
    if (change.muted !== undefined) e.muted = change.muted;
    msg.value = "";
  } catch (err) {
    msg.value = String(err);
  }
}

onMounted(refresh);
</script>

<template>
  <div class="audio">
    <h3>Audio Windows</h3>
    <p class="sub">
      Volume et sourdine des sorties et micros, comme le mélangeur de Windows.
      <template v-if="mine.length === 0">Aucune entrée reconnue pour cet appareil : toutes sont affichées.</template>
    </p>
    <div v-for="e in shown" :key="e.id" class="row">
      <span class="kind">{{ e.flow === "sortie" ? "Sortie" : "Micro" }}</span>
      <span class="name">{{ e.name }}<em v-if="e.is_default"> (par défaut)</em></span>
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        :value="e.volume"
        :aria-label="`Volume ${e.name}`"
        @change="apply(e, { volume: Number(($event.target as HTMLInputElement).value) })"
      />
      <span class="val">{{ e.volume }} %</span>
      <label class="mute"><input type="checkbox" :checked="e.muted" @change="apply(e, { muted: ($event.target as HTMLInputElement).checked })" /> Muet</label>
    </div>
    <p v-if="all.length === 0 && !msg" class="sub">Aucun périphérique audio actif.</p>
    <div class="actions">
      <button v-if="mine.length > 0 && all.length > mine.length" @click="showAll = !showAll">
        {{ showAll ? "Seulement ceux de cet appareil" : "Afficher tous les périphériques audio" }}
      </button>
      <button @click="refresh">Actualiser</button>
    </div>
    <p v-if="msg" class="warn-note">{{ msg }}</p>
  </div>
</template>

<style scoped>
.audio { margin-top: 16px; }
.audio h3 { font-size: 15px; }
.sub { color: var(--text-dim); font-size: 12px; margin: 4px 0 10px; }
.row { display: flex; align-items: center; gap: 12px; padding: 4px 0; font-size: 13px; flex-wrap: wrap; }
.kind { min-width: 52px; color: var(--text-dim); }
.name { min-width: 200px; flex: 1 1 200px; }
.row input[type="range"] { flex: 1 1 140px; max-width: 260px; }
.val { min-width: 48px; font-variant-numeric: tabular-nums; color: var(--text-dim); }
.mute { display: flex; gap: 4px; align-items: center; }
.actions { display: flex; gap: 8px; margin-top: 8px; }
</style>
