<script setup lang="ts">
import { computed, ref } from "vue";
import { THEME_CATEGORY_LABELS, THEMES, type Theme, type ThemeCategory } from "../themes";
import type { EffectConfig } from "../types";
import { colorToHex } from "../types";

const emit = defineEmits<{
  applyAll: [config: EffectConfig];
}>();

const lastApplied = ref<string | null>(null);
const activeCategory = ref<ThemeCategory | "all">("all");

const categories = computed(() => {
  const counts = new Map<ThemeCategory, number>();
  for (const t of THEMES) counts.set(t.category, (counts.get(t.category) ?? 0) + 1);
  return (Object.keys(THEME_CATEGORY_LABELS) as ThemeCategory[])
    .filter((cat) => counts.has(cat))
    .map((cat) => ({ id: cat, label: THEME_CATEGORY_LABELS[cat], count: counts.get(cat)! }));
});

const filtered = computed(() =>
  activeCategory.value === "all" ? THEMES : THEMES.filter((t) => t.category === activeCategory.value),
);

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
    <div class="category-filter">
      <button
        type="button"
        class="cat-chip"
        :class="{ active: activeCategory === 'all' }"
        @click="activeCategory = 'all'"
      >
        Tous<span class="count">{{ THEMES.length }}</span>
      </button>
      <button
        v-for="cat in categories"
        :key="cat.id"
        type="button"
        class="cat-chip"
        :class="{ active: activeCategory === cat.id }"
        @click="activeCategory = cat.id"
      >
        {{ cat.label }}<span class="count">{{ cat.count }}</span>
      </button>
    </div>

    <div class="grid">
      <button
        v-for="t in filtered"
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

.category-filter {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-4);
}

.cat-chip {
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.cat-chip.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.cat-chip .count {
  font-size: 10px;
  color: inherit;
  opacity: 0.7;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 12px;
  margin-top: 16px;
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
