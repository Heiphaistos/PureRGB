<script setup lang="ts">
import { reactive, ref } from "vue";
import { BLOCKS, BLOCK_CATEGORIES, defaultParam, type BlockDef } from "../../screens/blocks/index.ts";
import "./screens.css";

defineProps<{ full: boolean }>();
const emit = defineEmits<{ add: [def: BlockDef, ctx: { accent: string; param: string }] }>();

const accent = ref("#ff5000");
const params = reactive<Record<string, string>>(Object.fromEntries(BLOCKS.filter((b) => b.param).map((b) => [b.id, defaultParam(b)])));

const ctxOf = (b: BlockDef): { accent: string; param: string } => ({ accent: accent.value, param: params[b.id] ?? defaultParam(b) });

function onDragStart(e: DragEvent, b: BlockDef): void {
  if (!e.dataTransfer) return;
  e.dataTransfer.setData("application/x-purergb-block", JSON.stringify({ id: b.id, ...ctxOf(b) }));
  e.dataTransfer.effectAllowed = "copy";
}
</script>

<template>
  <nav class="blk" aria-label="Blocs prêts à l'emploi">
    <label class="acc">
      <span>Couleur d'accent</span>
      <input v-model="accent" type="color" aria-label="Couleur d'accent des blocs" />
    </label>
    <p class="hint">Cliquez ou glissez un bloc sur l'aperçu, puis déplacez-le.</p>
    <template v-for="c in BLOCK_CATEGORIES" :key="c">
      <h4>{{ c }}</h4>
      <div v-for="b in BLOCKS.filter((x) => x.category === c)" :key="b.id" class="row">
        <button :disabled="full" draggable="true" :title="`Ajouter : ${b.label}`" @click="emit('add', b, ctxOf(b))" @dragstart="onDragStart($event, b)">
          {{ b.label }}
        </button>
        <select v-if="b.param" v-model="params[b.id]" :aria-label="b.param.label" :title="b.param.label">
          <option v-for="o in b.param.options" :key="o" :value="o">{{ o }}</option>
        </select>
      </div>
    </template>
    <p v-if="full" class="sc-err">Limite de widgets atteinte.</p>
  </nav>
</template>

<style scoped>
.blk {
  width: 188px;
  flex: none;
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 12px 10px;
  border-right: 1px solid var(--border);
  background: var(--bg-panel);
  overflow-y: auto;
}

h4 {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
  margin: 10px 4px 2px;
}

.acc {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  padding: 0 4px;
}

.acc input {
  width: 36px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--border);
  background: none;
}

.hint {
  font-size: 11px;
  color: var(--text-dim);
  margin: 4px;
  line-height: 1.4;
}

.row {
  display: flex;
  gap: 4px;
}

button {
  flex: 1;
  min-width: 0;
  text-align: left;
  padding: 7px 9px;
  font-size: 12px;
  cursor: grab;
}

select {
  width: 52px;
  flex: none;
  padding: 2px 4px;
  font-size: 12px;
}
</style>
