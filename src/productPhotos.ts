// Photos de produits : identification (sans réseau) puis photo du fabricant (téléchargée par le
// backend, jamais par la page). Cache mémoire par produit ; échec silencieux -> l'appelant garde l'icône.
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { mockLookup, mockPhoto } from "./productMock.ts";

export interface ProductView {
  id: string;
  kind: string;
  vendor: string;
  name: string;
  has_image: boolean;
  source_page: string | null;
}

const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const forcedMock = (): boolean => typeof location !== "undefined" && new URLSearchParams(location.search).has("mock");
const useMock = (): boolean => forcedMock() || !isTauri();

/** Réglage « Afficher les photos des produits » (défaut : activé). Désactivé -> aucune requête. */
export const photosEnabled = ref(true);
let settingsRead = false;

async function ensureSettings(): Promise<void> {
  if (settingsRead || useMock()) return;
  settingsRead = true;
  try {
    const s = await invoke<{ product_photos?: boolean }>("get_settings");
    photosEnabled.value = s.product_photos !== false;
  } catch {
    settingsRead = false;
  }
}

export async function setPhotosEnabled(v: boolean): Promise<void> {
  if (!useMock()) await invoke<void>("set_product_photos", { enabled: v });
  photosEnabled.value = v;
}

const lookups = new Map<string, Promise<ProductView | null>>();
const photos = new Map<string, Promise<string | null>>();

export async function lookupProduct(vendor: string, name: string, usb: string | null): Promise<ProductView | null> {
  await ensureSettings();
  if (!photosEnabled.value) return null;
  const key = `${vendor}\u0000${name}\u0000${usb ?? ""}`;
  let p = lookups.get(key);
  if (!p) {
    p = useMock()
      ? Promise.resolve(mockLookup(vendor, name))
      : invoke<ProductView | null>("product_lookup", { vendor, name, usb }).catch(() => null);
    lookups.set(key, p);
  }
  return p;
}

export function productPhoto(id: string): Promise<string | null> {
  if (!photosEnabled.value) return Promise.resolve(null);
  let p = photos.get(id);
  if (!p) {
    p = (useMock() ? Promise.resolve(mockPhoto(id)) : invoke<string>("product_image", { id })).catch(() => {
      photos.delete(id); // l'échec est mémorisé côté backend (24 h) : ici on autorise un nouvel essai plus tard
      return null;
    });
    photos.set(id, p);
  }
  return p;
}

export function openProductSource(id: string): void {
  if (useMock()) return;
  void invoke<void>("product_open_source", { id }).catch(() => undefined);
}
