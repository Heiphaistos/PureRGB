<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { CaptureFileInfo, RawHidDevice } from "../types";
import { errMsg } from "../screens/api";
import CommunityFeedback from "./CommunityFeedback.vue";
import FeedbackButton from "./FeedbackButton.vue";
import RgbInventory from "./RgbInventory.vue";

type CardStatus = "ok" | "warn" | "error";
interface Card {
  id: string;
  title: string;
  status: CardStatus;
  rows: [string, string][];
}
interface DiagReport {
  cards: Card[];
  text: string;
  hid_raw: RawHidDevice[];
}

const props = defineProps<{ telemetryOptIn: boolean; trigger?: number }>();
const emit = defineEmits<{ restart: [] }>();

const STATUS_LABEL: Record<CardStatus, string> = { ok: "OK", warn: "Avertissement", error: "Erreur" };

const report = ref<DiagReport | null>(null);
const running = ref(false);
const msg = ref("");
const showUnrecognizedOnly = ref(true);

async function run(): Promise<void> {
  running.value = true;
  msg.value = "";
  try {
    report.value = await invoke<DiagReport>("diagnostic_report");
  } catch (e) {
    msg.value = `Diagnostic : ${errMsg(e)}`;
  } finally {
    running.value = false;
  }
}

watch(
  () => props.trigger,
  (v, old) => {
    if (v !== undefined && v !== old) void run();
  },
);

const counts = computed(() => {
  const c = { ok: 0, warn: 0, error: 0 };
  for (const card of report.value?.cards ?? []) c[card.status]++;
  return c;
});

async function copyReport(): Promise<void> {
  if (!report.value) return;
  try {
    await navigator.clipboard.writeText(report.value.text);
    msg.value = "Rapport copié dans le presse-papiers.";
  } catch (e) {
    msg.value = `Copie impossible : ${errMsg(e)}`;
  }
}

async function saveReport(): Promise<void> {
  if (!report.value) return;
  const path = await saveDialog({
    defaultPath: "purergb-diagnostic.txt",
    filters: [{ name: "Rapport texte", extensions: ["txt"] }],
  });
  if (!path) return;
  try {
    await invoke("diagnostic_report_save", { path, text: report.value.text });
    msg.value = "Rapport enregistré.";
  } catch (e) {
    msg.value = `Enregistrement : ${errMsg(e)}`;
  }
}

const telemetrySending = ref(false);
async function sendTelemetryNow(): Promise<void> {
  telemetrySending.value = true;
  msg.value = "";
  try {
    await invoke("send_telemetry_report");
    msg.value = "Rapport envoyé.";
  } catch (e) {
    msg.value = `Envoi : ${errMsg(e)}`;
  } finally {
    telemetrySending.value = false;
  }
}

const hidRows = computed(() =>
  (report.value?.hid_raw ?? []).filter((d) => !showUnrecognizedOnly.value || !d.recognized),
);

// --- Capture du protocole USB d'un appareil non reconnu ---
type CaptureTarget = { vid: string; pid: string; manufacturer: string; product: string };
const captureTargetDevice = ref<CaptureTarget | null>(null);
const captureStep = ref<"idle" | "warning" | "installing" | "running" | "summary">("idle");
const captureFiles = ref<CaptureFileInfo[]>([]);
const captureMsg = ref("");
const captureStartedAt = ref(0);
const captureElapsed = ref(0);
const captureUploading = ref(false);
let captureTimerHandle: ReturnType<typeof setInterval> | null = null;
const CAPTURE_MAX_SECONDS = 300;

function clearCaptureTimer(): void {
  if (captureTimerHandle) {
    clearInterval(captureTimerHandle);
    captureTimerHandle = null;
  }
}
onBeforeUnmount(clearCaptureTimer);

function openCaptureWarning(d: CaptureTarget): void {
  captureTargetDevice.value = d;
  captureStep.value = "warning";
  captureMsg.value = "";
}

function closeCaptureFlow(): void {
  clearCaptureTimer();
  captureStep.value = "idle";
  captureTargetDevice.value = null;
  captureFiles.value = [];
}

