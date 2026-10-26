<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, ref } from "vue";
import { errMsg } from "../screens/api";

interface Check {
  label: string;
  ok: boolean;
  detail: string;
}
interface Item {
  id: string;
  name: string;
  vendor: string;
  kind: string;
  source: string;
  risk: "standard" | "élevé";
  can_test: boolean;
  blocked: string | null;
  checks: Check[];
}
interface TestResult {
  ok: boolean;
  steps: Check[];
  restored: string;
}

const items = ref<Item[] | null>(null);
const busy = ref(false);
const error = ref("");
const results = ref<Record<string, TestResult>>({});
/** Appareil pour lequel la confirmation est affichée. */
const confirming = ref<Item | null>(null);
const understood = ref(false);
const testing = ref<string | null>(null);

async function scan(): Promise<void> {
  busy.value = true;
  error.value = "";
  try {
    items.value = await invoke<Item[]>("selftest_scan");
  } catch (e) {
    error.value = errMsg(e);
  } finally {
    busy.value = false;
  }
}

function ask(item: Item): void {
  confirming.value = item;
  understood.value = false;
}

async function runTest(): Promise<void> {
  const item = confirming.value;
  if (!item || !understood.value) return;
  confirming.value = null;
  testing.value = item.id;
  error.value = "";
  try {
    results.value[item.id] = await invoke<TestResult>("selftest_apply", { id: item.id, confirmed: true });
  } catch (e) {
    error.value = `${item.name} : ${errMsg(e)}`;
  } finally {
    testing.value = null;
  }
}

const CATEGORIES: { title: string; kinds: string[] }[] = [
  { title: "Mémoire", kinds: ["Dram", "Storage"] },
  { title: "Carte mère et carte graphique", kinds: ["Motherboard", "Gpu"] },
  { title: "Ventilateurs et refroidissement", kinds: ["Fan", "Cooler", "Aio", "Hub"] },
  { title: "Claviers, souris et accessoires", kinds: ["Keyboard", "Mouse", "Mousemat", "Headset", "HeadsetStand", "Gamepad", "Keypad", "Microphone", "Speaker"] },
  { title: "Boîtier et ARGB", kinds: ["LedStrip", "Case", "Light", "Accessory", "Virtual", "Unknown"] },
];
const SCREEN = /écran|ecran|lcd|screen|display/i;
const categoryOf = (it: Item): string =>
  SCREEN.test(it.name) ? "Écrans" : (CATEGORIES.find((c) => c.kinds.includes(it.kind))?.title ?? "Boîtier et ARGB");
const drivable = (it: Item): boolean => it.checks.find((c) => c.label === "Pilote prêt")?.ok !== false;

/** Appareils pilotables groupés par catégorie, puis un bloc replié pour ceux qu'aucun pilote ne contrôle. */
const sections = computed(() => {
  const list = items.value ?? [];
  const order = [...CATEGORIES.map((c) => c.title), "Écrans"];
  const out: { title: string; items: Item[]; fold: boolean }[] = order
    .map((title) => ({ title, items: list.filter((i) => drivable(i) && categoryOf(i) === title), fold: false }))
    .filter((g) => g.items.length);
  const rest = list.filter((i) => !drivable(i));
  if (rest.length) out.push({ title: `Détectés mais non pilotables (${rest.length})`, items: rest, fold: true });
  return out;
});

function allOk(item: Item): boolean {
  return item.checks.every((c) => c.ok);
}
</script>

