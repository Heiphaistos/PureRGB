// Miniatures rendues par le moteur : cache + file séquentielle (évite 40 rendus simultanés).
import { designGet, preview } from "./api.ts";
import type { Layout } from "./types.ts";
import { blankLayout } from "./widgets.ts";

const cache = new Map<string, string>();
let chain: Promise<unknown> = Promise.resolve();

function queued(key: string, build: () => Promise<string>): Promise<string> {
  const hit = cache.get(key);
  if (hit !== undefined) return Promise.resolve(hit);
  const p = chain.then(async () => {
    const again = cache.get(key);
    if (again !== undefined) return again;
    const png = await build();
    const url = png ? `data:image/png;base64,${png}` : "";
    if (url) cache.set(key, url);
    return url;
  });
  chain = p.catch(() => undefined);
  return p;
}

const THUMB_W = 240;

function thumbSize(w: number, h: number): { width: number; height: number } {
  const k = THUMB_W / Math.max(w, h);
  return { width: Math.max(1, Math.round(w * k)), height: Math.max(1, Math.round(h * k)) };
}

export function designThumb(id: string, w: number, h: number): Promise<string> {
  return queued(`design:${id}`, async () => {
    const layout = await designGet(id);
    const s = thumbSize(w, h);
    return preview(layout, s.width, s.height, 0);
  });
}

export function mediaThumb(mediaId: string, w: number, h: number): Promise<string> {
  return queued(`media:${mediaId}`, () => {
    const l: Layout = blankLayout(Math.max(w, 1), Math.max(h, 1), "miniature");
    l.background = { kind: "solid", color: "#0d0d10" };
    l.widgets.push({
      id: "t",
      x: 0,
      y: 0,
      w: l.width,
      h: l.height,
      rotation: 0,
      opacity: 1,
      type: "image",
      media_id: mediaId,
      fit: "contain",
    });
    const s = thumbSize(w, h);
    return preview(l, s.width, s.height, 0);
  });
}

export function forgetThumb(key: string): void {
  cache.delete(key);
}