async function beginCapture(): Promise<void> {
  captureStep.value = "installing";
  captureMsg.value = "";
  try {
    const ready = await invoke<boolean>("usb_capture_ready");
    if (!ready) await invoke("usb_capture_install");
    await invoke("usb_capture_start");
    captureStep.value = "running";
    captureStartedAt.value = Date.now();
    captureElapsed.value = 0;
    captureTimerHandle = setInterval(() => {
      captureElapsed.value = Math.floor((Date.now() - captureStartedAt.value) / 1000);
      if (captureElapsed.value >= CAPTURE_MAX_SECONDS) void stopCapture();
    }, 1000);
  } catch (e) {
    captureMsg.value = `Installation : ${e}`;
    captureStep.value = "warning";
  }
}

async function stopCapture(): Promise<void> {
  clearCaptureTimer();
  try {
    captureFiles.value = await invoke<CaptureFileInfo[]>("usb_capture_stop");
    captureStep.value = "summary";
  } catch (e) {
    captureMsg.value = `Arrêt : ${e}`;
  }
}

async function uploadCaptureFiles(): Promise<void> {
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
  captureMsg.value =
    remaining.length === 0
      ? "Fichiers envoyés."
      : `${succeeded}/${succeeded + remaining.length} envoyés, échec sur ${remaining.length} fichier(s) : ${firstError}`;
  captureUploading.value = false;
}
</script>

