<script setup lang="ts">
import { computed } from "vue";
import { formatValue } from "../../screens/format.ts";
import type { LiveValues } from "../../screens/types.ts";
import { SOURCES } from "../../screens/widgets.ts";
import "./screens.css";

const props = defineProps<{ modelValue: string; values: LiveValues }>();
const emit = defineEmits<{ "update:modelValue": [v: string]; commit: [] }>();

const groups = computed(() => {
  const m = new Map<string, { key: string; label: string }[]>();
  const known = new Set(SOURCES.map((s) => s.key));
  for (const s of SOURCES) m.set(s.group, [...(m.get(s.group) ?? []), { key: s.key, label: `${s.label}${s.unit ? ` (${s.unit})` : ""}` }]);
  const extra = Object.keys(props.values).filter((k) => !known.has(k)).sort();
  if (extra.length) m.set("Autres capteurs", extra.map((k) => ({ key: k, label: k })));
  return [...m.entries()];
});

const allKeys = computed(() => new Set(groups.value.flatMap(([, items]) => items.map((i) => i.key))));
const custom = computed(() => !allKeys.value.has(props.modelValue));
const live = computed(() => {
  const v = props.values[props.modelValue];
  return v === undefined ? "" : formatValue("{:.1}", v);
});

function onSelect(e: Event): void {
  const v = (e.target as HTMLSelectElement).value;
  emit("update:modelValue", v === "__custom" ? "sensor:" : v);
  emit("commit");
}
</script>

<template>
  <div class="src">
    <label class="sc-field">
      <span>Source</span>
      <select :value="custom ? '__custom' : modelValue" @change="onSelect">
        <optgroup v-for="[g, items] in groups" :key="g" :label="g">
          <option v-for="i in items" :key="i.key" :value="i.key">{{ i.label }}</option>
        </optgroup>
        <option value="__custom">Autre clé…</option>
      </select>
    </label>
    <label v-if="custom" class="sc-field">
      <span>Clé</span>
      <input
        type="text"
        :value="modelValue"
        placeholder="sensor:<id> ou ssd.0.temp"
        spellcheck="false"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        @change="emit('commit')"
      />
    </label>
    <div class="live">
      <code>{{ modelValue }}</code>
      <span v-if="live" class="v">valeur actuelle : {{ live }}</span>
    </div>
  </div>
</template>

<style scoped>
.live {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 11px;
  color: var(--text-dim);
  margin: -2px 0 8px 104px;
}

.v {
  color: var(--ok);
}
</style>
