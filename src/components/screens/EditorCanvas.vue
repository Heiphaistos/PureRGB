<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { HANDLES, type Box, type Handle } from "../../screens/geometry.ts";
import type { Guide } from "../../screens/snap.ts";
import type { Layout, Widget } from "../../screens/types.ts";
import { WIDGET_LABELS } from "../../screens/widgets.ts";
import "./screens.css";

const props = defineProps<{
  layout: Layout;
  previewSrc: string;
  selectedIds: string[];
  zoom: number;
  showGrid: boolean;
  grid: number;
  guides: Guide[];
  marquee: Box | null;
}>();

const emit = defineEmits<{
  widgetDown: [e: PointerEvent, w: Widget];
  handleDown: [e: PointerEvent, h: Handle];
  rotateDown: [e: PointerEvent];
  backgroundDown: [e: PointerEvent];
  zoom: [delta: number];
}>();

const wrap = ref<HTMLElement | null>(null);
const stageEl = ref<HTMLElement | null>(null);
const avail = ref({ w: 600, h: 400 });
let ro: ResizeObserver | undefined;

/** Pixels écran par pixel de design (ajusté à la zone puis multiplié par le zoom). */
const scale = computed(() => {
  const fit = Math.min((avail.value.w - 32) / props.layout.width, (avail.value.h - 32) / props.layout.height);
  return Math.max(0.05, fit) * props.zoom;
});
const stageStyle = computed(() => ({
  width: `${props.layout.width * scale.value}px`,
  height: `${props.layout.height * scale.value}px`,
}));
const single = computed(() => props.selectedIds.length === 1);

function boxStyle(w: Widget): Record<string, string> {
  const s = scale.value;
  return {
    left: `${w.x * s}px`,
    top: `${w.y * s}px`,
    width: `${w.w * s}px`,
    height: `${w.h * s}px`,
    transform: `rotate(${w.rotation}deg)`,
  };
}

const HANDLE_POS: Record<Handle, [string, string]> = {
  nw: ["0%", "0%"],
  n: ["50%", "0%"],
  ne: ["100%", "0%"],
  e: ["100%", "50%"],
  se: ["100%", "100%"],
  s: ["50%", "100%"],
  sw: ["0%", "100%"],
  w: ["0%", "50%"],
};
const CURSORS: Record<Handle, string> = {
  nw: "nwse-resize",
  se: "nwse-resize",
  ne: "nesw-resize",
  sw: "nesw-resize",
  n: "ns-resize",
  s: "ns-resize",
  e: "ew-resize",
  w: "ew-resize",
};

const gridStyle = computed(() => {
  const g = props.grid * scale.value;
  return {
    backgroundSize: `${g}px ${g}px`,
    backgroundImage: "linear-gradient(to right, rgba(255,255,255,.12) 1px, transparent 1px), linear-gradient(to bottom, rgba(255,255,255,.12) 1px, transparent 1px)",
  };
});

function onWheel(e: WheelEvent): void {
  if (!e.ctrlKey) return;
  e.preventDefault();
  emit("zoom", e.deltaY < 0 ? 0.1 : -0.1);
}

onMounted(() => {
  if (!wrap.value) return;
  ro = new ResizeObserver(() => {
    const r = wrap.value?.getBoundingClientRect();
    if (r && r.width > 0) avail.value = { w: r.width, h: r.height };
  });
  ro.observe(wrap.value);
});
onBeforeUnmount(() => ro?.disconnect());

defineExpose({ stageEl });
</script>

<template>
  <div ref="wrap" class="wrap" @wheel="onWheel">
    <div ref="stageEl" class="stage" :style="stageStyle" @pointerdown.self="emit('backgroundDown', $event)">
      <img v-if="previewSrc" :src="previewSrc" class="bg" alt="Aperçu du design" draggable="false" />
      <div v-if="showGrid" class="gridov" :style="gridStyle"></div>

      <div
        v-for="w in layout.widgets"
        :key="w.id"
        class="hit"
        :class="{ sel: selectedIds.includes(w.id) }"
        :style="boxStyle(w)"
        :title="WIDGET_LABELS[w.type]"
        @pointerdown.stop="emit('widgetDown', $event, w)"
      >
        <template v-if="single && selectedIds.includes(w.id)">
          <span class="rline"></span>
          <span class="rot" title="Pivoter (Maj : pas de 15°)" @pointerdown.stop="emit('rotateDown', $event)"></span>
          <span
            v-for="h in HANDLES"
            :key="h"
            class="hd"
            :style="{ left: HANDLE_POS[h][0], top: HANDLE_POS[h][1], cursor: CURSORS[h] }"
            @pointerdown.stop="emit('handleDown', $event, h)"
          ></span>
        </template>
      </div>

      <div
        v-for="g in guides"
        :key="`${g.axis}${g.pos}`"
        class="guide"
        :class="g.axis"
        :style="g.axis === 'x' ? { left: `${g.pos * scale}px` } : { top: `${g.pos * scale}px` }"
      ></div>
      <div
        v-if="marquee"
        class="marq"
        :style="{ left: `${marquee.x * scale}px`, top: `${marquee.y * scale}px`, width: `${marquee.w * scale}px`, height: `${marquee.h * scale}px` }"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  display: flex;
  background:
    linear-gradient(45deg, #121218 25%, transparent 25%, transparent 75%, #121218 75%),
    linear-gradient(45deg, #121218 25%, #0e0e13 25%, #0e0e13 75%, #121218 75%);
  background-size: 20px 20px;
  background-position: 0 0, 10px 10px;
}

.stage {
  position: relative;
  margin: auto;
  flex: none;
  background: #000;
  box-shadow: 0 0 0 1px var(--border), var(--shadow-md);
  touch-action: none;
  cursor: crosshair;
}

.bg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.gridov {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.hit {
  position: absolute;
  cursor: move;
  outline: 1px solid transparent;
}

.hit:hover {
  outline-color: rgba(255, 255, 255, 0.35);
}

.hit.sel {
  outline: 1.5px solid var(--accent);
}

.hd {
  position: absolute;
  width: 11px;
  height: 11px;
  margin: -5.5px 0 0 -5.5px;
  background: #fff;
  border: 1.5px solid var(--accent);
  border-radius: 2px;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5);
}

.rot {
  position: absolute;
  left: 50%;
  top: -30px;
  width: 13px;
  height: 13px;
  margin-left: -6.5px;
  background: var(--accent);
  border: 1.5px solid #fff;
  border-radius: 50%;
  cursor: grab;
}

.rline {
  position: absolute;
  left: 50%;
  top: -24px;
  width: 1px;
  height: 24px;
  background: var(--accent);
  pointer-events: none;
}

.guide {
  position: absolute;
  background: #ff2bd6;
  pointer-events: none;
}

.guide.x {
  top: 0;
  bottom: 0;
  width: 1px;
}

.guide.y {
  left: 0;
  right: 0;
  height: 1px;
}

.marq {
  position: absolute;
  border: 1px dashed var(--accent);
  background: rgba(255, 80, 0, 0.12);
  pointer-events: none;
}
</style>
