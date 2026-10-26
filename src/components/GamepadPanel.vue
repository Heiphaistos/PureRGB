<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { buttonName, drift, stick } from "../gamepad";
import GamepadRemap from "./GamepadRemap.vue";

interface Live {
  index: number;
  id: string;
  axes: number[];
  buttons: { pressed: boolean; value: number }[];
  canRumble: boolean;
}

const pads = ref<Live[]>([]);
const deadzone = ref(0.1);
const rest = ref<[number, number][]>([]);
const resting = ref(false);
const msg = ref("");
let raf = 0;

const sticks = (p: Live) => [stick(p.axes[0] ?? 0, p.axes[1] ?? 0, deadzone.value), stick(p.axes[2] ?? 0, p.axes[3] ?? 0, deadzone.value)];
const driftResult = computed(() => drift(rest.value));

function poll() {
  const list = Array.from(navigator.getGamepads?.() ?? []).filter((g): g is Gamepad => !!g);
  pads.value = list.map((g) => ({
    index: g.index,
    id: g.id,
    axes: [...g.axes],
    buttons: g.buttons.map((b) => ({ pressed: b.pressed, value: b.value })),
    canRumble: !!(g as unknown as { vibrationActuator?: unknown }).vibrationActuator,
  }));
  if (resting.value && list[0]) {
    const a = list[0].axes;
    rest.value.push([a[0] ?? 0, a[1] ?? 0], [a[2] ?? 0, a[3] ?? 0]);
  }
  raf = requestAnimationFrame(poll);
}

async function rumble(i: number) {
  const g = navigator.getGamepads?.()[i] as unknown as {
    vibrationActuator?: { playEffect: (t: string, p: object) => Promise<unknown> };
  } | null;
  try {
    await g?.vibrationActuator?.playEffect("dual-rumble", { duration: 400, strongMagnitude: 0.4, weakMagnitude: 0.4 });
  } catch {
    msg.value = "Cette manette n'accepte pas la vibration par le navigateur.";
  }
}

function toggleRest() {
  if (resting.value) {
    resting.value = false;
    return;
  }
  rest.value = [];
  resting.value = true;
  setTimeout(() => (resting.value = false), 3000);
}

onMounted(() => (raf = requestAnimationFrame(poll)));
onUnmounted(() => cancelAnimationFrame(raf));
</script>

<template>
  <section class="pad">
    <h2>Manettes</h2>
    <p class="sub">
      Testeur en direct (lecture seule) : boutons, gâchettes, sticks, zone morte, détection de dérive et vibration. Appuyez sur un bouton
      de la manette pour la faire apparaître. L'éclairage des manettes Sony (DualSense, DualShock 4) se règle dans l'onglet Éclairage.
    </p>
    <p v-if="pads.length === 0" class="note">
      Aucune manette active. Branchez-la et appuyez sur un bouton (la fenêtre doit être au premier plan).
    </p>

    <label class="dz">
      Zone morte : {{ Math.round(deadzone * 100) }} %
      <input v-model.number="deadzone" type="range" min="0" max="0.5" step="0.01" />
    </label>

    <div v-for="p in pads" :key="p.index" class="card">
      <h3>{{ p.id }}</h3>
      <div class="sticks">
        <div v-for="(s, i) in sticks(p)" :key="i" class="stick">
          <svg viewBox="-1.1 -1.1 2.2 2.2" width="96" height="96">
            <circle cx="0" cy="0" r="1" class="ring" />
            <circle cx="0" cy="0" :r="deadzone" class="dz-ring" />
            <circle :cx="s.x" :cy="s.y" r="0.1" :class="{ idle: s.inDeadzone }" class="dot" />
          </svg>
          <small>{{ i === 0 ? "Gauche" : "Droit" }} : {{ s.x.toFixed(2) }} ; {{ s.y.toFixed(2) }}</small>
        </div>
      </div>
      <div class="btns">
        <span v-for="(b, i) in p.buttons" :key="i" class="btn" :class="{ on: b.pressed }">
          {{ buttonName(i) }}<template v-if="i === 6 || i === 7"> {{ Math.round(b.value * 100) }} %</template>
        </span>
      </div>
      <div class="actions">
        <button v-if="p.canRumble" @click="rumble(p.index)">Tester la vibration</button>
        <button @click="toggleRest">Mesurer la dérive (3 s, sticks au repos)</button>
        <span v-if="rest.length" :class="{ bad: driftResult.drifting }">
          Déflexion max au repos : {{ (driftResult.max * 100).toFixed(1) }} % —
          {{ driftResult.drifting ? "dérive détectée (au-dessus de 5 %)" : "normal" }}
        </span>
      </div>
      <p v-if="msg" class="note">{{ msg }}</p>
    </div>
    <GamepadRemap />
  </section>
</template>

<style scoped>
.pad { flex: 1; overflow-y: auto; padding: 22px 26px; }
.pad h2 { font-size: 19px; }
.sub { color: var(--text-dim); font-size: 13px; margin: 6px 0 14px; max-width: 720px; }
.note { color: var(--text-dim); }
.dz { display: flex; gap: 12px; align-items: center; margin-bottom: 14px; font-size: 13px; }
.card { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 16px 18px; margin-bottom: 14px; max-width: 760px; }
.card h3 { font-size: 14px; margin-bottom: 10px; word-break: break-all; }
.sticks { display: flex; gap: 28px; margin-bottom: 12px; }
.stick { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.ring { fill: none; stroke: var(--border); stroke-width: 0.03; }
.dz-ring { fill: none; stroke: var(--text-dim); stroke-width: 0.02; stroke-dasharray: 0.06; }
.dot { fill: var(--accent, #ff2bd6); }
.dot.idle { opacity: 0.45; }
.btns { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px; }
.btn { font-size: 11px; border: 1px solid var(--border); border-radius: 6px; padding: 3px 8px; color: var(--text-dim); }
.btn.on { background: var(--accent, #ff2bd6); color: #000; border-color: transparent; }
.actions { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; font-size: 13px; }
.bad { color: #ff4d4d; }
</style>
