// Mode démonstration : un produit factice reconnu par son nom, avec une « photo » SVG.
import type { ProductView } from "./productPhotos.ts";

const DEMO_ID = "demo-produit";

export function mockLookup(vendor: string, name: string): ProductView | null {
  const hay = `${vendor} ${name}`.toLowerCase();
  if (!hay.includes("thermalright") && !hay.includes("demo")) return null;
  return { id: DEMO_ID, kind: "screen", vendor: "Démonstration", name: "Produit de démonstration", has_image: true, source_page: null };
}

export function mockPhoto(id: string): string | null {
  if (id !== DEMO_ID) return null;
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 100"><rect width="160" height="100" rx="8" fill="#223"/>' +
    '<rect x="12" y="12" width="136" height="76" rx="4" fill="#4a90d9"/><text x="80" y="56" font-size="12" fill="#fff" text-anchor="middle" font-family="sans-serif">Photo de démonstration</text></svg>';
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
