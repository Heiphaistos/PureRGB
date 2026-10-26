<script setup lang="ts">
import { computed, ref } from "vue";
import { padIdentity } from "../gamepad";
import type { ProductView } from "../productPhotos";
import ProductPhoto from "./ProductPhoto.vue";

const props = defineProps<{ padId: string; size?: number }>();
const ident = computed(() => padIdentity(props.padId));
const product = ref<ProductView | null>(null);
/** Libellé propre : nom du catalogue si reconnue, sinon le nom du navigateur. */
// XInput seul : le catalogue ne peut pas savoir si c'est une 360, One ou Series -> on garde le libellé de famille.
const title = computed(() => (!ident.value.usb && /XInput/.test(ident.value.label) ? ident.value.label : (product.value?.name ?? ident.value.label)));
</script>

<template>
  <div class="padp">
    <ProductPhoto :vendor="ident.vendor" :name="ident.name" :usb="ident.usb ?? undefined" :size="size ?? 110" caption @found="product = $event" />
    <div class="padp-t">
      <strong>{{ title }}</strong>
      <small class="padp-id" :title="padId">{{ padId }}</small>
    </div>
  </div>
</template>

<style scoped>
.padp { display: flex; align-items: center; gap: 16px; margin-bottom: 10px; flex-wrap: wrap; }
.padp-t { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.padp-t strong { font-size: 15px; }
.padp-id { color: var(--text-dim); font-size: 11px; word-break: break-all; }
</style>
