// Mise à jour automatique de PureRGB : manifeste signé (minisign) publié avec la
// release GitHub, téléchargement avec progression, installation NSIS passive, relance.
// Sans rapport avec la mise à jour du sidecar OpenRGB (backends/openrgb/updater.rs).
import { reactive } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { progressPercent } from "./updateProgress";

// phase : idle | checking | downloading | installing
export const updateState = reactive<{ phase: string; percent: number | null }>({
  phase: "idle",
  percent: null,
});

/**
 * @param silent true = rien d'affiché s'il n'y a rien / canal injoignable
 *               (vérification au démarrage) ; false = bouton manuel.
 * @returns true si une mise à jour a été installée (relance en cours).
 */
export async function checkForUpdate(silent = true): Promise<boolean> {
  // En dev le binaire n'est pas installé : rien à remplacer.
  if (import.meta.env.DEV || updateState.phase !== "idle") return false;
  updateState.phase = "checking";
  try {
    const update = await check();
    if (!update) {
      if (!silent) await message("PureRGB est à jour.", { title: "Mise à jour" });
      return false;
    }
    const ok = await ask(
      `PureRGB ${update.version} est disponible (vous avez la ${update.currentVersion}).` +
        (update.body ? `\n\nNouveautés :\n${update.body}` : "") +
        "\n\nInstaller maintenant ? L'application redémarrera.",
      { title: "Nouvelle version disponible", kind: "info", okLabel: "Installer", cancelLabel: "Plus tard" },
    );
    if (!ok) return false;

    let downloaded = 0;
    let total = 0;
    updateState.phase = "downloading";
    updateState.percent = 0;
    await update.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      else if (ev.event === "Progress") {
        downloaded += ev.data.chunkLength;
        updateState.percent = progressPercent(downloaded, total);
      } else if (ev.event === "Finished") updateState.phase = "installing";
    });
    await relaunch();
    return true;
  } catch (e) {
    // Canal injoignable, signature refusée... : on garde la version actuelle.
    console.error("[maj] échec", e);
    if (!silent) {
      await message(`Vérification impossible : ${String(e)}`, { title: "Mise à jour", kind: "error" });
    }
    return false;
  } finally {
    updateState.phase = "idle";
    updateState.percent = null;
  }
}
