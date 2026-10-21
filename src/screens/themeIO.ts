// Export / import de thèmes (design + médias) avec boîtes de dialogue : partagé par la galerie et l'éditeur.
import { demo, errMsg, pickFiles, pickThemeSavePath, THEME_FILTERS, themeExport, themeImport } from "./api.ts";
import { loadDesigns, loadMedia } from "./store.ts";
import type { Layout } from "./types.ts";

type Toast = (msg: string) => void;

export async function exportTheme(target: { id: string } | { layout: Layout }, name: string, toast: Toast): Promise<void> {
  try {
    const path = demo.value ? "" : await pickThemeSavePath("Exporter le thème (avec médias)", `${name}.purergb-theme`);
    if (path === null) return;
    await themeExport(target, path);
    toast("Thème exporté (design + médias)");
  } catch (e) {
    toast(`Export du thème : ${errMsg(e)}`);
  }
}

/** Retourne l'id du design importé, ou null (annulation, démo, erreur). */
export async function importTheme(toast: Toast): Promise<string | null> {
  if (demo.value) {
    toast("Import de thème indisponible en mode démonstration");
    return null;
  }
  try {
    const [path] = await pickFiles("Importer un thème", THEME_FILTERS, false);
    if (!path) return null;
    const id = await themeImport(path);
    await Promise.all([loadDesigns(), loadMedia()]);
    toast("Thème importé dans « Mes designs » (médias ajoutés à la médiathèque)");
    return id;
  } catch (e) {
    toast(`Import du thème : ${errMsg(e)}`);
    return null;
  }
}
