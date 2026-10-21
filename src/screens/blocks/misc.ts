// Blocs libres : texte et logo/image (à choisir ensuite dans l'inspecteur).
import { image, txt, type BlockDef } from "./kit.ts";

export const MISC_BLOCKS: BlockDef[] = [
  { id: "free-text", label: "Texte libre", category: "Libre", w: 240, h: 50, build: ({ accent }) => [txt(0, 0, 240, 50, "Mon texte", 36, accent)] },
  { id: "logo", label: "Logo / image", category: "Libre", w: 160, h: 160, build: () => [image(0, 0, 160, 160)] },
];
