<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { designGet, errMsg, screenState, setLayout, setOptions } from "../../screens/api.ts";
import { usePreview } from "../../screens/usePreview.ts";
import type { Layout, ScreenInfo, ScreenState } from "../../screens/types.ts";
import { validateLayout } from "../../screens/validate.ts";
import { cloneLayout } from "../../screens/widgets.ts";
import DesignEditor from "./DesignEditor.vue";
import DesignGallery from "./DesignGallery.vue";
import MediaLibrary from "./MediaLibrary.vue";
import RotatedPreview from "./RotatedPreview.vue";
import RotateButtons from "./RotateButtons.vue";
import ScreenOptions from "./ScreenOptions.vue";
import "./screens.css";

type Tab = "designs" | "editor" | "media" | "options";

const props = defineProps<{ screen: ScreenInfo }>();
const emit = defineEmits<{ back: []; toast: [msg: string] }>();

const TABS: { id: Tab; label: string }[] = [
  { id: "designs", label: "Designs" },
  { id: "editor", label: "Éditeur" },
  { id: "media", label: "Médias" },
  { id: "options", label: "Options" },
];

const tab = ref<Tab>("designs");
const state = ref<ScreenState | null>(null);
const editor = ref<InstanceType<typeof DesignEditor> | null>(null);
const live = computed<Layout | null>(() => state.value?.layout ?? null);
const { src, error } = usePreview(live, 560, 1000);

async function reload(): Promise<void> {
  try {
    state.value = await screenState(props.screen.id);
  } catch (e) {
    emit("toast", `État de l'écran : ${errMsg(e)}`);
  }
}

const rotation = computed(() => state.value?.rotation ?? props.screen.rotation);

/** Rotation rapide : appliquée tout de suite à l'écran (luminosité et cadence inchangées). */
async function rotate(deg: number): Promise<void> {
  const st = state.value;
  if (!st) return;
  try {
    await setOptions(props.screen.id, { brightness: st.brightness, rotation: deg, fps: st.fps });
    await reload();
  } catch (e) {
    emit("toast", `Rotation : ${errMsg(e)}`);
  }
}

async function applyLayout(layout: Layout): Promise<void> {
  const err = validateLayout(layout);
  if (err) {
    emit("toast", `Design refusé : ${err}`);
    return;
  }
  try {
    await setLayout(props.screen.id, layout);
    await reload();
    emit("toast", `« ${layout.name} » envoyé à l'écran`);
  } catch (e) {
    emit("toast", `Envoi impossible : ${errMsg(e)}`);
  }
}

async function onApplyDesign(id: string): Promise<void> {
  try {
    await applyLayout(await designGet(id));
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

async function onEditDesign(id: string): Promise<void> {
  try {
    editor.value?.load(await designGet(id));
    tab.value = "editor";
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

function onUseMedia(mediaId: string, as: "background" | "image"): void {
  editor.value?.applyMedia(mediaId, as);
  tab.value = "editor";
}

onMounted(reload);
</script>

<template>
  <div class="sc-page">
    <header class="top">
      <button class="sm" @click="emit('back')">← Écrans</button>
      <div class="sc-grow">
        <strong>{{ screen.vendor }} {{ screen.model }}</strong>
        <span class="dim"> · {{ screen.width }} × {{ screen.height }}</span>
      </div>
      <RotateButtons v-if="state" :rotation="rotation" @set="rotate" />
      <span v-if="!screen.verified" class="sc-badge warn">Expérimental</span>
      <span v-if="state?.error" class="sc-badge err" :title="state.error">Erreur</span>
      <span v-else-if="state" class="sc-badge" :class="{ ok: state.running }">{{ state.running ? "Actif" : "Éteint" }}</span>
    </header>

    <div class="body">
      <aside class="live">
        <div class="frame">
          <RotatedPreview v-if="src" :src="src" :rotation="rotation" :width="screen.width" :height="screen.height" />
          <div v-else class="off">{{ live ? "Rendu…" : "Écran éteint" }}</div>
        </div>
        <div class="cap">{{ live?.name ?? "Aucun design affiché" }}</div>
        <div v-if="error" class="sc-err">{{ error }}</div>
        <div v-if="state?.error" class="sc-err">{{ state.error }}</div>
      </aside>

      <div class="main">
        <nav class="sc-tabs">
          <button v-for="t in TABS" :key="t.id" :class="{ active: tab === t.id }" @click="tab = t.id">{{ t.label }}</button>
        </nav>
        <div class="pane">
          <DesignGallery v-show="tab === 'designs'" :screen="screen" @apply="onApplyDesign" @edit="onEditDesign" @toast="emit('toast', $event)" />
          <DesignEditor
            v-if="state"
            v-show="tab === 'editor'"
            ref="editor"
            :screen="screen"
            :initial="state.layout ? cloneLayout(state.layout) : null"
            :active="tab === 'editor'"
            :rotation="rotation"
            @rotate="rotate"
            @apply="applyLayout"
            @toast="emit('toast', $event)"
          />
          <MediaLibrary v-show="tab === 'media'" @use="onUseMedia" @toast="emit('toast', $event)" />
          <ScreenOptions v-if="state && tab === 'options'" :screen="screen" :state="state" @changed="reload" @toast="emit('toast', $event)" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.top {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 26px;
  border-bottom: 1px solid var(--border);
}

.dim {
  color: var(--text-dim);
  font-size: 12px;
}

.body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.live {
  width: clamp(240px, 24vw, 460px);
  flex: none;
  padding: 18px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.frame {
  width: 100%;
  background: #000;
  border: 2px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-md);
}

.off {
  padding: 40px 0;
  color: var(--text-dim);
  font-size: 13px;
}

.cap {
  color: var(--text-dim);
  font-size: 12px;
  text-align: center;
}

.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.pane > * {
  flex: 1;
  min-height: 0;
}

.pane > [style*="display: none"] {
  flex: none;
}
</style>
