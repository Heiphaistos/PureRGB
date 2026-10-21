import { test } from "node:test";
import assert from "node:assert/strict";
import { angleFromCenter, clientToDesign, normAngle, resizeBox, unionBox, intersects } from "../src/screens/geometry.ts";
import { snapAxis, snapMove, snapEdge } from "../src/screens/snap.ts";
import { History } from "../src/screens/history.ts";
import { validateLayout } from "../src/screens/validate.ts";
import { addWidget, align, duplicateWidgets, reorder, removeWidgets } from "../src/screens/layoutOps.ts";
import { blankLayout, makeWidget, WIDGET_TYPES } from "../src/screens/widgets.ts";

const near = (a: number, b: number, eps = 1e-6) => assert.ok(Math.abs(a - b) < eps, `${a} != ${b}`);

test("clientToDesign tient compte de l'échelle", () => {
  const [x, y] = clientToDesign(150, 60, { left: 50, top: 10, width: 240 }, 480);
  assert.deepEqual([x, y], [200, 100]);
});

test("resizeBox sans rotation garde le coin opposé fixe", () => {
  const b = resizeBox({ x: 10, y: 10, w: 100, h: 50 }, 0, "se", 20, 10);
  assert.deepEqual(b, { x: 10, y: 10, w: 120, h: 60 });
  const c = resizeBox({ x: 10, y: 10, w: 100, h: 50 }, 0, "nw", 5, 5);
  assert.deepEqual(c, { x: 15, y: 15, w: 95, h: 45 });
});

test("resizeBox pivoté : le coin opposé ne bouge pas dans le monde", () => {
  const start = { x: 100, y: 100, w: 80, h: 40 };
  const rot = 30;
  const corner = (b: typeof start, sx: number, sy: number): [number, number] => {
    const cx = b.x + b.w / 2;
    const cy = b.y + b.h / 2;
    const r = (rot * Math.PI) / 180;
    const lx = (sx * b.w) / 2;
    const ly = (sy * b.h) / 2;
    return [cx + lx * Math.cos(r) - ly * Math.sin(r), cy + lx * Math.sin(r) + ly * Math.cos(r)];
  };
  const before = corner(start, -1, -1);
  const after = resizeBox(start, rot, "se", 17, -9);
  const c2 = corner(after, -1, -1);
  near(before[0], c2[0]);
  near(before[1], c2[1]);
});

test("resizeBox respecte la taille minimale et le ratio", () => {
  assert.equal(resizeBox({ x: 0, y: 0, w: 20, h: 20 }, 0, "e", -100, 0).w, 4);
  const r = resizeBox({ x: 0, y: 0, w: 100, h: 50 }, 0, "se", 100, 0, true);
  near(r.w / r.h, 2);
});

test("angles", () => {
  near(angleFromCenter(0, 0, 0, -10), 0);
  near(angleFromCenter(0, 0, 10, 0), 90);
  assert.equal(normAngle(190), -170);
  assert.equal(normAngle(-190), 170);
  assert.equal(normAngle(360), 0);
});

test("boîtes", () => {
  assert.deepEqual(unionBox([{ x: 0, y: 0, w: 10, h: 10 }, { x: 20, y: 5, w: 5, h: 20 }]), { x: 0, y: 0, w: 25, h: 25 });
  assert.equal(unionBox([]), null);
  assert.ok(intersects({ x: 0, y: 0, w: 10, h: 10 }, { x: 9, y: 9, w: 5, h: 5 }));
  assert.ok(!intersects({ x: 0, y: 0, w: 10, h: 10 }, { x: 10, y: 0, w: 5, h: 5 }));
});

test("snapAxis choisit le plus proche dans le seuil", () => {
  assert.deepEqual(snapAxis([100], [96, 103, 130], 5), { delta: 3, pos: 103 });
  assert.equal(snapAxis([100], [120], 5), null);
});

test("snapMove aimante au centre du canevas et ajoute un guide", () => {
  const r = snapMove({ x: 197, y: 50, w: 100, h: 20 }, [], { w: 480, h: 480 }, 6);
  // centre x = 247, cible 240 -> delta -7 hors seuil ; bord droit 297 : rien ; bord gauche 197 : rien
  assert.equal(r.dx, 0);
  const r2 = snapMove({ x: 192, y: 50, w: 100, h: 20 }, [], { w: 480, h: 480 }, 6);
  assert.equal(r2.dx, -2);
  assert.deepEqual(r2.guides[0], { axis: "x", pos: 240 });
});

