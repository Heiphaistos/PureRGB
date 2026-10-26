//! Détection de TOUTES les manettes vues en HID (Windows expose aussi les manettes XInput comme collections HID), y compris
//! celles sans RGB : listées, non pilotables, avec la raison. Aucun handle n'est ouvert ici (énumération seule).
//!
//! L'identification se fait sur la chaîne produit en plus du VID:PID : une Flydigi Vader 4 se présente sous l'ID d'une
//! Xbox 360 (045E:028E) mais son nom de produit contient « Flydigi » ; une vraie manette Xbox garde son nom Xbox.
//! Source des faits : docs/rgb-protocols/manettes.md et docs/rgb-protocols/flydigi.md.

/// Page d'usage HID « Generic Desktop » et usages Joystick (0x04) / Gamepad (0x05) : définis par la spécification HID Usage Tables.
pub fn is_pad_collection(usage_page: u16, usage: u16) -> bool {
    usage_page == 0x01 && (usage == 0x04 || usage == 0x05)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    pub name: String,
    pub vendor: &'static str,
    /// Raison affichée : pourquoi la manette n'est pas pilotable.
    pub note: &'static str,
}

const NO_RGB: &str = "manette détectée — pas d'éclairage RGB pilotable connu";

/// Dongles 2,4 GHz Flydigi en D-Input : couples VID:PID de `FDGDevice.cs` (SFDG_VIDS/SFDG_PIDS, Space Station 3.4.4.3).
const FLYDIGI_DONGLES: [(u16, u16); 6] =
    [(0x04B4, 0x2410), (0x04B4, 0x2411), (0x04B4, 0x2412), (0x04B5, 0x2413), (0x04B4, 0x2414), (0x04B5, 0x2415)];

pub fn classify(vid: u16, pid: u16, product: &str) -> Pad {
    let lower = product.to_lowercase();
    let shown = |fallback: &str| if product.trim().is_empty() { fallback.to_string() } else { product.trim().to_string() };
    // Flydigi d'abord : elle emprunte l'ID d'une manette Xbox 360 (045E:028E) ou d'une Xbox One.
    if lower.contains("flydigi") || lower.contains("vader") || FLYDIGI_DONGLES.contains(&(vid, pid)) {
        return Pad {
            name: shown("Manette Flydigi"),
            vendor: "Flydigi",
            note: "éclairage non pris en charge pour l'instant (protocole non documenté publiquement, voir docs/rgb-protocols/flydigi.md)",
        };
    }
    match vid {
        0x045E => Pad { name: shown("Manette Xbox"), vendor: "Microsoft", note: "manette Xbox — pas d'éclairage RGB" },
        0x054C => Pad {
            name: shown("Manette Sony"),
            vendor: "Sony",
            note: if pid == 0x0BA0 { "récepteur DualShock 4 — éclairage non piloté via le récepteur" } else { "manette Sony sans protocole d'éclairage pris en charge" },
        },
        0x057E => Pad {
            name: shown("Manette Nintendo"),
            vendor: "Nintendo",
            note: "manette Nintendo — seulement des LED de joueur (pas de RGB), non pilotées",
        },
        _ => Pad { name: shown("Manette"), vendor: "", note: NO_RGB },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flydigi_with_xbox360_ids_is_told_apart_by_its_product_string() {
        let fly = classify(0x045E, 0x028E, "Controller (Flydigi VADER4)");
        assert_eq!(fly.vendor, "Flydigi");
        assert!(fly.note.contains("non pris en charge"));
    }

    #[test]
    fn flydigi_dongle_ids_are_recognised_without_a_product_string() {
        assert_eq!(classify(0x04B5, 0x2413, "").vendor, "Flydigi");
        assert_eq!(classify(0x04B4, 0x2410, "").vendor, "Flydigi");
        assert_ne!(classify(0x04B4, 0x2416, "").vendor, "Flydigi");
    }

    #[test]
    fn a_real_xbox_pad_is_never_taken_for_a_flydigi() {
        for product in ["Controller (XBOX 360 For Windows)", "Xbox Controller", "", "Controller (Xbox One For Windows)"] {
            let p = classify(0x045E, 0x028E, product);
            assert_eq!(p.vendor, "Microsoft", "{product}");
            assert!(!p.note.contains("Flydigi"));
        }
    }

    #[test]
    fn pad_collections_are_joysticks_and_gamepads_only() {
        assert!(is_pad_collection(0x01, 0x05));
        assert!(is_pad_collection(0x01, 0x04));
        assert!(!is_pad_collection(0x01, 0x06)); // clavier
        assert!(!is_pad_collection(0x01, 0x02)); // souris
        assert!(!is_pad_collection(0xFF00, 0x05));
    }

    #[test]
    fn other_brands_are_listed_with_a_reason() {
        assert_eq!(classify(0x054C, 0x0268, "PLAYSTATION(R)3 Controller").vendor, "Sony");
        assert_eq!(classify(0x057E, 0x2009, "Pro Controller").vendor, "Nintendo");
        assert!(classify(0x1234, 1, "").note.contains("pas d'éclairage"));
    }
}
