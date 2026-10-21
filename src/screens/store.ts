// État partagé de l'onglet Écrans (listes chargées une fois, valeurs live sondées).
import { ref } from "vue";
import * as api from "./api.ts";
import type { DesignSummary, LiveValues, MediaItem, ScreenInfo } from "./types.ts";

export const screens = ref<ScreenInfo[]>([]);
export const designs = ref<DesignSummary[]>([]);
export const media = ref<MediaItem[]>([]);
export const values = ref<LiveValues>({});
export const scanning = ref(false);
export const loaded = ref(false);

export async function loadScreens(): Promise<void> {
  screens.value = await api.listScreens();
  loaded.value = true;
}

export async function scan(withSimulated = false): Promise<void> {
  scanning.value = true;
  try {
    screens.value = await api.scanScreens(withSimulated);
  } finally {
    scanning.value = false;
  }
}

export async function loadDesigns(): Promise<void> {
  designs.value = await api.designsList();
}

export async function loadMedia(): Promise<void> {
  media.value = await api.mediaList();
}

let valuesTimer: ReturnType<typeof setInterval> | undefined;
let valuesUsers = 0;

async function pollValues(): Promise<void> {
  try {
    values.value = await api.screenValues();
  } catch {
    // Valeurs live indisponibles : l'inspecteur n'affiche simplement pas de valeur.
    values.value = {};
  }
}

/** Démarre le sondage des valeurs live (2 s) ; renvoie la fonction d'arrêt. */
export function watchValues(): () => void {
  valuesUsers++;
  if (valuesUsers === 1) {
    void pollValues();
    valuesTimer = setInterval(() => void pollValues(), 2000);
  }
  return () => {
    valuesUsers--;
    if (valuesUsers === 0) clearInterval(valuesTimer);
  };
}

export const isUserDesign = (d: DesignSummary): boolean => d.category === "user" || d.tags.includes("user");
