<script setup lang="ts">
import { ref } from "vue";
import { errMsg } from "../screens/api";
import { ffmpeg, ffmpegBusy, ffmpegPct, installFfmpeg, refreshFfmpeg } from "../screens/useFfmpeg";

const ffmpegErr = ref("");
void refreshFfmpeg();
async function installDecoder(): Promise<void> {
  ffmpegErr.value = "";
  try {
    await installFfmpeg();
  } catch (e) {
    ffmpegErr.value = errMsg(e);
  }
}
</script>

<template>
  <div class="card">
    <h3>Décodeur vidéo universel</h3>
    <p class="hint">
      Utilisé quand Windows ne sait pas lire une vidéo des écrans (ffmpeg, 115 Mo, vérifié par
      empreinte SHA-256, installé dans Program Files).
    </p>
    <div class="inline">
      <span>Décodeur vidéo universel : <strong>{{ ffmpeg === null ? "inconnu" : ffmpeg.installed ? `installé (ffmpeg ${ffmpeg.version})` : "non installé" }}</strong></span>
      <button v-if="ffmpeg && !ffmpeg.installed" :disabled="ffmpegBusy" @click="installDecoder">
        {{ ffmpegBusy ? `Installation… ${ffmpegPct} %` : "Installer" }}
      </button>
    </div>
    <p v-if="ffmpegErr" class="hint">{{ ffmpegErr }}</p>
  </div>
</template>

<style scoped>
.card {
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
  font-size: 12px;
  margin-bottom: 10px;
}

.inline {
  display: flex;
  align-items: center;
  gap: 10px;
}
</style>
