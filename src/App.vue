<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { computed, onMounted, ref } from "vue";
import ConflictPanel from "./components/ConflictPanel.vue";
import DeviceCanvas from "./components/DeviceCanvas.vue";
import DeviceGrid from "./components/DeviceGrid.vue";
import EffectDrawer from "./components/EffectDrawer.vue";
import FanPanel from "./components/FanPanel.vue";
import LcdPanel from "./components/LcdPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import type { LayoutMode } from "./components/SettingsPanel.vue";
import SmartPanel from "./components/SmartPanel.vue";
import ThemePanel from "./components/ThemePanel.vue";
import type {
  BackendStatus,
  ConflictReport,
  DeviceInfo,
  EffectConfig,
  OpenRgbStatus,
  Settings,
} from "./types";
import { deviceHasRgb } from "./types";

type TabId = "rgb" | "themes" | "smart" | "fans" | "lcd" | "conflicts" | "settings";

interface UnknownDeviceAlert {
  vid: string;
  pid: string;
  manufacturer: string;
  product: string;
}

// Chemins SVG (stroke, viewBox 0 0 24 24) pour la nav latérale.
const TABS: { id: TabId; label: string; icon: string[] }[] = [
  {
    id: "rgb",
    label: "Éclairage",
    icon: ["M12 3a6 6 0 0 0-4 10.5c.5.5 1 1.5 1 2.5h6c0-1 .5-2 1-2.5A6 6 0 0 0 12 3z", "M9 18h6", "M10 21h4"],
  },
  { id: "themes", label: "Thèmes", icon: ["M3 7l9-4 9 4-9 4-9-4z", "M3 12l9 4 9-4", "M3 17l9 4 9-4"] },
  { id: "smart", label: "Maison", icon: ["M3 11l9-7 9 7", "M5 10v10h14V10", "M10 20v-5h4v5"] },
  {
    id: "fans",
    label: "Ventilateurs",
    icon: [
      "M12 12m-1.6 0a1.6 1.6 0 1 0 3.2 0a1.6 1.6 0 1 0 -3.2 0",
      "M12 10.4C12 6 14 3 17 4c2 .7 1 3-1 4.4-1.3.9-2.7 1.4-4 2z",
      "M10.3 12.9C6.4 14 3.4 13 4 10c.5-2 3.1-1.4 4.7.4.9 1 1.4 1.6 1.6 2.5z",
      "M13.7 12.9c3.9 1.1 6.9.1 6.3-2.9-.5-2-3.1-1.4-4.7.4-.9 1-1.4 1.6-1.6 2.5z",
    ],
  },
  { id: "lcd", label: "Écran LCD", icon: ["M3 4h18v12H3z", "M8 20h8", "M12 16v4"] },
  { id: "conflicts", label: "Conflits", icon: ["M12 2 1 21h22L12 2z", "M12 9v5", "M12 17h.01"] },
  {
    id: "settings",
    label: "Réglages",
    icon: [
      "M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
      "M12 3v3",
      "M12 18v3",
      "M3 12h3",
      "M18 12h3",
      "M5.6 5.6l2.1 2.1",
      "M16.3 16.3l2.1 2.1",
      "M5.6 18.4l2.1-2.1",
      "M16.3 7.7l2.1-2.1",
    ],
  },
];

const devices = ref<DeviceInfo[]>([]);
const backends = ref<BackendStatus[]>([]);
const conflicts = ref<ConflictReport>({ conflicts: [], openrgb_running: false, guarded_families: [] });
const settings = ref<Settings | null>(null);
const selectedId = ref<string | null>(null);
const drawerOpen = ref(false);
const tab = ref<TabId>("rgb");
const layoutMode = ref<LayoutMode>(
  (localStorage.getItem("purergb-layout") as LayoutMode | null) ?? "grid",
);

function setLayout(mode: LayoutMode) {
  layoutMode.value = mode;
  localStorage.setItem("purergb-layout", mode);
}
const scanning = ref(false);
const toast = ref("");
const pendingAlerts = ref<UnknownDeviceAlert[]>([]);
const diagnosticTrigger = ref(0);

function openDiagnosticFor(index: number) {
  pendingAlerts.value.splice(index, 1);
  tab.value = "settings";
  diagnosticTrigger.value++;
}

function dismissAlert(index: number) {
  pendingAlerts.value.splice(index, 1);
}
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
const gridEffects = computed(() => settings.value?.effects ?? {});
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

function onSelectDevice(id: string) {
  if (selectedId.value === id && drawerOpen.value) {
    drawerOpen.value = false;
  } else {
    selectedId.value = id;
    drawerOpen.value = true;
  }
}

