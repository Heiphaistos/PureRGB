<script setup lang="ts">
import "./screens.css";

defineProps<{ label: string; modelValue: string; list?: string[]; placeholder?: string; title?: string }>();
const emit = defineEmits<{ "update:modelValue": [v: string]; commit: [] }>();
const listId = `sc-dl-${Math.random().toString(36).slice(2, 8)}`;
</script>

<template>
  <label class="sc-field">
    <span>{{ label }}</span>
    <span>
      <input
        type="text"
        :value="modelValue"
        :list="list ? listId : undefined"
        :placeholder="placeholder"
        :title="title"
        spellcheck="false"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        @change="emit('commit')"
      />
      <datalist v-if="list" :id="listId"><option v-for="o in list" :key="o" :value="o" /></datalist>
    </span>
  </label>
</template>
