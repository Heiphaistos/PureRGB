//! Catalogue des identifiants USB (VID/PID) de contrôleurs RGB connus : ~1 090 appareils de 85 fabricants.
//! Données factuelles d'identification (`assets/rgb_usb_catalog.json`, régénérées depuis les détecteurs OpenRGB
//! et usb.ids par `tools/rgb-catalog/`), aucun code repris. Sert à reconnaître un appareil exact, avant même
//! de savoir comment le piloter.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Raw {
    vendors: HashMap<String, String>,
    devices: Vec<(String, String, String)>,
}

pub struct Catalog {
    vendors: HashMap<u16, String>,
    devices: HashMap<(u16, u16), String>,
}

fn hex(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

impl Catalog {
    fn parse(json: &str) -> Result<Catalog, String> {
        let raw: Raw = serde_json::from_str(json).map_err(|e| format!("catalogue RGB illisible : {e}"))?;
        let vendors = raw.vendors.into_iter().filter_map(|(k, v)| Some((hex(&k)?, v))).collect();
        let devices = raw.devices.into_iter().filter_map(|(v, p, n)| Some(((hex(&v)?, hex(&p)?), n))).collect();
        Ok(Catalog { vendors, devices })
    }

    pub fn vendor(&self, vid: u16) -> Option<&str> {
        self.vendors.get(&vid).map(String::as_str)
    }

    /// Nom du modèle exact, si ce couple VID/PID est dans le catalogue.
    pub fn device(&self, vid: u16, pid: u16) -> Option<&str> {
        self.devices.get(&(vid, pid)).map(String::as_str)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.devices.len()
    }
}

/// Catalogue embarqué (analysé une seule fois ; un JSON invalide est une erreur de build détectée par les tests).
pub fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| {
        Catalog::parse(include_str!("../assets/rgb_usb_catalog.json")).unwrap_or(Catalog { vendors: HashMap::new(), devices: HashMap::new() })
    })
}

/// Ce VID (hexadécimal, ex. « 1b1c ») est-il celui d'un fabricant de contrôleurs RGB ?
pub fn is_rgb_vendor(vid_hex: &str) -> bool {
    hex(vid_hex).is_some_and(|v| catalog().vendor(v).is_some())
}

/// Périphériques USB connus SANS éclairage RGB (liste fermée, source = indication du mainteneur d'après les
/// fiches produit) : jamais signalés comme « non pris en charge ». (VID, PID, nom.)
const NO_RGB: [(u16, u16, &str); 21] = [
    (0x046D, 0x0ABA, "Casque Logitech PRO X"),
    (0x046D, 0x0AF7, "Casque Logitech PRO X"),
    // Concentrateurs USB : le descripteur lu sur la tour dit « Hub Controller » / « Concentrateur USB générique ».
    (0x187C, 0x1012, "Concentrateur USB (Hub Controller)"),
    (0x1A86, 0x8091, "Concentrateur USB générique"),
    // Alimentations Corsair AX/HXi/RMi : iCUE les déclare `power-supply` sans LED (manifeste AX1600i : MaxLightingsPerLed 0,
    // keyToLedMap vide, 12 canaux OCP) ; voir docs/rgb-protocols/analyse-corsair-icue.md §13. 1C25 (AX1600i SHIFT) non couvert.
    (0x1B1C, 0x1C00, "Alimentation Corsair AX860i"),
    (0x1B1C, 0x1C02, "Alimentation Corsair AX1500i"),
    (0x1B1C, 0x1C05, "Alimentation Corsair HX750i"),
    (0x1B1C, 0x1C06, "Alimentation Corsair HX850i"),
    (0x1B1C, 0x1C07, "Alimentation Corsair HX1000i"),
    (0x1B1C, 0x1C08, "Alimentation Corsair HX1200i"),
    (0x1B1C, 0x1C0A, "Alimentation Corsair RM650i"),
    (0x1B1C, 0x1C0B, "Alimentation Corsair RM750i"),
    (0x1B1C, 0x1C0C, "Alimentation Corsair RM850i"),
    (0x1B1C, 0x1C0D, "Alimentation Corsair RM1000i"),
    // Récepteurs 2,4 GHz génériques (puce OEM, VID 1EA7 = Sharkoon dans usb.ids, partagé par de nombreuses marques) : 0064 = souris
    // (interface d'entrée), 0066 = clavier/composite. Sur les packs Empire Gaming (ARMOR RF358, RF800, MK800) le rétroéclairage se règle
    // aux touches FN, « le pack ne propose aucun logiciel » ; aucun logiciel officiel ne vise ces PID. Voir docs/rgb-protocols/analyse-empire-gaming.md.
    (0x1EA7, 0x0064, "Récepteur 2,4 GHz souris (Empire Gaming / OEM)"),
    (0x1EA7, 0x0066, "Récepteur 2,4 GHz clavier-souris (Empire Gaming / OEM)"),
    (0x1B1C, 0x1C11, "Alimentation Corsair AX1600i"),
    (0x1B1C, 0x1C1E, "Alimentation Corsair HX1000i"),
    (0x1B1C, 0x1C1F, "Alimentation Corsair HX1500i"),
    (0x1B1C, 0x1C23, "Alimentation Corsair HX1200i"),
    (0x1B1C, 0x1C27, "Alimentation Corsair HX1200i"),
];

