<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { errMsg } from "../screens/api";
import { bannerText, gameMode, quitApp, refreshGameMode, saveGameMode } from "../gameMode";

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
  <div class="card">
    <h3>Mode jeu (anti-cheat)</h3>
    <p class="hint">
      Quand un jeu protégé par un anti-cheat (EasyAntiCheat, BattlEye, Vanguard, FACEIT, Ricochet…) est détecté,
      PureRGB arrête OpenRGB (celui qu'il a lancé), les capteurs et ventilateurs de la carte mère, l'accès à la RAM
      (SMBus, pilote PawnIO) et l'envoi aux mini-écrans USB. Tout reprend 10 secondes après la fermeture du jeu.
      PureRGB n'injecte rien dans les jeux et ne désinstalle jamais le pilote PawnIO : il cesse seulement de l'utiliser.
    </p>
    <div class="inline" style="margin-bottom: 12px">
      <input id="gamemode" type="checkbox" v-model="gameMode.enabled" @change="apply" />
      <label for="gamemode">Activer le mode jeu (recommandé)</label>
    </div>
    <div class="inline" style="margin-bottom: 12px">
      <input id="gamemode-hid" type="checkbox" v-model="gameMode.keep_hid" @change="apply" />
      <label for="gamemode-hid">Garder l'éclairage des périphériques USB (HID natif) pendant la partie</label>
    </div>
    <label for="gamemode-games">Jeux ou lanceurs supplémentaires (un nom d'exécutable par ligne, ex. monjeu.exe)</label>
    <textarea id="gamemode-games" v-model="gamesText" rows="3" spellcheck="false" @change="apply"></textarea>

    <p class="hint" role="status">
      <strong>État :</strong>
      {{ bannerText(gameMode) ?? (gameMode.enabled ? "en veille, aucun jeu protégé détecté" : "désactivé") }}
      <template v-if="gameMode.active && gameMode.external_openrgb">
        — un OpenRGB lancé par vous reste actif : PureRGB ne peut pas l'arrêter.
      </template>
    </p>

    <p class="hint">
      <strong>À savoir :</strong> le mode jeu réduit le risque, il ne peut pas garantir l'absence de bannissement.
      Le pilote PawnIO reste installé sur Windows, et PureRGB (non signé) reste ouvert dans la barre des tâches.
      Le seul moyen d'avoir un risque nul est de fermer PureRGB avant de jouer.
    </p>
    <div class="inline" style="gap: 10px">
      <button :disabled="quitting" @click="quit">{{ quitting ? "Fermeture…" : "Quitter PureRGB proprement" }}</button>
    </div>
    <p v-if="error" class="hint">{{ error }}</p>
  </div>
</template>
