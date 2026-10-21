import { test } from "node:test";
import assert from "node:assert/strict";
import { dynamicSources, sensorUnit } from "../src/screens/sourceCatalog.ts";
import { SOURCES } from "../src/screens/widgets.ts";

test("les clés du catalogue sont uniques et les groupes attendus", () => {
  const keys = SOURCES.map((s) => s.key);
  assert.equal(new Set(keys).size, keys.length);
  const groups = new Set(SOURCES.map((s) => s.group));
  for (const g of ["Processeur", "Carte graphique", "Mémoire", "Carte mère", "Disques", "Réseau", "Refroidissement", "Heure"]) {
    assert.ok(groups.has(g), g);
  }
  for (const k of ["cpu.temp", "gpu.hotspot", "gpu.vram_used_gb", "ram.free_gb", "mobo.temp", "chipset.temp", "vrm.temp", "disk.read", "disk.write", "cpu.voltage"]) {
    assert.ok(keys.includes(k), k);
  }
});

test("sources dynamiques : disques par lettre, SSD et ventilateurs numérotés", () => {
  const v = { "disk.C.used_pct": 1, "disk.D.free_gb": 2, "ssd.2.temp": 3, "mobo.fan.2": 4, "fan.3.rpm": 5, "sensor:/x": 6, "cpu.temp": 7 };
  const d = dynamicSources(v);
  assert.deepEqual(
    d.map((s) => s.key),
    ["disk.C.used_pct", "disk.D.free_gb", "fan.3.rpm", "mobo.fan.2", "ssd.2.temp"],
  );
  assert.equal(d.find((s) => s.key === "disk.D.free_gb")?.group, "Disques");
  assert.equal(sensorUnit("SmallData"), "Mo");
  assert.equal(sensorUnit("Inconnu"), "");
});
