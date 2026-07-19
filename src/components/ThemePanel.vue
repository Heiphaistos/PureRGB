<script setup lang="ts">
import { ref } from "vue";
import { THEMES, type Theme } from "../themes";
import type { EffectConfig } from "../types";
import { colorToHex } from "../types";

const emit = defineEmits<{
  applyAll: [config: EffectConfig];
}>();

const lastApplied = ref<string | null>(null);

function apply(t: Theme) {
  lastApplied.value = t.id;
  // Copie profonde : le thème est un modèle, jamais muté par l'appelant.
  emit("applyAll", JSON.parse(JSON.stringify(t.config)));
}

function preview(t: Theme): string {
  const cols = t.config.colors;
  if (cols.length === 0) {
    return "linear-gradient(90deg, red, orange, yellow, lime, cyan, blue, magenta)";
  }
  if (cols.length === 1) return colorToHex(cols[0]);
  return `linear-gradient(90deg, ${cols.map(colorToHex).join(", ")})`;
}
</script>

<template>
  <section class="theme-panel">
    <h2>Thèmes prédéfinis</h2>
    <p class="sub">
      Un clic applique le thème à <strong>tous les appareils détectés</strong>
      (PC + maison connectée). Ajustez ensuite appareil par appareil dans
      l'onglet Éclairage.
    </p>
    <div class="grid">
      <button
        v-for="t in THEMES"
        :key="t.id"
        class="theme-card"
        :class="{ active: lastApplied === t.id }"
        @click="apply(t)"
      >
        <span class="swatch" :style="{ background: preview(t) }"></span>
        <span class="emoji">{{ t.emoji }}</span>
        <span class="name">{{ t.name }}</span>
        <span class="desc">{{ t.description }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.theme-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.theme-panel h2 {
  font-size: 19px;
}

.sub {
  color: var(--text-dim);
  font-size: 13px;
  margin-top: 4px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 12px;
  margin-top: 20px;
}

.theme-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  padding: 14px;
  text-align: left;
}

.theme-card.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.swatch {
  width: 100%;
  height: 10px;
  border-radius: 999px;
}

.emoji {
  font-size: 22px;
  margin-top: 4px;
}

.name {
  font-weight: 600;
  font-size: 14px;
}

.desc {
  color: var(--text-dim);
  font-size: 12px;
}
</style>
