<script setup lang="ts">
import type { TextStyle } from "../../screens/types.ts";
import { FONTS } from "../../screens/widgets.ts";
import ColorField from "./ColorField.vue";
import NumField from "./NumField.vue";
import TextField from "./TextField.vue";
import "./screens.css";

defineProps<{ textStyle: TextStyle; fonts: string[] }>();
const emit = defineEmits<{ commit: [] }>();

const ALIGNS = [
  { v: "left", l: "Gauche" },
  { v: "center", l: "Centre" },
  { v: "right", l: "Droite" },
] as const;

</script>

<template>
  <TextField v-model="textStyle.font" label="Police" :list="[...FONTS, ...fonts]" @commit="emit('commit')" />
  <NumField v-model="textStyle.size" label="Taille" :min="4" :max="1000" unit="px" @commit="emit('commit')" />
  <ColorField label="Couleur" v-model="textStyle.color" @commit="emit('commit')" />
  <div class="sc-field">
    <span>Alignement</span>
    <span class="sc-row">
      <button
        v-for="a in ALIGNS"
        :key="a.v"
        class="sm"
        :class="{ on: textStyle.align === a.v }"
        @click="(textStyle.align = a.v), emit('commit')"
      >
        {{ a.l }}
      </button>
    </span>
  </div>
  <label class="sc-field">
    <span>Gras</span>
    <input v-model="textStyle.bold" type="checkbox" @change="emit('commit')" />
  </label>
</template>
