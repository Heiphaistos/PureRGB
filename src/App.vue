<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import DeviceList from "./components/DeviceList.vue";
import EffectPanel from "./components/EffectPanel.vue";
import FanPanel from "./components/FanPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import type {
  BackendStatus,
  ConflictReport,
  DeviceInfo,
  EffectConfig,
  Settings,
} from "./types";

const devices = ref<DeviceInfo[]>([]);
const backends = ref<BackendStatus[]>([]);
const conflicts = ref<ConflictReport>({ conflicts: [], openrgb_running: false });
const settings = ref<Settings | null>(null);
const selectedId = ref<string | null>(null);
const tab = ref<"rgb" | "fans" | "settings">("rgb");
const scanning = ref(false);
const toast = ref("");

const selected = computed(
  () => devices.value.find((d) => d.id === selectedId.value) ?? null,
);
const fanDevices = computed(() =>
  devices.value.filter((d) => d.fan_channels.length > 0),
);

let toastTimer: ReturnType<typeof setTimeout> | undefined;
function showToast(msg: string) {
  toast.value = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ""), 3500);
}

async function refresh() {
  scanning.value = true;
  try {
    devices.value = await invoke<DeviceInfo[]>("scan_devices");
    backends.value = await invoke<BackendStatus[]>("backend_status");
    conflicts.value = await invoke<ConflictReport>("check_conflicts");
    if (!selected.value && devices.value.length > 0) {
      selectedId.value = devices.value[0].id;
    }
  } catch (e) {
    showToast(`Erreur de scan : ${e}`);
  } finally {
    scanning.value = false;
  }
}

async function loadSettings() {
  settings.value = await invoke<Settings>("get_settings");
}

async function onApplyEffect(deviceId: string, config: EffectConfig) {
  try {
    await invoke("apply_effect", { deviceId, config });
    showToast("Effet appliqué");
  } catch (e) {
    showToast(`Échec : ${e}`);
  }
}

async function onApplyAll(config: EffectConfig) {
  try {
    const n = await invoke<number>("apply_effect_all", { config });
    showToast(`Effet appliqué sur ${n} appareil(s)`);
  } catch (e) {
    showToast(`Échec : ${e}`);
  }
}

onMounted(async () => {
  await loadSettings();
  await refresh();
});
</script>

<template>
  <div class="layout">
    <header class="topbar">
      <div class="brand">
        <span class="brand-dot"></span>
        <h1>PureRGB</h1>
      </div>
      <nav class="tabs">
        <button :class="{ active: tab === 'rgb' }" @click="tab = 'rgb'">
          Éclairage
        </button>
        <button :class="{ active: tab === 'fans' }" @click="tab = 'fans'">
          Ventilateurs
        </button>
        <button :class="{ active: tab === 'settings' }" @click="tab = 'settings'">
          Réglages
        </button>
      </nav>
      <div class="top-actions">
        <span
          v-for="b in backends"
          :key="b.name"
          class="backend-pill"
          :class="{ up: b.available }"
          :title="b.available ? 'connecté' : 'indisponible'"
        >
          {{ b.name }}
        </span>
        <button :disabled="scanning" @click="refresh">
          {{ scanning ? "Scan..." : "↻ Scanner" }}
        </button>
      </div>
    </header>

    <div v-if="conflicts.conflicts.length > 0" class="conflict-banner">
      ⚠️ Logiciels RGB actifs détectés :
      <strong>{{ conflicts.conflicts.map((c) => c.name).join(", ") }}</strong>
      — risque de conflit d'accès au matériel. Fermez-les pour un contrôle fiable.
    </div>
    <div
      v-if="!conflicts.openrgb_running && backends.every((b) => b.name !== 'openrgb' || !b.available)"
      class="info-banner"
    >
      OpenRGB non détecté. Lancez OpenRGB avec « Enable SDK Server » pour piloter
      900+ appareils, ou activez les drivers natifs dans Réglages.
    </div>

    <main class="content">
      <template v-if="tab === 'rgb'">
        <DeviceList
          :devices="devices"
          :selected-id="selectedId"
          @select="selectedId = $event"
        />
        <EffectPanel
          :device="selected"
          :saved-effects="settings?.effects ?? {}"
          @apply="onApplyEffect"
          @apply-all="onApplyAll"
        />
      </template>
      <FanPanel v-else-if="tab === 'fans'" :devices="fanDevices" @toast="showToast" />
      <SettingsPanel
        v-else
        :settings="settings"
        @saved="loadSettings(); refresh(); showToast('Réglages enregistrés')"
      />
    </main>

    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 12px 18px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: conic-gradient(#ff5000, #ff00c8, #0090ff, #3ecf6e, #ff5000);
}

.brand h1 {
  font-size: 17px;
  letter-spacing: 0.5px;
}

.tabs {
  display: flex;
  gap: 6px;
}

.tabs button {
  border: none;
  background: transparent;
  color: var(--text-dim);
  padding: 8px 14px;
}

.tabs button.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.top-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.backend-pill {
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--text-dim);
}

.backend-pill.up {
  border-color: var(--ok);
  color: var(--ok);
}

.conflict-banner {
  background: rgba(245, 185, 74, 0.12);
  border-bottom: 1px solid rgba(245, 185, 74, 0.4);
  color: var(--warn);
  padding: 9px 18px;
  font-size: 13px;
}

.info-banner {
  background: var(--accent-soft);
  border-bottom: 1px solid var(--border);
  color: var(--text-dim);
  padding: 9px 18px;
  font-size: 13px;
}

.content {
  flex: 1;
  display: flex;
  min-height: 0;
}

.toast {
  position: fixed;
  bottom: 22px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--accent);
  border-radius: 999px;
  padding: 9px 20px;
  font-size: 13px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
