// Mode jeu : état partagé (réglage + jeu détecté), alimenté par la commande `game_mode_status`
// et l'événement `game-mode-changed` émis par le backend (src-tauri/src/gamemode).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { reactive } from "vue";

export interface GameModeView {
  enabled: boolean;
  keep_hid: boolean;
  games: string[];
  active: boolean;
  game: string | null;
  grace: boolean;
  external_openrgb: boolean;
}

export const gameMode = reactive<GameModeView>({
  enabled: true,
  keep_hid: false,
  games: [],
  active: false,
  game: null,
  grace: false,
  external_openrgb: false,
});

/** Texte du bandeau (null = rien à afficher). */
export function bannerText(s: Pick<GameModeView, "active" | "game" | "grace">): string | null {
  if (!s.active) return null;
  const base = `Mode jeu actif : accès matériel suspendu pour ${s.game ?? "le jeu en cours"}`;
  return s.grace ? `${base} (jeu fermé, reprise dans quelques secondes)` : base;
}

const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function refreshGameMode(): Promise<void> {
  if (!isTauri()) return;
  Object.assign(gameMode, await invoke<GameModeView>("game_mode_status"));
}

let listening = false;
export async function watchGameMode(): Promise<void> {
  if (!isTauri() || listening) return;
  listening = true;
  await refreshGameMode();
  await listen("game-mode-changed", () => void refreshGameMode());
}

export async function saveGameMode(enabled: boolean, keepHid: boolean, games: string[]): Promise<void> {
  await invoke("set_game_mode", { enabled, keepHid, games });
  await refreshGameMode();
}

export async function quitApp(): Promise<void> {
  await invoke("quit_app");
}
