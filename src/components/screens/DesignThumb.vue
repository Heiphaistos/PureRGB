<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { designThumb } from "../../screens/thumbs.ts";
import type { DesignSummary } from "../../screens/types.ts";
import "./screens.css";

const props = defineProps<{ design: DesignSummary }>();
const el = ref<HTMLElement | null>(null);
const src = ref("");
const failed = ref(false);
let io: IntersectionObserver | undefined;

async function load(): Promise<void> {
  try {
    src.value = await designThumb(props.design.id, props.design.width, props.design.height);
  } catch {
    // Vignette indisponible : on garde le damier, le design reste utilisable.
    failed.value = true;
  }
}

onMounted(() => {
  if (!el.value) return;
  io = new IntersectionObserver((entries) => {
    if (entries.some((e) => e.isIntersecting)) {
      io?.disconnect();
      void load();
    }
  });
  io.observe(el.value);
});
onBeforeUnmount(() => io?.disconnect());
</script>

<template>
  <div ref="el" class="t" :style="{ aspectRatio: `${design.width} / ${design.height}` }">
    <img v-if="src" :src="src" :alt="design.name" class="sc-thumb" draggable="false" />
    <div v-else class="sc-ph ph">{{ failed ? "Aperçu indisponible" : "" }}</div>
  </div>
</template>

<style scoped>
.t {
  width: 100%;
  max-height: 200px;
  display: flex;
  justify-content: center;
}

.t img,
.ph {
  height: 100%;
  width: auto;
  max-width: 100%;
  aspect-ratio: inherit;
}

.ph {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: var(--text-dim);
}
</style>
