import { test } from "node:test";
import assert from "node:assert/strict";
import { bannerText } from "../src/gameMode.ts";

test("bandeau du mode jeu : absent hors partie, nomme le jeu, signale le délai de reprise", () => {
  assert.equal(bannerText({ active: false, game: null, grace: false }), null);
  assert.equal(
    bannerText({ active: true, game: "BattlEye", grace: false }),
    "Mode jeu actif : accès matériel suspendu pour BattlEye",
  );
  assert.match(bannerText({ active: true, game: "FACEIT", grace: true }) ?? "", /reprise dans quelques secondes/);
});
