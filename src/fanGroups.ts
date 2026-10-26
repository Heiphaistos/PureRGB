// Regroupement des appareils à ventilateurs par catégorie (mise en page seule, aucune logique d'écriture).
import type { DeviceInfo } from "./types";

export const FAN_CATEGORIES = [
  { id: "bios", label: "Carte mère (BIOS et courbes)", hint: "Headers PWM de la carte mère, rendus au BIOS à la fermeture." },
  { id: "aio", label: "AIO / watercooling", hint: "Pompe et ventilateurs de radiateur. Minimum 20 % pour ne pas arrêter la pompe." },
  { id: "cooler", label: "Ventirad", hint: "Refroidisseurs processeur à air." },
  { id: "hub", label: "Hubs et ventilateurs un par un", hint: "Contrôleurs de ventilateurs (NZXT, Corsair, Lian Li, Thermaltake…)." },
  { id: "case", label: "Boîtier / E/S", hint: "Ventilateurs de boîtier et autres sorties." },
] as const;

export type FanCategoryId = (typeof FAN_CATEGORIES)[number]["id"];

export function fanCategoryOf(d: Pick<DeviceInfo, "backend" | "device_type">): FanCategoryId {
  if (d.backend === "mobo") return "bios";
  if (d.device_type === "Aio") return "aio";
  if (d.device_type === "Cooler") return "cooler";
  if (d.device_type === "Hub" || d.device_type === "Fan") return "hub";
  return "case";
}

export function groupFanDevices(devices: DeviceInfo[]): { id: FanCategoryId; label: string; hint: string; devices: DeviceInfo[] }[] {
  return FAN_CATEGORIES.map((c) => ({ ...c, devices: devices.filter((d) => fanCategoryOf(d) === c.id) })).filter((g) => g.devices.length > 0);
}
