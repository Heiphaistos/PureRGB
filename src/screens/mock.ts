// Backend factice (mode démo) : mêmes signatures que les commandes Tauri, état en mémoire.
import { MOCK_DESIGNS } from "./mockDesigns.ts";
import { mockImages, mockValues, renderMock } from "./mockRender.ts";
import { mockSensors } from "./mockSensors.ts";
import type { DesignSummary, Layout, LiveValues, MediaItem, ScreenInfo, ScreenOptions, ScreenState, SensorsReport } from "./types.ts";
import { cloneLayout } from "./widgets.ts";
import { validateLayout } from "./validate.ts";

const SIM: ScreenInfo = {
  id: "sim:0000:demo",
  vendor: "PureRGB",
  model: "Écran simulé",
  protocol: "simulated",
  width: 480,
  height: 480,
  rotation: 0,
  resolution_confirmed: true,
  verified: true,
};

const TR: ScreenInfo = {
  id: "87cd:70db:demo",
  vendor: "Thermalright",
  model: "Frozen Warframe 240 (LCD 320×240)",
  protocol: "thermalright_scsi",
  width: 320,
  height: 240,
  rotation: 0,
  resolution_confirmed: false,
  verified: false,
};

const params = new URLSearchParams(typeof location === "undefined" ? "" : location.search);
let screens: ScreenInfo[] = params.get("mock") === "empty" ? [] : [SIM, TR];
const states = new Map<string, ScreenState>();
const userDesigns = new Map<string, Layout>();
// Média vidéo factice : le moteur réel lit les vidéos via Windows, la démo n'affiche qu'un espace réservé.
let media: MediaItem[] = [
  { id: "demo-video", name: "Vidéo de démonstration", kind: "video", width: 1920, height: 1080, frames: 0, duration_ms: 12000 },
];
let seq = 1;

function stateOf(id: string): ScreenState {
  let s = states.get(id);
  if (!s) {
    const first = MOCK_DESIGNS.values().next().value;
    s = {
      layout: first ? cloneLayout(first.layout) : null,
      brightness: 80,
      rotation: 0,
      fps: 10,
      running: true,
      live: true,
      error: null,
      perf: { fps: 0, target_fps: 5, render_ms: 0, encode_ms: 0, send_ms: 0, dropped: 0, quality: 0 },
    };
    states.set(id, s);
  }
  return s;
}

/** `?mock=many` : 300 designs factices pour éprouver la galerie (filtres, repli, pagination). */
function manyDesigns(): DesignSummary[] {
  const sizes: [number, number][] = [[480, 480], [320, 240], [1280, 480], [240, 320]];
  return Array.from({ length: 300 }, (_, i) => {
    const [width, height] = sizes[i % sizes.length];
    return { id: `many-${i}`, name: `Design ${i + 1}`, category: ["gaming", "bureau", "minimal", "horloge", "monitoring"][i % 5], width, height, tags: [`série ${i % 7}`] };
  });
}

function manyLayout(id: string): Layout {
  const d = manyDesigns().find((m) => m.id === id);
  const base = cloneLayout(MOCK_DESIGNS.values().next().value?.layout as Layout);
  return { ...base, name: d?.name ?? id, width: d?.width ?? base.width, height: d?.height ?? base.height };
}

const tick = (): Promise<void> => new Promise((r) => setTimeout(r, 30));

