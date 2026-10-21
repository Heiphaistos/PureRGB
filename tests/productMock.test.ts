import { test } from "node:test";
import assert from "node:assert/strict";
import { mockLookup, mockPhoto } from "../src/productMock.ts";

test("le mock ne reconnaît que ses produits et ne fabrique pas de photo d'un autre modèle", () => {
  assert.equal(mockLookup("Corsair", "H150i"), null);
  assert.equal(mockLookup("", ""), null);
  const p = mockLookup("Thermalright", "Trofeo Vision");
  assert.ok(p?.has_image);
  assert.match(mockPhoto(p.id) ?? "", /^data:image\/svg\+xml/);
  assert.equal(mockPhoto("inconnu"), null);
});
