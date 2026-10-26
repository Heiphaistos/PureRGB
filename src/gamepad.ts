// Testeur de manette (lecture seule, API Gamepad du navigateur) : noms de boutons, zone morte, dérive.

/** Disposition « standard » de l'API Gamepad (index -> nom). */
export const BUTTON_NAMES = [
  "A / Croix", "B / Rond", "X / Carré", "Y / Triangle", "LB / L1", "RB / R1", "LT / L2", "RT / R2",
  "Select / Share", "Start / Options", "Clic stick gauche", "Clic stick droit",
  "Haut", "Bas", "Gauche", "Droite", "Guide / PS", "Tactile / Misc",
];

export const buttonName = (i: number): string => BUTTON_NAMES[i] ?? `Bouton ${i}`;

export interface Stick {
  x: number;
  y: number;
  magnitude: number;
  inDeadzone: boolean;
}

/** Position d'un stick (-1..1) et appartenance à la zone morte circulaire `deadzone` (0..1). */
export function stick(x: number, y: number, deadzone = 0.1): Stick {
  const magnitude = Math.min(1, Math.hypot(x, y));
  return { x, y, magnitude, inDeadzone: magnitude <= deadzone };
}

/** Dérive : plus grande déflexion mesurée sticks au repos ; au-delà de 5 % la manette dérive. */
export function drift(samples: [number, number][]): { max: number; drifting: boolean } {
  const max = samples.reduce((m, [x, y]) => Math.max(m, Math.hypot(x, y)), 0);
  return { max, drifting: max > 0.05 };
}

const PAD_VENDORS: Record<string, string> = {
  "045e": "Microsoft",
  "054c": "Sony",
  "057e": "Nintendo",
  "046d": "Logitech",
  "2dc8": "8BitDo",
  "28de": "Valve",
  "1532": "Razer",
};

export interface PadIdentity {
  vendor: string;
  /** Nom à confronter au catalogue (sans le suffixe technique du navigateur). */
  name: string;
  /** `vid:pid` quand le navigateur le donne (Chrome/Edge « Vendor: … Product: … », Firefox « vid-pid-nom »). */
  usb: string | null;
  /** Nom affiché si le catalogue ne reconnaît pas la manette. */
  label: string;
}

/** Identité d'une manette d'après `Gamepad.id`. XInput seul = famille Xbox : Windows ne dit pas 360, One ou Series. */
export function padIdentity(id: string): PadIdentity {
  const m = /Vendor:\s*([0-9a-f]{4})\s+Product:\s*([0-9a-f]{4})/i.exec(id) ?? /^([0-9a-f]{4})-([0-9a-f]{4})-/i.exec(id);
  const usb = m ? `${m[1]}:${m[2]}`.toLowerCase() : null;
  const vendor = (m && PAD_VENDORS[m[1].toLowerCase()]) || "";
  let name = id.replace(/\s*\([^)]*\)\s*$/, "").replace(/^[0-9a-f]{4}-[0-9a-f]{4}-/i, "").trim();
  if (/xinput/i.test(id) && !usb) {
    // Chrome/Edge nomment « Xbox 360 Controller (XInput STANDARD GAMEPAD) » toute manette XInput, y compris One et Series.
    return { vendor: "Microsoft", name: "Xbox Wireless Controller", usb: null, label: "Manette Xbox (XInput : 360, One ou Series)" };
  }
  if (name === "Wireless Controller" && vendor === "Sony") name = "Sony Wireless Controller";
  return { vendor, name, usb, label: name || id };
}
