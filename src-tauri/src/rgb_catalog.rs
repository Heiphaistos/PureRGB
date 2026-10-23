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
    fn hex_ids_with_prefix_and_case() {
        assert!(is_rgb_vendor("0x1B1C") && is_rgb_vendor("1532"));
        assert!(!is_rgb_vendor("zzzz") && !is_rgb_vendor("0000"));
    }
}
