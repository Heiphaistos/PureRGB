<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { errMsg } from "../screens/api";
import anticheatData from "../../src-tauri/data/gamemode_anticheat.json";
import { gameMode, quitApp, refreshGameMode, saveGameMode } from "../gameMode";

type Anticheat = { name: string; exes: string[]; games?: string[]; services?: string[]; drivers?: string[]; verified?: boolean; source?: string };
const anticheats: Anticheat[] = anticheatData.anticheat;
const search = ref("");
const filtered = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return anticheats;
  return anticheats.filter((a) => [a.name, ...a.exes, ...(a.games ?? []), ...(a.drivers ?? [])].some((t) => t.toLowerCase().includes(q)));
});
const exeCount = anticheats.reduce((n, a) => n + a.exes.length + (a.games?.length ?? 0), 0);
const status = computed(() => {
  if (!gameMode.enabled) return { cls: "off", label: "DÉSACTIVÉ", why: "Le mode jeu est coupé : PureRGB continue d'utiliser le matériel même pendant une partie protégée." };
  if (gameMode.active)
    return {
      cls: "on",
      label: "ACTIF",
      why: gameMode.grace
        ? `${gameMode.game ?? "Le jeu"} vient de se fermer : reprise dans quelques secondes.`
        : `${gameMode.game ?? "Un jeu protégé"} est en cours : l'accès au matériel est suspendu.`,
    };
  return { cls: "idle", label: "EN VEILLE", why: "Aucun jeu protégé détecté pour l'instant. Le mode se déclenche tout seul au lancement d'un jeu." };
});
const suspended = [
  "OpenRGB lancé par PureRGB (arrêté)",
  "Capteurs et ventilateurs de la carte mère",
  "Accès à la RAM (SMBus, pilote PawnIO)",
  "Envoi aux mini-écrans USB",
  "Remappage de manette (manette virtuelle)",
];

const gamesText = ref("");
const error = ref("");
const quitting = ref(false);

watch(
  () => gameMode.games,
  (g) => {
    gamesText.value = g.join("\n");
  },
  { immediate: true },
);

onMounted(() => void refreshGameMode().catch((e) => (error.value = errMsg(e))));

async function apply(): Promise<void> {
  error.value = "";
  try {
    const games = gamesText.value.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
    await saveGameMode(gameMode.enabled, gameMode.keep_hid, games);
  } catch (e) {
    error.value = errMsg(e);
    await refreshGameMode().catch(() => undefined);
  }
}

async function quit(): Promise<void> {
  quitting.value = true;
  try {
    await quitApp();
  } catch (e) {
    quitting.value = false;
    error.value = errMsg(e);
  }
}
</script>

