<script setup lang="ts">
import { computed, ref } from "vue";
import { formatValue } from "../../screens/format.ts";
import { dynamicSources } from "../../screens/sourceCatalog.ts";
import type { LiveValues } from "../../screens/types.ts";
import { SOURCES } from "../../screens/widgets.ts";
import RawSensorPicker from "./RawSensorPicker.vue";
import "./screens.css";

const props = defineProps<{ modelValue: string; values: LiveValues }>();
const emit = defineEmits<{ "update:modelValue": [v: string]; commit: [] }>();

const GROUP_ORDER = ["Processeur", "Carte graphique", "Mémoire", "Carte mère", "Disques", "Réseau", "Refroidissement", "Heure"];

const groups = computed(() => {
  const m = new Map<string, { key: string; label: string }[]>(GROUP_ORDER.map((g) => [g, []]));
  const seen = new Set<string>();
  for (const s of [...SOURCES, ...dynamicSources(props.values)]) {
    if (seen.has(s.key)) continue;
    seen.add(s.key);
    m.get(s.group)?.push({ key: s.key, label: `${s.label}${s.unit ? ` (${s.unit})` : ""}` });
  }
  return [...m.entries()].filter(([, items]) => items.length);
});

const allKeys = computed(() => new Set(groups.value.flatMap(([, items]) => items.map((i) => i.key))));
const rawOpen = ref(false);
const isRaw = computed(() => rawOpen.value || props.modelValue.startsWith("sensor:"));
const custom = computed(() => !isRaw.value && !allKeys.value.has(props.modelValue));
const selected = computed(() => (isRaw.value ? "__raw" : custom.value ? "__custom" : props.modelValue));
const live = computed(() => {
  const v = props.values[props.modelValue];
  return v === undefined ? "" : formatValue("{:.1}", v);
});

function onSelect(e: Event): void {
  const v = (e.target as HTMLSelectElement).value;
  rawOpen.value = v === "__raw";
  if (v === "__raw") return; // la clé change au choix d'un capteur
  emit("update:modelValue", v === "__custom" ? "" : v);
  emit("commit");
}

function pick(key: string): void {
  emit("update:modelValue", key);
  emit("commit");
}
</script>

<template>
  <div class="src">
    <label class="sc-field">
      <span>Source</span>
      <select :value="selected" @change="onSelect">
        <optgroup v-for="[g, items] in groups" :key="g" :label="g">
          <option v-for="i in items" :key="i.key" :value="i.key">{{ i.label }}</option>
        </optgroup>
        <option value="__raw">Capteur brut…</option>
        <option value="__custom">Autre clé…</option>
      </select>
    </label>
    <RawSensorPicker v-if="isRaw" :current="modelValue" @pick="pick" />
    <label v-if="custom" class="sc-field">
      <span>Clé</span>
      <input
        type="text"
        :value="modelValue"
        placeholder="ex. disk.D.used_pct"
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
