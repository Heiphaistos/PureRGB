import test from "node:test";
import assert from "node:assert/strict";
import { formatValue, groupSensors, levelOf, pushHistory, sparkPoints } from "../src/monitor.ts";

const S = (hardware: string, name: string, type: string, value: number, id = `${hardware}/${name}`): any => ({ id, hardware, name, type, value });

test("groupe par composant puis type dans l'ordre d'affichage", () => {
  const g = groupSensors([S("CPU", "Package", "Temperature", 55), S("CPU", "Total", "Load", 10), S("CPU", "Core", "Clock", 4500), S("GPU", "Core", "Temperature", 60)]);
  assert.deepEqual(g.map((x) => x.hardware), ["CPU", "GPU"]);
  assert.deepEqual(g[0].types.map((t) => t.type), ["Temperature", "Load", "Clock"]);
});

test("filtre sans accents ni casse", () => {
  const g = groupSensors([S("Carte mère", "Système", "Temperature", 40), S("GPU", "Core", "Temperature", 60)], "SYSTEME");
  assert.equal(g.length, 1);
  assert.equal(g[0].hardware, "Carte mère");
  assert.equal(groupSensors([S("GPU", "x", "Fan", 900)], "ventilateurs").length, 1);
});

test("formats et niveaux", () => {
  assert.equal(formatValue({ type: "Temperature", value: 55.04 }), "55.0 °C");
  assert.equal(formatValue({ type: "Voltage", value: 1.1 }), "1.100 V");
  assert.equal(formatValue({ type: "Clock", value: 4523.6 }), "4524 MHz");
  assert.equal(levelOf({ type: "Temperature", value: 85 }), "warn");
  assert.equal(levelOf({ type: "Temperature", value: 95 }), "hot");
  assert.equal(levelOf({ type: "Load", value: 100 }), "ok");
});

test("historique borné et mini-courbe", () => {
  const h: Record<string, number[]> = {};
  for (let i = 0; i < 100; i++) pushHistory(h, [S("a", "b", "Fan", i, "k")], 90);
  assert.equal(h.k.length, 90);
  assert.equal(h.k[0], 10);
  assert.equal(sparkPoints([5]), "");
  assert.equal(sparkPoints([0, 10], 80, 20), "0.0,20.0 80.0,0.0");
  assert.doesNotThrow(() => sparkPoints([3, 3, 3]));
});

test("catégories et jauges", async () => {
  const { categoryOf, groupByCategory, headlineTemp, gaugePercent } = await import("../src/monitor.ts");
  assert.equal(categoryOf({ hardware: "NVIDIA GeForce RTX 4070", type: "Temperature" }), "gpu");
  assert.equal(categoryOf({ hardware: "AMD Ryzen 7 5800X", type: "Load" }), "cpu");
  assert.equal(categoryOf({ hardware: "Samsung SSD 980", type: "Temperature" }), "disk");
  assert.equal(categoryOf({ hardware: "Nuvoton NCT6798D", type: "Fan" }), "fans");
  assert.equal(categoryOf({ hardware: "Generic Memory", type: "Load" }), "memory");
  const all = [S("AMD Ryzen", "Core (Tctl/Tdie)", "Temperature", 60), S("AMD Ryzen", "Core #1", "Temperature", 70), S("Nuvoton", "Fan #1", "Fan", 900)];
  assert.deepEqual(groupByCategory(all).map((g) => g.id), ["cpu", "fans"]);
  assert.equal(headlineTemp(all, "cpu")?.value, 60);
  assert.equal(headlineTemp(all, "gpu"), null);
  assert.equal(gaugePercent({ type: "Fan", value: 900 }), null);
  assert.equal(gaugePercent({ type: "Load", value: 150 }), 100);
});