<template>
  <div class="card st">
    <h3>Test de compatibilité et de sécurité</h3>
    <p class="hint">
      Étape 1 : PureRGB passe en revue chaque composant détecté sans rien lui envoyer (pilote disponible, nombre de LEDs,
      logiciels concurrents, accès au bus). Étape 2, composant par composant et seulement si vous cliquez : une couleur
      très faible est envoyée (2 écritures en tout) puis l'état d'origine est remis. Commencez par un composant à risque
      « standard » (clavier, ventilateurs) avant la mémoire.
    </p>
    <div class="row">
      <button :disabled="busy" @click="scan">{{ busy ? "Analyse…" : items ? "Relancer l'analyse" : "Analyser mes composants" }}</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>

    <div v-if="items" class="groups">
      <component :is="g.fold ? 'details' : 'section'" v-for="g in sections" :key="g.title" class="grp">
        <component :is="g.fold ? 'summary' : 'h4'" class="gt">{{ g.title }}</component>
        <ul class="list">
          <li v-for="it in g.items" :key="it.id" :class="{ high: it.risk === 'élevé' }">
            <div class="head">
              <strong>{{ it.name }}</strong>
              <span class="badge" :class="allOk(it) ? 'ok' : 'warn'">{{ allOk(it) ? "Prêt" : "À vérifier" }}</span>
              <span class="badge" :class="it.risk === 'élevé' ? 'risk' : ''">{{ it.risk === "élevé" ? "Risque élevé" : "Risque standard" }}</span>
            </div>
            <span class="src">via {{ it.source }}</span>
            <ul class="checks">
              <li v-for="c in it.checks" :key="c.label" :class="c.ok ? 'ok' : 'ko'">
                <span aria-hidden="true">{{ c.ok ? "✓" : "✗" }}</span> {{ c.label }} <em>— {{ c.detail }}</em>
              </li>
            </ul>
            <p v-if="it.blocked" class="blocked">Test d'écriture indisponible : {{ it.blocked }}</p>
            <button v-else :disabled="testing !== null" @click="ask(it)">{{ testing === it.id ? "Test en cours…" : "Tester l'écriture" }}</button>
            <template v-if="results[it.id]">
              <ul class="checks res">
                <li v-for="s in results[it.id].steps" :key="s.label" :class="s.ok ? 'ok' : 'ko'">
                  <span aria-hidden="true">{{ s.ok ? "✓" : "✗" }}</span> {{ s.label }} <em>— {{ s.detail }}</em>
                </li>
              </ul>
              <p class="restored">{{ results[it.id].ok ? "Test réussi." : "Test interrompu : aucune autre écriture n'a été envoyée." }} {{ results[it.id].restored }}.</p>
            </template>
          </li>
        </ul>
      </component>
    </div>

    <div v-if="confirming" class="modal" role="dialog" aria-modal="true">
      <div class="inner">
        <h4>Tester l'écriture sur « {{ confirming.name }} » ?</h4>
        <p v-if="confirming.risk === 'élevé'" class="warnbox">
          Composant à risque ÉLEVÉ (bus mémoire). N'allez pas plus loin si un logiciel de monitoring ou RGB
          (iCUE, HWiNFO, CPU-Z, Armoury Crate…) est ouvert. Sauvegardez votre travail : en cas de doute, ne testez pas.
        </p>
        <p v-else>PureRGB va envoyer une couleur bleue très faible à cet appareil pendant 1,5 s, puis l'éteindre, puis remettre son effet enregistré.</p>
        <label class="inline">
          <input type="checkbox" v-model="understood" />
          Je comprends et j'ai fermé les logiciels concurrents.
        </label>
        <div class="row">
          <button class="primary" :disabled="!understood" @click="runTest">Lancer le test</button>
          <button @click="confirming = null">Annuler</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  grid-column: 1 / -1;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 18px;
  margin-bottom: 14px;
}
h3 {
  font-size: 14px;
  margin-bottom: 8px;
}
.hint {
  color: var(--text-dim);
  font-size: 13px;
  margin-bottom: 10px;
}
.row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.err {
  color: #f87171;
  font-size: 12px;
  margin-top: 8px;
}
.list {
  list-style: none;
  padding: 0;
  margin: 12px 0 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.list > li {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 14px;
  font-size: 13px;
  color: var(--text);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.list > li.high {
  border-color: rgba(245, 185, 74, 0.5);
}
.head {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.badge {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 10px;
  border-radius: 999px;
  background: var(--border);
}
.badge.ok {
  background: rgba(52, 211, 153, 0.25);
  color: #34d399;
}
.badge.warn,
.badge.risk {
  background: rgba(245, 185, 74, 0.25);
  color: #f5b94a;
}
.src {
  color: var(--text-dim);
  font-size: 12px;
}
.groups { columns: 380px 3; column-gap: 16px; margin-top: 8px; }
.grp { margin: 0 0 14px; break-inside: avoid; }
details.grp > .list { display: grid; grid-template-columns: repeat(auto-fill, minmax(max(380px, 31%), 1fr)); }
details.grp { column-span: all; }
.gt { font-size: 14px; font-weight: 600; margin: 0 0 4px; cursor: default; }
details.grp > .gt { cursor: pointer; color: var(--text-dim); }
.grp .list { margin-top: 8px; }
.checks {
  list-style: none;
  padding: 0;
  margin: 0;
}
.checks li {
  padding: 1px 0;
}
.checks .ok span {
  color: #34d399;
}
.checks .ko span {
  color: #f87171;
}
.checks em {
  color: #b9b6c4;
  font-style: normal;
}
.blocked,
.restored {
  color: var(--text-dim);
  margin: 0;
}
.modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: grid;
  place-items: center;
  z-index: 50;
}
.inner {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 20px;
  max-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 13px;
}
.warnbox {
  border: 1px solid rgba(245, 185, 74, 0.6);
  background: rgba(245, 185, 74, 0.1);
  border-radius: 8px;
  padding: 10px;
}
.inline {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}
</style>
