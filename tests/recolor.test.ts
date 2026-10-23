import test from "node:test";
import assert from "node:assert/strict";
import { replaceColor, usedColors } from "../src/screens/recolor.ts";

const mk = (): any => ({
  background: { kind: "gradient", from: "#07030f", to: "#FF2BD6", angle: 90 },
  widgets: [
    { type: "rect", fill: "#ff2bd6", stroke: null },
    { type: "bar", track: "#271a4d", colors: [[0, "#39ff88"], [1, "#ff2bd6"]] },
    { type: "text", text: "#notacolor", style: { color: "#ff2bd6" } },
  ],
});

test("usedColors trie par fréquence et ignore le texte", () => {
  const l = mk();
  const c = usedColors(l);
  assert.equal(c[0], "#ff2bd6");
  assert.ok(!c.includes("#notacolor"));
});

test("replaceColor change fond, widgets et rampes, pas le texte", () => {
  const l = mk();
  replaceColor(l, "#ff2bd6", "#00aaff");
  assert.equal(l.background.to, "#00aaff");
  assert.equal(l.widgets[0].fill, "#00aaff");
  assert.equal(l.widgets[1].colors[1][1], "#00aaff");
  assert.equal(l.widgets[2].style.color, "#00aaff");
  assert.equal(l.widgets[2].text, "#notacolor");
  assert.equal(l.widgets[1].colors[0][1], "#39ff88");
});
