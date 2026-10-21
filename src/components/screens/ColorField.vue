<script setup lang="ts" generic="T extends string | null">
import { parseHex, toColorInput } from "../../screens/format.ts";
import "./screens.css";

/** Couleur #rgb/#rrggbb/#rrggbbaa ; `nullable` ajoute une case « aucune ». */
const props = defineProps<{ label: string; modelValue: T; nullable?: boolean; bare?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [v: T]; commit: [] }>();

function onPicker(e: Event): void {
  const hex = (e.target as HTMLInputElement).value;
  // Conserve l'alpha existant (#rrggbbaa) quand on change seulement la teinte.
  const p = props.modelValue ? parseHex(props.modelValue) : null;
  const a = p && p[3] < 1 ? Math.round(p[3] * 255).toString(16).padStart(2, "0") : "";
  emit("update:modelValue", (hex + a) as T);
}

function onText(e: Event): void {
  const v = (e.target as HTMLInputElement).value.trim();
  if (parseHex(v)) emit("update:modelValue", v as T);
}

function toggle(e: Event): void {
  emit("update:modelValue", ((e.target as HTMLInputElement).checked ? "#ffffff" : null) as T);
  emit("commit");
}
</script>

<template>
  <component :is="bare ? 'div' : 'label'" :class="bare ? 'sc-row' : 'sc-field'">
    <span v-if="!bare">{{ label }}</span>
    <span class="sc-row">
      <input v-if="nullable" type="checkbox" :checked="modelValue !== null" :title="`Activer : ${label}`" @change="toggle" />
      <template v-if="modelValue !== null">
        <input class="pick" type="color" :value="toColorInput(modelValue)" :aria-label="label" @input="onPicker" @change="emit('commit')" />
        <input type="text" class="hex" :value="modelValue" spellcheck="false" @input="onText" @change="emit('commit')" />
      </template>
      <span v-else class="none">aucune</span>
    </span>
  </component>
</template>

<style scoped>
input.pick {
  width: 30px;
  height: 28px;
  flex: none;
  padding: 1px;
  border-radius: 6px;
}

.hex {
  min-width: 0;
}

.none {
  color: var(--text-dim);
  font-size: 12px;
}
</style>
