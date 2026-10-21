// Blocs refroidissement : AIO (liquide + pompe) et ventilateurs numérotés.
import { DIM, arc, bar, card, txt, val, type BlockDef } from "./kit.ts";

export const COOLING_BLOCKS: BlockDef[] = [
  {
    id: "aio",
    label: "AIO (liquide, pompe)",
    category: "Refroidissement",
    w: 220,
    h: 130,
    build: ({ accent }) => [
      card(0, 0, 220, 130),
      txt(10, 8, 200, 20, "REFROIDISSEMENT", 14, DIM),
      val(10, 28, 100, 44, "liquid.temp", "{:.0}°C", 36, accent),
      txt(10, 70, 100, 16, "LIQUIDE", 11, DIM),
      val(112, 36, 100, 28, "pump.rpm", "{:.0} tr/min", 17),
      txt(112, 62, 100, 16, "POMPE", 11, DIM),
      bar(14, 96, 192, 8, "liquid.temp", accent, 20, 60),
      val(10, 108, 200, 18, "cpu.fan", "Ventilo CPU {:.0} tr/min", 12, DIM),
    ],
  },
  {
    id: "fan-n",
    label: "Ventilateur (n°)",
    category: "Refroidissement",
    w: 160,
    h: 160,
    param: { label: "Ventilo n°", options: ["1", "2", "3", "4", "5", "6"], default: "1" },
    build: ({ accent, param }) => [
      card(0, 0, 160, 160),
      txt(8, 8, 144, 20, `VENTILO ${param}`, 13, DIM),
      arc(30, 28, 100, `fan.${param}.rpm`, accent, 9, 0, 3000),
      val(30, 62, 100, 30, `fan.${param}.rpm`, "{:.0}", 24),
      txt(30, 90, 100, 16, "tr/min", 11, DIM),
    ],
  },
];
