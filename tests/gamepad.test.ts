import test from "node:test";
import assert from "node:assert/strict";
import { buttonName, drift, stick } from "../src/gamepad.ts";

test("zone morte circulaire", () => {
  assert.equal(stick(0.05, 0.05).inDeadzone, true);
  assert.equal(stick(0.2, 0).inDeadzone, false);
  assert.equal(stick(1, 1).magnitude, 1);
});

test("dérive", () => {
  assert.equal(drift([[0, 0.01], [0.02, 0]]).drifting, false);
  assert.equal(drift([[0.08, 0.02]]).drifting, true);
  assert.equal(drift([]).max, 0);
});

test("noms de boutons", () => {
  assert.equal(buttonName(0), "A / Croix");
  assert.equal(buttonName(40), "Bouton 40");
});
