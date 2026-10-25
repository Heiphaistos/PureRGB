import { test } from "node:test";
import assert from "node:assert/strict";
import { cleanNote, NOTE_MAX, summaryLabel, usbIdOf } from "../src/communityFeedback.ts";

test("cleanNote retire contrôles, bidi, zéro-largeur et borne à 280 caractères", () => {
  assert.equal(cleanNote("  a\u0000b‮c​d\n\n\te "), "a b c d e");
  assert.equal(Array.from(cleanNote("é".repeat(500))).length, NOTE_MAX);
  assert.equal(cleanNote(" \t\n"), "");
  assert.equal(cleanNote("<b>x</b>"), "<b>x</b>"); // inerte : l'affichage échappe
});

test("usbIdOf extrait VID:PID des identifiants d'appareil", () => {
  assert.deepEqual(usbIdOf("hid:1B1C:0c10:0"), { vid: "1b1c", pid: "0c10" });
  assert.equal(usbIdOf("openrgb:2"), null);
  assert.equal(usbIdOf("hid:zzzz:0c10:0"), null);
});

test("summaryLabel accorde et reste lisible", () => {
  assert.equal(summaryLabel({ vid: "a", pid: "b", works: 1, partial: 0, broken: 0 }), "confirmé fonctionnel par 1 utilisateur");
  assert.match(summaryLabel({ vid: "a", pid: "b", works: 3, partial: 2, broken: 1 }), /3 utilisateurs.*2 signalements.*1 signalement /);
  assert.equal(summaryLabel({ vid: "a", pid: "b", works: 0, partial: 0, broken: 0 }), "aucun retour");
});
