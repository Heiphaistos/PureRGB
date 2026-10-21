<script setup lang="ts">
import { onMounted, ref } from "vue";
import { mediaThumb } from "../../screens/thumbs.ts";
import type { MediaItem } from "../../screens/types.ts";
import "./screens.css";

const props = defineProps<{ item: MediaItem }>();
const src = ref("");

onMounted(async () => {
  if (props.item.kind === "font") return;
  try {
    src.value = await mediaThumb(props.item.id, props.item.width, props.item.height);
  } catch {
    // Pas de miniature : le damier reste affiché.
  }
});
</script>

<template>
  <div class="m">
    <img v-if="src" :src="src" :alt="item.name" class="sc-thumb" draggable="false" />
    <div v-else class="sc-ph ph">{{ item.kind === "font" ? "Aa" : "" }}</div>
  </div>
</template>

<style scoped>
.m {
  aspect-ratio: 16 / 10;
  display: flex;
}

.m img,
.ph {
  width: 100%;
  height: 100%;
}

.ph {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: var(--text-dim);
}
</style>
