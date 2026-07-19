<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import ConflictPanel from "./components/ConflictPanel.vue";
import DeviceList from "./components/DeviceList.vue";
import EffectPanel from "./components/EffectPanel.vue";
import FanPanel from "./components/FanPanel.vue";
import LcdPanel from "./components/LcdPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import type {
  BackendStatus,
  ConflictReport,
  DeviceInfo,
  EffectConfig,
  OpenRgbStatus,
  Settings,
} from "./types";

const devices = ref<DeviceInfo[]>([]);
const backends = ref<BackendStatus[]>([]);
const conflicts = ref<ConflictReport>({ conflicts: [], openrgb_running: false });
const settings = ref<Settings | null>(null);
const selectedId = ref<string | null>(null);
const tab = ref<"rgb" | "fans" | "lcd" | "conflicts" | "settings">("rgb");
const scanning = ref(false);
const toast = ref("");
const orgb = ref<OpenRgbStatus>({
  exe_path: null,
  server_reachable: false,
  managed: false,
  pawnio_installed: true,
  pawnio_ready: true,
});
const orgbStarting = ref(false);
const pawnioInstalling = ref(false);

const activeConflicts = computed(
  () => conflicts.value.conflicts.filter((c) => c.active).length,
);

const selected = computed(
  () => devices.value.find((d) => d.id === selectedId.value) ?? null,
);
const fanDevices = computed(() =>
  devices.value.filter((d) => d.fan_channels.length > 0),
);
const lcdDevices = computed(() => devices.value.filter((d) => d.has_lcd));

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
    orgb.value = await invoke<OpenRgbStatus>("openrgb_status");
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

async function onApplyEffect(deviceId: string, config: EffectConfig, zone: number | null) {
  try {
    await invoke("apply_effect", { deviceId, config, zone });
    showToast(zone === null ? "Effet appliqué" : "Effet appliqué à la zone");
    await loadSettings();
  } catch (e) {
    showToast(`Échec : ${e}`);
  }
}

async function onApplyMode(
  deviceId: string,
  modeIndex: number,
  speed: number | null,
  direction: number | null,
  colors: { r: number; g: number; b: number }[] | null,
) {
  try {
    await invoke("set_hardware_mode", { deviceId, modeIndex, speed, direction, colors });
    showToast("Mode matériel appliqué");
    await refresh();
  } catch (e) {
    showToast(`Mode matériel : ${e}`);
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

async function installPawnio() {
  pawnioInstalling.value = true;
  try {
    await invoke("pawnio_install");
    showToast("PawnIO installé — redémarrage d'OpenRGB…");
    if (orgb.value.managed) {
      await invoke("openrgb_restart");
    }
    await refresh();
  } catch (e) {
    showToast(`PawnIO : ${e}`);
  } finally {
    pawnioInstalling.value = false;
  }
}

async function startOpenRgb() {
  orgbStarting.value = true;
  try {
    await invoke<boolean>("openrgb_start");
    showToast("OpenRGB démarré");
    await refresh();
  } catch (e) {
    showToast(`OpenRGB : ${e}`);
  } finally {
    orgbStarting.value = false;
  }
}

onMounted(async () => {
  await loadSettings();
  await refresh();
  // L'init matériel en arrière-plan peut finir après le premier rendu :
  // re-scanner tant que le serveur n'est pas joignable (max ~30 s).
  for (let i = 0; i < 6 && !orgb.value.server_reachable; i++) {
    await new Promise((r) => setTimeout(r, 5000));
    await refresh();
  }
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
        <button v-if="lcdDevices.length" :class="{ active: tab === 'lcd' }" @click="tab = 'lcd'">
          Écran LCD
        </button>
        <button :class="{ active: tab === 'conflicts' }" @click="tab = 'conflicts'">
          Conflits<span v-if="activeConflicts > 0" class="badge">{{ activeConflicts }}</span>
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

    <div v-if="activeConflicts > 0" class="conflict-banner">
      <span>
        ⚠️ Logiciels RGB actifs :
        <strong>{{ conflicts.conflicts.filter((c) => c.active).map((c) => c.name).join(", ") }}</strong>
        — ils masquent des appareils à OpenRGB.
      </span>
      <button @click="tab = 'conflicts'">Gérer les conflits</button>
    </div>
    <div
      v-if="orgb.server_reachable && !orgb.pawnio_ready"
      class="info-banner"
    >
      <span>
        Driver PawnIO {{ orgb.pawnio_installed ? "inactif" : "absent" }} — la RAM
        et la carte mère ne peuvent pas être détectées sans lui (accès SMBus).
      </span>
      <button class="primary" :disabled="pawnioInstalling" @click="installPawnio">
        {{ pawnioInstalling ? "Installation..." : orgb.pawnio_installed ? "Réparer PawnIO" : "Installer PawnIO" }}
      </button>
    </div>
    <div v-if="!orgb.server_reachable" class="info-banner">
      <span>
        Serveur OpenRGB non joignable — nécessaire pour piloter 900+ appareils.
        {{ orgb.exe_path ? "OpenRGB embarqué prêt." : "OpenRGB sera téléchargé (officiel, vérifié)." }}
      </span>
      <button class="primary" :disabled="orgbStarting" @click="startOpenRgb">
        {{ orgbStarting ? "Démarrage..." : "Démarrer OpenRGB" }}
      </button>
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
          @apply-mode="onApplyMode"
        />
      </template>
      <FanPanel
        v-else-if="tab === 'fans'"
        :devices="fanDevices"
        :settings="settings"
        @toast="showToast"
        @saved="loadSettings"
      />
      <LcdPanel v-else-if="tab === 'lcd'" :devices="lcdDevices" @toast="showToast" />
      <ConflictPanel
        v-else-if="tab === 'conflicts'"
        :conflicts="conflicts"
        :openrgb-managed="orgb.managed"
        @refresh="refresh"
        @toast="showToast"
      />
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.badge {
  display: inline-block;
  margin-left: 6px;
  min-width: 18px;
  padding: 1px 5px;
  border-radius: 999px;
  background: var(--warn);
  color: #1a1206;
  font-size: 11px;
  font-weight: 700;
  text-align: center;
}

.info-banner {
  background: var(--accent-soft);
  border-bottom: 1px solid var(--border);
  color: var(--text-dim);
  padding: 9px 18px;
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
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
