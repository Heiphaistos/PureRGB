<script setup lang="ts">
import "./screens.css";

defineProps<{ label: string; modelValue: number; min?: number; max?: number; step?: number; unit?: string }>();
const emit = defineEmits<{ "update:modelValue": [v: number]; commit: [] }>();

function onInput(e: Event): void {
  const v = (e.target as HTMLInputElement).valueAsNumber;
  if (Number.isFinite(v)) emit("update:modelValue", v);
}
</script>

<template>
  <label class="sc-field">
    <span>{{ label }}</span>
    <span class="sc-row">
      <input
        type="number"
        :value="Math.round(modelValue * 100) / 100"
        :min="min"
        :max="max"
        :step="step ?? 1"
        @input="onInput"
        @change="emit('commit')"
      />
      <span v-if="unit" class="u">{{ unit }}</span>
    </span>
  </label>
</template>

<style scoped>
.u {
  color: var(--text-dim);
  font-size: 11px;
}
</style>
