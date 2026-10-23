//! Caractéristiques d'éclairage des cartes mères d'après la documentation des constructeurs
//! (`docs/led-catalog/boards.json`) : connecteurs ARGB/RGB du modèle et limite de LEDs par connecteur.
//! Un connecteur ne sait pas ce qui y est branché : ces données servent à expliquer et à borner, pas à compter.

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct Header {
    pub voltage: Option<String>,
    pub max_leds: Option<u32>,
    pub max_power_a: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct VendorRule {
    pub vendor: String,
    pub argb_header: Option<Header>,
}

#[derive(Debug, Deserialize)]
pub struct BoardEntry {
    pub vendor: String,
    pub model: String,
    pub board_id: Option<String>,
    #[serde(default)]
    pub argb_headers: Vec<String>,
    #[serde(default)]
    pub rgb_headers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BoardCatalog {
    pub vendor_rules: Vec<VendorRule>,
    pub boards: Vec<BoardEntry>,
}

pub fn catalog() -> &'static BoardCatalog {
    static C: OnceLock<BoardCatalog> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../assets/led_boards.json")).unwrap_or(BoardCatalog { vendor_rules: Vec::new(), boards: Vec::new() }))
}

/// Nom de fabricant WMI (« Micro-Star International Co., Ltd. ») vers la clé du catalogue.
pub fn vendor_key(manufacturer: &str) -> Option<&'static str> {
    let m = manufacturer.to_lowercase();
    [("micro-star", "MSI"), ("msi", "MSI"), ("asus", "ASUS"), ("gigabyte", "Gigabyte"), ("asrock", "ASRock"), ("biostar", "Biostar")]
        .into_iter()
        .find(|(k, _)| m.contains(k))
        .map(|(_, v)| v)
}

pub fn vendor_rule(manufacturer: &str) -> Option<&'static VendorRule> {
    let key = vendor_key(manufacturer)?;
    catalog().vendor_rules.iter().find(|r| r.vendor.eq_ignore_ascii_case(key))
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

/// Carte exacte : code MSI « MS-7D93 » présent dans le nom, sinon modèle identique (casse et séparateurs ignorés).
pub fn board(manufacturer: &str, product: &str) -> Option<&'static BoardEntry> {
    let key = vendor_key(manufacturer)?;
    let p = norm(product);
    if p.is_empty() {
        return None;
    }
    catalog().boards.iter().filter(|b| b.vendor.eq_ignore_ascii_case(key)).find(|b| {
        b.board_id.as_deref().is_some_and(|id| !id.is_empty() && p.contains(&norm(id))) || norm(&b.model) == p
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_every_vendor_rule_names_a_limit_or_a_note() {
        let c = catalog();
        assert!(c.boards.len() >= 100 && c.vendor_rules.len() >= 4);
        assert!(c.vendor_rules.iter().any(|r| r.vendor == "MSI" && r.argb_header.as_ref().and_then(|h| h.max_leds) == Some(75)));
    }

    #[test]
    fn vendor_names_from_windows_map_to_catalog_keys() {
        assert_eq!(vendor_key("Micro-Star International Co., Ltd."), Some("MSI"));
        assert_eq!(vendor_key("ASUSTeK COMPUTER INC."), Some("ASUS"));
        assert_eq!(vendor_key("Gigabyte Technology Co., Ltd."), Some("Gigabyte"));
        assert_eq!(vendor_key("ASRock"), Some("ASRock"));
        assert_eq!(vendor_key("Intel"), None);
    }

    #[test]
    fn a_catalogued_board_is_found_by_name_and_an_unknown_one_is_not() {
        let b = catalog().boards.iter().find(|b| b.vendor == "MSI").expect("une MSI");
        let found = board("Micro-Star International Co., Ltd.", &b.model).expect("trouvée par son nom");
        assert_eq!(found.model, b.model);
        assert!(board("Micro-Star International Co., Ltd.", "Carte inexistante XYZ").is_none());
        assert!(board("Intel", &b.model).is_none());
    }
}
