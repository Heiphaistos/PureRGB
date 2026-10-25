// Logique pure du catalogue communautaire (testée dans tests/communityFeedback.test.ts).

export type Verdict = "works" | "partial" | "broken";

export const VERDICTS: { value: Verdict; label: string }[] = [
  { value: "works", label: "Ça marche" },
  { value: "partial", label: "Marche en partie" },
  { value: "broken", label: "Ça ne marche pas" },
];

export const NOTE_MAX = 280;

export interface CommunityEntry {
  vid: string;
  pid: string;
  works: number;
  partial: number;
  broken: number;
}

// Contrôles, bidi, zéro-largeur, zone privée : même liste que le nettoyage Rust (device_feedback.rs).
const INVISIBLE = /[\p{Cc}­​-‏‪-‮⁠-⁯﻿￹-￻-]/gu;

/** Aperçu de ce que le nettoyage serveur/Rust fera de la note (le contenu envoyé reste celui de l'aperçu Rust). */
export function cleanNote(s: string): string {
  return Array.from(s.replace(INVISIBLE, " ").split(/\s+/).filter(Boolean).join(" "))
    .slice(0, NOTE_MAX)
    .join("")
    .trim();
}

/** VID:PID d'un identifiant d'appareil ("hid:1b1c:0c10:0"), ou null (OpenRGB, capteurs…). */
export function usbIdOf(id: string): { vid: string; pid: string } | null {
  const m = /(?:^|:)([0-9a-f]{4}):([0-9a-f]{4})(?::|$)/i.exec(id);
  return m ? { vid: m[1].toLowerCase(), pid: m[2].toLowerCase() } : null;
}

export function summaryLabel(e: CommunityEntry): string {
  const parts: string[] = [];
  if (e.works) parts.push(`confirmé fonctionnel par ${e.works} utilisateur${e.works > 1 ? "s" : ""}`);
  if (e.partial) parts.push(`${e.partial} signalement${e.partial > 1 ? "s" : ""} « marche en partie »`);
  if (e.broken) parts.push(`${e.broken} signalement${e.broken > 1 ? "s" : ""} « ne marche pas »`);
  return parts.join(" · ") || "aucun retour";
}
