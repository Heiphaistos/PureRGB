<script setup lang="ts">
import { computed } from "vue";
import { WIDGET_LABELS } from "../../screens/widgets.ts";
import type { Background, Layout, LiveValues, MediaItem, Widget } from "../../screens/types.ts";
import { MAX_CANVAS } from "../../screens/types.ts";
import ColorField from "./ColorField.vue";
import MediaPicker from "./MediaPicker.vue";
import InspectorFields from "./InspectorFields.vue";
import NumField from "./NumField.vue";
import TextField from "./TextField.vue";
import "./screens.css";

const props = defineProps<{ layout: Layout; selected: Widget[]; values: LiveValues; media: MediaItem[] }>();
const emit = defineEmits<{ commit: [key: string | null] }>();

const single = computed<Widget | null>(() => (props.selected.length === 1 ? props.selected[0] : null));
const images = computed(() => props.media.filter((m) => m.kind !== "font"));
const commit = (): void => emit("commit", null);

const BG_KINDS: { v: Background["kind"]; l: string }[] = [
  { v: "solid", l: "Couleur unie" },
  { v: "gradient", l: "Dégradé" },
  { v: "media", l: "Image / GIF / vidéo" },
];

function setBgKind(kind: Background["kind"]): void {
  const bg = props.layout.background;
  if (bg.kind === kind) return;
  const base = bg.kind === "solid" ? bg.color : bg.kind === "gradient" ? bg.from : "#0d0d10";
  if (kind === "solid") props.layout.background = { kind, color: base };
  else if (kind === "gradient") props.layout.background = { kind, from: base, to: "#000000", angle: 180 };
  else props.layout.background = { kind, media_id: images.value[0]?.id ?? "", fit: "cover" };
  commit();
}
</script>

<template>
  <aside class="insp">
    <template v-if="single">
      <h4>{{ WIDGET_LABELS[single.type] }}</h4>
      <div class="grp">
        <NumField v-model="single.x" label="X" unit="px" @commit="commit" />
        <NumField v-model="single.y" label="Y" unit="px" @commit="commit" />
        <NumField v-model="single.w" label="Largeur" :min="4" unit="px" @commit="commit" />
        <NumField v-model="single.h" label="Hauteur" :min="4" unit="px" @commit="commit" />
        <NumField v-model="single.rotation" label="Rotation" :min="-180" :max="180" unit="°" @commit="commit" />
        <NumField v-model="single.opacity" label="Opacité" :min="0" :max="1" :step="0.05" @commit="commit" />
      </div>
      <h4>Propriétés</h4>
      <div class="grp">
        <InspectorFields :widget="single" :values="values" :media="media" @commit="commit" />
      </div>
    </template>

    <template v-else-if="selected.length > 1">
      <h4>{{ selected.length }} widgets sélectionnés</h4>
      <p class="hint">
        Déplacez-les ensemble à la souris ou aux flèches. Utilisez la barre d'outils pour aligner, changer l'ordre,
        dupliquer ou supprimer. Sélectionnez un seul widget pour modifier ses propriétés.
      </p>
    </template>

    <template v-else>
      <h4>Design</h4>
      <div class="grp">
        <TextField v-model="layout.name" label="Nom" @commit="commit" />
        <NumField v-model="layout.width" label="Largeur" :min="1" :max="MAX_CANVAS" unit="px" @commit="commit" />
        <NumField v-model="layout.height" label="Hauteur" :min="1" :max="MAX_CANVAS" unit="px" @commit="commit" />
      </div>
      <h4>Fond</h4>
      <div class="grp">
        <label class="sc-field">
          <span>Type</span>
          <select :value="layout.background.kind" @change="setBgKind(($event.target as HTMLSelectElement).value as Background['kind'])">
            <option v-for="k in BG_KINDS" :key="k.v" :value="k.v">{{ k.l }}</option>
          </select>
        </label>
        <template v-if="layout.background.kind === 'solid'">
          <ColorField v-model="layout.background.color" label="Couleur" @commit="commit" />
        </template>
        <template v-else-if="layout.background.kind === 'gradient'">
          <ColorField v-model="layout.background.from" label="Début" @commit="commit" />
          <ColorField v-model="layout.background.to" label="Fin" @commit="commit" />
          <NumField v-model="layout.background.angle" label="Angle" unit="°" @commit="commit" />
        </template>
        <template v-else>
          <MediaPicker v-model="layout.background.media_id" v-model:fit="layout.background.fit" :media="images" @commit="commit" />
        </template>
      </div>
      <p class="hint">Cliquez sur un widget du canevas ou ajoutez-en un depuis la palette.</p>
    </template>
  </aside>
</template>

<style scoped>
.insp {
  width: clamp(260px, 22vw, 380px);
  flex: none;
  border-left: 1px solid var(--border);
  background: var(--bg-panel);
  overflow-y: auto;
  padding: 14px 16px;
}

h4 {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
  margin: 6px 0 10px;
}

.grp {
  margin-bottom: 14px;
}

.hint {
  color: var(--text-dim);
  font-size: 12px;
  line-height: 1.5;
}
</style>
