<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { reactive, ref, watch } from "vue";
import type { Settings } from "../types";
import { checkForUpdate, updateState } from "../autoUpdate";
import { errMsg } from "../screens/api";
import { photosEnabled, setPhotosEnabled } from "../productPhotos";
import DiagnosticPanel from "./DiagnosticPanel.vue";
import DecoderSetting from "./DecoderSetting.vue";
import DriversSetting from "./DriversSetting.vue";
import GameModeSetting from "./GameModeSetting.vue";
import BackgroundSetting from "./BackgroundSetting.vue";
import SelfTest from "./SelfTest.vue";

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

async function restartOpenRgb(): Promise<void> {
  try {
    await invoke("openrgb_restart");
    emit("saved");
  } catch (e) {
    error.value = errMsg(e);
  }
}

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

const form = reactive({
  openrgb_host: "127.0.0.1",
  openrgb_port: 6742,
  auto_start_openrgb: true,
  native_drivers_enabled: false,
  smbus_writes_enabled: false,
  fps: 30,
  start_minimized: false,
  auto_manage_conflicts: true,
  telemetry_opt_in: false,
  auto_update_check: true,
});
type Section = "general" | "drivers" | "media" | "security" | "diag";
const SECTIONS: { id: Section; label: string }[] = [
  { id: "general", label: "Général" },
  { id: "drivers", label: "Appareils et pilotes" },
  { id: "media", label: "Photos et vidéo" },
  { id: "security", label: "Test de sécurité" },
  { id: "diag", label: "Diagnostic" },
];
const section = ref<Section>("general");
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
    form.smbus_writes_enabled = s.smbus_writes_enabled;
    form.fps = s.fps;
    form.start_minimized = s.start_minimized;
    form.auto_manage_conflicts = s.auto_manage_conflicts;
    form.telemetry_opt_in = s.telemetry_opt_in;
    form.auto_update_check = s.auto_update_check ?? true;
    photosEnabled.value = s.product_photos !== false;
  },
  { immediate: true },
);

async function onPhotosToggle(e: Event) {
  const on = (e.target as HTMLInputElement).checked;
  try {
    await setPhotosEnabled(on);
  } catch (err) {
    error.value = errMsg(err);
  }
}

