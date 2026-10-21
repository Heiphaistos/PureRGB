import { computed, ref } from "vue";
import { History } from "./history.ts";
import type { Layout } from "./types.ts";
import { cloneLayout } from "./widgets.ts";

/** Layout édité + annuler/refaire. `commit()` après chaque modification terminée. */
export function useHistory(initial: Layout) {
  const layout = ref<Layout>(cloneLayout(initial));
  const hist = new History<Layout>(layout.value);
  const tick = ref(0);
  const dirty = ref(false);

  const canUndo = computed(() => (tick.value, hist.canUndo));
  const canRedo = computed(() => (tick.value, hist.canRedo));

  function commit(key: string | null = null): void {
    if (hist.push(layout.value, key)) dirty.value = true;
    tick.value++;
  }

  function undo(): void {
    const s = hist.undo();
    if (s) {
      layout.value = s;
      dirty.value = true;
      tick.value++;
    }
  }

  function redo(): void {
    const s = hist.redo();
    if (s) {
      layout.value = s;
      dirty.value = true;
      tick.value++;
    }
  }

  /** Charge un nouveau design (historique vidé). */
  function reset(l: Layout): void {
    layout.value = cloneLayout(l);
    hist.reset(layout.value);
    dirty.value = false;
    tick.value++;
  }

  return { layout, canUndo, canRedo, dirty, commit, undo, redo, reset };
}
