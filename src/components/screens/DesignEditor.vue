<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { demo, designExport, designGet, designUserSave, errMsg, pickSavePath } from "../../screens/api.ts";
import { clamp, type Handle } from "../../screens/geometry.ts";
import { ICONS } from "../../screens/icons.ts";
import { BLOCKS, instantiateBlock, type BlockDef } from "../../screens/blocks/index.ts";
import { exportTheme, importTheme } from "../../screens/themeIO.ts";
import { addWidget, align, duplicateWidgets, removeWidgets, reorder, type AlignKind, type ZMove } from "../../screens/layoutOps.ts";
import { loadDesigns, media, values, watchValues, loadMedia } from "../../screens/store.ts";
import type { Layout, ScreenInfo, Widget, WidgetType } from "../../screens/types.ts";
import { MAX_WIDGETS } from "../../screens/types.ts";
import { useDrag } from "../../screens/useDrag.ts";
import { useHistory } from "../../screens/useHistory.ts";
import { usePreview } from "../../screens/usePreview.ts";
import { useSelection } from "../../screens/useSelection.ts";
import { validateLayout } from "../../screens/validate.ts";
import { blankLayout, cloneLayout, makeWidget } from "../../screens/widgets.ts";
import BlockPanel from "./BlockPanel.vue";
import EditorCanvas from "./EditorCanvas.vue";
import Icon from "./Icon.vue";
import RotateButtons from "./RotateButtons.vue";
import WidgetInspector from "./WidgetInspector.vue";
import WidgetPalette from "./WidgetPalette.vue";
import "./screens.css";

const props = defineProps<{ screen: ScreenInfo; initial: Layout | null; active: boolean; rotation: number }>();
const emit = defineEmits<{ apply: [layout: Layout]; toast: [msg: string]; rotate: [deg: number] }>();

const history = useHistory(props.initial ?? blankLayout(props.screen.width, props.screen.height));
const { layout, canUndo, canRedo, dirty } = history;
const sel = useSelection(layout);

const canvas = ref<InstanceType<typeof EditorCanvas> | null>(null);
const stage = computed<HTMLElement | null>(() => canvas.value?.stageEl ?? null);
const zoom = ref(1);
const showGrid = ref(false);
const gridSize = ref(10);
const snapOn = ref(true);
const savedId = ref<string | null>(null);
const leftTab = ref<"widgets" | "blocks">("widgets");

const layoutRef = computed<Layout>(() => layout.value);
const preview = usePreview(layoutRef, 960, 1000);
const drag = useDrag({ stage, layout, selection: sel, snapOn, grid: computed(() => (showGrid.value ? gridSize.value : 0)), onCommit: () => history.commit() });

const full = computed(() => layout.value.widgets.length >= MAX_WIDGETS);
const hasSel = computed(() => sel.ids.value.length > 0);
const zoomPct = computed(() => `${Math.round(zoom.value * 100)} %`);

// --- Actions sur le layout ---

function mutate(fn: () => void, key: string | null = null): void {
  fn();
  history.commit(key);
}

function onAdd(type: WidgetType): void {
  const w = makeWidget(type, layout.value);
  mutate(() => addWidget(layout.value, w));
  sel.set([w.id]);
}

/** Insère un bloc prêt à l'emploi (au centre, ou au point `at` en pixels de design) et le sélectionne. */
function onAddBlock(def: BlockDef, ctx: { accent: string; param: string }, at?: { x: number; y: number }): void {
  const room = MAX_WIDGETS - layout.value.widgets.length;
  const ws = instantiateBlock(def, ctx, layout.value, at);
  if (ws.length > room) {
    emit("toast", `Pas assez de place pour ce bloc (${ws.length} widgets, ${Math.max(room, 0)} libres)`);
    return;
  }
  mutate(() => ws.forEach((w) => addWidget(layout.value, w)));
  sel.set(ws.map((w) => w.id));
}

function onDropBlock(e: DragEvent): void {
  const raw = e.dataTransfer?.getData("application/x-purergb-block");
  const r = stage.value?.getBoundingClientRect();
  if (!raw || !r || r.width === 0) return;
  e.preventDefault();
  try {
    const d = JSON.parse(raw) as { id: string; accent: string; param: string };
    const def = BLOCKS.find((b) => b.id === d.id);
    if (def) onAddBlock(def, d, { x: ((e.clientX - r.left) / r.width) * layout.value.width, y: ((e.clientY - r.top) / r.height) * layout.value.height });
  } catch {
    emit("toast", "Bloc illisible");
  }
}

