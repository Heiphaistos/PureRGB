<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { designImport, DESIGN_FILTERS, demo, designUserDelete, errMsg, pickFiles } from "../../screens/api.ts";
import { buildSections, matchesQuery, ratioLabel, ratioOptions, sameRatio } from "../../screens/galleryFilter.ts";
import { designs, isUserDesign, loadDesigns } from "../../screens/store.ts";
import { exportTheme, importTheme } from "../../screens/themeIO.ts";
import { forgetThumb } from "../../screens/thumbs.ts";
import type { DesignSummary, ScreenInfo } from "../../screens/types.ts";
import DesignThumb from "./DesignThumb.vue";
import "./screens.css";

const props = defineProps<{ screen: ScreenInfo }>();
const emit = defineEmits<{ apply: [id: string]; edit: [id: string]; toast: [msg: string] }>();

/** Designs rendus par section avant « Afficher plus » : la galerie reste fluide avec 500 designs. */
const PAGE = 12;

const query = ref("");
const ratio = ref("all");
const closed = reactive(new Set<string>());
const shown = reactive<Record<string, number>>({});

const myRatio = computed(() => ratioLabel(props.screen.width, props.screen.height));
const formats = computed(() => ratioOptions(designs.value, props.screen));
const results = computed(() => designs.value.filter((d) => matchesQuery(d, { text: query.value, ratio: ratio.value })));
const sections = computed(() => buildSections(results.value, props.screen));
const fits = (d: DesignSummary): boolean => sameRatio(d, props.screen.width, props.screen.height);
const limit = (key: string): number => shown[key] ?? PAGE;
const allClosed = computed(() => sections.value.length > 0 && sections.value.every((s) => closed.has(s.key)));

watch([query, ratio], () => Object.keys(shown).forEach((k) => delete shown[k]));

function toggle(key: string): void {
  if (!closed.delete(key)) closed.add(key);
}

function toggleAll(): void {
  if (allClosed.value) closed.clear();
  else sections.value.forEach((s) => closed.add(s.key));
}

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
    emit("toast", "Design importé dans « Mes designs »");
  } catch (e) {
    emit("toast", `Import : ${errMsg(e)}`);
  }
}

async function importThemeFile(): Promise<void> {
  if (await importTheme((m) => emit("toast", m))) closed.delete("user");
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
      <input v-model="query" type="text" placeholder="Rechercher (nom, tag, catégorie, format)…" class="q" aria-label="Rechercher un design" />
      <select v-model="ratio" class="fmt" aria-label="Format d'écran">
        <option value="all">Tous les formats</option>
        <option v-for="f in formats" :key="f.label" :value="f.label">{{ f.label }}{{ f.label === myRatio ? " (mon écran)" : "" }} · {{ f.n }}</option>
      </select>
      <button class="sm" @click="toggleAll">{{ allClosed ? "Tout déplier" : "Tout replier" }}</button>
      <span class="sc-grow"></span>
      <button class="sm" @click="importDesign">Importer .json</button>
      <button class="sm" title="Importer une archive .purergb-theme (design + médias)" @click="importThemeFile">Importer un thème</button>
    </div>
    <p class="count" role="status">
      {{ results.length }} design{{ results.length > 1 ? "s" : "" }} sur {{ designs.length }} · les designs au format de l'écran ({{ myRatio }}) passent en premier
    </p>

    <section v-for="sec in sections" :key="sec.key" class="sec">
      <button class="sh" :aria-expanded="!closed.has(sec.key)" @click="toggle(sec.key)">
        <span class="chev" aria-hidden="true">{{ closed.has(sec.key) ? "▸" : "▾" }}</span>
        <strong>{{ sec.title }}</strong>
        <span class="sc-badge">{{ sec.items.length }}</span>
        <span v-if="sec.fits && sec.fits !== sec.items.length" class="dim">{{ sec.fits }} au format de l'écran</span>
      </button>
      <template v-if="!closed.has(sec.key)">
        <div class="grid">
          <article
            v-for="d in sec.items.slice(0, limit(sec.key))"
            :key="d.id"
            class="sc-card item"
            tabindex="0"
            @dblclick="emit('apply', d.id)"
            @keydown.enter="emit('apply', d.id)"
          >
            <DesignThumb :design="d" />
            <div class="meta">
              <strong class="nm" :title="d.name">{{ d.name }}</strong>
              <span class="dim">{{ d.width }}×{{ d.height }}<template v-if="!fits(d)"> · format différent</template></span>
            </div>
            <div class="acts">
              <button class="primary sm" @click="emit('apply', d.id)">Appliquer</button>
              <button class="sm" @click="emit('edit', d.id)">Modifier</button>
              <button class="sm" title="Exporter le thème (avec médias) en .purergb-theme" @click="exportTheme({ id: d.id }, d.name, (m) => emit('toast', m))">Exporter le thème</button>
              <button v-if="isUserDesign(d)" class="sm danger" title="Supprimer ce design" @click="remove(d)">Supprimer</button>
            </div>
          </article>
        </div>
        <button v-if="sec.items.length > limit(sec.key)" class="sm more" @click="shown[sec.key] = limit(sec.key) + PAGE * 2">
          Afficher plus ({{ sec.items.length - limit(sec.key) }} restants)
        </button>
      </template>
    </section>
    <p v-if="!sections.length" class="sc-sub none">Aucun design ne correspond. Créez-en un dans l'éditeur puis « Enregistrer sous ».</p>
  </div>
</template>

<style scoped>
.gal {
  overflow-y: auto;
  padding: 18px 26px;
}

.bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.q {
  width: 340px;
  max-width: 100%;
}

.fmt {
  width: 210px;
}

.count {
  margin: 10px 0 0;
  font-size: 12px;
  color: var(--text-dim);
}

.sec {
  margin-top: 14px;
}

.sh {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  text-align: left;
  padding: 8px 10px;
  font-size: 13px;
}

.chev {
  width: 12px;
}

.more {
  margin-top: 10px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 14px;
  margin-top: 12px;
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
