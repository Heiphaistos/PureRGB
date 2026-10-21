// État partagé du décodeur vidéo universel : statut, installation avec progression (interrogée toutes les 400 ms).
import { ref } from "vue";
import { errMsg, ffmpegInstall, ffmpegProgress, ffmpegStatus } from "./api.ts";
import type { FfmpegProgress, FfmpegStatus } from "./types.ts";

export const ffmpeg = ref<FfmpegStatus | null>(null);
export const ffmpegBusy = ref(false);
export const ffmpegPct = ref(0);

export async function refreshFfmpeg(): Promise<void> {
  try {
    ffmpeg.value = await ffmpegStatus();
  } catch {
    ffmpeg.value = null; // inconnu : l'interface n'affiche rien de faux
  }
}

/** Installe ffmpeg ; lève l'erreur (en français) si le téléchargement ou la vérification échoue. */
export async function installFfmpeg(): Promise<void> {
  if (ffmpegBusy.value) return;
  ffmpegBusy.value = true;
  ffmpegPct.value = 0;
  const timer = setInterval(() => {
    void ffmpegProgress()
      .then((p: FfmpegProgress) => (ffmpegPct.value = p.total > 0 ? Math.min(100, Math.round((p.done / p.total) * 100)) : 0))
      .catch(() => undefined);
  }, 400);
  try {
    ffmpeg.value = await ffmpegInstall();
    ffmpegPct.value = 100;
  } catch (e) {
    throw new Error(errMsg(e));
  } finally {
    clearInterval(timer);
    ffmpegBusy.value = false;
  }
}
