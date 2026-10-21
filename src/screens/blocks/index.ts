// Catalogue des blocs prêts à l'emploi (données pures : un fichier par catégorie).
import { COOLING_BLOCKS } from "./cooling.ts";
import { GAUGE_BLOCKS } from "./gauges.ts";
import { HARDWARE_BLOCKS } from "./hardware.ts";
import type { BlockDef } from "./kit.ts";
import { MISC_BLOCKS } from "./misc.ts";
import { NETWORK_BLOCKS } from "./network.ts";
import { STORAGE_BLOCKS } from "./storage.ts";
import { TIME_BLOCKS } from "./time.ts";

export const BLOCKS: BlockDef[] = [...TIME_BLOCKS, ...HARDWARE_BLOCKS, ...STORAGE_BLOCKS, ...NETWORK_BLOCKS, ...COOLING_BLOCKS, ...GAUGE_BLOCKS, ...MISC_BLOCKS];

export const BLOCK_CATEGORIES: string[] = [...new Set(BLOCKS.map((b) => b.category))];

export { defaultParam, instantiateBlock, type BlockCtx, type BlockDef } from "./kit.ts";
