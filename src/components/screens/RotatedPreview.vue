<script setup lang="ts">
import { computed } from "vue";
import "./screens.css";

/** Image du design telle qu'elle apparaît sur l'écran physique `width`×`height` une fois pivotée de `rotation`° (sens horaire). */
const props = defineProps<{ src: string; rotation: number; width: number; height: number }>();

const turned = computed(() => props.rotation === 90 || props.rotation === 270);
const imgStyle = computed(() => ({
  width: turned.value ? "100cqh" : "100cqw",
  height: turned.value ? "100cqw" : "100cqh",
  transform: `translate(-50%, -50%) rotate(${props.rotation}deg)`,
}));
</script>

<template>
  <div class="rp" :style="{ aspectRatio: `${width} / ${height}` }">
    <img :src="src" alt="Aperçu sur l'écran" :style="imgStyle" draggable="false" />
  </div>
</template>

<style scoped>
.rp {
  position: relative;
  width: 100%;
  container-type: size;
  overflow: hidden;
  background: #000;
}

img {
  position: absolute;
  left: 50%;
  top: 50%;
  object-fit: fill;
  max-width: none;
  border-radius: 0;
}
</style>
