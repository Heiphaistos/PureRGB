import { computed, ref, watch, type Ref } from "vue";
import type { Layout, Widget } from "./types.ts";

/** Sélection de widgets (par id), élaguée automatiquement quand un widget disparaît. */
export function useSelection(layout: Ref<Layout>) {
  const ids = ref<string[]>([]);

  const widgets = computed<Widget[]>(() => layout.value.widgets.filter((w) => ids.value.includes(w.id)));
  const single = computed<Widget | null>(() => (widgets.value.length === 1 ? widgets.value[0] : null));

  const isSelected = (id: string): boolean => ids.value.includes(id);
  const set = (next: string[]): void => {
    ids.value = next;
  };
  const clear = (): void => set([]);
  const toggle = (id: string): void => set(isSelected(id) ? ids.value.filter((i) => i !== id) : [...ids.value, id]);
  const selectAll = (): void => set(layout.value.widgets.map((w) => w.id));

  watch(
    () => layout.value.widgets.map((w) => w.id).join(","),
    () => {
      const present = new Set(layout.value.widgets.map((w) => w.id));
      const kept = ids.value.filter((i) => present.has(i));
      if (kept.length !== ids.value.length) ids.value = kept;
    },
  );

  return { ids, widgets, single, isSelected, set, clear, toggle, selectAll };
}
