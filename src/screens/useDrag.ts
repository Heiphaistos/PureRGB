import { ref, type Ref } from "vue";
import {
  angleFromCenter,
  boxCenter,
  clientToDesign,
  intersects,
  normAngle,
  normBox,
  resizeBox,
  round1,
  snapAngle,
  unionBox,
  type Box,
  type Handle,
  HANDLE_DIR,
} from "./geometry.ts";
import { snapEdge, snapMove, type Guide } from "./snap.ts";
import type { Layout } from "./types.ts";
import type { useSelection } from "./useSelection.ts";

export type DragMode = { kind: "move" } | { kind: "resize"; handle: Handle } | { kind: "rotate" } | { kind: "marquee" };

interface Start {
  id: string;
  box: Box;
  rotation: number;
}

export interface DragOptions {
  stage: Readonly<Ref<HTMLElement | null>>;
  layout: Ref<Layout>;
  selection: ReturnType<typeof useSelection>;
  snapOn: Ref<boolean>;
  grid: Readonly<Ref<number>>;
  /** Appelé en fin de geste si le layout a changé. */
  onCommit: () => void;
}

const SNAP_PX = 6;
const CLICK_PX = 3;

/** Gestes souris de l'éditeur : déplacer, redimensionner, pivoter, sélection rectangle. */
export function useDrag(o: DragOptions) {
  const guides = ref<Guide[]>([]);
  const marquee = ref<Box | null>(null);

  function begin(e: PointerEvent, mode: DragMode): void {
    const el = o.stage.value;
    if (!el || e.button !== 0) return;
    e.preventDefault();
    const rect = el.getBoundingClientRect();
    const l = o.layout.value;
    const scale = rect.width / l.width;
    const [px0, py0] = clientToDesign(e.clientX, e.clientY, rect, l.width);
    const starts: Start[] = o.selection.widgets.value.map((w) => ({
      id: w.id,
      box: { x: w.x, y: w.y, w: w.w, h: w.h },
      rotation: w.rotation,
    }));
    const startIds = new Set(starts.map((s) => s.id));
    const others: Box[] = l.widgets
      .filter((w) => !startIds.has(w.id))
      .map((w) => ({ x: w.x, y: w.y, w: w.w, h: w.h }));
    const additive = e.shiftKey;
    const base = [...o.selection.ids.value];
    let moved = false;

    const onMove = (ev: PointerEvent): void => {
      const [px, py] = clientToDesign(ev.clientX, ev.clientY, rect, l.width);
      const dx = px - px0;
      const dy = py - py0;
      if (!moved && Math.hypot(dx, dy) * scale < CLICK_PX) return;
      moved = true;
      const thr = SNAP_PX / scale;
      const snap = o.snapOn.value && !ev.altKey;
      const grid = snap ? o.grid.value : 0;
      guides.value = [];

      if (mode.kind === "move") {
        let mx = dx;
        let my = dy;
        if (snap) {
          const u = unionBox(starts.map((st) => st.box));
          if (!u) return;
          const prop: Box = { ...u, x: u.x + dx, y: u.y + dy };
          const r = snapMove(prop, others, { w: l.width, h: l.height }, thr, grid);
          mx += r.dx;
          my += r.dy;
          guides.value = r.guides;
        }
        for (const s of starts) {
          const w = l.widgets.find((x) => x.id === s.id);
          if (!w) continue;
          w.x = round1(s.box.x + mx);
          w.y = round1(s.box.y + my);
        }
      } else if (mode.kind === "resize") {
        const s = starts[0];
        const w = s && l.widgets.find((x) => x.id === s.id);
        if (!s || !w) return;
        let rdx = dx;
        let rdy = dy;
        if (snap && s.rotation === 0) {
          const [sx, sy] = HANDLE_DIR[mode.handle];
          if (sx !== 0) {
            const edge = (sx > 0 ? s.box.x + s.box.w : s.box.x) + dx;
            const r = snapEdge(edge, "x", others, { w: l.width, h: l.height }, thr, grid);
            rdx += r.delta;
            if (r.guide) guides.value.push(r.guide);
          }
          if (sy !== 0) {
            const edge = (sy > 0 ? s.box.y + s.box.h : s.box.y) + dy;
            const r = snapEdge(edge, "y", others, { w: l.width, h: l.height }, thr, grid);
            rdy += r.delta;
            if (r.guide) guides.value.push(r.guide);
          }
        }
        const b = resizeBox(s.box, s.rotation, mode.handle, rdx, rdy, ev.shiftKey);
        w.x = round1(b.x);
        w.y = round1(b.y);
        w.w = round1(b.w);
        w.h = round1(b.h);
      } else if (mode.kind === "rotate") {
        const s = starts[0];
        const w = s && l.widgets.find((x) => x.id === s.id);
        if (!s || !w) return;
        const [cx, cy] = boxCenter(s.box);
        const a0 = angleFromCenter(cx, cy, px0, py0);
        let rot = s.rotation + (angleFromCenter(cx, cy, px, py) - a0);
        if (ev.shiftKey) rot = snapAngle(rot, 15);
        w.rotation = round1(normAngle(rot));
      } else {
        const m = normBox(px0, py0, px, py);
        marquee.value = m;
        const hit = l.widgets.filter((w) => intersects(m, { x: w.x, y: w.y, w: w.w, h: w.h })).map((w) => w.id);
        o.selection.set(additive ? [...new Set([...base, ...hit])] : hit);
      }
    };

    const onUp = (): void => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      guides.value = [];
      marquee.value = null;
      if (!moved && mode.kind === "marquee" && !additive) o.selection.clear();
      if (moved && mode.kind !== "marquee") o.onCommit();
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  return { guides, marquee, begin };
}
