// Magnétisme et guides d'alignement (pur, testable).
import type { Box } from "./geometry.ts";

export interface Guide {
  axis: "x" | "y";
  pos: number;
}

export interface SnapResult {
  dx: number;
  dy: number;
  guides: Guide[];
}

/** Plus petit écart entre une ligne mobile et une cible, dans le seuil. */
export function snapAxis(
  lines: number[],
  targets: number[],
  threshold: number,
): { delta: number; pos: number } | null {
  let best: { delta: number; pos: number } | null = null;
  for (const l of lines) {
    for (const t of targets) {
      const d = t - l;
      if (Math.abs(d) <= threshold && (best === null || Math.abs(d) < Math.abs(best.delta))) {
        best = { delta: d, pos: t };
      }
    }
  }
  return best;
}

export function boxLinesX(b: Box): number[] {
  return [b.x, b.x + b.w / 2, b.x + b.w];
}

export function boxLinesY(b: Box): number[] {
  return [b.y, b.y + b.h / 2, b.y + b.h];
}

/** Lignes cibles : bords/centre du canevas + de chaque autre widget. */
export function targetLines(canvas: { w: number; h: number }, others: Box[]): { xs: number[]; ys: number[] } {
  const xs = [0, canvas.w / 2, canvas.w];
  const ys = [0, canvas.h / 2, canvas.h];
  for (const o of others) {
    xs.push(...boxLinesX(o));
    ys.push(...boxLinesY(o));
  }
  return { xs, ys };
}

/** Magnétisme d'un déplacement : `box` = position proposée (déjà déplacée). */
export function snapMove(
  box: Box,
  others: Box[],
  canvas: { w: number; h: number },
  threshold: number,
  grid = 0,
): SnapResult {
  const { xs, ys } = targetLines(canvas, others);
  const guides: Guide[] = [];
  let dx = 0;
  let dy = 0;
  const sx = snapAxis(boxLinesX(box), xs, threshold);
  if (sx) {
    dx = sx.delta;
    guides.push({ axis: "x", pos: sx.pos });
  } else if (grid > 0) {
    dx = Math.round(box.x / grid) * grid - box.x;
  }
  const sy = snapAxis(boxLinesY(box), ys, threshold);
  if (sy) {
    dy = sy.delta;
    guides.push({ axis: "y", pos: sy.pos });
  } else if (grid > 0) {
    dy = Math.round(box.y / grid) * grid - box.y;
  }
  return { dx, dy, guides };
}

/** Magnétisme d'un bord unique (redimensionnement) : renvoie le delta et le guide. */
export function snapEdge(
  edge: number,
  axis: "x" | "y",
  others: Box[],
  canvas: { w: number; h: number },
  threshold: number,
  grid = 0,
): { delta: number; guide: Guide | null } {
  const t = targetLines(canvas, others);
  const s = snapAxis([edge], axis === "x" ? t.xs : t.ys, threshold);
  if (s) return { delta: s.delta, guide: { axis, pos: s.pos } };
  if (grid > 0) return { delta: Math.round(edge / grid) * grid - edge, guide: null };
  return { delta: 0, guide: null };
}
