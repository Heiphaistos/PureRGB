<script setup lang="ts">
import { rampCss } from "../../screens/format.ts";
import type { ColorRamp } from "../../screens/types.ts";
import ColorField from "./ColorField.vue";
import "./screens.css";

const props = defineProps<{ ramp: ColorRamp }>();
const emit = defineEmits<{ commit: [] }>();

function sort(): void {
  props.ramp.sort((a, b) => a[0] - b[0]);
  emit("commit");
}

function add(): void {
  const last = props.ramp[props.ramp.length - 1];
  props.ramp.push([Math.min(1, (last?.[0] ?? 0) + 0.1), last?.[1] ?? "#ffffff"]);
  sort();
}

function remove(i: number): void {
  if (props.ramp.length <= 1) return;
  props.ramp.splice(i, 1);
  emit("commit");
}

function setPos(i: number, e: Event): void {
  const v = (e.target as HTMLInputElement).valueAsNumber;
  if (Number.isFinite(v)) props.ramp[i][0] = Math.min(1, Math.max(0, v));
}

</script>

<template>
  <div class="ramp">
    <div class="bar" :style="{ background: rampCss(ramp) }" title="Couleur selon la valeur : 0 = minimum, 1 = maximum"></div>
    <div v-for="(stop, i) in ramp" :key="i" class="stop">
      <input type="number" min="0" max="1" step="0.05" :value="stop[0]" aria-label="Position (0 à 1)" @input="setPos(i, $event)" @change="sort" />
      <ColorField bare label="Couleur du palier" v-model="stop[1]" @commit="emit('commit')" />
      <button class="icon danger" :disabled="ramp.length <= 1" title="Retirer ce palier" @click="remove(i)">×</button>
    </div>
    <button class="sm" @click="add">+ Palier</button>
  </div>
</template>

<style scoped>
.ramp {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}

.bar {
  height: 14px;
  border-radius: 7px;
  border: 1px solid var(--border);
}

.stop {
  display: grid;
  grid-template-columns: 58px 1fr 30px;
  gap: 6px;
  align-items: center;
}

.stop input[type="number"] {
  padding: 5px 6px;
  font-size: 12px;
}

.ramp > button {
  align-self: flex-start;
}
</style>
