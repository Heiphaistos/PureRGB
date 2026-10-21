// Blocs matériel : cartes CPU, GPU, RAM et température carte mère.
import { DIM, arc, bar, card, txt, val, type BlockDef, type Proto } from "./kit.ts";

/** Carte standard 200×200 : titre, jauge de charge, température, ligne d'info, barre. */
function hwCard(title: string, load: string, temp: string, info: Proto, accent: string): Proto[] {
  return [
    card(0, 0, 200, 200),
    txt(10, 8, 180, 22, title, 15, DIM),
    arc(40, 32, 120, load, accent, 10),
    val(50, 70, 100, 40, load, "{:.0}%", 34),
    val(50, 108, 100, 22, temp, "{:.0}°C", 18, accent),
    info,
    bar(14, 184, 172, 8, temp, accent, 20, 100),
  ];
}

export const HARDWARE_BLOCKS: BlockDef[] = [
  {
    id: "card-cpu",
    label: "Carte CPU",
    category: "Matériel",
    w: 200,
    h: 200,
    build: ({ accent }) => hwCard("CPU", "cpu.load", "cpu.temp", val(10, 158, 180, 20, "cpu.clock", "{:.0} MHz", 13, DIM), accent),
  },
  {
    id: "card-gpu",
    label: "Carte GPU",
    category: "Matériel",
    w: 200,
    h: 200,
    build: ({ accent }) => hwCard("GPU", "gpu.load", "gpu.temp", val(10, 158, 180, 20, "gpu.vram_used_gb", "VRAM {:.1} Go", 13, DIM), accent),
  },
  {
    id: "card-ram",
    label: "Carte RAM",
    category: "Matériel",
    w: 200,
    h: 200,
    build: ({ accent }) => [
      card(0, 0, 200, 200),
      txt(10, 8, 180, 22, "RAM", 15, DIM),
      arc(40, 32, 120, "ram.used_pct", accent, 10),
      val(50, 70, 100, 40, "ram.used_pct", "{:.0}%", 34),
      val(50, 108, 100, 22, "ram.used_gb", "{:.1} Go", 18, accent),
      val(10, 158, 180, 20, "ram.free_gb", "{:.1} Go libres", 13, DIM),
      bar(14, 184, 172, 8, "ram.used_pct", accent),
    ],
  },
  {
    id: "mobo-temp",
    label: "Température carte mère",
    category: "Matériel",
    w: 220,
    h: 110,
    build: ({ accent }) => [
      card(0, 0, 220, 110),
      txt(10, 8, 200, 20, "CARTE MÈRE", 14, DIM),
      val(10, 28, 200, 44, "mobo.temp", "{:.0}°C", 38, accent),
      bar(14, 78, 192, 8, "mobo.temp", accent, 20, 90),
      val(10, 88, 200, 18, "chipset.temp", "Chipset {:.0}°C", 12, DIM),
    ],
  },
];
