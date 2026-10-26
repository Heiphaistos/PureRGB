<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { lookupProduct, openProductSource, photosEnabled, productPhoto, type ProductView } from "../productPhotos";

const props = defineProps<{
  vendor: string;
  name: string;
  /** `vid:pid` ou id d'appareil (`hid:vid:pid:…`, `vid:pid:série`). */
  usb?: string;
  /** Hauteur en px de la zone photo. */
  size?: number;
  /** Légende « Photo : fabricant » avec lien vers la page source. */
  caption?: boolean;
  /** Photo pleine largeur du conteneur (bandeau), au lieu d'une vignette. */
  fill?: boolean;
}>();
const emit = defineEmits<{ found: [product: ProductView | null] }>();

const product = ref<ProductView | null>(null);
const src = ref<string | null>(null);
const loading = ref(false);
let seq = 0;

async function load(): Promise<void> {
  const my = ++seq;
  product.value = null;
  src.value = null;
  emit("found", null);
  if (!photosEnabled.value) return;
  const p = await lookupProduct(props.vendor, props.name, props.usb ?? null);
  if (my !== seq) return;
  product.value = p;
  emit("found", p);
  if (!p?.has_image) return;
  loading.value = true;
  const img = await productPhoto(p.id);
  if (my !== seq) return;
  src.value = img;
  loading.value = false;
}

onMounted(load);
watch(() => [props.vendor, props.name, props.usb, photosEnabled.value], load);
</script>

<template>
  <span v-if="loading || src" class="pp" :class="{ fill }">
    <span class="pp-box" :style="{ height: `${size ?? 48}px` }">
      <span v-if="loading" class="pp-skel" :style="{ width: `${(size ?? 48) * 1.4}px` }"></span>
      <img v-else-if="src" :src="src" :alt="product?.name ?? name" :title="product?.name" class="pp-img" draggable="false" />
    </span>
    <a
      v-if="caption && src && product?.source_page"
      class="pp-cap"
      href="#"
      @click.prevent.stop="openProductSource(product.id)"
    >Photo : {{ product.vendor }}</a>
  </span>
  <slot v-else />
</template>

<style scoped>
.pp {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  max-width: 100%;
}
.pp.fill {
  display: flex;
  width: 100%;
}
.pp.fill .pp-box {
  width: 100%;
  justify-content: center;
  background: #fff; /* les photos fabricant ont un fond blanc : bandeau continu, sans bords noirs */
  border-radius: var(--radius, 8px);
}
.pp-box {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
}
.pp-img {
  max-height: 100%;
  max-width: 100%;
  object-fit: contain;
  border-radius: var(--radius-sm, 4px);
}
.pp-skel {
  display: block;
  height: 100%;
  border-radius: var(--radius-sm, 4px);
  background: linear-gradient(90deg, var(--bg-card, #222) 25%, var(--border, #333) 50%, var(--bg-card, #222) 75%);
  background-size: 200% 100%;
  animation: pp-shimmer 1.2s linear infinite;
}
.pp-cap {
  font-size: 10px;
  color: var(--text-dim, #888);
  text-decoration: none;
}
.pp-cap:hover {
  text-decoration: underline;
}
@keyframes pp-shimmer {
  to {
    background-position: -200% 0;
  }
}
</style>
