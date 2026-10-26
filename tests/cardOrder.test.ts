import test from "node:test";
import assert from "node:assert/strict";
import { columnCount, masonry, mergeOrder, moveBefore, moveBy } from "../src/cardOrder.ts";

test("mergeOrder garde l'ordre sauvegardé, retire les disparus, ajoute les nouveaux à la fin", () => {
  assert.deepEqual(mergeOrder(["gpu", "x", "cpu", "gpu"], ["cpu", "gpu", "disk"]), ["gpu", "cpu", "disk"]);
  assert.deepEqual(mergeOrder([], ["a", "b"]), ["a", "b"]);
});

test("moveBefore descend et remonte", () => {
  assert.deepEqual(moveBefore(["a", "b", "c", "d"], "a", "c"), ["b", "c", "a", "d"]);
  assert.deepEqual(moveBefore(["a", "b", "c", "d"], "d", "b"), ["a", "d", "b", "c"]);
  assert.deepEqual(moveBefore(["a", "b"], "a", "zzz"), ["a", "b"]);
});

test("moveBy borne aux extrémités", () => {
  assert.deepEqual(moveBy(["a", "b", "c"], "a", -1), ["a", "b", "c"]);
  assert.deepEqual(moveBy(["a", "b", "c"], "b", 1), ["a", "c", "b"]);
  assert.deepEqual(moveBy(["a", "b", "c"], "c", -1), ["a", "c", "b"]);
});

test("masonry comble la colonne la plus courte en respectant l'ordre", () => {
  const w: Record<string, number> = { cpu: 10, gpu: 6, board: 8, disk: 4, power: 2 };
  const cols = masonry(["cpu", "gpu", "board", "disk", "power"], (id) => w[id], 2);
  // cpu -> c0(10), gpu -> c1(6), board -> c1(14), disk -> c0(14), power -> c0 (egalite, gauche)
  assert.deepEqual(cols, [["cpu", "disk", "power"], ["gpu", "board"]]);
  assert.deepEqual(masonry(["a"], () => 1, 0), [["a"]]);
});

test("columnCount par largeur", () => {
  assert.deepEqual([700, 1000, 1280, 1600, 1920].map(columnCount), [1, 2, 3, 4, 4]);
});