async function save() {
  saving.value = true;
  error.value = "";
  try {
    await invoke("update_settings", {
      openrgbHost: form.openrgb_host,
      openrgbPort: form.openrgb_port,
      autoStartOpenrgb: form.auto_start_openrgb,
      nativeDriversEnabled: form.native_drivers_enabled,
      smbusWritesEnabled: form.smbus_writes_enabled,
      fps: form.fps,
      startMinimized: form.start_minimized,
      autoManageConflicts: form.auto_manage_conflicts,
      telemetryOptIn: form.telemetry_opt_in,
      autoUpdateCheck: form.auto_update_check,
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
    <nav class="sections" aria-label="Sections des réglages">
      <button v-for="x in SECTIONS" :key="x.id" type="button" :class="{ active: section === x.id }" @click="section = x.id">{{ x.label }}</button>
    </nav>

    <div class="cols">
    <h3 v-show="section === 'general'" class="cat">Démarrage et fenêtre</h3>
    <div v-show="section === 'general'" class="card">
      <h3>Lancement</h3>
      <div class="inline" style="margin-bottom: 12px">
        <input id="winstart" type="checkbox" :checked="autostartEnabled" :disabled="autostartBusy" @change="toggleAutostart" />
        <label for="winstart">Lancer PureRGB au démarrage de Windows (tâche planifiée, sans fenêtre UAC)</label>
      </div>
      <div class="inline">
        <input id="startmin" type="checkbox" v-model="form.start_minimized" />
        <label for="startmin">Démarrer minimisé dans la barre système</label>
      </div>
    </div>

    <div v-show="section === 'general'" class="card">
      <h3>Disposition de l'onglet Éclairage</h3>
      <div class="layout-options">
        <button v-for="l in LAYOUTS" :key="l.id" type="button" class="layout-option" :class="{ active: layout === l.id }" @click="emit('layout-change', l.id)">
          <strong>{{ l.label }}</strong>
          <span>{{ l.hint }}</span>
        </button>
      </div>
    </div>

    <div v-show="section === 'general'" class="card">
      <h3>Performance</h3>
      <label>Images/seconde des animations — {{ form.fps }} FPS</label>
      <input type="range" min="5" max="144" step="5" v-model.number="form.fps" />
      <p class="hint">
        Plus bas = moins de CPU/USB. 30 FPS est fluide, 60+ pour des transitions très nettes. Les effets statiques ne consomment rien
        quel que soit ce réglage.
      </p>
    </div>

    <h3 v-show="section === 'general'" class="cat">Mises à jour</h3>
    <div v-show="section === 'general'" class="card">
      <h3>Mises à jour de PureRGB</h3>
      <p class="hint">
        Les mises à jour sont signées et vérifiées avant installation, puis
        PureRGB redémarre. Le réglage est enregistré avec le bouton « Enregistrer ».
      </p>
      <div class="inline" style="margin-bottom: 12px">
        <input id="autoupdate" type="checkbox" v-model="form.auto_update_check" />
        <label for="autoupdate">Vérifier les mises à jour automatiquement</label>
      </div>
      <button :disabled="updateState.phase !== 'idle'" @click="checkForUpdate(false)">
        {{
          updateState.phase === "checking"
            ? "Recherche en cours…"
            : updateState.phase === "downloading"
              ? `Téléchargement ${updateState.percent ?? ""}${updateState.percent === null ? "" : " %"}`
              : updateState.phase === "installing"
                ? "Installation…"
                : "Rechercher une mise à jour"
        }}
      </button>
    </div>

    <h3 v-show="section === 'general'" class="cat">Profils et confidentialité</h3>
    <div v-show="section === 'general'" class="card">
      <h3>Profils</h3>
      <div class="inline" style="gap: 10px; margin-bottom: 12px">
        <button @click="exportProfile">Exporter le profil…</button>
        <button @click="importProfile">Importer un profil…</button>
      </div>
      <span class="hint" v-if="profileMsg">{{ profileMsg }}</span>
      <p class="hint">
        Un profil contient tout : effets (globaux et par zone), modes matériels,
        courbes ventilateurs et réglages.
      </p>
    </div>

    <div v-show="section === 'general'" class="card">
      <h3>Confidentialité</h3>
      <div class="inline">
        <input id="telemetry" type="checkbox" v-model="form.telemetry_opt_in" />
        <label for="telemetry">
          Envoyer les informations de diagnostic (modèle de carte mère, mémoire, VID/PID USB, constats
          de l'inventaire, erreurs de détection et de démarrage, derniers avertissements du journal) pour
          corriger le matériel non reconnu. Ni numéro de série, ni chemin, ni nom de compte. Désactivé par défaut.
        </label>
      </div>
    </div>

    <h3 v-show="section === 'general'" class="cat">Mode jeu</h3>
    <GameModeSetting v-show="section === 'general'" />

    <h3 v-show="section === 'general'" class="cat">Arrière-plan</h3>
    <BackgroundSetting v-show="section === 'general'" />

    <div v-show="section === 'media'" class="card">
      <h3>Photos des produits</h3>
      <p class="hint">
        Quand un appareil est reconnu avec certitude (identifiant USB ou modèle exact), PureRGB affiche sa photo,
        téléchargée une seule fois depuis le site du fabricant (images vérifiées, réduites, mises en cache 30 jours).
        Désactivé : aucune connexion n'est faite.
      </p>
      <div class="inline">
        <input id="productphotos" type="checkbox" :checked="photosEnabled" @change="onPhotosToggle" />
        <label for="productphotos">Afficher les photos des produits (téléchargées depuis le site du fabricant)</label>
      </div>
    </div>

    <DecoderSetting v-show="section === 'media'" />

    <DriversSetting v-show="section === 'drivers'" :form="form" />

    <SelfTest v-show="section === 'security'" />

    </div>

    <div class="actions">
      <button class="primary" :disabled="saving" @click="save">
        {{ saving ? "Enregistrement..." : "Enregistrer" }}
      </button>
      <span v-if="error" class="error">{{ error }}</span>
    </div>

    <DiagnosticPanel
      v-show="section === 'diag'"
      :telemetry-opt-in="props.settings?.telemetry_opt_in ?? false"
      :trigger="props.diagnosticTrigger"
      @restart="restartOpenRgb"
    />
  </section>
</template>

<style scoped>
.sections {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.sections button {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 6px 14px;
  font-size: 12px;
}

.sections button.active {
  background: var(--accent, #ff5a1f);
  border-color: transparent;
  color: #fff;
}

.settings {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

/* Cartes réparties sur 2 à 3 colonnes selon la largeur (au lieu d'une pile étroite). */
.cols {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(max(420px, 31%), 1fr));
  gap: 14px;
  align-items: start;
}

.cols .card {
  margin-bottom: 0;
}

.cat {
  grid-column: 1 / -1;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-dim);
  border-bottom: 1px solid var(--border);
  padding-bottom: 4px;
  margin-top: 6px;
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
</style>