test("snapMove aimante sur un autre widget et sur la grille", () => {
  const r = snapMove({ x: 52, y: 200, w: 40, h: 40 }, [{ x: 50, y: 0, w: 10, h: 10 }], { w: 480, h: 480 }, 6);
  assert.equal(r.dx, -2);
  const g = snapMove({ x: 13, y: 27, w: 7, h: 7 }, [], { w: 480, h: 480 }, 0, 10);
  assert.deepEqual([g.dx, g.dy], [-3, 3]);
});

test("snapEdge", () => {
  const s = snapEdge(238, "x", [], { w: 480, h: 480 }, 6);
  assert.equal(s.delta, 2);
  assert.equal(s.guide?.pos, 240);
});

test("History : undo/redo, coalescence, troncature, limite", () => {
  const h = new History<{ n: number }>({ n: 0 });
  assert.equal(h.push({ n: 1 }, null, 0), true);
  assert.equal(h.push({ n: 1 }, null, 1), false);
  h.push({ n: 2 }, "k", 100);
  h.push({ n: 3 }, "k", 200); // fusionné avec n:2
  assert.deepEqual(h.undo(), { n: 1 });
  assert.deepEqual(h.redo(), { n: 3 });
  assert.equal(h.redo(), null);
  h.undo();
  h.push({ n: 9 }, null, 5000); // tronque le futur
  assert.equal(h.canRedo, false);
  const small = new History<number>(0, 3);
  [1, 2, 3, 4].forEach((v) => small.push(v));
  assert.equal(small.undo(), 3);
  assert.equal(small.undo(), 2);
  assert.equal(small.undo(), null);
  small.reset(7);
  assert.equal(small.canUndo, false);
});

test("validateLayout", () => {
  const ok = blankLayout(480, 480);
  assert.equal(validateLayout(ok), null);
  assert.match(validateLayout({ ...ok, version: 2 }) ?? "", /version/);
  assert.match(validateLayout({ ...ok, width: 0 }) ?? "", /canevas/);
  assert.match(validateLayout({ ...ok, width: 5000 }) ?? "", /canevas/);
  assert.match(validateLayout(null) ?? "", /invalide/);
  const l = blankLayout(480, 480);
  l.widgets = WIDGET_TYPES.map((t) => makeWidget(t, l));
  assert.equal(validateLayout(l), null);
  const dup = { ...l, widgets: [l.widgets[0], l.widgets[0]] };
  assert.match(validateLayout(dup) ?? "", /double/);
  const bad = { ...l, widgets: [{ ...l.widgets[0], type: "nope" }] };
  assert.match(validateLayout(bad) ?? "", /inconnu/);
  const nan = { ...l, widgets: [{ ...l.widgets[0], x: NaN }] };
  assert.match(validateLayout(nan) ?? "", /« x »/);
  const many = { ...l, widgets: Array.from({ length: 201 }, (_, i) => ({ ...l.widgets[0], id: `a${i}` })) };
  assert.match(validateLayout(many) ?? "", /trop/);
});

test("makeWidget : widgets centrés dans le canevas, ids uniques", () => {
  const l = blankLayout(480, 320);
  const ws = WIDGET_TYPES.map((t) => makeWidget(t, l));
  assert.equal(new Set(ws.map((w) => w.id)).size, ws.length);
  for (const w of ws) near(w.x + w.w / 2, 240, 1);
});

test("layoutOps : ordre Z, copie, alignement, suppression", () => {
  const l = blankLayout(100, 100);
  for (const id of ["a", "b", "c", "d"]) {
    const w = makeWidget("rect", l);
    w.id = id;
    w.w = 10;
    w.h = 10;
    w.x = id.charCodeAt(0) - 90;
    addWidget(l, w);
  }
  const ids = (): string => l.widgets.map((w) => w.id).join("");
  reorder(l, new Set(["a"]), "front");
  assert.equal(ids(), "bcda");
  reorder(l, new Set(["a"]), "backward");
  assert.equal(ids(), "bcad");
  reorder(l, new Set(["b", "c"]), "forward");
  assert.equal(ids(), "abcd");
  reorder(l, new Set(["d"]), "back");
  assert.equal(ids()[0], "d");
  const copies = duplicateWidgets(l, new Set(["a"]));
  assert.equal(copies.length, 1);
  assert.equal(l.widgets.length, 5);
  align(l, new Set(["a", "b"]), "left");
  assert.equal(l.widgets.find((w) => w.id === "a")?.x, l.widgets.find((w) => w.id === "b")?.x);
  align(l, new Set(["a"]), "hcenter");
  assert.equal(l.widgets.find((w) => w.id === "a")?.x, 45);
  removeWidgets(l, new Set(["a", "b"]));
  assert.equal(l.widgets.length, 3);
});
