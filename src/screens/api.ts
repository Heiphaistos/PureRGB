// Couche unique d'accès au backend Écrans : commandes Tauri + mode démo (mock).
// Le mode démo s'active via ?mock=1, hors Tauri, ou si `screens_list` échoue au démarrage.
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ref } from "vue";
import { mock } from "./mock.ts";
import type {
  DesignSummary,
  Layout,
  LiveValues,
  MediaItem,
  ScreenInfo,
  ScreenOptions,
  ScreenState,
  SensorsReport,
} from "./types.ts";

const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const forced = typeof location !== "undefined" && new URLSearchParams(location.search).has("mock");

/** Vrai quand les données viennent du mock (bandeau « mode démonstration »). */
export const demo = ref<boolean>(forced || !isTauri());

export function errMsg(e: unknown): string {
  if (e instanceof Error) return e.message;
  return typeof e === "string" ? e : JSON.stringify(e);
}

export async function listScreens(): Promise<ScreenInfo[]> {
  if (demo.value) return mock.screensList();
  try {
    return await invoke<ScreenInfo[]>("screens_list");
  } catch {
    // Backend Écrans absent ou en échec au démarrage : bascule durable en démo.
    demo.value = true;
    return mock.screensList();
  }
}

/** `withSimulated` : ajoute l'écran simulé (extension du contrat, ignorée si non supportée). */
export const scanScreens = (withSimulated = false): Promise<ScreenInfo[]> =>
  demo.value ? mock.screensScan(withSimulated) : invoke<ScreenInfo[]>("screens_scan", { withSimulated });

export const screenState = (id: string): Promise<ScreenState> =>
  demo.value ? mock.screenState(id) : invoke<ScreenState>("screen_state", { id });

export const setLayout = (id: string, layout: Layout): Promise<void> =>
  demo.value ? mock.setLayout(id, layout) : invoke<void>("screen_set_layout", { id, layout });

export const setOptions = (id: string, options: ScreenOptions): Promise<void> =>
  demo.value ? mock.setOptions(id, options) : invoke<void>("screen_set_options", { id, options });

export const sendStill = (id: string): Promise<void> =>
  demo.value ? Promise.resolve() : invoke<void>("screen_send_still", { id });

export const clearScreen = (id: string): Promise<void> =>
  demo.value ? mock.clear(id) : invoke<void>("screen_clear", { id });

/** PNG base64 (sans préfixe data:) rendu par le moteur. */
export const preview = (layout: Layout, width: number, height: number, tMs: number): Promise<string> =>
  demo.value ? mock.preview(layout, width, height, tMs) : invoke<string>("screen_preview", { layout, width, height, tMs });

export const designsList = (): Promise<DesignSummary[]> =>
  demo.value ? mock.designsList() : invoke<DesignSummary[]>("designs_list");

export const designGet = (id: string): Promise<Layout> =>
  demo.value ? mock.designGet(id) : invoke<Layout>("design_get", { id });

export const designUserSave = (layout: Layout): Promise<string> =>
  demo.value ? mock.designUserSave(layout) : invoke<string>("design_user_save", { layout });

export const designUserDelete = (id: string): Promise<void> =>
  demo.value ? mock.designUserDelete(id) : invoke<void>("design_user_delete", { id });

export const designExport = (id: string, path: string): Promise<void> =>
  demo.value ? mock.designExport(id, path) : invoke<void>("design_export", { id, path });

export const designImport = (path: string): Promise<string> =>
  invoke<string>("design_import", { path });

/** Thème = design + médias utilisés, dans une archive `.purergb-theme`. `id` (bibliothèque) ou `layout` (éditeur). */
export async function themeExport(target: { id: string } | { layout: Layout }, path: string): Promise<void> {
  if (demo.value) return mock.themeExport("layout" in target ? target.layout : await mock.designGet(target.id));
  return invoke<void>("theme_export", { id: "id" in target ? target.id : null, layout: "layout" in target ? target.layout : null, path });
}

/** Retourne l'id du design importé (médias ajoutés à la médiathèque). */
export const themeImport = (path: string): Promise<string> => invoke<string>("theme_import", { path });

export const mediaList = (): Promise<MediaItem[]> =>
  demo.value ? mock.mediaList() : invoke<MediaItem[]>("media_list");

export const mediaImport = (path: string): Promise<MediaItem> => invoke<MediaItem>("media_import", { path });

/** Démo seulement : import d'un fichier glissé dans le navigateur. */
export const mediaImportFile = (file: File): Promise<MediaItem> => mock.mediaImportFile(file);

export const mediaDelete = (id: string): Promise<void> =>
  demo.value ? mock.mediaDelete(id) : invoke<void>("media_delete", { id });

export const screenValues = (): Promise<LiveValues> =>
  demo.value ? mock.screenValues() : invoke<LiveValues>("screen_values");

/** Tous les capteurs bruts + diagnostic sensord (panneau « Capteurs détectés »). */
export const screenSensors = (): Promise<SensorsReport> =>
  demo.value ? mock.screenSensors() : invoke<SensorsReport>("screen_sensors");

// --- Boîtes de dialogue (Tauri uniquement) ---

export async function pickFiles(
  title: string,
  filters: { name: string; extensions: string[] }[],
  multiple = true,
): Promise<string[]> {
  const r = await open({ title, filters, multiple, directory: false });
  if (r === null) return [];
  return Array.isArray(r) ? r : [r];
}

export async function pickSavePath(title: string, defaultPath: string): Promise<string | null> {
  return save({ title, defaultPath, filters: [{ name: "Design PureRGB", extensions: ["json"] }] });
}

export async function pickThemeSavePath(title: string, defaultPath: string): Promise<string | null> {
  return save({ title, defaultPath, filters: THEME_FILTERS });
}

export const THEME_FILTERS = [{ name: "Thème PureRGB (design + médias)", extensions: ["purergb-theme"] }];

export const MEDIA_FILTERS = [
  { name: "Images, GIF et vidéos", extensions: ["png", "jpg", "jpeg", "bmp", "webp", "gif", "mp4", "m4v", "webm", "mkv", "avi", "mov", "wmv"] },
];

export const DESIGN_FILTERS = [{ name: "Design PureRGB", extensions: ["json"] }];

/** Petit canevas de miniature : largeur max du rendu demandé au moteur. */
export function previewSize(w: number, h: number, maxW: number): { width: number; height: number } {
  const k = Math.min(1, maxW / w);
  return { width: Math.max(1, Math.round(w * k)), height: Math.max(1, Math.round(h * k)) };
}
