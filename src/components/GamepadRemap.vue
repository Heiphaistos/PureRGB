<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { errMsg } from "../screens/api";
import {
  SOURCES, BUTTON_LABELS, TARGET_CHOICES, choiceToTarget, curvePath, defaultProfile, targetLabel, targetToChoice,
  withoutIdentity, type Button, type PadState, type Profile,
} from "../gamepadRemap";

interface Live { virtual_active: boolean; pad_key: string | null; raw: PadState; out: PadState; error: string | null; message: string | null }
interface View { enabled: boolean; profiles: Record<string, Profile>; assign: Record<string, string>; live: Live; game_mode_active: boolean }
interface Drivers { vigem_ready: boolean; hidhide_installed: boolean; vigem_version: string; vigem_sha256: string; hidhide_url: string }

const DEFAULT_NAME = "Défaut";
const view = ref<View | null>(null);
const drivers = ref<Drivers | null>(null);
const name = ref(DEFAULT_NAME);
const draft = ref<Profile>(defaultProfile());
const dirty = ref(false);
const error = ref("");
const busy = ref(false);
let timer = 0;

const names = computed(() => [DEFAULT_NAME, ...Object.keys(view.value?.profiles ?? {}).filter((n) => n !== DEFAULT_NAME)]);
const padKey = computed(() => view.value?.live.pad_key ?? null);
const assigned = computed(() => (padKey.value ? (view.value?.assign[padKey.value] ?? DEFAULT_NAME) : null));

function load(n: string): void {
  name.value = n;
  draft.value = structuredClone(view.value?.profiles[n] ?? defaultProfile());
  dirty.value = false;
}

async function refresh(): Promise<void> {
  try {
    const first = view.value === null;
    view.value = await invoke<View>("gamepad_remap_status");
    if (first) load(DEFAULT_NAME);
  } catch (e) {
    error.value = errMsg(e);
  }
}

async function refreshDrivers(): Promise<void> {
  drivers.value = await invoke<Drivers>("gamepad_remap_drivers").catch(() => null);
}

async function run(f: () => Promise<unknown>): Promise<void> {
  error.value = "";
  try {
    await f();
  } catch (e) {
    error.value = errMsg(e);
  }
  await refresh();
}

const toggle = (on: boolean) => run(() => invoke("gamepad_remap_enable", { enabled: on }));
const save = () =>
  run(async () => {
    await invoke("gamepad_remap_save_profile", { name: name.value, profile: withoutIdentity(draft.value) });
    dirty.value = false;
  });
const remove = () =>
  run(async () => {
    await invoke("gamepad_remap_delete_profile", { name: name.value });
    load(DEFAULT_NAME);
  });
const assign = () => run(() => invoke("gamepad_remap_assign", { padKey: padKey.value, profile: name.value }));
const calibrate = () => run(() => invoke("gamepad_remap_calibrate"));

function newProfile(): void {
  const n = window.prompt("Nom du nouveau profil (40 caractères maximum)");
  if (!n?.trim()) return;
  name.value = n.trim();
  draft.value = defaultProfile();
  dirty.value = true;
}

function setTarget(src: Button, choice: string): void {
  draft.value.buttons[src] = choiceToTarget(choice);
  dirty.value = true;
}

async function installDriver(): Promise<void> {
  const d = drivers.value;
  const ok = window.confirm(
    `Installer le pilote ViGEmBus ${d?.vigem_version ?? ""} ?\n\n` +
      "PureRGB va télécharger l'installeur officiel (github.com/nefarius/ViGEmBus), vérifier son empreinte SHA-256 " +
      `(${d?.vigem_sha256.slice(0, 16) ?? ""}…) puis le lancer : c'est un pilote noyau, Windows demandera votre accord. ` +
      "Ne fermez pas PureRGB pendant l'installation. ViGEmBus n'est plus maintenu par son auteur.",
  );
  if (!ok) return;
  busy.value = true;
  await run(() => invoke("gamepad_remap_install_driver", { confirmed: true }));
  await refreshDrivers();
  busy.value = false;
}

const pct = (v: number) => `${Math.round(v * 100)} %`;
const stickArea = (s: "left_stick" | "right_stick") => (s === "left_stick" ? "Stick gauche" : "Stick droit");

onMounted(() => {
  void refresh();
  void refreshDrivers();
  timer = window.setInterval(() => void refresh(), 250);
});
onUnmounted(() => clearInterval(timer));
</script>