<template>
  <div class="card gm wide">
    <h3>Mode jeu (anti-cheat)</h3>
    <div class="state" :class="status.cls" role="status">
      <strong>{{ status.label }}</strong>
      <span>{{ status.why }}</span>
      <small v-if="gameMode.active && gameMode.external_openrgb">Un OpenRGB lancé par vous reste actif : PureRGB ne peut pas l'arrêter.</small>
    </div>
    <ul class="what">
      <li>Certains jeux (Valorant, Fortnite, Call of Duty…) embarquent un anti-cheat qui n'aime pas les logiciels d'éclairage.</li>
      <li>Quand PureRGB en détecte un, il arrête d'utiliser le matériel pendant toute la partie.</li>
      <li>Tout reprend 10 secondes après la fermeture du jeu. PureRGB n'injecte rien dans les jeux.</li>
    </ul>

    <div class="cols3">
      <div>
        <h4>Réglages</h4>
        <div class="inline sw">
          <input id="gamemode" type="checkbox" v-model="gameMode.enabled" @change="apply" />
          <label for="gamemode">Suspendre PureRGB quand un jeu protégé est lancé (recommandé)</label>
        </div>
        <div class="inline sw">
          <input id="gamemode-hid" type="checkbox" v-model="gameMode.keep_hid" @change="apply" />
          <label for="gamemode-hid">Garder l'éclairage des périphériques USB pendant la partie</label>
        </div>
        <label for="gamemode-games">Autres jeux à protéger (un nom de fichier .exe par ligne)</label>
        <textarea id="gamemode-games" v-model="gamesText" rows="4" spellcheck="false" placeholder="monjeu.exe" @change="apply"></textarea>
      </div>

      <div>
        <h4>Suspendu pendant la partie</h4>
        <ul class="list">
          <li v-for="x in suspended" :key="x">{{ x }}</li>
          <li v-if="!gameMode.keep_hid">Éclairage USB natif (HID)</li>
        </ul>
        <h4 v-if="gameMode.keep_hid">Reste actif</h4>
        <ul v-if="gameMode.keep_hid" class="list keep"><li>Éclairage USB natif (HID)</li></ul>
      </div>

      <div>
        <h4>Anti-cheats reconnus ({{ anticheats.length }} familles, {{ exeCount }} exécutables)</h4>
        <details class="acbox">
          <summary>Afficher la liste</summary>
          <p class="legend">
            <em class="ok">Vérifié</em> : noms confirmés par la source officielle. <em class="todo">À confirmer</em> : anti-cheat
            surveillé aussi, mais ses noms n'ont pas encore été recoupés. Tous les anti-cheats de la liste déclenchent le mode jeu.
          </p>
          <input v-model="search" type="search" placeholder="Rechercher (nom, .exe, pilote)…" aria-label="Rechercher un anti-cheat" />
          <ul class="list ac">
            <li v-for="a in filtered" :key="a.name">
              <strong>{{ a.name }} <em v-if="a.verified" class="ok" title="Noms confirmés par la source">Vérifié</em><em v-else class="todo" title="Surveillé, noms à recouper">À confirmer</em></strong>
              <small>{{ [...a.exes, ...(a.games ?? [])].join(", ") }}</small>
              <small v-if="a.drivers?.length">Pilote noyau : {{ a.drivers.join(", ") }} (non surveillé directement)</small>
            </li>
            <li v-if="!filtered.length">Aucun résultat.</li>
          </ul>
        </details>
        <template v-if="gameMode.games.length">
          <h4>Vos jeux ajoutés</h4>
          <p class="hint">{{ gameMode.games.join(", ") }}</p>
        </template>
      </div>
    </div>

    <p class="hint">
      Le mode jeu réduit le risque, il ne garantit pas l'absence de bannissement : le pilote PawnIO reste installé et PureRGB reste
      ouvert dans la barre des tâches. Risque nul seulement en fermant PureRGB avant de jouer.
    </p>
    <button :disabled="quitting" @click="quit">{{ quitting ? "Fermeture…" : "Quitter PureRGB complètement" }}</button>
    <p v-if="error" class="hint">{{ error }}</p>
  </div>
</template>

<style scoped>
.legend { font-size: 11px; color: var(--text-dim); margin: 6px 0; line-height: 1.5; }
em.ok, em.todo { font-style: normal; font-size: 10px; padding: 1px 6px; border-radius: 999px; border: 1px solid var(--border); }
em.ok { color: var(--ok); border-color: var(--ok); }
em.todo { color: var(--text-dim); }
.wide { grid-column: 1 / -1; }
.hint { font-size: 12px; color: var(--text-dim); line-height: 1.6; margin-bottom: 12px; }
.inline { display: flex; align-items: center; gap: 8px; }
.inline label { margin: 0; }
label { display: block; font-size: 13px; color: var(--text-dim); margin-bottom: 6px; }
.state { display: flex; flex-wrap: wrap; align-items: baseline; gap: 6px 14px; padding: 12px 16px; border-radius: var(--radius); border: 1px solid var(--border); margin-bottom: 12px; font-size: 13px; }
.state strong { font-size: 16px; letter-spacing: 0.04em; }
.state.on { border-color: #d9a21b; background: rgba(217, 162, 27, 0.14); }
.state.on strong { color: #d9a21b; }
.state.idle strong { color: #3ecf6e; }
.state.off strong { color: var(--text-dim); }
.state small { flex-basis: 100%; color: var(--text-dim); }
.what { margin: 0 0 14px 18px; font-size: 13px; color: var(--text-dim); line-height: 1.6; }
.cols3 { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 18px; margin-bottom: 12px; }
h4 { font-size: 13px; margin: 0 0 8px; }
.sw { margin-bottom: 10px; align-items: flex-start; }
.list { margin: 0 0 12px 16px; font-size: 13px; line-height: 1.6; }
.list.keep { color: #3ecf6e; }
.ac { list-style: none; margin-left: 0; max-height: 320px; overflow-y: auto; }
.acbox summary { cursor: pointer; font-size: 13px; margin-bottom: 8px; }
.acbox input { width: 100%; margin-bottom: 8px; }
.ac em { color: #3ecf6e; font-style: normal; }
.ac li { display: flex; flex-direction: column; margin-bottom: 4px; }
.ac small { color: var(--text-dim); font-size: 11px; word-break: break-all; }
textarea { width: 100%; background: var(--bg); color: inherit; border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 6px 8px; }
</style>