function removeSel(): void {
  if (!hasSel.value) return;
  const ids = new Set(sel.ids.value);
  mutate(() => removeWidgets(layout.value, ids));
}

function dupSel(): void {
  if (!hasSel.value) return;
  const ids = new Set(sel.ids.value);
  let created: string[] = [];
  mutate(() => (created = duplicateWidgets(layout.value, ids)));
  sel.set(created);
}

function zMove(m: ZMove): void {
  if (hasSel.value) mutate(() => reorder(layout.value, new Set(sel.ids.value), m));
}

function alignSel(k: AlignKind): void {
  if (hasSel.value) mutate(() => align(layout.value, new Set(sel.ids.value), k));
}

function nudge(dx: number, dy: number): void {
  if (!hasSel.value) return;
  mutate(() => {
    for (const w of sel.widgets.value) {
      w.x += dx;
      w.y += dy;
    }
  }, `nudge`);
}

function load(l: Layout): void {
  const err = validateLayout(l);
  if (err) {
    emit("toast", `Design refusé : ${err}`);
    return;
  }
  history.reset(l);
  sel.clear();
  savedId.value = null;
  zoom.value = 1;
}

/** Utilise un média de la bibliothèque comme fond ou comme widget Image. */
function applyMedia(mediaId: string, as: "background" | "image"): void {
  if (as === "background") {
    mutate(() => (layout.value.background = { kind: "media", media_id: mediaId, fit: "cover" }));
    return;
  }
  const item = media.value.find((m) => m.id === mediaId);
  const w = makeWidget("image", layout.value);
  if (w.type === "image") w.media_id = mediaId;
  if (item && item.width > 0 && item.height > 0) {
    const k = Math.min((layout.value.width * 0.6) / item.width, (layout.value.height * 0.6) / item.height);
    w.w = Math.round(item.width * k);
    w.h = Math.round(item.height * k);
    w.x = Math.round((layout.value.width - w.w) / 2);
    w.y = Math.round((layout.value.height - w.h) / 2);
  }
  mutate(() => addWidget(layout.value, w));
  sel.set([w.id]);
}

// --- Souris ---

function onWidgetDown(e: PointerEvent, w: Widget): void {
  if (e.shiftKey) {
    sel.toggle(w.id);
    if (!sel.isSelected(w.id)) return;
  } else if (!sel.isSelected(w.id)) {
    sel.set([w.id]);
  }
  drag.begin(e, { kind: "move" });
}

const onHandleDown = (e: PointerEvent, handle: Handle): void => drag.begin(e, { kind: "resize", handle });
const onRotateDown = (e: PointerEvent): void => drag.begin(e, { kind: "rotate" });
function onBackgroundDown(e: PointerEvent): void {
  if (!e.shiftKey) sel.clear();
  drag.begin(e, { kind: "marquee" });
}

function setZoom(z: number): void {
  zoom.value = clamp(Math.round(z * 100) / 100, 0.25, 4);
}

// --- Clavier ---

