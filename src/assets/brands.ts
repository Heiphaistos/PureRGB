export interface Chip {
  text: string;
  color: string;
}

interface BrandRule {
  tokens: string[];
  label: string;
  color: string;
}

// Tokens are whole brand names/short codes only — no generic short
// substrings (avoids false positives against unrelated device names).
const BRANDS: BrandRule[] = [
  { tokens: ["corsair"], label: "COR", color: "#ffcc00" },
  { tokens: ["nzxt"], label: "NZXT", color: "#4ac8ec" },
  { tokens: ["asus", "aura sync", "rog "], label: "ASUS", color: "#e2001a" },
  { tokens: ["msi", "mystic light"], label: "MSI", color: "#ff0000" },
  { tokens: ["gigabyte", "aorus"], label: "GB", color: "#ff6a13" },
  { tokens: ["razer", "chroma"], label: "RZR", color: "#44d62c" },
  { tokens: ["logitech", "logi "], label: "LOGI", color: "#00b8fc" },
  { tokens: ["steelseries"], label: "SS", color: "#f04a4a" },
  { tokens: ["hyperx"], label: "HX", color: "#e6373a" },
  { tokens: ["evga"], label: "EVGA", color: "#1c4fa3" },
  { tokens: ["cooler master"], label: "CM", color: "#9a9a9a" },
  { tokens: ["lian li"], label: "LL", color: "#c40000" },
  { tokens: ["thermaltake"], label: "TT", color: "#ffd200" },
  { tokens: ["deepcool"], label: "DC", color: "#0075c9" },
  { tokens: ["asrock"], label: "ASR", color: "#00a3e0" },
];

function norm(s: string): string {
  return s.trim().toLowerCase();
}

// Priority: vendor first (structured field, populated by the OpenRGB/HID
// backends), then device name — mobo/liquidctl backends always send an
// empty vendor, and OpenRGB's vendor field is plugin-dependent.
export function detectBrand(vendor: string, name: string): Chip | null {
  const v = norm(vendor);
  if (v) {
    const hit = BRANDS.find((b) => b.tokens.some((t) => v.includes(t)));
    if (hit) return { text: hit.label, color: hit.color };
  }
  const n = norm(name);
  const hit = BRANDS.find((b) => b.tokens.some((t) => n.includes(t)));
  return hit ? { text: hit.label, color: hit.color } : null;
}

// Deterministic, cheap hash → stable hue per device name, so a given
// device always renders the same monogram color across sessions.
function hashHue(str: string): number {
  let h = 0;
  for (let i = 0; i < str.length; i++) {
    h = (h * 31 + str.charCodeAt(i)) >>> 0;
  }
  return h % 360;
}

export function monogramChip(name: string): Chip {
  const trimmed = name.trim();
  const letter = (trimmed.match(/[a-zA-Z0-9]/)?.[0] ?? "?").toUpperCase();
  return { text: letter, color: `hsl(${hashHue(trimmed || "?")}, 55%, 42%)` };
}

export function brandChip(vendor: string, name: string): Chip {
  return detectBrand(vendor, name) ?? monogramChip(name);
}
