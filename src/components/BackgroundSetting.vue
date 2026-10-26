<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { errMsg } from "../screens/api";
import { quitApp } from "../gameMode";

interface Modules {
  screens: boolean;
  lighting: boolean;
  fans: boolean;
  gamepad: boolean;
}

const m = reactive<Modules>({ screens: false, lighting: false, fans: false, gamepad: false });
const saved = reactive<Modules>({ screens: false, lighting: false, fans: false, gamepad: false });
const msg = ref("");
const error = ref("");
const running = ref(false);
const busy = ref(false);
let timer: ReturnType<typeof setInterval> | undefined;

const items: { key: keyof Modules; label: string; hint: string }[] = [
  { key: "screens", label: "Écrans (design, thème, vidéo)", hint: "Les mini-écrans gardent leur affichage." },
  { key: "lighting", label: "Éclairage appliqué", hint: "Couleurs, effets et thèmes déjà réglés (OpenRGB et pilotes natifs)." },
  { key: "fans", label: "Ventilateurs", hint: "Seulement les courbes et réglages manuels déjà actifs. Jamais de réglage automatique." },
  { key: "gamepad", label: "Mode manette", hint: "Remappage par manette virtuelle, si activé." },
];

const dirty = () => (Object.keys(m) as (keyof Modules)[]).some((k) => m[k] !== saved[k]);

async function refreshStatus(): Promise<void> {
  try {
    running.value = await invoke<boolean>("background_status");
  } catch {
    /* hors application (aperçu navigateur) */
  }
}

onMounted(async () => {
  try {
    const cur = await invoke<Modules>("background_get");
    Object.assign(m, cur);
    Object.assign(saved, cur);
  } catch {
    /* hors application (aperçu navigateur) : cases non enregistrées */
  }
  await refreshStatus();
  timer = setInterval(refreshStatus, 3000);
});
onUnmounted(() => clearInterval(timer));

async function save(): Promise<void> {
  error.value = "";
  msg.value = "";
  try {
    await invoke("background_set", { modules: { ...m } });
    Object.assign(saved, m);
    msg.value = "Enregistré. Pris en compte au prochain « Quitter ».";
  } catch (e) {
    error.value = errMsg(e);
  }
}

async function stopBackground(): Promise<void> {
  busy.value = true;
  error.value = "";
  try {
    await invoke("background_stop");
    await refreshStatus();
    msg.value = "Le processus d'arrière-plan est arrêté.";
  } catch (e) {
    error.value = errMsg(e);
  } finally {
    busy.value = false;
  }
}

async function quitAll(): Promise<void> {
  busy.value = true;
  try {
    await quitApp();
  } catch (e) {
    busy.value = false;
    error.value = errMsg(e);
  }
}
</script>

<template>
  <div class="bg">
    <div class="card">
      <h3>Ce qui reste actif quand PureRGB est fermé</h3>
      <p class="hint">
        Au « Quitter » du menu de l'icône (zone de notification), PureRGB peut laisser un petit processus sans fenêtre et sans
        interface (le même programme) qui ne garde que ce que vous cochez <strong>et qui est déjà en service</strong>. Rien
        d'autre ne reste : ni service Windows, ni pilote ajouté. Le mode jeu s'applique aussi à ce processus.
      </p>
      <div v-for="it in items" :key="it.key" class="opt">
        <input :id="'bg-' + it.key" v-model="m[it.key]" type="checkbox" />
        <label :for="'bg-' + it.key"><strong>{{ it.label }}</strong><small>{{ it.hint }}</small></label>
      </div>
      <div class="row">
        <button class="primary" :disabled="!dirty()" @click="save">Enregistrer</button>
        <span v-if="dirty()" class="hint">Modifications non enregistrées.</span>
        <span v-else-if="msg" class="ok">{{ msg }}</span>
      </div>
      <p v-if="error" class="err">{{ error }}</p>
    </div>

    <div class="card">
      <h3>Quitter</h3>
      <p class="state" :class="{ on: running }">
        {{ running ? "Un processus d'arrière-plan PureRGB tourne en ce moment (sans fenêtre)." : "Aucun processus d'arrière-plan PureRGB ne tourne." }}
      </p>
      <ul class="hint list">
        <li>La croix de la fenêtre <strong>ne ferme pas</strong> PureRGB : il se réduit dans la zone de notification.</li>
        <li><strong>Quitter</strong> (menu de l'icône) : ferme PureRGB et laisse seulement ce que vous avez coché.</li>
        <li><strong>Quitter complètement</strong> : ferme tout, aucun processus PureRGB ne reste.</li>
        <li>Rouvrir PureRGB arrête le processus d'arrière-plan et reprend la main.</li>
      </ul>
      <div class="row">
        <button :disabled="busy" @click="quitAll">Quitter complètement maintenant</button>
        <button :disabled="busy || !running" @click="stopBackground">Arrêter le processus d'arrière-plan</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.bg { display: contents; }
.card { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 18px; }
h3 { font-size: 14px; margin-bottom: 8px; }
.hint { font-size: 12px; color: var(--text-dim); line-height: 1.6; margin-bottom: 12px; }
.list { padding-left: 18px; }
.opt { display: flex; gap: 10px; align-items: flex-start; padding: 6px 0; }
.opt input { margin-top: 3px; flex: none; }
.opt label { margin: 0; display: flex; flex-direction: column; gap: 2px; color: var(--text); }
.opt small { color: var(--text-dim); font-size: 12px; }
.row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-top: 12px; }
.ok { color: var(--ok); font-size: 12px; }
.err { color: var(--err); font-size: 12px; margin-top: 8px; }
.state { font-size: 13px; margin-bottom: 8px; color: var(--text-dim); }
.state.on { color: var(--warn); }
</style>
