// Filtrage, regroupement et tri de la galerie (pur, testé) : conçu pour 100 à 500 designs.
import type { DesignSummary } from "./types.ts";

export const isUserDesign = (d: DesignSummary): boolean => d.category === "user" || d.tags.includes("user");

const KNOWN: [number, string][] = [
  [1, "1:1"], [4 / 3, "4:3"], [3 / 4, "3:4"], [16 / 9, "16:9"], [9 / 16, "9:16"], [16 / 10, "16:10"],
  [10 / 16, "10:16"], [3 / 2, "3:2"], [2 / 3, "2:3"], [5 / 4, "5:4"], [4 / 5, "4:5"],
];

/** Libellé de ratio (« 16:9 ») ; ratio exotique : valeur décimale (« 4.16:1 »). */
export function ratioLabel(w: number, h: number): string {
  const r = w / h;
  return KNOWN.find(([k]) => Math.abs(k - r) < 0.03)?.[1] ?? `${r.toFixed(2)}:1`;
}

export const sameRatio = (d: { width: number; height: number }, w: number, h: number): boolean => ratioLabel(d.width, d.height) === ratioLabel(w, h);

/** Minuscules sans accents : « Écran » trouve « ecran ». */
export const fold = (s: string): string => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

export interface GalleryQuery {
  text: string;
  /** "all" ou un libellé de ratio (« 16:9 »). */
  ratio: string;
}

/** Tous les mots de la recherche doivent apparaître dans le nom, les tags, la catégorie ou le format. */
export function matchesQuery(d: DesignSummary, q: GalleryQuery): boolean {
  if (q.ratio !== "all" && ratioLabel(d.width, d.height) !== q.ratio) return false;
  const words = fold(q.text).split(/\s+/).filter(Boolean);
  if (!words.length) return true;
  const hay = fold(`${d.name} ${d.tags.join(" ")} ${d.category} ${d.width}x${d.height} ${ratioLabel(d.width, d.height)}`);
  return words.every((w) => hay.includes(w));
}

export interface Section {
  key: string;
  title: string;
  items: DesignSummary[];
  /** Nombre de designs au même ratio que l'écran dans la section. */
  fits: number;
}

/** Sections par catégorie (« Mes designs » en tête, puis celles qui contiennent des designs au format de l'écran) ; tri : même ratio d'abord, puis nom. */
export function buildSections(list: DesignSummary[], screen: { width: number; height: number }): Section[] {
  const fit = (d: DesignSummary): boolean => sameRatio(d, screen.width, screen.height);
  const by = new Map<string, DesignSummary[]>();
  for (const d of list) {
    const key = isUserDesign(d) ? "user" : d.category;
    by.set(key, [...(by.get(key) ?? []), d]);
  }
  const sections: Section[] = [...by.entries()].map(([key, items]) => ({
    key,
    title: key === "user" ? "Mes designs" : key,
    items: items.sort((a, b) => Number(fit(b)) - Number(fit(a)) || a.name.localeCompare(b.name, "fr", { numeric: true })),
    fits: items.filter(fit).length,
  }));
  return sections.sort((a, b) => Number(b.key === "user") - Number(a.key === "user") || b.fits - a.fits || a.title.localeCompare(b.title, "fr"));
}

/** Formats présents, avec leur nombre de designs (le format de l'écran en premier). */
export function ratioOptions(list: DesignSummary[], screen: { width: number; height: number }): { label: string; n: number }[] {
  const m = new Map<string, number>();
  for (const d of list) m.set(ratioLabel(d.width, d.height), (m.get(ratioLabel(d.width, d.height)) ?? 0) + 1);
  const mine = ratioLabel(screen.width, screen.height);
  return [...m.entries()].map(([label, n]) => ({ label, n })).sort((a, b) => Number(b.label === mine) - Number(a.label === mine) || b.n - a.n);
}