/// Nom du périphérique s'il est connu pour ne PAS avoir d'éclairage RGB.
pub fn known_without_rgb(vid: u16, pid: u16) -> Option<&'static str> {
    NO_RGB.iter().find(|(v, p, _)| *v == vid && *p == pid).map(|(_, _, n)| *n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_valid_and_large() {
        let c = Catalog::parse(include_str!("../assets/rgb_usb_catalog.json")).expect("JSON valide");
        assert!(c.len() > 1000, "{} appareils", c.len());
        assert!(c.vendor(0x1B1C).is_some_and(|v| v.contains("Corsair")), "Corsair connu");
        assert!(c.vendor(0x1532).is_some(), "Razer connu");
    }

    #[test]
    fn exact_and_vendor_lookups() {
        let c = Catalog::parse(r#"{"vendors":{"1b1c":"Corsair"},"devices":[["1b1c","0c0b","Lighting Node Pro"]]}"#).unwrap();
        assert_eq!(c.device(0x1B1C, 0x0C0B), Some("Lighting Node Pro"));
        assert_eq!(c.device(0x1B1C, 0xFFFF), None);
        assert_eq!(c.vendor(0x1B1C), Some("Corsair"));
        assert!(Catalog::parse("pas du json").is_err());
    }

    #[test]
    fn corsair_psu_is_known_without_rgb() {
        assert_eq!(known_without_rgb(0x1B1C, 0x1C11), Some("Alimentation Corsair AX1600i"));
        assert_eq!(known_without_rgb(0x1B1C, 0x0C11), None, "Commander Pro garde son RGB");
        assert!(catalog().device(0x1B1C, 0x0C11).is_some_and(|n| n.contains("Commander")));
    }

    #[test]
    fn hex_ids_with_prefix_and_case() {
        assert!(is_rgb_vendor("0x1B1C") && is_rgb_vendor("1532"));
        assert!(!is_rgb_vendor("zzzz") && !is_rgb_vendor("0000"));
    }

    #[test]
    fn known_without_rgb_list_is_exact() {
        assert!(known_without_rgb(0x046D, 0x0ABA).is_some() && known_without_rgb(0x046D, 0x0AF7).is_some());
        assert!(known_without_rgb(0x187C, 0x1012).is_some() && known_without_rgb(0x1A86, 0x8091).is_some());
        assert!(known_without_rgb(0x046D, 0xC08B).is_none());
        assert!(known_without_rgb(0x1EA7, 0x0064).is_some() && known_without_rgb(0x1EA7, 0x0066).is_some(), "récepteurs 2,4 G Empire Gaming");
        assert!(known_without_rgb(0x1EA7, 0x0030).is_none(), "autre PID du même VID : inchangé");
    }
}
