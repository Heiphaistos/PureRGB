import { test } from "node:test";
import assert from "node:assert/strict";
import { FAN_GROUPS, FAN_PRESETS, ledsFor } from "../src/data/fanPresets.ts";

test("le catalogue vient de la documentation constructeur et n'a aucun nombre absurde", () => {
  const sourced = FAN_PRESETS.filter((p) => p.brand !== "Générique");
  assert.ok(sourced.length >= 60, `trop peu de références : ${sourced.length}`);
  for (const p of sourced) {
    assert.ok(Number.isInteger(p.ledsPerUnit) && p.ledsPerUnit > 0 && p.ledsPerUnit <= 400, `${p.brand} ${p.model}`);
    assert.match(p.source ?? "", /^https:\/\//, `${p.brand} ${p.model} sans source`);
  }
});

test("références connues et calcul pour plusieurs ventilateurs", () => {
  const ql = FAN_PRESETS.find((p) => p.model.includes("QL120"));
  assert.equal(ql?.ledsPerUnit, 34);
  assert.equal(ledsFor(ql!, 3), 102);
  assert.equal(ledsFor(ql!, 0), 0);
  assert.ok(FAN_GROUPS.some((g) => g.brand === "Corsair" && g.items.length > 1));
});