export const mock = {
  async screensList(): Promise<ScreenInfo[]> {
    await tick();
    return screens;
  },
  async screensScan(withSimulated: boolean): Promise<ScreenInfo[]> {
    await tick();
    if (withSimulated && !screens.some((s) => s.id === SIM.id)) screens = [SIM, ...screens];
    return screens;
  },
  async screenState(id: string): Promise<ScreenState> {
    await tick();
    return structuredClone(stateOf(id));
  },
  async setLayout(id: string, layout: Layout): Promise<void> {
    const err = validateLayout(layout);
    if (err) throw new Error(err);
    const s = stateOf(id);
    s.layout = cloneLayout(layout);
    s.running = true;
  },
  async setOptions(id: string, o: ScreenOptions): Promise<void> {
    Object.assign(stateOf(id), o);
  },
  async clear(id: string): Promise<void> {
    const s = stateOf(id);
    s.layout = null;
    s.running = false;
  },
  async preview(layout: Layout, width: number, height: number, tMs: number): Promise<string> {
    await tick();
    return renderMock(layout, width, height, tMs);
  },
  async designsList(): Promise<DesignSummary[]> {
    const out: DesignSummary[] = [];
    for (const [id, d] of MOCK_DESIGNS) {
      out.push({ id, name: d.layout.name, category: d.category, width: d.layout.width, height: d.layout.height, tags: d.tags });
    }
    if (params.get("mock") === "many") out.push(...manyDesigns());
    for (const [id, l] of userDesigns) {
      out.push({ id, name: l.name, category: "user", width: l.width, height: l.height, tags: ["user"] });
    }
    return out;
  },
  async designGet(id: string): Promise<Layout> {
    const l = MOCK_DESIGNS.get(id)?.layout ?? userDesigns.get(id) ?? (id.startsWith("many-") ? manyLayout(id) : undefined);
    if (!l) throw new Error("design introuvable");
    return cloneLayout(l);
  },
  async designUserSave(layout: Layout): Promise<string> {
    const err = validateLayout(layout);
    if (err) throw new Error(err);
    const id = `user-${seq++}`;
    userDesigns.set(id, cloneLayout(layout));
    return id;
  },
  async designUserDelete(id: string): Promise<void> {
    userDesigns.delete(id);
  },
  async designExport(id: string, _path: string): Promise<void> {
    const l = await mock.designGet(id);
    const url = URL.createObjectURL(new Blob([JSON.stringify(l, null, 2)], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `${l.name}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  },
  /** Démo : pas d'archive zip, on télécharge le design seul (les médias de démo ne sont pas sur disque). */
  async themeExport(layout: Layout): Promise<void> {
    const url = URL.createObjectURL(new Blob([JSON.stringify(layout, null, 2)], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `${layout.name}.purergb-theme.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  },
  async mediaList(): Promise<MediaItem[]> {
    return media;
  },
  /** Démo : importe un File du navigateur. */
  async mediaImportFile(file: File): Promise<MediaItem> {
    const url = URL.createObjectURL(file);
    if (file.type.startsWith("video/")) {
      const v = document.createElement("video");
      await new Promise<void>((res, rej) => {
        v.onloadedmetadata = () => res();
        v.onerror = () => rej(new Error("vidéo illisible par le navigateur"));
        v.src = url;
      });
      const item: MediaItem = {
        id: `media-${seq++}`,
        name: file.name,
        kind: "video",
        width: v.videoWidth,
        height: v.videoHeight,
        frames: 0,
        duration_ms: Math.round((Number.isFinite(v.duration) ? v.duration : 0) * 1000),
      };
      media = [...media, item];
      return item;
    }
    const img = new Image();
    await new Promise<void>((res, rej) => {
      img.onload = () => res();
      img.onerror = () => rej(new Error("image illisible"));
      img.src = url;
    });
    const id = `media-${seq++}`;
    mockImages.set(id, img);
    const item: MediaItem = {
      id,
      name: file.name,
      kind: file.type === "image/gif" ? "gif" : "image",
      width: img.naturalWidth,
      height: img.naturalHeight,
      frames: 1,
      duration_ms: 0,
    };
    media = [...media, item];
    return item;
  },
  async mediaDelete(id: string): Promise<void> {
    media = media.filter((m) => m.id !== id);
    mockImages.delete(id);
  },
  async screenValues(): Promise<LiveValues> {
    return mockValues(Date.now());
  },
  async screenSensors(): Promise<SensorsReport> {
    return mockSensors(Date.now());
  },
};
