<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { reactive, ref, watch } from "vue";
import type { CaptureFileInfo, HardwareDiagnostics, Settings } from "../types";
import { diagOk, diagText } from "../types";

export type LayoutMode = "grid" | "list" | "canvas";

const props = defineProps<{
  settings: Settings | null;
  layout: LayoutMode;
  diagnosticTrigger?: number;
}>();
const emit = defineEmits<{ saved: []; "layout-change": [mode: LayoutMode] }>();

const LAYOUTS: { id: LayoutMode; label: string; hint: string }[] = [
  { id: "grid", label: "Grille", hint: "Cartes périphérique groupées par catégorie." },
  { id: "list", label: "Liste dense", hint: "Lignes compactes, idéal avec beaucoup d'appareils." },
  { id: "canvas", label: "Canvas immersif", hint: "Grosses tuiles, effet appliqué directement dessus." },
];

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

watch(
  () => props.diagnosticTrigger,
  (v, old) => {
    if (v !== undefined && v !== old) runDiagnostics();
  },
);

const telemetrySending = ref(false);
const telemetryMsg = ref("");

async function sendTelemetryNow() {
  telemetrySending.value = true;
  telemetryMsg.value = "";
  try {
    await invoke("send_telemetry_report");
    telemetryMsg.value = "Rapport envoyé.";
  } catch (e) {
    telemetryMsg.value = `Envoi : ${e}`;
  } finally {
    telemetrySending.value = false;
  }
}

const captureTargetDevice = ref<{ vid: string; pid: string; manufacturer: string; product: string } | null>(null);
const captureStep = ref<"idle" | "warning" | "installing" | "running" | "summary">("idle");
const captureFiles = ref<CaptureFileInfo[]>([]);
const captureMsg = ref("");
const captureStartedAt = ref(0);
const captureElapsed = ref(0);
let captureTimerHandle: ReturnType<typeof setInterval> | null = null;

const CAPTURE_MAX_SECONDS = 300;

function openCaptureWarning(d: { vid: string; pid: string; manufacturer: string; product: string }) {
  captureTargetDevice.value = d;
  captureStep.value = "warning";
  captureMsg.value = "";
}

function closeCaptureFlow() {
  if (captureTimerHandle) {
    clearInterval(captureTimerHandle);
    captureTimerHandle = null;
  }
  captureStep.value = "idle";
  captureTargetDevice.value = null;
  captureFiles.value = [];
}

async function beginCapture() {
  captureStep.value = "installing";
  captureMsg.value = "";
  try {
    const ready = await invoke<boolean>("usb_capture_ready");
    if (!ready) {
      await invoke("usb_capture_install");
    }
    await invoke("usb_capture_start");
    captureStep.value = "running";
    captureStartedAt.value = Date.now();
    captureElapsed.value = 0;
    captureTimerHandle = setInterval(() => {
      captureElapsed.value = Math.floor((Date.now() - captureStartedAt.value) / 1000);
      if (captureElapsed.value >= CAPTURE_MAX_SECONDS) {
        stopCapture();
      }
    }, 1000);
  } catch (e) {
    captureMsg.value = `Installation : ${e}`;
    captureStep.value = "warning";
  }
}

async function stopCapture() {
  if (captureTimerHandle) {
    clearInterval(captureTimerHandle);
    captureTimerHandle = null;
  }
  try {
    captureFiles.value = await invoke<CaptureFileInfo[]>("usb_capture_stop");
    captureStep.value = "summary";
  } catch (e) {
    captureMsg.value = `Arrêt : ${e}`;
  }
}

const captureUploading = ref(false);