<template>
  <section class="remap">
    <h3>Remappage (manette Xbox 360 virtuelle)</h3>
    <p class="sub">
      PureRGB lit votre manette XInput (Xbox, manettes compatibles XInput), applique le profil puis l'envoie à une manette Xbox 360
      virtuelle que les jeux voient. Désactivé par défaut. Les manettes Sony ou génériques non XInput ne sont pas lues.
    </p>

    <div class="warn">
      Attention anti-cheat : certains anti-cheats (Vanguard, EAC, BattlEye, FACEIT…) refusent les pilotes de manette virtuelle.
      Le remappage se coupe tout seul pendant le mode jeu si un exécutable de la liste (data/gamemode_anticheat.json) est détecté.
      <b v-if="view?.game_mode_active">Mode jeu actif : remappage suspendu.</b>
    </div>

    <div class="card">
      <h4>Pilote</h4>
      <p v-if="drivers">
        ViGEmBus : <b :class="drivers.vigem_ready ? 'ok' : 'bad'">{{ drivers.vigem_ready ? "installé" : "absent" }}</b>
        <template v-if="!drivers.vigem_ready">
          — <button :disabled="busy || view?.game_mode_active" @click="installDriver">Installer le pilote</button>
          <small> (jamais automatique : confirmation demandée, version {{ drivers.vigem_version }})</small>
        </template>
      </p>
      <label class="row">
        <input type="checkbox" :checked="view?.enabled" :disabled="!drivers?.vigem_ready" @change="toggle(($event.target as HTMLInputElement).checked)" />
        Activer le remappage
      </label>
      <p v-if="view?.live.error" class="bad">{{ view.live.error }}</p>
      <p v-if="view?.live.virtual_active" class="note">
        Manette virtuelle active (lecture de {{ padKey }}).
        Les jeux voient aussi la manette physique : <b>deux manettes</b>. Pour la masquer, option avancée :
        <a :href="drivers?.hidhide_url" target="_blank" rel="noopener">HidHide</a>
        ({{ drivers?.hidhide_installed ? "détecté sur ce PC ; ajoutez PureRGB à sa liste d'applications autorisées" : "non installé, PureRGB ne l'installe pas" }}).
      </p>
    </div>

    <template v-if="view">
      <div class="card">
        <h4>Profil</h4>
        <div class="row">
          <select :value="name" @change="load(($event.target as HTMLSelectElement).value)">
            <option v-for="n in names" :key="n" :value="n">{{ n }}</option>
            <option v-if="!names.includes(name)" :value="name">{{ name }} (nouveau)</option>
          </select>
          <button @click="newProfile">Nouveau</button>
          <button :disabled="!dirty" @click="save">Enregistrer</button>
          <button :disabled="name === DEFAULT_NAME || !view.profiles[name]" @click="remove">Supprimer</button>
          <button :disabled="!padKey || assigned === name || (!view.profiles[name] && name !== DEFAULT_NAME)" @click="assign">
            Utiliser pour {{ padKey ?? "la manette" }}
          </button>
        </div>
        <small v-if="padKey">Profil actuel de {{ padKey }} : {{ assigned }}. Les profils sont liés à l'emplacement XInput de la manette.</small>
      </div>

      <div class="card">
        <h4>Boutons</h4>
        <div class="map">
          <template v-for="s in SOURCES" :key="s">
            <span>{{ BUTTON_LABELS[s] }}</span>
            <select :value="targetToChoice(draft.buttons[s], s)" @change="setTarget(s, ($event.target as HTMLSelectElement).value)">
              <option v-for="c in TARGET_CHOICES" :key="c" :value="c">{{ targetLabel(c) }}</option>
            </select>
          </template>
        </div>
      </div>

      <div v-for="s in (['left_stick', 'right_stick'] as const)" :key="s" class="card">
        <h4>{{ stickArea(s) }}</h4>
        <div class="stick">
          <div class="ctl">
            <label>Zone morte
              <select v-model="draft[s].mode" @change="dirty = true">
                <option value="radial">Radiale (cercle)</option>
                <option value="axial">Axiale (par axe)</option>
              </select>
            </label>
            <label>Intérieure : {{ pct(draft[s].inner) }}
              <input v-model.number="draft[s].inner" type="range" min="0" max="0.9" step="0.01" @input="dirty = true" />
            </label>
            <label>Saturation extérieure : {{ pct(draft[s].outer) }}
              <input v-model.number="draft[s].outer" type="range" min="0" max="0.5" step="0.01" @input="dirty = true" />
            </label>
            <label>Courbe (exposant {{ draft[s].exponent.toFixed(2) }} : &lt; 1 réactif, &gt; 1 précis)
              <input v-model.number="draft[s].exponent" type="range" min="0.3" max="4" step="0.05" @input="dirty = true" />
            </label>
            <label class="row"><input v-model="draft[s].invert_x" type="checkbox" @change="dirty = true" /> Inverser X</label>
            <label class="row"><input v-model="draft[s].invert_y" type="checkbox" @change="dirty = true" /> Inverser Y</label>
            <small>Anti-dérive enregistrée : {{ pct(draft[s].center_x) }} ; {{ pct(draft[s].center_y) }}</small>
          </div>
          <svg viewBox="-0.05 -0.05 1.1 1.1" width="120" height="120" class="curve">
            <rect x="0" y="0" width="1" height="1" class="frame" />
            <line x1="0" y1="1" x2="1" y2="0" class="ref" />
            <polyline :points="curvePath(draft[s].inner, draft[s].outer, draft[s].exponent)" class="line" />
          </svg>
        </div>
      </div>

      <div v-for="t in (['left_trigger', 'right_trigger'] as const)" :key="t" class="card">
        <h4>{{ t === "left_trigger" ? "Gâchette gauche" : "Gâchette droite" }}</h4>
        <div class="ctl">
          <label>Seuil bas (0 en dessous) : {{ draft[t].low }}
            <input v-model.number="draft[t].low" type="range" min="0" max="254" @input="dirty = true" />
          </label>
          <label>Seuil haut (255 au-dessus) : {{ draft[t].high }}
            <input v-model.number="draft[t].high" type="range" min="1" max="255" @input="dirty = true" />
          </label>
          <label>Courbe (exposant {{ draft[t].exponent.toFixed(2) }})
            <input v-model.number="draft[t].exponent" type="range" min="0.3" max="4" step="0.05" @input="dirty = true" />
          </label>
        </div>
      </div>

      <div class="card">
        <h4>Anti-dérive et contrôle en direct</h4>
        <div class="row">
          <button :disabled="!view.live.virtual_active" @click="calibrate">Mesurer la dérive (1 s, sticks au repos)</button>
          <small v-if="view.live.message">{{ view.live.message }}</small>
        </div>
        <small v-if="view.live.virtual_active">
          Entrée : G {{ view.live.raw.lx }} ; {{ view.live.raw.ly }} — D {{ view.live.raw.rx }} ; {{ view.live.raw.ry }} — LT {{ view.live.raw.lt }} RT {{ view.live.raw.rt }}<br />
          Sortie : G {{ view.live.out.lx }} ; {{ view.live.out.ly }} — D {{ view.live.out.rx }} ; {{ view.live.out.ry }} — LT {{ view.live.out.lt }} RT {{ view.live.out.rt }}
        </small>
        <small v-else>Activez le remappage avec une manette connectée pour voir les valeurs. Enregistrez le profil pour appliquer les réglages.</small>
      </div>
    </template>
    <p v-if="error" class="bad">{{ error }}</p>
  </section>
