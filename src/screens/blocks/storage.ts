// Blocs stockage : disque par lettre (C:, D:…) et SSD par numéro.
import { DIM, bar, card, txt, val, type BlockDef } from "./kit.ts";

export const STORAGE_BLOCKS: BlockDef[] = [
  {
    id: "disk-letter",
    label: "Carte disque (lettre)",
    category: "Stockage",
    w: 220,
    h: 110,
    param: { label: "Lettre", options: ["C", "D", "E", "F", "G", "H"], default: "C" },
    build: ({ accent, param }) => [
      card(0, 0, 220, 110),
      txt(10, 8, 200, 20, `DISQUE ${param}:`, 14, DIM),
      val(10, 26, 200, 44, `disk.${param}.used_pct`, "{:.0}%", 38, accent),
      bar(14, 74, 192, 8, `disk.${param}.used_pct`, accent),
      val(10, 86, 200, 20, `disk.${param}.free_gb`, "{:.0} Go libres", 13, DIM),
    ],
  },
  {
    id: "ssd-n",
    label: "Carte SSD (n°)",
    category: "Stockage",
    w: 220,
    h: 110,
    param: { label: "SSD n°", options: ["1", "2", "3", "4"], default: "1" },
    build: ({ accent, param }) => [
      card(0, 0, 220, 110),
      txt(10, 8, 200, 20, `SSD ${param}`, 14, DIM),
      val(10, 26, 100, 44, `ssd.${param}.temp`, "{:.0}°C", 34, accent),
      val(110, 34, 100, 30, `ssd.${param}.used_pct`, "{:.0}% plein", 16),
      bar(14, 78, 192, 8, `ssd.${param}.used_pct`, accent),
      val(10, 88, 200, 18, "disk.read", "Lecture {:.1} Mo/s", 12, DIM),
    ],
  },
];
