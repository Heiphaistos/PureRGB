<script setup lang="ts">
import { DEVICE_ICONS } from "../assets/icons";

defineProps<{ type: string }>();
</script>

<template>
  <component :is="DEVICE_ICONS[type] ?? DEVICE_ICONS.unknown" class="device-icon" />
</template>

<style scoped>
.device-icon {
  width: 100%;
  height: 100%;
  color: inherit;
}
</style>