</template>

<style scoped>
.remap { max-width: 760px; margin-top: 22px; }
.remap h3 { font-size: 16px; }
.sub { color: var(--text-dim); font-size: 13px; margin: 6px 0 12px; }
.warn { border: 1px solid #d9a21b; border-radius: var(--radius); padding: 10px 14px; font-size: 13px; margin-bottom: 12px; }
.card { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 18px; margin-bottom: 12px; }
.card h4 { font-size: 14px; margin-bottom: 8px; }
.row { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; font-size: 13px; }
.map { display: grid; grid-template-columns: 180px 1fr; gap: 6px 12px; align-items: center; font-size: 13px; }
.stick { display: flex; gap: 24px; align-items: flex-start; }
.ctl { display: flex; flex-direction: column; gap: 8px; flex: 1; font-size: 13px; }
.ctl label { display: flex; flex-direction: column; gap: 3px; }
.ctl label.row { flex-direction: row; }
.curve .frame { fill: none; stroke: var(--border); stroke-width: 0.01; }
.curve .ref { stroke: var(--text-dim); stroke-width: 0.01; stroke-dasharray: 0.03; }
.curve .line { fill: none; stroke: var(--accent, #ff2bd6); stroke-width: 0.025; }
.note { color: var(--text-dim); font-size: 13px; }
.ok { color: #3ecf6e; }
.bad { color: #ff4d4d; }
</style>
