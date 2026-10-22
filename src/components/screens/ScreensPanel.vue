<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { demo, errMsg, scanErrors, screenState } from "../../screens/api.ts";
import { loadScreens, loaded, scan, scanning, screens } from "../../screens/store.ts";
import type { ScreenInfo, ScreenState } from "../../screens/types.ts";
import ProductPhoto from "../ProductPhoto.vue";
import type { ProductView } from "../../productPhotos";
import ScreenDetail from "./ScreenDetail.vue";
import "./screens.css";

const emit = defineEmits<{ toast: [msg: string] }>();

const selectedId = ref<string | null>(null);
const states = ref<Record<string, ScreenState>>({});
const products = ref<Record<string, ProductView | null>>({});
const reasons = ref<string[]>([]);
let unlisten: UnlistenFn | undefined;

const selected = computed<ScreenInfo | null>(() => screens.value.find((s) => s.id === selectedId.value) ?? null);

const PROTOCOLS: Record<string, string> = {
  thermalright_scsi: "Thermalright SCSI",
  thermalright_hid: "Thermalright HID",
  thermalright_bulk: "Thermalright Bulk",
  thermalright_ly: "Thermalright LY",
  turing_serial: "Turing (série)",
  corsair_lcd: "Corsair iCUE LINK",
  deepcool_lm: "DeepCool LM",
  asus_ryujin: "ASUS Ryujin",
  thermaltake_lcd: "Thermaltake",
  lian_li_hid: "Lian Li",
  turzx_usb: "TURZX / Lian Li USB",
  simulated: "Simulé",
};

async function loadStates(): Promise<void> {
  for (const s of screens.value) {
    try {
      states.value[s.id] = await screenState(s.id);
    } catch (e) {
      states.value[s.id] = { layout: null, brightness: 0, rotation: 0, fps: 0, running: false, live: true, perf: { fps: 0, target_fps: 0, render_ms: 0, encode_ms: 0, send_ms: 0, dropped: 0, quality: 0 }, error: errMsg(e) };
    }
  }
}

async function doScan(withSim = false): Promise<void> {
  try {
    await scan(withSim);
    await refreshReasons();
    emit("toast", screens.value.length ? `${screens.value.length} écran(s) détecté(s)` : "Aucun écran détecté");
  } catch (e) {
    emit("toast", `Recherche d'écrans : ${errMsg(e)}`);
  }
}

function stateLabel(id: string): { text: string; cls: string } {
  const s = states.value[id];
  if (!s) return { text: "…", cls: "" };
  if (s.error) return { text: "Erreur", cls: "err" };
  return s.running ? { text: "Actif", cls: "ok" } : { text: "Éteint", cls: "" };
}

async function refreshReasons(): Promise<void> {
  reasons.value = await scanErrors();
}

onMounted(async () => {
  try {
    await loadScreens();
    await refreshReasons();
  } catch (e) {
    emit("toast", `Écrans : ${errMsg(e)}`);
  }
  // Détection automatique d'un écran branché après coup : la liste se met à jour toute seule.
  if (!demo.value) {
    unlisten = await listen("screens-changed", () => void loadScreens().then(refreshReasons));
  }
});

onUnmounted(() => unlisten?.());

watch(screens, () => void loadStates(), { immediate: true });
</script>

<template>
  <section class="sc-page">
    <div v-if="demo" class="sc-demo">
      Mode démonstration : le moteur Écrans n'est pas disponible, les données et l'aperçu sont simulés.
    </div>

    <ScreenDetail v-if="selected" :key="selected.id" :screen="selected" @back="selectedId = null" @toast="emit('toast', $event)" />

    <div v-else class="sc-scroll">
      <div class="sc-row head">
        <div class="sc-grow">
          <h2 class="sc-h2">Écrans</h2>
          <p class="sc-sub">Mini-écrans LCD des boîtiers, AIO et ventirads : designs, widgets live, images et vidéos.</p>
        </div>
        <button :disabled="scanning" @click="doScan(false)">{{ scanning ? "Recherche…" : "Rechercher" }}</button>
      </div>

      <div v-if="screens.length" class="cards">
        <button v-for="s in screens" :key="s.id" class="sc-card scr" @click="selectedId = s.id">
          <ProductPhoto :vendor="s.vendor" :name="s.model" :usb="s.id" :size="96" caption @found="products[s.id] = $event">
            <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 4h18v12H3z" /><path d="M8 20h8" /><path d="M12 16v4" />
            </svg>
          </ProductPhoto>
          <span class="name">{{ products[s.id]?.name ?? `${s.vendor} ${s.model}` }}</span>
          <span class="res">
            {{ s.width }} × {{ s.height }}<template v-if="!s.resolution_confirmed"> (non confirmée)</template>
          </span>
          <span class="badges">
            <span class="sc-badge">{{ PROTOCOLS[s.protocol] ?? s.protocol }}</span>
            <span v-if="!s.verified" class="sc-badge warn" title="Pas encore validé sur du matériel réel">Expérimental</span>
            <span class="sc-badge" :class="stateLabel(s.id).cls">{{ stateLabel(s.id).text }}</span>
          </span>
          <span v-if="states[s.id]?.error" class="sc-err">{{ states[s.id]?.error }}</span>
        </button>
      </div>

      <div v-else-if="loaded" class="sc-card empty">
        <svg class="big" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 4h18v12H3z" /><path d="M8 20h8" /><path d="M12 16v4" /><path d="M7 9h10" />
        </svg>
        <h3>Aucun mini-écran détecté</h3>
        <p>
          Fermez TRCC, L-Connect, ou tout logiciel constructeur qui pilote l'écran : un seul programme à la fois peut
          parler au périphérique. Vérifiez ensuite le câble USB interne, puis cliquez sur « Rechercher ».
        </p>
        <div class="sc-row center">
          <button class="primary" :disabled="scanning" @click="doScan(false)">Rechercher</button>
          <button :disabled="scanning" @click="doScan(true)">Écran simulé</button>
        </div>
        <ul v-if="reasons.length" class="sc-err">
          <li v-for="r in reasons" :key="r">{{ r }}</li>
        </ul>
        <p class="hint">L'écran simulé permet d'essayer les designs et l'éditeur sans aucun matériel.</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.head {
  align-items: flex-start;
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 14px;
  margin-top: 18px;
}

.scr {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  padding: 16px;
  text-align: left;
}

.scr:hover {
  border-color: var(--accent);
}

.ico {
  width: 30px;
  height: 30px;
  color: var(--accent);
}

.name {
  font-weight: 600;
  font-size: 15px;
}

.res {
  color: var(--text-dim);
  font-size: 12px;
}

.badges {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 4px;
}

.empty {
  margin-top: 28px;
  padding: 36px 28px;
  text-align: center;
  max-width: 640px;
  margin-inline: auto;
}

.big {
  width: 56px;
  height: 56px;
  color: var(--text-dim);
}

.empty h3 {
  margin: 10px 0 8px;
  font-size: 17px;
}

.empty p {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.6;
  margin-bottom: 16px;
}

.empty .hint {
  margin: 12px 0 0;
  font-size: 12px;
}

.center {
  justify-content: center;
}
</style>