<template>
  <div class="diag">
    <div class="card head">
      <div class="head-text">
        <h3>Diagnostic matériel</h3>
        <p class="hint">
          Interroge liquidctl, sensord, OpenRGB, l'USB et les pilotes en lecture seule : le message d'erreur exact
          apparaît ici au lieu de rester dans les logs. Rien n'est écrit sur le matériel ni envoyé sans votre action.
        </p>
      </div>
      <div class="head-actions">
        <button class="primary" :disabled="running" @click="run">
          {{ running ? "Diagnostic en cours…" : report ? "Relancer le diagnostic" : "Lancer le diagnostic" }}
        </button>
        <button :disabled="!report" @click="copyReport">Copier le rapport</button>
        <button :disabled="!report" @click="saveReport">Enregistrer le rapport</button>
        <button v-if="props.telemetryOptIn" :disabled="telemetrySending" @click="sendTelemetryNow">
          {{ telemetrySending ? "Envoi…" : "Envoyer maintenant" }}
        </button>
      </div>
      <div v-if="report" class="summary">
        <span class="pill ok">{{ counts.ok }} OK</span>
        <span class="pill warn">{{ counts.warn }} avertissement(s)</span>
        <span class="pill error">{{ counts.error }} erreur(s)</span>
      </div>
      <span v-if="msg" class="hint msg">{{ msg }}</span>
    </div>

    <div v-if="report" class="cards">
      <article v-for="c in report.cards" :key="c.id" class="card dcard" :class="[`st-${c.status}`, { wide: c.id === 'log' || c.id === 'findings' }]">
        <header>
          <h4>{{ c.title }}</h4>
          <span class="pill" :class="c.status">{{ STATUS_LABEL[c.status] }}</span>
        </header>
        <dl>
          <template v-for="(r, i) in c.rows" :key="i">
            <dt>{{ r[0] }}</dt>
            <dd :class="{ mono: c.id === 'log' }">{{ r[1] }}</dd>
          </template>
        </dl>
      </article>
    </div>

    <div v-if="report" class="card hid">
      <h4>
        Périphériques USB/HID bruts ({{ report.hid_raw.length }})
        <label class="filter-toggle">
          <input type="checkbox" v-model="showUnrecognizedOnly" />
          non reconnus seulement
        </label>
      </h4>
      <p class="hint">
        Un appareil « non reconnu » n'est identifié par aucune table PureRGB : souvent une marque bas de gamme ou un
        connecteur ARGB direct. Communiquez le VID/PID pour l'ajouter à une prochaine version.
      </p>
      <div class="table-wrap">
        <table class="diag-table hid-table">
          <tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>État</th><th></th></tr>
          <tr v-for="d in hidRows" :key="`${d.vid}:${d.pid}`">
            <td>{{ d.vid }}:{{ d.pid }}</td>
            <td>{{ d.manufacturer || "—" }}</td>
            <td>{{ d.product || "—" }}</td>
            <td :class="d.recognized ? 'ok' : 'fail'">
              {{ d.recognized ? (d.has_native_driver ? "driver natif" : "reconnu") : "non reconnu" }}
            </td>
            <td class="row-actions">
              <button v-if="!d.recognized" @click="openCaptureWarning(d)">Capturer le protocole USB</button>
              <FeedbackButton :vid="d.vid" :pid="d.pid" :manufacturer="d.manufacturer" :product="d.product" />
            </td>
          </tr>
        </table>
      </div>
      <p v-if="hidRows.length === 0" class="hint">Aucun appareil dans ce filtre.</p>
    </div>

    <RgbInventory @restart="emit('restart')" />
    <CommunityFeedback v-if="report" />

    <div v-if="captureStep !== 'idle'" class="capture-modal">
      <div class="capture-modal-inner">
        <template v-if="captureStep === 'warning'">
          <h4>Capturer le protocole USB — {{ captureTargetDevice?.product || captureTargetDevice?.manufacturer }}</h4>
          <p class="hint">
            Cette capture enregistre TOUT le trafic USB de cet ordinateur pendant la fenêtre, pas seulement cet
            appareil — d'autres périphériques branchés sur le même port apparaîtront aussi. Si un clavier est
            branché, vos frappes peuvent être incluses dans la capture. <strong>Ne tapez rien de sensible</strong>
            (mots de passe, etc.) pendant que la capture est active. Le fichier reste en local — vous choisirez
            ensuite de l'envoyer ou non.
          </p>
          <p v-if="captureMsg" class="hint" style="color: #c00">{{ captureMsg }}</p>
          <div class="inline">
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
            Ouvrez maintenant le logiciel officiel de cet appareil et changez une couleur ou un effet, puis cliquez
            Arrêter.
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
          <div class="inline">
            <button :disabled="captureUploading || captureFiles.length === 0" @click="uploadCaptureFiles">
              {{ captureUploading ? "Envoi…" : "Envoyer pour analyse" }}
            </button>
            <button @click="closeCaptureFlow">Garder en local seulement</button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diag {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
}
.diag > .card {
  margin: 0;
}
.head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3) var(--space-5);
}
.head-text {
  flex: 1 1 360px;
}
.head h3 {
  font-size: 15px;
  margin-bottom: 6px;
}
.head .hint {
  margin-bottom: 0;
}
.head-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.summary {
  display: flex;
  gap: var(--space-2);
  flex: 1 1 100%;
}
.msg {
  flex: 1 1 100%;
  margin: 0;
}
.hint {
  font-size: 13px;
  color: #b4b1c0;
  line-height: 1.6;
  margin-bottom: 12px;
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: var(--space-3);
  align-items: start;
}
.dcard {
  min-width: 0;
}
.dcard.wide {
  grid-column: 1 / -1;
}
.dcard header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}
.dcard h4,
.hid h4 {
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.hid h4 {
  margin-bottom: 8px;
}
dl {
  display: grid;
  grid-template-columns: minmax(110px, 38%) 1fr;
  gap: 6px 12px;
  font-size: 13px;
  margin: 0;
}
.dcard.wide dl {
  grid-template-columns: minmax(160px, 22%) 1fr;
}
dt {
  color: #b4b1c0;
}
dd {
  color: #f4f2ef;
  font-weight: 500;
  margin: 0;
  overflow-wrap: anywhere;
}
dd.mono {
  font-family: monospace;
  font-size: 13px;
}
.pill {
  font-size: 13px;
  font-weight: 600;
  padding: 2px 12px;
  border-radius: 999px;
  color: #111;
  white-space: nowrap;
}
.pill.ok {
  background: var(--ok);
}
.pill.warn {
  background: var(--warn);
}
.pill.error {
  background: var(--err);
}
.dcard {
  border-left: 4px solid var(--ok);
}
.dcard.st-warn {
  border-left-color: var(--warn);
}
.dcard.st-error {
  border-left-color: var(--err);
}
.filter-toggle {
  font-size: 13px;
  font-weight: 400;
  color: #b4b1c0;
  display: flex;
  align-items: center;
  gap: 5px;
  margin: 0;
}
.table-wrap {
  overflow-x: auto;
}
.diag-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}
.diag-table td,
.diag-table th {
  border: 1px solid var(--border);
  padding: 6px 8px;
  text-align: left;
  vertical-align: top;
}
.diag-table .ok {
  color: var(--ok);
  font-weight: 600;
}
.diag-table .fail {
  color: var(--warn);
  font-weight: 600;
}
.hid-table td:not(.row-actions) {
  font-family: monospace;
}
.row-actions {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.inline {
  display: flex;
  gap: 10px;
  align-items: center;
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
