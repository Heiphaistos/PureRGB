// Ordre des cartes réorganisables (glisser-déposer) : logique pure, sans dépendance à Vue.

/** Fusionne un ordre sauvegardé avec les ids actuels : inconnus retirés, doublons ignorés, nouveaux ajoutés à la fin. */
export function mergeOrder(saved: readonly string[], ids: readonly string[]): string[] {
  const known = new Set(ids);
  const out: string[] = [];
  for (const id of saved) if (known.has(id) && !out.includes(id)) out.push(id);
  for (const id of ids) if (!out.includes(id)) out.push(id);
  return out;
}

/** Déplace `id` à la place de `target` (avant elle si on remonte, après si on descend). Renvoie une copie. */
export function moveBefore(order: readonly string[], id: string, target: string): string[] {
  const from = order.indexOf(id);
  const to = order.indexOf(target);
  if (from < 0 || to < 0 || from === to) return [...order];
  const out = [...order];
  out.splice(from, 1);
  out.splice(to, 0, id);
  return out;
}

/** Décale `id` de `delta` rang (clavier : -1 monte, +1 descend), borné aux extrémités. */
export function moveBy(order: readonly string[], id: string, delta: number): string[] {
  const from = order.indexOf(id);
  if (from < 0) return [...order];
  const to = Math.max(0, Math.min(order.length - 1, from + delta));
  return moveBefore(order, id, order[to]);
}

/**
 * Maçonnerie : répartit les cartes, dans l'ordre, chacune dans la colonne la plus courte (poids = hauteur estimée).
 * Renvoie `cols` listes d'ids ; à poids égal, la colonne de gauche gagne.
 */
export function masonry(ids: readonly string[], weight: (id: string) => number, cols: number): string[][] {
  const n = Math.max(1, Math.floor(cols));
  const columns: string[][] = Array.from({ length: n }, () => []);
  const heights = new Array<number>(n).fill(0);
  for (const id of ids) {
    let k = 0;
    for (let i = 1; i < n; i++) if (heights[i] < heights[k]) k = i;
    columns[k].push(id);
    heights[k] += weight(id);
  }
  return columns;
}

/** Nombre de colonnes selon la largeur disponible (px) : 1 sous 760, 2 jusqu'à 1100, 3 jusqu'à 1500, 4 au-delà. */
export function columnCount(width: number): number {
  return width < 760 ? 1 : width < 1100 ? 2 : width < 1500 ? 3 : 4;
}
