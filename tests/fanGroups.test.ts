import test from "node:test";
import assert from "node:assert/strict";
import { fanCategoryOf, groupFanDevices } from "../src/fanGroups.ts";

test("catégories de ventilation", () => {
  assert.equal(fanCategoryOf({ backend: "mobo", device_type: "Fan" }), "bios");
  assert.equal(fanCategoryOf({ backend: "hid", device_type: "Aio" }), "aio");
  assert.equal(fanCategoryOf({ backend: "hid", device_type: "Hub" }), "hub");
  assert.equal(fanCategoryOf({ backend: "hid", device_type: "Case" }), "case");
  const g = groupFanDevices([{ backend: "hid", device_type: "Aio" }, { backend: "mobo", device_type: "Fan" }] as any);
  assert.deepEqual(g.map((x) => x.id), ["bios", "aio"]);
});
