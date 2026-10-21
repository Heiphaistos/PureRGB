<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { clearScreen, errMsg, screenState, sendStill, setOptions } from "../../screens/api.ts";
import type { ScreenInfo, ScreenPerf, ScreenState } from "../../screens/types.ts";
import SensorsPanel from "./SensorsPanel.vue";
import "./screens.css";

const props = defineProps<{ screen: ScreenInfo; state: ScreenState }>();
const emit = defineEmits<{ changed: []; toast: [msg: string] }>();

const brightness = ref(props.state.brightness);
const rotation = ref(props.state.rotation);
const fps = ref(props.state.fps);
let timer: ReturnType<typeof setTimeout> | undefined;
const perf = ref<ScreenPerf>(props.state.perf);
let poll: ReturnType<typeof setInterval> | undefined;

onMounted(() => {
  poll = setInterval(() => {
    screenState(props.screen.id)
      .then((s) => (perf.value = s.perf))
      .catch(() => undefined); // mesure facultative : un échec ponctuel n'a rien à signaler
  }, 1500);
});
onUnmounted(() => clearInterval(poll));

const ROTATIONS = [0, 90, 180, 270];

async function save(): Promise<void> {
  try {
    await setOptions(props.screen.id, { brightness: brightness.value, rotation: rotation.value, fps: fps.value });
    emit("changed");
  } catch (e) {
    emit("toast", `Options : ${errMsg(e)}`);
  }
}

/** Glisser un curseur envoie la valeur au plus toutes les 250 ms. */
function later(): void {
  clearTimeout(timer);
  timer = setTimeout(() => void save(), 250);
}

async function still(): Promise<void> {
  try {
    await sendStill(props.screen.id);
    emit("toast", "Image envoyée à l'écran (mémoire de l'appareil)");
    emit("changed");
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

async function clear(): Promise<void> {
  try {
    await clearScreen(props.screen.id);
    emit("changed");
    emit("toast", "Écran effacé");
  } catch (e) {
    emit("toast", errMsg(e));
  }
}
</script>

<template>
  <div class="opt">
    <div class="sc-card box">
      <div class="sc-field">
        <label for="sc-br">Luminosité</label>
        <div class="sc-row">
          <input id="sc-br" v-model.number="brightness" type="range" min="0" max="100" @input="later" />
          <span class="val">{{ brightness }} %</span>
        </div>
      </div>
      <div class="sc-field">
        <label>Rotation</label>
        <div class="sc-row">
          <button
            v-for="r in ROTATIONS"
            :key="r"
            class="sm"
            :class="{ on: rotation === r }"
            @click="(rotation = r), save()"
          >
            {{ r }}°
          </button>
        </div>
      </div>
      <div class="sc-field">
        <label for="sc-fps">Cadence max</label>
        <div class="sc-row">
          <input id="sc-fps" v-model.number="fps" type="range" min="1" max="30" @input="later" />
          <span class="val">{{ fps }} i/s</span>
        </div>
      </div>
      <p class="sc-sub">
        Cadence réelle : {{ perf.fps > 0 ? perf.fps.toFixed(1) + " i/s" : "aucune image envoyée (design fixe)" }} pour
        {{ perf.target_fps.toFixed(0) }} i/s visées · {{ perf.dropped }} images abandonnées<br />
        Temps moyens : rendu {{ perf.render_ms.toFixed(1) }} ms · encodage {{ perf.encode_ms.toFixed(1) }} ms · envoi
        {{ perf.send_ms.toFixed(1) }} ms<br />
        <template v-if="perf.quality > 0">Qualité JPEG : {{ perf.quality }} (s'adapte au débit USB)</template>
      </p>
      <p class="sc-sub">
        Une cadence basse économise le processeur : 1 i/s suffit pour des capteurs et une horloge, 15 à 30 i/s pour
        les GIF et les vidéos.
      </p>
    </div>

    <div v-if="!state.live" class="sc-card box">
      <div class="sc-row">
        <div class="sc-grow">
          <strong>Image fixe</strong>
          <p class="sc-sub">
            Cet écran n'accepte pas encore de flux en direct : le design actuel est envoyé comme une image fixe
            (stockée dans l'appareil, à utiliser avec parcimonie). Les valeurs capteurs ne se mettent pas à jour.
          </p>
        </div>
        <button class="primary" @click="still">Envoyer comme image fixe</button>
      </div>
    </div>

    <div class="sc-card box">
      <div class="sc-row">
        <div class="sc-grow">
          <strong>Éteindre / effacer l'écran</strong>
          <p class="sc-sub">Arrête l'affichage et envoie une image noire.</p>
        </div>
        <button class="danger" @click="clear">Effacer l'écran</button>
      </div>
    </div>

    <SensorsPanel />

    <div class="sc-card box warn">
      <strong>Un seul logiciel à la fois</strong>
      <p class="sc-sub">
        Si TRCC, L-Connect ou un autre logiciel constructeur tourne en parallèle, les deux se disputent le périphérique :
        l'écran peut clignoter ou rester figé. Fermez-le (y compris dans la zone de notification) pendant que PureRGB
        pilote l'écran.
      </p>
    </div>
  </div>
</template>

<style scoped>
.opt {
  padding: 18px 26px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 720px;
}

.box {
  padding: 16px 18px;
}

.box .sc-field {
  grid-template-columns: 110px 1fr;
  font-size: 13px;
}

.val {
  min-width: 56px;
  text-align: right;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.warn {
  border-color: rgba(245, 185, 74, 0.5);
  background: rgba(245, 185, 74, 0.06);
}
</style>
