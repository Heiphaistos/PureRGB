import { computed, ref, type Ref } from "vue";
import { mergeOrder, moveBefore, moveBy } from "./cardOrder";

/** Ordre de cartes réorganisable (glisser-déposer HTML5 + clavier), mémorisé dans localStorage. */
export function useCardOrder(storageKey: string, ids: Ref<string[]>) {
  const saved = ref<string[]>(load());
  const dragId = ref<string | null>(null);
  const overId = ref<string | null>(null);
  const order = computed(() => mergeOrder(saved.value, ids.value));

  function load(): string[] {
    try {
      const v = JSON.parse(localStorage.getItem(storageKey) ?? "[]");
      return Array.isArray(v) ? v.filter((x) => typeof x === "string") : [];
    } catch {
      return [];
    }
  }
  function commit(next: string[]) {
    saved.value = next;
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
    } catch {
      /* stockage indisponible : l'ordre vaut pour la session */
    }
  }
  function onDragStart(id: string, e: DragEvent) {
    dragId.value = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id);
    }
  }
  function onDragOver(id: string, e: DragEvent) {
    if (!dragId.value) return;
    e.preventDefault();
    overId.value = id;
  }
  function onDrop(id: string, e: DragEvent) {
    e.preventDefault();
    if (dragId.value && dragId.value !== id) commit(moveBefore(order.value, dragId.value, id));
    onDragEnd();
  }
  function onDragEnd() {
    dragId.value = null;
    overId.value = null;
  }
  function move(id: string, delta: number) {
    commit(moveBy(order.value, id, delta));
  }
  function reset() {
    commit([]);
  }
  return { order, dragId, overId, onDragStart, onDragOver, onDrop, onDragEnd, move, reset, customized: computed(() => saved.value.length > 0) };
}
