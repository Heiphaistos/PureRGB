import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { previewSize, preview as renderPreview } from "./api.ts";
import type { Layout } from "./types.ts";

const t0 = Date.now();

/**
 * Aperçu rendu par le moteur : se met à jour quand le layout change (regroupé, une
 * seule requête en vol) et toutes les `liveMs` ms (valeurs live). `liveMs` = 0 : pas de rafraîchissement périodique.
 */
export function usePreview(layout: Ref<Layout | null>, maxW: Ref<number> | number, liveMs = 1000) {
  const src = ref("");
  const error = ref("");
  let busy = false;
  let dirty = false;
  let alive = true;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function render(): Promise<void> {
    const l = layout.value;
    if (!l) {
      src.value = "";
      return;
    }
    if (busy) {
      dirty = true;
      return;
    }
    busy = true;
    try {
      const { width, height } = previewSize(l.width, l.height, typeof maxW === "number" ? maxW : maxW.value);
      const png = await renderPreview(l, width, height, Date.now() - t0);
      if (alive) {
        src.value = png ? `data:image/png;base64,${png}` : "";
        error.value = "";
      }
    } catch (e) {
      if (alive) error.value = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      if (dirty && alive) {
        dirty = false;
        void render();
      }
    }
  }

  function schedule(): void {
    clearTimeout(timer);
    timer = setTimeout(() => void render(), 60);
  }

  watch(layout, schedule, { deep: true, immediate: true });
  const live = liveMs > 0 ? setInterval(() => void render(), liveMs) : undefined;
  onBeforeUnmount(() => {
    alive = false;
    clearTimeout(timer);
    clearInterval(live);
  });

  return { src, error, refresh: render };
}
