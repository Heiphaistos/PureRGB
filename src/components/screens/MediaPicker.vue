<script setup lang="ts">
import type { Fit, MediaItem } from "../../screens/types.ts";
import MediaThumb from "./MediaThumb.vue";
import "./screens.css";

defineProps<{ modelValue: string; fit: Fit; media: MediaItem[] }>();
const emit = defineEmits<{ "update:modelValue": [id: string]; "update:fit": [fit: Fit]; commit: [] }>();

const FITS: { v: Fit; l: string; t: string }[] = [
  { v: "cover", l: "Remplir", t: "Remplir l'écran en rognant ce qui dépasse" },
  { v: "contain", l: "Contenir", t: "Tout afficher, avec des bandes si le format diffère" },
  { v: "stretch", l: "Étirer", t: "Étirer aux dimensions exactes (déforme)" },
];

function pick(id: string): void {
  emit("update:modelValue", id);
  emit("commit");
}

function setFit(f: Fit): void {
  emit("update:fit", f);
  emit("commit");
}
</script>

<template>
  <div class="mp">
    <div class="tiles" role="listbox" aria-label="Média">
      <button class="tile none" :class="{ sel: !modelValue }" role="option" :aria-selected="!modelValue" @click="pick('')">Aucun</button>
      <button
        v-for="m in media"
        :key="m.id"
        class="tile"
        :class="{ sel: modelValue === m.id }"
        role="option"
        :aria-selected="modelValue === m.id"
        :title="m.name"
        @click="pick(m.id)"
      >
        <MediaThumb :item="m" />
        <span class="nm">{{ m.name }}</span>
      </button>
    </div>
    <p v-if="!media.length" class="help">Importez d'abord une image, un GIF ou une vidéo dans l'onglet Médias.</p>
    <div class="fits" role="group" aria-label="Ajustement">
      <button v-for="f in FITS" :key="f.v" class="sm" :class="{ on: fit === f.v }" :title="f.t" @click="setFit(f.v)">{{ f.l }}</button>
    </div>
  </div>
</template>

<style scoped>
.mp {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 8px;
}

.tiles {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
  max-height: 210px;
  overflow-y: auto;
}

.tile {
  padding: 3px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  border: 2px solid transparent;
  font-size: 10px;
}

.tile.sel {
  border-color: var(--accent);
}

.none {
  align-items: center;
  justify-content: center;
  aspect-ratio: 16 / 10;
}

.nm {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  color: var(--text-dim);
}

.fits {
  display: flex;
  gap: 4px;
}

.help {
  color: var(--text-dim);
  font-size: 11px;
  margin: 0;
}
</style>
