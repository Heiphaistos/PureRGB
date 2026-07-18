<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { reactive, ref, watch } from "vue";
import type { Settings } from "../types";

const props = defineProps<{ settings: Settings | null }>();
const emit = defineEmits<{ saved: [] }>();

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
      <h3>Performance</h3>
      <div class="grid2">
        <div>
          <label>Images/seconde des animations — {{ form.fps }} FPS</label>
          <input type="range" min="5" max="60" step="5" v-model.number="form.fps" />
          <p class="hint">
            Plus bas = moins de CPU/USB. 30 FPS est fluide. Les effets statiques
            ne consomment rien quel que soit ce réglage.
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
</style>
