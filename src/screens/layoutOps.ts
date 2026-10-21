// Opérations pures sur un layout (mutent le layout reçu) : ordre Z, copie, alignement.
import type { Box } from "./geometry.ts";
import { unionBox } from "./geometry.ts";
import type { Layout, Widget } from "./types.ts";
import { MAX_WIDGETS } from "./types.ts";
import { newId } from "./widgets.ts";

export type ZMove = "front" | "back" | "forward" | "backward";
export type AlignKind = "left" | "hcenter" | "right" | "top" | "vcenter" | "bottom";

export function widgetBox(w: Widget): Box {
  return { x: w.x, y: w.y, w: w.w, h: w.h };
}

export function addWidget(layout: Layout, w: Widget): boolean {
  if (layout.widgets.length >= MAX_WIDGETS) return false;
  layout.widgets.push(w);
  return true;
}

export function removeWidgets(layout: Layout, ids: ReadonlySet<string>): void {
  layout.widgets = layout.widgets.filter((w) => !ids.has(w.id));
}

/** Duplique les widgets (décalés de `offset`), renvoie les nouveaux ids. */
export function duplicateWidgets(layout: Layout, ids: ReadonlySet<string>, offset = 12): string[] {
  const out: string[] = [];
  for (const w of [...layout.widgets]) {
    if (!ids.has(w.id) || layout.widgets.length >= MAX_WIDGETS) continue;
    const copy = JSON.parse(JSON.stringify(w)) as Widget;
    copy.id = newId();
    copy.x += offset;
    copy.y += offset;
    layout.widgets.push(copy);
    out.push(copy.id);
  }
  return out;
}

/** Ordre Z : le dernier de la liste est dessiné au-dessus. */
export function reorder(layout: Layout, ids: ReadonlySet<string>, move: ZMove): void {
  const sel = layout.widgets.filter((w) => ids.has(w.id));
  const rest = layout.widgets.filter((w) => !ids.has(w.id));
  if (move === "front") layout.widgets = [...rest, ...sel];
  else if (move === "back") layout.widgets = [...sel, ...rest];
  else {
    const arr = [...layout.widgets];
    const order = move === "forward" ? [...arr.keys()].reverse() : [...arr.keys()];
    const d = move === "forward" ? 1 : -1;
    for (const i of order) {
      const j = i + d;
      if (ids.has(arr[i].id) && j >= 0 && j < arr.length && !ids.has(arr[j].id)) {
        [arr[i], arr[j]] = [arr[j], arr[i]];
      }
    }
    layout.widgets = arr;
  }
}

/** Aligne sur la boîte englobante de la sélection (≥2) ou sur le canevas (1 widget). */
export function align(layout: Layout, ids: ReadonlySet<string>, kind: AlignKind): void {
  const sel = layout.widgets.filter((w) => ids.has(w.id));
  if (sel.length === 0) return;
  const ref: Box =
    sel.length === 1 ? { x: 0, y: 0, w: layout.width, h: layout.height } : (unionBox(sel.map(widgetBox)) as Box);
  for (const w of sel) {
    if (kind === "left") w.x = ref.x;
    else if (kind === "hcenter") w.x = ref.x + (ref.w - w.w) / 2;
    else if (kind === "right") w.x = ref.x + ref.w - w.w;
    else if (kind === "top") w.y = ref.y;
    else if (kind === "vcenter") w.y = ref.y + (ref.h - w.h) / 2;
    else w.y = ref.y + ref.h - w.h;
  }
}
