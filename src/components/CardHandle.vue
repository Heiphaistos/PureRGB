<script setup lang="ts">
// Poignée de réorganisation d'une carte : glisser (souris) ou flèches haut/bas (clavier).
defineProps<{ label: string }>();
const emit = defineEmits<{ move: [delta: number]; dragstart: [e: DragEvent]; dragend: [] }>();
</script>

<template>
  <span
    class="handle"
    role="button"
    tabindex="0"
    draggable="true"
    :title="`Glisser pour déplacer « ${label} » (ou flèches haut/bas au clavier)`"
    :aria-label="`Déplacer la carte ${label}`"
    @dragstart="emit('dragstart', $event)"
    @dragend="emit('dragend')"
    @keydown.up.prevent="emit('move', -1)"
    @keydown.left.prevent="emit('move', -1)"
    @keydown.down.prevent="emit('move', 1)"
    @keydown.right.prevent="emit('move', 1)"
  >⋮⋮</span>
</template>

<style scoped>
.handle { cursor: grab; user-select: none; color: var(--text-dim); letter-spacing: -2px; padding: 0 6px; margin-right: 6px; border-radius: 4px; font-size: 14px; line-height: 1; }
.handle:hover, .handle:focus-visible { color: var(--accent); background: var(--accent-soft); outline: none; }
.handle:active { cursor: grabbing; }
</style>
