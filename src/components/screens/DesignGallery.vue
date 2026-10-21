<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { designImport, DESIGN_FILTERS, demo, designUserDelete, errMsg, pickFiles } from "../../screens/api.ts";
import { designs, isUserDesign, loadDesigns } from "../../screens/store.ts";
import { forgetThumb } from "../../screens/thumbs.ts";
import type { DesignSummary, ScreenInfo } from "../../screens/types.ts";
import DesignThumb from "./DesignThumb.vue";
import "./screens.css";

const props = defineProps<{ screen: ScreenInfo }>();
const emit = defineEmits<{ apply: [id: string]; edit: [id: string]; toast: [msg: string] }>();

const category = ref<string>("all");
const query = ref("");
const mineOnly = ref(false);
const sameFormat = ref(false);

const ratio = computed(() => props.screen.width / props.screen.height);
const matches = (d: DesignSummary): boolean => Math.abs(d.width / d.height - ratio.value) < 0.03;

const categories = computed(() => {
  const m = new Map<string, number>();
  for (const d of designs.value) if (!isUserDesign(d)) m.set(d.category, (m.get(d.category) ?? 0) + 1);
  return [...m.entries()].map(([name, n]) => ({ name, n }));
});

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return designs.value
    .filter((d) => (mineOnly.value ? isUserDesign(d) : category.value === "all" || d.category === category.value))
    .filter((d) => !sameFormat.value || matches(d))
    .filter((d) => !q || d.name.toLowerCase().includes(q) || d.tags.some((t) => t.toLowerCase().includes(q)))
    .sort((a, b) => Number(matches(b)) - Number(matches(a)) || a.name.localeCompare(b.name, "fr"));
});

const mineCount = computed(() => designs.value.filter(isUserDesign).length);

async function remove(d: DesignSummary): Promise<void> {
  if (!window.confirm(`Supprimer définitivement « ${d.name} » ?`)) return;
  try {
    await designUserDelete(d.id);
    forgetThumb(`design:${d.id}`);
    await loadDesigns();
    emit("toast", "Design supprimé");
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

async function importDesign(): Promise<void> {
  if (demo.value) {
    emit("toast", "Import de fichier indisponible en mode démonstration");
    return;
  }
  try {
    const [path] = await pickFiles("Importer un design", DESIGN_FILTERS, false);
    if (!path) return;
    await designImport(path);
    await loadDesigns();
    mineOnly.value = true;
    emit("toast", "Design importé dans « Mes designs »");
  } catch (e) {
    emit("toast", `Import : ${errMsg(e)}`);
  }
}

onMounted(async () => {
  try {
    await loadDesigns();
  } catch (e) {
    emit("toast", `Designs : ${errMsg(e)}`);
  }
});
</script>

<template>
  <div class="gal">
    <div class="bar">
      <input v-model="query" type="text" placeholder="Rechercher un design…" class="q" aria-label="Rechercher un design" />
      <button class="sm" :class="{ on: sameFormat }" title="Seulement les designs au même format que l'écran" @click="sameFormat = !sameFormat">
        Mon format
      </button>
      <button class="sm" @click="importDesign">Importer .json</button>
    </div>
    <div class="chips">
      <button class="sc-chip" :class="{ active: !mineOnly && category === 'all' }" @click="(mineOnly = false), (category = 'all')">
        Tous<span class="n">{{ designs.length - mineCount }}</span>
      </button>
      <button
        v-for="c in categories"
        :key="c.name"
        class="sc-chip"
        :class="{ active: !mineOnly && category === c.name }"
        @click="(mineOnly = false), (category = c.name)"
      >
        {{ c.name }}<span class="n">{{ c.n }}</span>
      </button>
      <button class="sc-chip" :class="{ active: mineOnly }" @click="mineOnly = true">Mes designs<span class="n">{{ mineCount }}</span></button>
    </div>

    <div class="grid">
      <article v-for="d in filtered" :key="d.id" class="sc-card item" tabindex="0" @dblclick="emit('apply', d.id)" @keydown.enter="emit('apply', d.id)">
        <DesignThumb :design="d" />
        <div class="meta">
          <strong class="nm" :title="d.name">{{ d.name }}</strong>
          <span class="dim">{{ d.width }}×{{ d.height }}<template v-if="!matches(d)"> · format différent</template></span>
        </div>
        <div class="acts">
          <button class="primary sm" @click="emit('apply', d.id)">Appliquer</button>
          <button class="sm" @click="emit('edit', d.id)">Modifier</button>
          <button v-if="isUserDesign(d)" class="sm danger" title="Supprimer ce design" @click="remove(d)">Supprimer</button>
        </div>
      </article>
    </div>
    <p v-if="!filtered.length" class="sc-sub none">
      {{ mineOnly ? "Aucun design personnel. Créez-en un dans l'éditeur puis « Enregistrer sous »." : "Aucun design ne correspond." }}
    </p>
  </div>
</template>

<style scoped>
.gal {
  overflow-y: auto;
  padding: 18px 26px;
}

.bar {
  display: flex;
  gap: 8px;
  align-items: center;
}

.q {
  max-width: 360px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 14px;
  margin-top: 16px;
}

.item {
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.item:focus-visible,
.item:hover {
  border-color: var(--accent);
  outline: none;
}

.meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nm {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dim {
  color: var(--text-dim);
  font-size: 11px;
}

.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.none {
  margin-top: 24px;
}
</style>
