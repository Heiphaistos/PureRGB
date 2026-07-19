<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { reactive, ref, watch } from "vue";
import type { HardwareDiagnostics, Settings } from "../types";
import { diagOk, diagText } from "../types";

const props = defineProps<{ settings: Settings | null }>();
const emit = defineEmits<{ saved: [] }>();

const autostartBusy = ref(false);
const autostartEnabled = ref(false);
const profileMsg = ref("");

watch(
  () => props.settings?.autostart,
  (v) => {
    autostartEnabled.value = v ?? false;
  },
  { immediate: true },
);

async function toggleAutostart() {
  autostartBusy.value = true;
  try {
    await invoke("set_autostart", { enabled: !autostartEnabled.value });
    autostartEnabled.value = !autostartEnabled.value;
    emit("saved");
  } catch (e) {
    profileMsg.value = `Autostart : ${e}`;
  } finally {
    autostartBusy.value = false;
  }
}

async function exportProfile() {
  const path = await saveDialog({
    defaultPath: "purergb-profil.json",
    filters: [{ name: "Profil PureRGB", extensions: ["json"] }],
  });
  if (!path) return;
  try {
    await invoke("profile_export", { path });
    profileMsg.value = "Profil exporté.";
  } catch (e) {
    profileMsg.value = `Export : ${e}`;
  }
}

async function importProfile() {
  const path = await open({
    multiple: false,
    filters: [{ name: "Profil PureRGB", extensions: ["json"] }],
  });
  if (typeof path !== "string") return;
  try {
    await invoke("profile_import", { path });
    profileMsg.value = "Profil importé et appliqué.";
    emit("saved");
  } catch (e) {
    profileMsg.value = `Import : ${e}`;
  }
}

const diag = ref<HardwareDiagnostics | null>(null);
const diagRunning = ref(false);
const showUnrecognizedOnly = ref(true);

async function runDiagnostics() {
  diagRunning.value = true;
  try {
    diag.value = await invoke<HardwareDiagnostics>("hardware_diagnostics");
  } catch (e) {
    profileMsg.value = `Diagnostic : ${e}`;
  } finally {
    diagRunning.value = false;
  }
}

const hidRows = () =>
  diag.value
    ? diag.value.hid_raw.filter((d) => !showUnrecognizedOnly.value || !d.recognized)
    : [];

const form = reactive({
  openrgb_host: "127.0.0.1",
  openrgb_port: 6742,
  auto_start_openrgb: true,
  native_drivers_enabled: false,
  fps: 30,
  start_minimized: false,
});
const saving = ref(false);
const error = ref("");

watch(
  () => props.settings,
  (s) => {
    if (!s) return;
    form.openrgb_host = s.openrgb_host;
    form.openrgb_port = s.openrgb_port;
    form.auto_start_openrgb = s.auto_start_openrgb;
    form.native_drivers_enabled = s.native_drivers_enabled;
    form.fps = s.fps;
    form.start_minimized = s.start_minimized;
  },
  { immediate: true },
);

