import test from "node:test";
import assert from "node:assert/strict";
import { CURVE_PRESETS, presetPoints } from "../src/fanCurves.ts";

test("préréglages croissants et jamais sous 20 %", () => {
  for (const p of CURVE_PRESETS) {
    assert.ok(p.points.length >= 3, p.id);
    p.points.forEach((q, i) => {
      assert.ok(q.duty >= 20 && q.duty <= 100, `${p.id} duty`);
      if (i) assert.ok(q.temp > p.points[i - 1].temp && q.duty >= p.points[i - 1].duty, `${p.id} monotone`);
    });
  }
});

test("copie indépendante", () => {
  const a = presetPoints("silent");
  a[0].duty = 99;
  assert.equal(presetPoints("silent")[0].duty, 20);
  assert.deepEqual(presetPoints("inconnu"), []);
});
