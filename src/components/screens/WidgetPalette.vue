<script setup lang="ts">
import type { WidgetType } from "../../screens/types.ts";
import { WIDGET_ICONS, WIDGET_LABELS, WIDGET_TYPES } from "../../screens/widgets.ts";
import Icon from "./Icon.vue";
import "./screens.css";

defineProps<{ full: boolean }>();
const emit = defineEmits<{ add: [type: WidgetType] }>();
</script>

<template>
  <nav class="pal" aria-label="Palette de widgets">
    <h4>Widgets</h4>
    <button v-for="t in WIDGET_TYPES" :key="t" :disabled="full" :title="`Ajouter : ${WIDGET_LABELS[t]}`" @click="emit('add', t)">
      <Icon :paths="WIDGET_ICONS[t]" />
      <span>{{ WIDGET_LABELS[t] }}</span>
    </button>
    <p v-if="full" class="sc-err">Limite de widgets atteinte.</p>
  </nav>
</template>

<style scoped>
.pal {
  width: 152px;
  flex: none;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 10px;
  border-right: 1px solid var(--border);
  background: var(--bg-panel);
  overflow-y: auto;
}

h4 {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
  margin: 2px 4px 6px;
}

button {
  display: flex;
  align-items: center;
  gap: 8px;
  text-align: left;
  padding: 8px 10px;
  font-size: 12px;
}

svg {
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--accent);
}
</style>
