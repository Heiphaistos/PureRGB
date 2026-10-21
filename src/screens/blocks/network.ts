// Blocs réseau et débit disque : valeurs montant/descendant + courbe.
import { DIM, card, graph, txt, val, type BlockDef } from "./kit.ts";

export const NETWORK_BLOCKS: BlockDef[] = [
  {
    id: "network",
    label: "Réseau",
    category: "Réseau",
    w: 260,
    h: 150,
    build: ({ accent }) => [
      card(0, 0, 260, 150),
      txt(10, 8, 100, 20, "RÉSEAU", 14, DIM),
      val(10, 28, 120, 28, "net.up", "↑ {:.1} Mo/s", 17, accent),
      val(130, 28, 120, 28, "net.down", "↓ {:.1} Mo/s", 17),
      graph(12, 62, 236, 76, "net.down", accent, 0, 20),
    ],
  },
  {
    id: "disk-io",
    label: "Débit disque",
    category: "Réseau",
    w: 260,
    h: 110,
    build: ({ accent }) => [
      card(0, 0, 260, 110),
      txt(10, 8, 100, 20, "DISQUES", 14, DIM),
      val(10, 28, 120, 28, "disk.read", "Lire {:.1}", 17, accent),
      val(130, 28, 120, 28, "disk.write", "Écrire {:.1}", 17),
      graph(12, 60, 236, 44, "disk.read", accent, 0, 200),
    ],
  },
];