async function save() {
  saving.value = true;
  error.value = "";
  try {
    await invoke("update_settings", {
      openrgbHost: form.openrgb_host,
      openrgbPort: form.openrgb_port,
      autoStartOpenrgb: form.auto_start_openrgb,
      nativeDriversEnabled: form.native_drivers_enabled,
      fps: form.fps,
      startMinimized: form.start_minimized,
    });
    emit("saved");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="settings">
    <h2>Réglages</h2>

    <div class="card">
      <h3>Serveur OpenRGB</h3>
      <p class="hint">
        PureRGB embarque OpenRGB et pilote 900+ appareils via son SDK. Le serveur
        est démarré automatiquement en arrière-plan si aucun n'est joignable
        (installation NSIS : inclus ; portable : téléchargé et vérifié SHA-256 au
        premier besoin). Un OpenRGB déjà lancé par vous est réutilisé tel quel.
      </p>
      <div class="inline" style="margin-bottom: 12px">
        <input id="autostart" type="checkbox" v-model="form.auto_start_openrgb" />
        <label for="autostart">Démarrer OpenRGB automatiquement avec PureRGB</label>
      </div>
      <div class="grid2">
        <div>
          <label>Hôte</label>
          <input type="text" v-model="form.openrgb_host" maxlength="253" />
        </div>
        <div>
          <label>Port</label>
          <input type="number" v-model.number="form.openrgb_port" min="1" max="65535" />
        </div>
      </div>
    </div>

    <div class="card">
      <h3>Drivers natifs (expérimental)</h3>
      <p class="hint">
        Pilotage USB direct sans OpenRGB : Corsair Lighting Node Pro/Core, NZXT
        HUE 2 / Smart Device V2 / RGB &amp; Fan Controller (LED + ventilateurs).
        <strong>Fermez iCUE/CAM avant d'activer</strong> — deux logiciels sur le
        même appareil provoquent des comportements erratiques.
      </p>
      <div class="inline">
        <input id="native" type="checkbox" v-model="form.native_drivers_enabled" />
        <label for="native">Activer les drivers natifs</label>
      </div>
    </div>

    <div class="card">
      <h3>Système &amp; profils</h3>
      <div class="inline" style="margin-bottom: 12px">
        <input
          id="winstart"
          type="checkbox"
          :checked="autostartEnabled"
          :disabled="autostartBusy"
          @change="toggleAutostart"
        />
        <label for="winstart">
          Lancer PureRGB au démarrage de Windows (tâche planifiée, sans fenêtre UAC)
        </label>
      </div>
      <div class="inline" style="gap: 10px">
        <button @click="exportProfile">Exporter le profil…</button>
        <button @click="importProfile">Importer un profil…</button>
        <span class="hint" v-if="profileMsg">{{ profileMsg }}</span>
      </div>
      <p class="hint">
        Un profil contient tout : effets (globaux et par zone), modes matériels,
        courbes ventilateurs et réglages.
      </p>
    </div>

    <div class="card">
      <h3>Performance</h3>
      <div class="grid2">
        <div>
          <label>Images/seconde des animations — {{ form.fps }} FPS</label>
          <input type="range" min="5" max="144" step="5" v-model.number="form.fps" />
          <p class="hint">
            Plus bas = moins de CPU/USB. 30 FPS est fluide, 60+ pour des
            transitions très nettes. Cadence à échéance fixe (pas de dérive
            même avec plusieurs appareils). Les effets statiques ne
            consomment rien quel que soit ce réglage.
          </p>
        </div>
        <div class="inline top">
          <input id="startmin" type="checkbox" v-model="form.start_minimized" />
          <label for="startmin">Démarrer minimisé dans la barre système</label>
        </div>
      </div>
    </div>

    <div class="actions">
      <button class="primary" :disabled="saving" @click="save">
        {{ saving ? "Enregistrement..." : "Enregistrer" }}
      </button>
      <span v-if="error" class="error">{{ error }}</span>
    </div>

    <div class="card">
      <h3>Diagnostic matériel</h3>
      <p class="hint">
        Interroge directement liquidctl, sensord, OpenRGB et l'énumération USB
        brute — utile quand un module « ne se charge pas » silencieusement :
        le message d'erreur exact apparaît ici au lieu de rester dans les logs.
      </p>
      <button :disabled="diagRunning" @click="runDiagnostics">
        {{ diagRunning ? "Diagnostic en cours…" : "Lancer le diagnostic" }}
      </button>

      <div v-if="diag" class="diag-out">
        <h4>liquidctl</h4>
        <table class="diag-table">
          <tr>
            <td>Binaire</td>
            <td>{{ diag.liquidctl.exe_path ?? "introuvable" }}</td>
          </tr>
          <tr v-for="key in (['version', 'list', 'initialize', 'status'] as const)" :key="key">
            <td>{{ key }}</td>
            <td :class="{ ok: diagOk(diag.liquidctl[key]), fail: !diagOk(diag.liquidctl[key]) }">
              <pre>{{ diagText(diag.liquidctl[key]) || "(vide)" }}</pre>
            </td>
          </tr>
        </table>

        <h4>sensord</h4>
        <table class="diag-table">
          <tr><td>Binaire</td><td>{{ diag.sensord.exe_path ?? "introuvable" }}</td></tr>
          <tr><td>En cours</td><td :class="{ ok: diag.sensord.running, fail: !diag.sensord.running }">{{ diag.sensord.running ? "oui" : "non" }}</td></tr>
          <tr><td>Capteurs remontés</td><td>{{ diag.sensord.sensor_count }}</td></tr>
        </table>

        <h4>OpenRGB</h4>
        <table class="diag-table">
          <tr><td>Binaire</td><td>{{ diag.openrgb.exe_path ?? "introuvable" }}</td></tr>
          <tr><td>Serveur joignable</td><td :class="{ ok: diag.openrgb.server_reachable, fail: !diag.openrgb.server_reachable }">{{ diag.openrgb.server_reachable ? "oui" : "non" }}</td></tr>
          <tr><td>Géré par PureRGB</td><td>{{ diag.openrgb.managed ? "oui" : "non" }}</td></tr>
          <tr><td>PawnIO prêt</td><td :class="{ ok: diag.openrgb.pawnio_ready, fail: !diag.openrgb.pawnio_ready }">{{ diag.openrgb.pawnio_ready ? "oui" : "non" }}</td></tr>
        </table>

        <h4>
          Périphériques USB bruts ({{ diag.hid_raw.length }})
          <label class="filter-toggle">
            <input type="checkbox" v-model="showUnrecognizedOnly" />
            non reconnus seulement
          </label>
        </h4>
        <p class="hint">
          Un appareil listé ici « non reconnu » n'est identifié par aucune
          table PureRGB — souvent une marque bas de gamme ou un connecteur
          ARGB direct (pas de VID/PID propre). Communiquez le VID/PID pour
          l'ajouter à une prochaine version.
        </p>
        <table class="diag-table hid-table">
          <tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>État</th></tr>
          <tr v-for="d in hidRows()" :key="`${d.vid}:${d.pid}`">
            <td>{{ d.vid }}:{{ d.pid }}</td>
            <td>{{ d.manufacturer || "—" }}</td>
            <td>{{ d.product || "—" }}</td>
            <td :class="{ ok: d.recognized, fail: !d.recognized }">
              {{ d.recognized ? (d.has_native_driver ? "driver natif" : "reconnu") : "non reconnu" }}
            </td>
          </tr>
        </table>
        <p v-if="hidRows().length === 0" class="hint">Aucun appareil dans ce filtre.</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
  max-width: 720px;
}

.settings h2 {
  font-size: 19px;
  margin-bottom: 18px;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 18px;
  margin-bottom: 14px;
}

.card h3 {
  font-size: 14px;
  margin-bottom: 8px;
}

.hint {
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.6;
  margin-bottom: 12px;
}

.grid2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

label {
  display: block;
  font-size: 13px;
  color: var(--text-dim);
  margin-bottom: 6px;
}

.inline {
  display: flex;
  align-items: center;
  gap: 8px;
}

.inline label {
  margin: 0;
}

.inline.top {
  align-items: flex-start;
  padding-top: 24px;
}

.actions {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-top: 8px;
}

.error {
  color: var(--err);
  font-size: 13px;
}

.diag-out {
  margin-top: 16px;
}

.diag-out h4 {
  font-size: 13px;
  margin: 16px 0 8px;
  display: flex;
  align-items: center;
  gap: 10px;
}

.filter-toggle {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  gap: 5px;
}

.diag-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.diag-table td,
.diag-table th {
  border: 1px solid var(--border);
  padding: 6px 8px;
  text-align: left;
  vertical-align: top;
}

.diag-table pre {
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  max-height: 140px;
  overflow-y: auto;
}

.diag-table .ok {
  color: var(--ok);
}

.diag-table .fail {
  color: var(--warn);
}

.hid-table td {
  font-family: monospace;
}
</style>
