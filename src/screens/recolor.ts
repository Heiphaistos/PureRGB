// Recolorer un design entier : liste des couleurs distinctes puis remplacement global.
import type { Layout } from "./types.ts";

const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

function walk(node: unknown, fn: (v: string) => string | void): unknown {
  if (typeof node === "string") return HEX.test(node) ? (fn(node) ?? node) : node;
  if (Array.isArray(node)) {
    node.forEach((v, i) => (node[i] = walk(v, fn)));
  } else if (node && typeof node === "object") {
    const o = node as Record<string, unknown>;
    for (const k of Object.keys(o)) o[k] = walk(o[k], fn);
  }
  return node;
}

/** Couleurs distinctes du design (fond + widgets), les plus utilisées d'abord. */
export function usedColors(layout: Layout): string[] {
  const count = new Map<string, number>();
  const seen = (c: string): void => void count.set(c.toLowerCase(), (count.get(c.toLowerCase()) ?? 0) + 1);
  walk(JSON.parse(JSON.stringify({ b: layout.background, w: layout.widgets })), (c) => void seen(c));
  return [...count.entries()].sort((a, b) => b[1] - a[1]).map(([c]) => c);
}

/** Remplace partout `from` par `to` (insensible à la casse). */
export function replaceColor(layout: Layout, from: string, to: string): void {
  const f = from.toLowerCase();
  const sw = (c: string): string | void => (c.toLowerCase() === f ? to : undefined);
  walk(layout.background, sw);
  walk(layout.widgets, sw);
}
