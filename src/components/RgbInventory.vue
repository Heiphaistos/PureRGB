<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { errMsg } from "../screens/api";

interface Finding {
  category: string;
  status: "ok" | "info" | "warn";
  title: string;
  detail: string;
}

const props = defineProps<{ telemetryOptIn: boolean }>();
const emit = defineEmits<{ restart: [] }>();

const findings = ref<Finding[] | null>(null);
const busy = ref(false);
const error = ref("");
const sent = ref("");
const sending = ref(false);
const sentOk = ref(false);

const ICON = { ok: "✓", info: "i", warn: "!" } as const;

async function run(): Promise<void> {
  busy.value = true;
  error.value = "";
  try {
    findings.value = await invoke<Finding[]>("rgb_inventory");
    sent.value = "";
    if (props.telemetryOptIn) await send();
  } catch (e) {
    error.value = errMsg(e);
  } finally {
    busy.value = false;
  }
}

/** Envoie le diagnostic complet (dont cet inventaire) : ce que PureRGB reconnaît, ne reconnaît pas, et pourquoi. */
async function send(): Promise<void> {
  sending.value = true;
  sent.value = "";
  try {
    await invoke("send_telemetry_report");
    sentOk.value = true;
    sent.value = "Analyse envoyée, merci : elle sert à corriger les appareils non reconnus.";
  } catch (e) {
    sentOk.value = false;
    sent.value = `Envoi impossible : ${errMsg(e)}`;
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <div class="card inv">
    <h3>Inventaire RGB et ventilateurs</h3>
    <p class="hint">
      Compare ce que Windows voit (carte mère, mémoire, périphériques USB) avec ce que PureRGB peut piloter, et explique chaque écart :
      pilote manquant, logiciel du constructeur en conflit, connecteur ARGB sans nombre de LEDs…
    </p>
    <div class="row">
      <button :disabled="busy" @click="run">{{ busy ? "Analyse…" : findings ? "Relancer l'analyse" : "Analyser mon PC" }}</button>
      <button v-if="findings && props.telemetryOptIn" :disabled="busy || sending" @click="send">{{ sending ? "Envoi…" : "Envoyer l'analyse" }}</button>
      <button v-if="findings?.some((f) => f.status !== 'ok')" :disabled="busy" @click="emit('restart')">Redémarrer OpenRGB</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="findings && !props.telemetryOptIn" class="hint">
      Pour envoyer cette analyse, cochez « Envoyer les informations de diagnostic » dans Réglages › Général, puis Enregistrer.
    </p>
    <p v-if="sent" class="status" :class="sentOk ? 'good' : 'bad'" role="status">{{ sentOk ? "✓" : "✗" }} {{ sent }}</p>
    <ul v-if="findings" class="list">
      <li v-for="(f, i) in findings" :key="i" :class="f.status">
        <span class="ico" aria-hidden="true">{{ ICON[f.status] }}</span>
        <div>
          <strong>{{ f.category }} · {{ f.title }}</strong>
          <p>{{ f.detail }}</p>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 18px;
  margin-bottom: 14px;
}

h3 {
  font-size: 14px;
  margin-bottom: 8px;
}

.hint {
  color: var(--text-dim);
  font-size: 12px;
  margin-bottom: 10px;
}

.row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.status { margin-top: 10px; padding: 8px 12px; border-radius: 8px; font-size: 13px; border: 1px solid var(--border); }
.status.good { color: var(--ok); border-color: var(--ok); }
.status.bad { color: var(--err); border-color: var(--err); }

.err {
  color: #f87171;
  font-size: 12px;
  margin-top: 8px;
}

.list {
  list-style: none;
  padding: 0;
  margin: 12px 0 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 8px;
}

li {
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 12px;
}

li p {
  margin: 4px 0 0;
  color: var(--text-dim);
}

.ico {
  flex: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-weight: 700;
  background: var(--border);
}

.ok .ico {
  background: rgba(52, 211, 153, 0.25);
  color: #34d399;
}

.warn {
  border-color: rgba(245, 185, 74, 0.5);
}

.warn .ico {
  background: rgba(245, 185, 74, 0.25);
  color: #f5b94a;
}
</style>
