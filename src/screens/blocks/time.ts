// Blocs Horloge : heure numérique, heure analogique, date, durée de fonctionnement.
import { DIM, card, txt, val, type BlockDef } from "./kit.ts";

export const TIME_BLOCKS: BlockDef[] = [
  {
    id: "time-digital",
    label: "Heure numérique",
    category: "Horloge",
    w: 240,
    h: 100,
    build: ({ accent }) => [card(0, 0, 240, 100), val(10, 8, 220, 62, "time", "%H:%M", 54, "#ffffff"), val(10, 68, 220, 24, "date", "%d/%m/%Y", 18, accent)],
  },
  {
    id: "time-analog",
    label: "Heure analogique",
    category: "Horloge",
    w: 200,
    h: 200,
    build: ({ accent }) => [{ type: "analog_clock", x: 0, y: 0, w: 200, h: 200, rotation: 0, opacity: 1, face: "#16161c", hands: "#ffffff", accent }],
  },
  {
    id: "date",
    label: "Date",
    category: "Horloge",
    w: 220,
    h: 70,
    build: ({ accent }) => [card(0, 0, 220, 70), txt(10, 8, 200, 22, "DATE", 13, DIM), val(10, 28, 200, 36, "date", "%d/%m/%Y", 28, accent)],
  },
  {
    id: "uptime",
    label: "Temps allumé",
    category: "Horloge",
    w: 220,
    h: 70,
    build: ({ accent }) => [card(0, 0, 220, 70), txt(10, 8, 200, 22, "ALLUMÉ DEPUIS", 13, DIM), val(10, 28, 200, 36, "uptime", "{}", 28, accent)],
  },
];