async function uploadCaptureFiles() {
  if (!captureTargetDevice.value) return;
  captureUploading.value = true;
  captureMsg.value = "";
  const remaining: CaptureFileInfo[] = [];
  let succeeded = 0;
  let firstError = "";
  for (const f of captureFiles.value) {
    try {
      await invoke("usb_capture_upload", {
        vid: captureTargetDevice.value.vid,
        pid: captureTargetDevice.value.pid,
        deviceName: captureTargetDevice.value.product || captureTargetDevice.value.manufacturer || "inconnu",
        path: f.path,
      });
      succeeded++;
    } catch (e) {
      remaining.push(f);
      if (!firstError) firstError = String(e);
    }
  }
  captureFiles.value = remaining;
  if (remaining.length === 0) {
    captureMsg.value = "Fichiers envoyés.";
  } else {
    captureMsg.value = `${succeeded}/${succeeded + remaining.length} envoyés, échec sur ${remaining.length} fichier(s) : ${firstError}`;
  }
  captureUploading.value = false;
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
  auto_manage_conflicts: true,
  telemetry_opt_in: false,
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
    form.auto_manage_conflicts = s.auto_manage_conflicts;
    form.telemetry_opt_in = s.telemetry_opt_in;
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
      autoManageConflicts: form.auto_manage_conflicts,
      telemetryOptIn: form.telemetry_opt_in,
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
      <h3>Disposition</h3>
      <p class="hint">Agencement de l'onglet Éclairage.</p>
      <div class="layout-options">
        <button
          v-for="l in LAYOUTS"
          :key="l.id"
          type="button"
          class="layout-option"
          :class="{ active: layout === l.id }"
          @click="emit('layout-change', l.id)"
        >
          <strong>{{ l.label }}</strong>
          <span>{{ l.hint }}</span>
        </button>
      </div>
    </div>

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
      <div class="inline" style="margin-bottom: 12px">
        <input id="automanage" type="checkbox" v-model="form.auto_manage_conflicts" />
        <label for="automanage">
          Gérer automatiquement les conflits (arrête les logiciels constructeur au
          lancement, les relance à la fermeture)
        </label>
      </div>
      <div class="inline" style="margin-bottom: 12px">
        <input id="telemetry" type="checkbox" v-model="form.telemetry_opt_in" />
        <label for="telemetry">
          Envoyer les informations de diagnostic matériel (VID/PID détectés,
          état OpenRGB/liquidctl/sensord) pour aider à identifier le matériel
          non reconnu. Aucune donnée personnelle, désactivé par défaut.
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
      <button
        v-if="props.settings?.telemetry_opt_in"
        :disabled="telemetrySending"
        @click="sendTelemetryNow"
        style="margin-left: 8px"
      >
        {{ telemetrySending ? "Envoi…" : "Envoyer maintenant" }}
      </button>
      <span v-if="telemetryMsg" class="hint">{{ telemetryMsg }}</span>

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
          <tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>État</th><th></th></tr>
          <tr v-for="d in hidRows()" :key="`${d.vid}:${d.pid}`">
            <td>{{ d.vid }}:{{ d.pid }}</td>
            <td>{{ d.manufacturer || "—" }}</td>
            <td>{{ d.product || "—" }}</td>
            <td :class="{ ok: d.recognized, fail: !d.recognized }">
              {{ d.recognized ? (d.has_native_driver ? "driver natif" : "reconnu") : "non reconnu" }}
            </td>
            <td>
              <button v-if="!d.recognized" @click="openCaptureWarning(d)">Capturer le protocole USB</button>
            </td>
          </tr>
        </table>
        <p v-if="hidRows().length === 0" class="hint">Aucun appareil dans ce filtre.</p>

        <div v-if="captureStep !== 'idle'" class="capture-modal">
          <div class="capture-modal-inner">
            <template v-if="captureStep === 'warning'">
              <h4>Capturer le protocole USB — {{ captureTargetDevice?.product || captureTargetDevice?.manufacturer }}</h4>
              <p class="hint">
                Cette capture enregistre TOUT le trafic USB de cet ordinateur pendant la
                fenêtre, pas seulement cet appareil — d'autres périphériques branchés sur
                le même port apparaîtront aussi. Si un clavier est branché, vos frappes
                peuvent être incluses dans la capture. <strong>Ne tapez rien de sensible</strong>
                (mots de passe, etc.) pendant que la capture est active. Le fichier reste
                en local — vous choisirez ensuite de l'envoyer ou non.
              </p>
              <p v-if="captureMsg" class="hint" style="color: #c00">{{ captureMsg }}</p>
              <div class="inline" style="gap: 10px">
                <button @click="beginCapture">Démarrer</button>
                <button @click="closeCaptureFlow">Annuler</button>
              </div>
            </template>
            <template v-else-if="captureStep === 'installing'">
              <p>Installation d'USBPcap si nécessaire…</p>
            </template>
            <template v-else-if="captureStep === 'running'">
              <h4>Capture en cours — {{ captureElapsed }}s / {{ CAPTURE_MAX_SECONDS }}s</h4>
              <p class="hint">
                Ouvrez maintenant le logiciel officiel de cet appareil et changez une
                couleur ou un effet, puis cliquez Arrêter.
              </p>
              <button @click="stopCapture">Arrêter</button>
            </template>
            <template v-else-if="captureStep === 'summary'">
              <h4>Capture terminée ({{ captureFiles.length }} fichier(s))</h4>
              <table class="diag-table">
                <tr><th>Hub</th><th>Taille</th></tr>
                <tr v-for="f in captureFiles" :key="f.path">
                  <td>{{ f.hub }}</td>
                  <td>{{ (f.size_bytes / 1024).toFixed(1) }} Ko</td>
                </tr>
              </table>
              <p v-if="captureFiles.length === 0" class="hint">
                Aucun trafic capturé — réessayez en changeant bien une couleur pendant la fenêtre.
              </p>
              <p v-if="captureMsg" class="hint">{{ captureMsg }}</p>
              <div class="inline" style="gap: 10px">
                <button :disabled="captureUploading || captureFiles.length === 0" @click="uploadCaptureFiles">
                  {{ captureUploading ? "Envoi…" : "Envoyer pour analyse" }}
                </button>
                <button @click="closeCaptureFlow">Garder en local seulement</button>
              </div>
            </template>
          </div>
        </div>
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

.layout-options {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: var(--space-2);
}

.layout-option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  text-align: left;
  padding: var(--space-3);
  border: 1px solid var(--border);
  background: var(--bg);
  border-radius: var(--radius-sm);
}

.layout-option strong {
  font-size: 13px;
}

.layout-option span {
  font-size: 11px;
  color: var(--text-dim);
}

.layout-option.active {
  border-color: var(--accent);
  background: var(--accent-soft);
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

.capture-modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.capture-modal-inner {
  background: #1a1a1a;
  padding: 24px;
  border-radius: 8px;
  max-width: 480px;
  width: 90%;
}
</style>
