<script setup lang="ts">
import { computed } from "vue";
import type { LiveValues, MediaItem, Widget } from "../../screens/types.ts";
import { FORMAT_HELP } from "../../screens/widgets.ts";
import ColorField from "./ColorField.vue";
import ColorRampEditor from "./ColorRampEditor.vue";
import MediaPicker from "./MediaPicker.vue";
import NumField from "./NumField.vue";
import SourceField from "./SourceField.vue";
import TextField from "./TextField.vue";
import TextStyleFields from "./TextStyleFields.vue";
import "./screens.css";

const props = defineProps<{ widget: Widget; values: LiveValues; media: MediaItem[] }>();
const emit = defineEmits<{ commit: [] }>();

const commit = (): void => emit("commit");
const fontNames = computed(() => props.media.filter((m) => m.kind === "font").map((m) => m.name));
const images = computed(() => props.media.filter((m) => m.kind !== "font"));

</script>

<template>
  <template v-if="widget.type === 'text'">
    <TextField v-model="widget.text" label="Texte" @commit="commit" />
    <TextStyleFields :text-style="widget.style" :fonts="fontNames" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'value'">
    <SourceField v-model="widget.source" :values="values" @commit="commit" />
    <TextField v-model="widget.format" label="Format" :title="FORMAT_HELP" placeholder="{:.0}°C" @commit="commit" />
    <p class="help">{{ FORMAT_HELP }}</p>
    <TextStyleFields :text-style="widget.style" :fonts="fontNames" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'gauge_arc'">
    <SourceField v-model="widget.source" :values="values" @commit="commit" />
    <NumField v-model="widget.min" label="Minimum" @commit="commit" />
    <NumField v-model="widget.max" label="Maximum" @commit="commit" />
    <NumField v-model="widget.start_deg" label="Départ" unit="°" :min="-360" :max="360" @commit="commit" />
    <NumField v-model="widget.sweep_deg" label="Ouverture" unit="°" :min="10" :max="360" @commit="commit" />
    <NumField v-model="widget.thickness" label="Épaisseur" unit="px" :min="1" @commit="commit" />
    <ColorField label="Fond de jauge" v-model="widget.track" @commit="commit" />
    <div class="sc-field"><span>Rampe</span><span></span></div>
    <ColorRampEditor :ramp="widget.colors" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'bar'">
    <SourceField v-model="widget.source" :values="values" @commit="commit" />
    <NumField v-model="widget.min" label="Minimum" @commit="commit" />
    <NumField v-model="widget.max" label="Maximum" @commit="commit" />
    <NumField v-model="widget.radius" label="Arrondi" unit="px" :min="0" @commit="commit" />
    <label class="sc-field">
      <span>Verticale</span>
      <input v-model="widget.vertical" type="checkbox" @change="commit" />
    </label>
    <ColorField label="Fond de barre" v-model="widget.track" @commit="commit" />
    <div class="sc-field"><span>Rampe</span><span></span></div>
    <ColorRampEditor :ramp="widget.colors" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'graph'">
    <SourceField v-model="widget.source" :values="values" @commit="commit" />
    <NumField v-model="widget.min" label="Minimum" @commit="commit" />
    <NumField v-model="widget.max" label="Maximum" @commit="commit" />
    <NumField v-model="widget.seconds" label="Historique" unit="s" :min="5" :max="3600" @commit="commit" />
    <NumField v-model="widget.thickness" label="Épaisseur" unit="px" :min="1" @commit="commit" />
    <ColorField label="Ligne" v-model="widget.line" @commit="commit" />
    <ColorField v-model="widget.fill" label="Remplissage" nullable @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'rect'">
    <ColorField label="Remplissage" v-model="widget.fill" @commit="commit" />
    <ColorField v-model="widget.stroke" label="Contour" nullable @commit="commit" />
    <NumField v-model="widget.stroke_width" label="Épaisseur" unit="px" :min="0" @commit="commit" />
    <NumField v-model="widget.radius" label="Arrondi" unit="px" :min="0" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'image'">
    <MediaPicker v-model="widget.media_id" v-model:fit="widget.fit" :media="images" @commit="commit" />
  </template>

  <template v-else-if="widget.type === 'analog_clock'">
    <ColorField label="Cadran" v-model="widget.face" @commit="commit" />
    <ColorField label="Aiguilles" v-model="widget.hands" @commit="commit" />
    <ColorField label="Accent" v-model="widget.accent" @commit="commit" />
  </template>
</template>

<style scoped>
.help {
  color: var(--text-dim);
  font-size: 11px;
  margin: -2px 0 8px;
  line-height: 1.4;
}
</style>
