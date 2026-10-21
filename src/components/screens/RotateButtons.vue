<script setup lang="ts">
import { computed } from "vue";
import "./screens.css";

const props = defineProps<{ rotation: number }>();
const emit = defineEmits<{ set: [deg: number] }>();

const next = computed(() => (props.rotation + 90) % 360);
const flipped = computed(() => (props.rotation + 180) % 360);
</script>

<template>
  <span class="rot" role="group" aria-label="Rotation de l'écran">
    <button class="sm turn" title="Pivoter l'écran (90°)" aria-label="Pivoter l'écran de 90°" @click="emit('set', next)">
      <span aria-hidden="true">↻</span> {{ rotation }}°
    </button>
    <button class="sm" title="Retourner l'écran d'un coup (180°)" aria-label="Retourner l'écran de 180°" @click="emit('set', flipped)">180°</button>
  </span>
</template>

<style scoped>
.rot {
  display: inline-flex;
  gap: 4px;
  align-items: center;
}

.turn {
  min-width: 62px;
  font-variant-numeric: tabular-nums;
}
</style>
