<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { stick } from "../gamepad";

interface Slot {
  slot: number;
  buttons: number;
  lt: number;
  rt: number;
  lx: number;
  ly: number;
  rx: number;
  ry: number;
}

const props = defineProps<{ deadzone: number; browserCount: number }>();

const slots = ref<Slot[]>([]);
const failed = ref(false);
let timer: ReturnType<typeof setInterval> | undefined;

/** Masque XInput (XINPUT_GAMEPAD_*) -> nom français. */
const BUTTONS: [number, string][] = [
  [0x1000, "A"], [0x2000, "B"], [0x4000, "X"], [0x8000, "Y"],
  [0x0100, "LB"], [0x0200, "RB"], [0x0020, "Retour"], [0x0010, "Démarrer"],
  [0x0040, "Clic stick gauche"], [0x0080, "Clic stick droit"],
  [0x0001, "Haut"], [0x0002, "Bas"], [0x0004, "Gauche"], [0x0008, "Droite"],
];

async function poll(): Promise<void> {
  try {
    slots.value = (await invoke<Slot[] | null>("xinput_state")) ?? [];
    failed.value = false;
  } catch {
    failed.value = true; // hors application (aperçu navigateur)
  }
}

onMounted(() => {
  void poll();
  timer = setInterval(poll, 120);
});
onUnmounted(() => clearInterval(timer));

const norm = (v: number) => Math.max(-1, v / 32767);
const sticksOf = (s: Slot) => [stick(norm(s.lx), -norm(s.ly), props.deadzone), stick(norm(s.rx), -norm(s.ry), props.deadzone)];
const pressed = (s: Slot) => BUTTONS.filter(([m]) => (s.buttons & m) !== 0).map(([, n]) => n);
const mismatch = computed(() => slots.value.length > 0 && props.browserCount === 0);
</script>

<template>
  <div class="card xin">
    <h3>Lecture directe par Windows (XInput)</h3>
    <p class="hint">
      Ce que Windows voit réellement, même fenêtre en arrière-plan : toute manette Xbox, ou compatible Xbox (Flydigi, 8BitDo en
      mode X, GameSir…). Lecture seule : rien n'est écrit dans la manette.
    </p>
    <p v-if="failed" class="hint">Disponible dans l'application PureRGB uniquement.</p>
    <p v-else-if="slots.length === 0" class="warn">
      Aucune manette XInput détectée. Si votre manette est branchée : fermez le logiciel du fabricant (il peut la masquer), vérifiez
      son mode (X-Input / Xbox) et rebranchez-la.
    </p>
    <p v-if="mismatch" class="warn">
      Windows voit bien la manette, mais pas le navigateur intégré à PureRGB : appuyez sur un bouton fenêtre au premier plan.
      Le test ci-dessous reste exact.
    </p>
    <div v-for="s in slots" :key="s.slot" class="slot">
      <strong>Manette XInput n° {{ s.slot + 1 }}</strong>
      <div class="sticks">
        <div v-for="(st, i) in sticksOf(s)" :key="i" class="stick">
          <svg viewBox="-1.1 -1.1 2.2 2.2" width="84" height="84">
            <circle cx="0" cy="0" r="1" class="ring" />
            <circle cx="0" cy="0" :r="deadzone" class="dz-ring" />
            <circle :cx="st.x" :cy="st.y" r="0.1" :class="{ idle: st.inDeadzone }" class="dot" />
          </svg>
          <small>{{ i === 0 ? "Gauche" : "Droit" }} : {{ st.x.toFixed(2) }} ; {{ st.y.toFixed(2) }}</small>
        </div>
        <div class="trig">
          <small>LT {{ Math.round((s.lt / 255) * 100) }} %</small>
          <progress max="255" :value="s.lt" />
          <small>RT {{ Math.round((s.rt / 255) * 100) }} %</small>
          <progress max="255" :value="s.rt" />
        </div>
      </div>
      <div class="btns">
        <span v-for="n in pressed(s)" :key="n" class="btn on">{{ n }}</span>
        <span v-if="pressed(s).length === 0" class="hint">Aucun bouton appuyé.</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.xin { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 16px 18px; margin-bottom: 14px; }
h3 { font-size: 14px; margin-bottom: 6px; }
.hint { font-size: 12px; color: var(--text-dim); margin-bottom: 10px; }
.warn { font-size: 12px; color: var(--warn); margin-bottom: 10px; }
.slot { border-top: 1px solid var(--border); padding-top: 10px; margin-top: 6px; display: flex; flex-direction: column; gap: 8px; }
.sticks { display: flex; gap: 26px; flex-wrap: wrap; align-items: center; }
.stick { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.trig { display: flex; flex-direction: column; gap: 4px; min-width: 160px; }
.ring { fill: none; stroke: var(--border); stroke-width: 0.03; }
.dz-ring { fill: none; stroke: var(--text-dim); stroke-width: 0.02; stroke-dasharray: 0.06; }
.dot { fill: var(--accent, #ff2bd6); }
.dot.idle { opacity: 0.45; }
.btns { display: flex; flex-wrap: wrap; gap: 6px; }
.btn { font-size: 11px; border: 1px solid var(--border); border-radius: 6px; padding: 3px 8px; }
.btn.on { background: var(--accent, #ff2bd6); color: #000; border-color: transparent; }
</style>
