import { test } from "node:test";
import assert from "node:assert/strict";
import { BLOCKS, defaultParam, instantiateBlock } from "../src/screens/blocks/index.ts";
import { buildSections, matchesQuery, ratioLabel, ratioOptions } from "../src/screens/galleryFilter.ts";
import { SOURCES } from "../src/screens/widgets.ts";
import { blankLayout } from "../src/screens/widgets.ts";
import { validateLayout } from "../src/screens/validate.ts";
import type { DesignSummary } from "../src/screens/types.ts";

const CANVASES: [number, number][] = [[480, 480], [320, 240], [1280, 480], [240, 320]];

test("chaque bloc donne des widgets valides, uniques et dans le canevas", () => {
  const ids = new Set<string>();
  for (const [w, h] of CANVASES) {
    for (const b of BLOCKS) {
      const layout = blankLayout(w, h);
      const ctx = { accent: "#ff5000", param: defaultParam(b) };
      layout.widgets = instantiateBlock(b, ctx, layout);
      assert.ok(layout.widgets.length > 0, b.id);
      assert.equal(validateLayout(layout), null, `${b.id} @${w}x${h}`);
      for (const x of layout.widgets) {
        assert.ok(!ids.has(x.id), `id en double ${x.id}`);
        ids.add(x.id);
        assert.ok(x.x >= -1 && x.y >= -1 && x.x + x.w <= w + 1 && x.y + x.h <= h + 1, `${b.id} déborde de ${w}x${h}`);
      }
    }
  }
});

test("les paramètres (lettre, n°) se retrouvent dans les clés de données", () => {
  const disk = BLOCKS.find((b) => b.id === "disk-letter");
  const ssd = BLOCKS.find((b) => b.id === "ssd-n");
  assert.ok(disk && ssd);
  const keys = (b: typeof disk, param: string) => b.build({ accent: "#fff", param }).flatMap((p) => ("source" in p ? [p.source] : []));
  assert.ok(keys(disk, "D").includes("disk.D.used_pct") && keys(disk, "D").includes("disk.D.free_gb"));
  assert.ok(keys(ssd, "2").includes("ssd.2.temp"));
});

test("les sources des blocs sont des clés stables connues", () => {
  const known = /^(cpu|gpu|ram|mobo|chipset|vrm)\.[a-z_]+$|^(ssd|fan)\.\d\.[a-z_]+$|^disk\.([A-Z]\.[a-z_]+|read|write)$|^net\.(up|down)$|^(liquid\.temp|pump\.rpm|uptime|time|date)$/;
  for (const b of BLOCKS) {
    for (const p of b.build({ accent: "#fff", param: defaultParam(b) })) {
      if ("source" in p) assert.match(p.source, known, `${b.id}: ${p.source}`);
    }
  }
  assert.ok(SOURCES.some((s) => s.key === "mobo.temp"));
});

const D = (id: string, name: string, category: string, width: number, height: number, tags: string[] = []): DesignSummary => ({ id, name, category, width, height, tags });

test("ratioLabel reconnaît les formats usuels", () => {
  assert.equal(ratioLabel(480, 480), "1:1");
  assert.equal(ratioLabel(320, 240), "4:3");
  assert.equal(ratioLabel(1920, 1080), "16:9");
});

test("recherche plein texte : mots multiples, sans accents ni casse, tags et format", () => {
  const d = D("a", "Thème Glacé", "gaming", 480, 480, ["froid", "bleu"]);
  const q = (text: string, ratio = "all") => matchesQuery(d, { text, ratio });
  assert.ok(q("theme glace"));
  assert.ok(q("GAMING bleu"));
  assert.ok(q("480x480"));
  assert.ok(!q("chaud"));
  assert.ok(q("", "1:1") && !q("", "16:9"));
});

test("sections : mes designs d'abord, puis celles au format de l'écran, tri même ratio d'abord", () => {
  const list = [
    D("1", "B", "gaming", 320, 240),
    D("2", "A", "gaming", 480, 480),
    D("3", "Z", "minimal", 480, 480),
    D("4", "M", "user", 240, 320, ["user"]),
  ];
  const s = buildSections(list, { width: 480, height: 480 });
  assert.deepEqual(s.map((x) => x.key), ["user", "gaming", "minimal"]);
  assert.deepEqual(s[1].items.map((x) => x.id), ["2", "1"]);
  assert.deepEqual(ratioOptions(list, { width: 480, height: 480 })[0], { label: "1:1", n: 2 });
});
