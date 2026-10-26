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
