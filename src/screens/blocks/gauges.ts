// Blocs génériques : jauge arc, anneaux concentriques, barre de charge, courbe d'historique.
import { DIM, arc, bar, graph, txt, val, type BlockDef } from "./kit.ts";

export const GAUGE_BLOCKS: BlockDef[] = [
  {
    id: "gauge-arc",
    label: "Jauge arc",
    category: "Jauges",
    w: 180,
    h: 180,
    build: ({ accent }) => [arc(0, 0, 180, "cpu.load", accent, 14), val(30, 62, 120, 48, "cpu.load", "{:.0}%", 40), txt(30, 108, 120, 20, "CPU", 14, DIM)],
  },
  {
    id: "rings",
    label: "Anneaux concentriques",
    category: "Jauges",
    w: 200,
    h: 200,
    build: ({ accent }) => [
      arc(0, 0, 200, "cpu.load", accent, 12),
      arc(20, 20, 160, "gpu.load", "#4aa8ff", 12),
      arc(40, 40, 120, "ram.used_pct", "#b86bff", 12),
      txt(60, 78, 80, 18, "CPU · GPU · RAM", 9, DIM),
      val(60, 94, 80, 28, "cpu.load", "{:.0}%", 22),
    ],
  },
  {
    id: "load-bar",
    label: "Barre de charge",
    category: "Jauges",
    w: 260,
    h: 40,
    build: ({ accent }) => [
      txt(0, 0, 60, 18, "CPU", 13, DIM, { align: "left" }),
      val(190, 0, 70, 18, "cpu.load", "{:.0}%", 13, "#ffffff", { align: "right" }),
      bar(0, 22, 260, 14, "cpu.load", accent),
    ],
  },
  {
    id: "history",
    label: "Courbe d'historique",
    category: "Jauges",
    w: 260,
    h: 120,
    build: ({ accent }) => [graph(0, 0, 260, 120, "cpu.temp", accent, 20, 100, 120)],
  },
];