function onKey(e: KeyboardEvent): void {
  if (!props.active) return;
  const t = e.target as HTMLElement | null;
  if (t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA")) return;
  const mod = e.ctrlKey || e.metaKey;
  const k = e.key.toLowerCase();
  const step = e.shiftKey ? 10 : 1;
  if (mod && k === "z" && !e.shiftKey) history.undo();
  else if (mod && (k === "y" || (k === "z" && e.shiftKey))) history.redo();
  else if (mod && k === "d") dupSel();
  else if (mod && k === "a") sel.selectAll();
  else if (k === "delete" || k === "backspace") removeSel();
  else if (k === "escape") sel.clear();
  else if (k === "arrowleft") nudge(-step, 0);
  else if (k === "arrowright") nudge(step, 0);
  else if (k === "arrowup") nudge(0, -step);
  else if (k === "arrowdown") nudge(0, step);
  else return;
  e.preventDefault();
}

// --- Enregistrer / exporter / appliquer ---

async function saveUser(): Promise<string | null> {
  const err = validateLayout(layout.value);
  if (err) {
    emit("toast", `Design invalide : ${err}`);
    return null;
  }
  try {
    const id = await designUserSave(cloneLayout(layout.value));
    savedId.value = id;
    dirty.value = false;
    await loadDesigns();
    return id;
  } catch (e) {
    emit("toast", `Enregistrement : ${errMsg(e)}`);
    return null;
  }
}

async function saveAs(): Promise<void> {
  if (await saveUser()) emit("toast", `« ${layout.value.name} » enregistré dans Mes designs`);
}

async function exportJson(): Promise<void> {
  const id = savedId.value ?? (await saveUser());
  if (!id) return;
  try {
    const path = demo.value ? "" : await pickSavePath("Exporter le design", `${layout.value.name}.json`);
    if (path === null) return;
    await designExport(id, path);
    emit("toast", "Design exporté");
  } catch (e) {
    emit("toast", `Export : ${errMsg(e)}`);
  }
}

async function exportThemeFile(): Promise<void> {
  const err = validateLayout(layout.value);
  if (err) return emit("toast", `Design invalide : ${err}`);
  await exportTheme({ layout: cloneLayout(layout.value) }, layout.value.name, (m) => emit("toast", m));
}

async function importThemeFile(): Promise<void> {
  const id = await importTheme((m) => emit("toast", m));
  if (!id) return;
  try {
    load(await designGet(id));
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

let stopValues: (() => void) | undefined;
onMounted(() => {
  window.addEventListener("keydown", onKey);
  stopValues = watchValues();
  void loadMedia();
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  stopValues?.();
});

defineExpose({ load, applyMedia });

const A: [AlignKind, string, string][] = [
  ["left", "alignLeft", "Aligner à gauche"],
  ["hcenter", "alignHCenter", "Centrer horizontalement"],
  ["right", "alignRight", "Aligner à droite"],
  ["top", "alignTop", "Aligner en haut"],
  ["vcenter", "alignVCenter", "Centrer verticalement"],
  ["bottom", "alignBottom", "Aligner en bas"],
];
const Z: [ZMove, string, string][] = [
  ["front", "front", "Premier plan"],
  ["forward", "forward", "Avancer d'un plan"],
  ["backward", "backward", "Reculer d'un plan"],
  ["back", "back", "Arrière-plan"],
];
</script>

<template>
  <div class="ed">
    <div class="tb">
      <div class="grp">
        <button class="icon" :disabled="!canUndo" title="Annuler (Ctrl+Z)" @click="history.undo()"><Icon :paths="ICONS.undo" /></button>
        <button class="icon" :disabled="!canRedo" title="Refaire (Ctrl+Y)" @click="history.redo()"><Icon :paths="ICONS.redo" /></button>
      </div>
      <div class="grp">
        <button v-for="[k, ic, t] in A" :key="k" class="icon" :disabled="!hasSel" :title="t" @click="alignSel(k)"><Icon :paths="ICONS[ic]" /></button>
      </div>
      <div class="grp">
        <button v-for="[m, ic, t] in Z" :key="m" class="icon" :disabled="!hasSel" :title="t" @click="zMove(m)"><Icon :paths="ICONS[ic]" /></button>
      </div>
      <div class="grp">
        <button class="icon" :disabled="!hasSel" title="Dupliquer (Ctrl+D)" @click="dupSel"><Icon :paths="ICONS.copy" /></button>
        <button class="icon danger" :disabled="!hasSel" title="Supprimer (Suppr)" @click="removeSel"><Icon :paths="ICONS.trash" /></button>
      </div>
      <div class="grp">
        <button class="icon" :class="{ on: showGrid }" title="Afficher la grille" @click="showGrid = !showGrid"><Icon :paths="ICONS.grid" /></button>
        <select v-if="showGrid" v-model.number="gridSize" class="gs" aria-label="Pas de la grille">
          <option v-for="g in [5, 10, 20, 40]" :key="g" :value="g">{{ g }} px</option>
        </select>
        <button class="icon" :class="{ on: snapOn }" title="Magnétisme et guides (Alt : désactiver pendant le geste)" @click="snapOn = !snapOn"><Icon :paths="ICONS.magnet" /></button>
      </div>
      <div class="grp">
        <button class="icon" title="Zoom arrière" @click="setZoom(zoom - 0.25)"><Icon :paths="ICONS.zoomOut" /></button>
        <span class="zv">{{ zoomPct }}</span>
        <button class="icon" title="Zoom avant (Ctrl+molette)" @click="setZoom(zoom + 0.25)"><Icon :paths="ICONS.zoomIn" /></button>
        <button class="icon" title="Ajuster à la fenêtre" @click="setZoom(1)"><Icon :paths="ICONS.fit" /></button>
      </div>
      <div class="grp">
        <RotateButtons :rotation="rotation" @set="emit('rotate', $event)" />
      </div>
      <span class="sc-grow"></span>
      <button class="sm" title="Enregistrer dans Mes designs" @click="saveAs"><Icon :paths="ICONS.save" class="bi" /> Enregistrer sous</button>
      <button class="sm" title="Exporter en fichier .json" @click="exportJson"><Icon :paths="ICONS.export" class="bi" /> Exporter</button>
      <button class="sm" title="Archive .purergb-theme : design + fonds, GIF, vidéos et logos utilisés" @click="exportThemeFile">Exporter le thème (avec médias)</button>
      <button class="sm" title="Importer une archive .purergb-theme" @click="importThemeFile">Importer un thème</button>
      <button class="primary sm" title="Envoyer ce design à l'écran" @click="emit('apply', layout)"><Icon :paths="ICONS.send" class="bi" /> Appliquer à l'écran</button>
    </div>

    <div class="body">
      <div class="rail">
        <div class="sc-tabs mini">
          <button :class="{ active: leftTab === 'widgets' }" @click="leftTab = 'widgets'">Widgets</button>
          <button :class="{ active: leftTab === 'blocks' }" @click="leftTab = 'blocks'">Blocs</button>
        </div>
        <WidgetPalette v-show="leftTab === 'widgets'" :full="full" @add="onAdd" />
        <BlockPanel v-show="leftTab === 'blocks'" :full="full" @add="(d, c) => onAddBlock(d, c)" />
      </div>
      <div class="center" @dragover.prevent @drop="onDropBlock">
        <EditorCanvas
          ref="canvas"
          :layout="layout"
          :preview-src="preview.src.value"
          :selected-ids="sel.ids.value"
          :zoom="zoom"
          :show-grid="showGrid"
          :grid="gridSize"
          :guides="drag.guides.value"
          :marquee="drag.marquee.value"
          @widget-down="onWidgetDown"
          @handle-down="onHandleDown"
          @rotate-down="onRotateDown"
          @background-down="onBackgroundDown"
          @zoom="setZoom(zoom + $event)"
        />
        <div class="status">
          <span>{{ layout.name }}{{ dirty ? " •" : "" }}</span>
          <span>{{ layout.width }} × {{ layout.height }} px</span>
          <span>{{ layout.widgets.length }} widget(s)</span>
          <span v-if="preview.error.value" class="sc-err">{{ preview.error.value }}</span>
          <span class="sc-grow"></span>
          <span class="keys">Maj : multi / ratio / 15° · Alt : sans magnétisme · flèches : 1 px (Maj 10)</span>
        </div>
      </div>
      <WidgetInspector :layout="layout" :selected="sel.widgets.value" :values="values" :media="media" @commit="history.commit($event)" />
    </div>
  </div>
</template>

<style scoped>
.ed {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tb {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 14px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
}

.grp {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-right: 12px;
  border-right: 1px solid var(--border);
}

.grp:last-of-type {
  border-right: none;
}

.zv {
  min-width: 48px;
  text-align: center;
  font-size: 12px;
  color: var(--text-dim);
}

.gs {
  width: 76px;
  padding: 4px 6px;
  font-size: 12px;
}

.bi {
  width: 14px;
  height: 14px;
  vertical-align: -2px;
}

.body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.rail {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-right: 1px solid var(--border);
}

.rail > nav {
  flex: 1;
  min-height: 0;
  border-right: none;
}

.rail .mini {
  padding: 0 6px;
}

.center {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.status {
  display: flex;
  gap: 18px;
  padding: 6px 14px;
  font-size: 11px;
  color: var(--text-dim);
  border-top: 1px solid var(--border);
  background: var(--bg-panel);
  white-space: nowrap;
  overflow: hidden;
}

.keys {
  opacity: 0.7;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
