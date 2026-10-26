import { test } from "node:test";
import assert from "node:assert/strict";
import {
  SOURCES, choiceToTarget, curvePath, defaultProfile, response, targetLabel, targetToChoice, withoutIdentity,
} from "../src/gamepadRemap.ts";

test("réponse d'un stick : zone morte, saturation, exposant", () => {
  assert.equal(response(0.1, 0.2, 0, 1), 0);
  assert.equal(response(1, 0.2, 0, 1), 1);
  assert.equal(response(0.9, 0, 0.1, 1), 1);
  assert.ok(Math.abs(response(0.5, 0, 0, 2) - 0.25) < 1e-9);
  assert.match(curvePath(0, 0, 1, 2), /^0\.000,1\.000 0\.500,0\.500 1\.000,0\.000$/);
});

test("choix de cible : aller-retour et libellés", () => {
  for (const c of ["button:x", "left_trigger", "right_trigger", "disabled"]) {
    assert.equal(targetToChoice(choiceToTarget(c), "a"), c);
  }
  assert.equal(targetToChoice(undefined, "b"), "button:b");
  assert.equal(targetLabel("button:left_shoulder"), "LB");
  assert.equal(targetLabel("disabled"), "Désactivé");
  assert.ok(!SOURCES.includes("guide"));
});

test("withoutIdentity retire les boutons inchangés", () => {
  const p = defaultProfile();
  p.buttons = { a: { kind: "button", button: "a" }, b: { kind: "button", button: "x" }, y: { kind: "disabled" } };
  assert.deepEqual(Object.keys(withoutIdentity(p).buttons).sort(), ["b", "y"]);
});