function closeDrawer() {
  drawerOpen.value = false;
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
  if (zone === null) {
    const target = devices.value.find((d) => d.id === deviceId);
    if (target && !deviceHasRgb(target)) {
      showToast("Pas de RGB sur cet appareil (pilotage PWM/vitesse uniquement)");
      return;
    }
  }
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

  // Notification OS best-effort — l'événement in-app ci-dessous reste le
  // canal garanti même si la permission est refusée ou jamais accordée.
  try {
    if (!(await isPermissionGranted())) {
      await requestPermission();
    }
  } catch {
    /* plateforme sans notifications ou permission indisponible — ignoré */
  }

  await listen<UnknownDeviceAlert[]>("unknown-device-detected", (event) => {
    pendingAlerts.value.push(...event.payload);
  });
});
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-dot"></span>
      </div>
      <nav class="tabs">
        <button
          v-for="t in TABS"
          v-show="t.id !== 'lcd' || lcdDevices.length"
          :key="t.id"
          :class="{ active: tab === t.id }"
          :title="t.label"
          @click="tab = t.id"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path v-for="d in t.icon" :key="d" :d="d" />
          </svg>
          <span class="tab-label">{{ t.label }}</span>
          <span v-if="t.id === 'conflicts' && activeConflicts > 0" class="badge">{{ activeConflicts }}</span>
        </button>
      </nav>
    </aside>

    <div class="main-col">
    <header class="topbar">
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
      <DeviceGrid
        v-if="tab === 'rgb' && layoutMode !== 'canvas'"
        :devices="devices"
        :selected-id="selectedId"
        :effects="gridEffects"
        :dense="layoutMode === 'list'"
        @select="onSelectDevice"
      />
      <DeviceCanvas
        v-else-if="tab === 'rgb' && layoutMode === 'canvas'"
        :devices="devices"
        :saved-effects="gridEffects"
        @apply="onApplyEffect"
        @apply-all="onApplyAll"
        @apply-mode="onApplyMode"
        @toast="showToast"
        @refresh="refresh"
      />
      <ThemePanel v-else-if="tab === 'themes'" @apply-all="onApplyAll" />
      <SmartPanel v-else-if="tab === 'smart'" @toast="showToast" @refresh="refresh" />
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
        :layout="layoutMode"
        :diagnostic-trigger="diagnosticTrigger"
        @saved="loadSettings(); refresh(); showToast('Réglages enregistrés')"
        @layout-change="setLayout"
      />
    </main>
    </div>

    <EffectDrawer
      :open="drawerOpen && tab === 'rgb' && layoutMode !== 'canvas'"
      :device="selected"
      :saved-effects="settings?.effects ?? {}"
      @apply="onApplyEffect"
      @apply-all="onApplyAll"
      @apply-mode="onApplyMode"
      @toast="showToast"
      @refresh="refresh"
      @close="closeDrawer"
    />

    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>

    <div v-if="pendingAlerts.length" class="hotplug-alerts">
      <div v-for="(alert, i) in pendingAlerts" :key="`${alert.vid}:${alert.pid}`" class="hotplug-alert">
        <span
          >Nouveau matériel non reconnu détecté : {{ alert.manufacturer }} {{ alert.product }} ({{ alert.vid }}:{{
            alert.pid
          }})</span
        >
        <div class="hotplug-alert-actions">
          <button @click="openDiagnosticFor(i)">Ouvrir le diagnostic</button>
          <button @click="dismissAlert(i)">Ignorer</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex-direction: row;
  height: 100vh;
}

.sidebar {
  width: 88px;
  min-width: 88px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-4) var(--space-2);
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  overflow-y: auto;
}

.main-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: var(--space-3) var(--space-4);
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
}

.brand {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: var(--space-3);
}

.brand-dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: conic-gradient(#ff5000, #ff00c8, #0090ff, #3ecf6e, #ff5000);
}

.tabs {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  width: 100%;
}

.tabs button {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-dim);
  padding: var(--space-2) 2px;
  border-radius: var(--radius-sm);
  font-size: 10px;
  line-height: 1.2;
  text-align: center;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.tabs button svg {
  width: 20px;
  height: 20px;
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
  padding: var(--space-2) var(--space-4);
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.badge {
  position: absolute;
  top: 2px;
  right: 10px;
  min-width: 15px;
  height: 15px;
  padding: 0 3px;
  border-radius: 999px;
  background: var(--warn);
  color: #1a1206;
  font-size: 9px;
  font-weight: 700;
  line-height: 15px;
  text-align: center;
}

.info-banner {
  background: var(--accent-soft);
  border-bottom: 1px solid var(--border);
  color: var(--text-dim);
  padding: var(--space-2) var(--space-4);
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
  box-shadow: var(--shadow-md);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.hotplug-alerts {
  position: fixed;
  bottom: 16px;
  right: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 50;
  max-width: 360px;
}
.hotplug-alert {
  background: #1c1c1c;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 12px;
  color: #eee;
  font-size: 0.9em;
}
.hotplug-alert-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
</style>
