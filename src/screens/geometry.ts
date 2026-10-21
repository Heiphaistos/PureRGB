// Géométrie pure de l'éditeur (coordonnées en pixels du canevas du design).

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Handle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

export const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

/** Direction de déplacement du bord pour chaque poignée (repère local du widget). */
export const HANDLE_DIR: Record<Handle, [number, number]> = {
  nw: [-1, -1],
  n: [0, -1],
  ne: [1, -1],
  e: [1, 0],
  se: [1, 1],
  s: [0, 1],
  sw: [-1, 1],
  w: [-1, 0],
};

export const MIN_SIZE = 4;

export function rotatePoint(x: number, y: number, deg: number): [number, number] {
  const r = (deg * Math.PI) / 180;
  const c = Math.cos(r);
  const s = Math.sin(r);
  return [x * c - y * s, x * s + y * c];
}

export function round1(v: number): number {
  return Math.round(v * 10) / 10;
}

/** Pixel écran (clientX/Y) -> pixel du design. `rect` = boîte écran du canevas. */
export function clientToDesign(
  clientX: number,
  clientY: number,
  rect: { left: number; top: number; width: number },
  designWidth: number,
): [number, number] {
  const scale = rect.width / designWidth;
  return [(clientX - rect.left) / scale, (clientY - rect.top) / scale];
}

export function boxCenter(b: Box): [number, number] {
  return [b.x + b.w / 2, b.y + b.h / 2];
}

/**
 * Redimensionne `start` en tirant la poignée `handle` de (dx, dy) (delta monde).
 * Le bord/coin opposé reste fixe, même si le widget est pivoté.
 */
export function resizeBox(
  start: Box,
  rotation: number,
  handle: Handle,
  dx: number,
  dy: number,
  keepAspect = false,
): Box {
  const [sx, sy] = HANDLE_DIR[handle];
  const [ldx, ldy] = rotatePoint(dx, dy, -rotation);
  let w = sx === 0 ? start.w : Math.max(MIN_SIZE, start.w + sx * ldx);
  let h = sy === 0 ? start.h : Math.max(MIN_SIZE, start.h + sy * ldy);
  if (keepAspect && sx !== 0 && sy !== 0) {
    const k = Math.max(w / start.w, h / start.h);
    w = Math.max(MIN_SIZE, start.w * k);
    h = Math.max(MIN_SIZE, start.h * k);
  }
  const [wx, wy] = rotatePoint((sx * (w - start.w)) / 2, (sy * (h - start.h)) / 2, rotation);
  const cx = start.x + start.w / 2 + wx;
  const cy = start.y + start.h / 2 + wy;
  return { x: cx - w / 2, y: cy - h / 2, w, h };
}

/** Angle (degrés) de la poignée de rotation vue depuis le centre, 0 = poignée en haut. */
export function angleFromCenter(cx: number, cy: number, px: number, py: number): number {
  return (Math.atan2(py - cy, px - cx) * 180) / Math.PI + 90;
}

/** Normalise dans ]-180, 180]. */
export function normAngle(deg: number): number {
  let a = ((deg % 360) + 360) % 360;
  if (a > 180) a -= 360;
  return a;
}

export function snapAngle(deg: number, step: number): number {
  return Math.round(deg / step) * step;
}

export function intersects(a: Box, b: Box): boolean {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

export function normBox(x1: number, y1: number, x2: number, y2: number): Box {
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

export function unionBox(boxes: Box[]): Box | null {
  if (boxes.length === 0) return null;
  const x1 = Math.min(...boxes.map((b) => b.x));
  const y1 = Math.min(...boxes.map((b) => b.y));
  const x2 = Math.max(...boxes.map((b) => b.x + b.w));
  const y2 = Math.max(...boxes.map((b) => b.y + b.h));
  return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

export function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}
