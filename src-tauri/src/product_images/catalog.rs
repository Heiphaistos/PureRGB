//! Catalogue de produits : schéma, validation au chargement, identification.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const CATALOG_JSON: &str = include_str!("../../assets/product_catalog.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Screen,
    Light,
    Fan,
    Cooler,
    Aio,
    Keyboard,
    Mouse,
    Ram,
    Motherboard,
    Gpu,
    Other,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Match {
    /// `VID:PID` en hexadécimal minuscule (4 chiffres chacun).
    #[serde(default)]
    pub usb: Vec<String>,
    /// Sous-chaînes de nom (comparées sans accents, casse ni ponctuation).
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Image {
    pub url: String,
    pub source_page: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Product {
    pub id: String,
    pub kind: Kind,
    pub vendor: String,
    pub name: String,
    #[serde(rename = "match")]
    pub matching: Match,
    #[serde(default)]
    pub image: Option<Image>,
}

#[derive(Deserialize)]
struct Catalog {
    version: u32,
    products: Vec<Product>,
}

/// Minuscules, accents retirés, tout non alphanumérique -> espace simple.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        };
        out.push(if c.is_alphanumeric() { c } else { ' ' });
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `vvvv:pppp` en minuscules si `s` est un couple VID:PID valide.
fn norm_usb(s: &str) -> Option<String> {
    let (v, p) = s.split_once(':')?;
    let ok = |x: &str| x.len() == 4 && x.chars().all(|c| c.is_ascii_hexdigit());
    (ok(v) && ok(p)).then(|| s.to_ascii_lowercase())
}

/// Extrait un VID:PID d'une chaîne libre : `vid:pid`, `hid:vid:pid:…`, `vid:pid:série`.
pub fn extract_usb(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split(':').collect();
    parts.windows(2).find_map(|w| norm_usb(&format!("{}:{}", w[0], w[1])))
}

/// URL du catalogue : https, hôte nominatif (pas d'IP), pas de caractère exotique.
pub(super) fn url_shape_ok(url: &str) -> bool {
    let chars_ok = url.len() <= 500 && url.chars().all(|c| c.is_ascii_alphanumeric() || "-._~/:%+?=&#".contains(c));
    chars_ok && !url.contains("..") && crate::trust::https_host(url).is_some_and(super::fetch::host_is_name)
}

fn validate(c: &Catalog) -> Result<(), String> {
    if c.version != 1 {
        return Err(format!("version de catalogue non gérée : {}", c.version));
    }
    let mut seen = std::collections::HashSet::new();
    for p in &c.products {
        let ctx = |m: &str| format!("produit « {} » : {m}", p.id);
        let id_ok = !p.id.is_empty() && p.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !id_ok || !seen.insert(p.id.as_str()) {
            return Err(ctx("id vide, invalide ou en double"));
        }
        if p.name.trim().is_empty() {
            return Err(ctx("nom vide"));
        }
        if p.matching.usb.iter().any(|u| norm_usb(u).is_none_or(|n| n != *u)) {
            return Err(ctx("usb doit être `vvvv:pppp` en minuscules hexadécimales"));
        }
        if p.matching.usb.is_empty() && p.matching.names.is_empty() {
            return Err(ctx("aucun critère de correspondance"));
        }
        let vendor = normalize(&p.vendor);
        for n in &p.matching.names {
            let n = normalize(n);
            if n.len() < 4 || n == vendor {
                return Err(ctx("motif de nom trop court ou limité au fabricant"));
            }
        }
        if let Some(i) = &p.image {
            if !url_shape_ok(&i.url) || !url_shape_ok(&i.source_page) {
                return Err(ctx("URL d'image ou de page source refusée (https + hôte nominatif requis)"));
            }
        }
    }
    Ok(())
}

fn parse(json: &str) -> Result<Vec<Product>, String> {
    let c: Catalog = serde_json::from_str(json).map_err(|e| format!("catalogue illisible : {e}"))?;
    validate(&c)?;
    Ok(c.products)
}

fn products() -> &'static [Product] {
    static P: OnceLock<Vec<Product>> = OnceLock::new();
    P.get_or_init(|| {
        parse(CATALOG_JSON).unwrap_or_else(|e| {
            log::error!("catalogue de produits invalide, photos désactivées : {e}");
            Vec::new()
        })
    })
}

pub fn product_by_id(id: &str) -> Option<&'static Product> {
    products().iter().find(|p| p.id == id)
}

/// Correspondance précise : USB exact d'abord (départagé par le nom si plusieurs produits
/// partagent le VID:PID), sinon nom le plus spécifique. Égalité ou ambiguïté -> `None`.
pub fn identify(vendor: &str, name: &str, usb_ids: &[&str]) -> Option<&'static Product> {
    identify_in(products(), vendor, name, usb_ids)
}

fn best_by_name<'a>(cands: impl Iterator<Item = &'a Product>, hay: &str) -> Option<&'a Product> {
    let hay = format!(" {hay} ");
    let mut best: Option<(usize, &Product)> = None;
    let mut tie = false;
    for p in cands {
        let score = p
            .matching
            .names
            .iter()
            .map(|n| normalize(n))
            .filter(|n| hay.contains(&format!(" {n} ")))
            .map(|n| n.len())
            .max();
        let Some(s) = score else { continue };
        match best {
            Some((b, _)) if s < b => {}
            Some((b, bp)) if s == b => tie |= bp.id != p.id,
            _ => {
                tie = false;
                best = Some((s, p));
            }
        }
    }
    if tie {
        None
    } else {
        best.map(|(_, p)| p)
    }
}

fn identify_in<'a>(all: &'a [Product], vendor: &str, name: &str, usb_ids: &[&str]) -> Option<&'a Product> {
    let hay = normalize(&format!("{vendor} {name}"));
    let usbs: Vec<String> = usb_ids.iter().filter_map(|u| extract_usb(u)).collect();
    let by_usb: Vec<&Product> = all.iter().filter(|p| p.matching.usb.iter().any(|u| usbs.contains(u))).collect();
    match by_usb.len() {
        0 => best_by_name(all.iter(), &hay),
        1 => Some(by_usb[0]),
        _ => best_by_name(by_usb.into_iter(), &hay),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prod(id: &str, usb: &[&str], names: &[&str]) -> Product {
        Product {
            id: id.into(),
            kind: Kind::Screen,
            vendor: "Thermalright".into(),
            name: id.into(),
            matching: Match { usb: usb.iter().map(|s| s.to_string()).collect(), names: names.iter().map(|s| s.to_string()).collect() },
            image: None,
        }
    }

    fn id_of(p: Option<&Product>) -> Option<&str> {
        p.map(|p| p.id.as_str())
    }

    #[test]
    fn shipped_catalog_is_valid() {
        assert!(parse(CATALOG_JSON).is_ok(), "{:?}", parse(CATALOG_JSON).err());
    }

    #[test]
    fn normalize_strips_accents_case_punct() {
        assert_eq!(normalize("  Écran  LCD-9.16_Été!"), "ecran lcd 9 16 ete");
    }

    #[test]
    fn accents_match_in_names() {
        let all = [prod("a", &[], &["écran été"])];
        assert!(identify_in(&all, "", "ECRAN ETE", &[]).is_some());
    }

    #[test]
    fn usb_extraction() {
        assert_eq!(extract_usb("87CD:70DB:demo").as_deref(), Some("87cd:70db"));
        assert_eq!(extract_usb("hid:1b1c:0c10:0").as_deref(), Some("1b1c:0c10"));
        assert_eq!(extract_usb("openrgb:2"), None);
        assert_eq!(extract_usb("12345:abcd"), None);
    }

    #[test]
    fn vendor_alone_never_matches() {
        let all = [prod("a", &[], &["trofeo vision"])];
        assert!(identify_in(&all, "Thermalright", "Thermalright", &[]).is_none());
        assert!(identify_in(&all, "Thermalright", "Peerless Assassin", &[]).is_none());
        assert!(identify_in(&all, "x", "Trofeo Vision 9.16", &[]).is_some());
    }

    #[test]
    fn nearby_names_are_not_confused() {
        let all = [prod("h150i", &[], &["h150i"]), prod("h150", &[], &["h150"])];
        assert_eq!(id_of(identify_in(&all, "Corsair", "H150i Elite", &[])), Some("h150i"));
        assert_eq!(id_of(identify_in(&all, "Corsair", "H150", &[])), Some("h150"));
        assert!(identify_in(&all, "Corsair", "H1500", &[]).is_none());
    }

    #[test]
    fn most_specific_name_wins_and_ties_refuse() {
        let all = [prod("base", &[], &["trofeo vision"]), prod("full", &[], &["trofeo vision 9 16"])];
        assert_eq!(id_of(identify_in(&all, "", "Trofeo Vision 9.16 LCD", &[])), Some("full"));
        assert_eq!(id_of(identify_in(&all, "", "Trofeo Vision", &[])), Some("base"));
        let tie = [prod("x", &[], &["trofeo vision"]), prod("y", &[], &["trofeo vision"])];
        assert!(identify_in(&tie, "", "Trofeo Vision", &[]).is_none());
    }

    #[test]
    fn usb_has_priority_over_name() {
        let all = [prod("byname", &[], &["trofeo vision"]), prod("byusb", &["0416:5408"], &["autre"])];
        assert_eq!(id_of(identify_in(&all, "", "Trofeo Vision", &["0416:5408:serie"])), Some("byusb"));
    }

    #[test]
    fn shared_usb_is_disambiguated_by_name_or_refused() {
        let all = [prod("a", &["0416:5408"], &["modele a"]), prod("b", &["0416:5408"], &["modele b"])];
        assert_eq!(id_of(identify_in(&all, "", "Modele B LCD", &["0416:5408"])), Some("b"));
        assert!(identify_in(&all, "", "Inconnu", &["0416:5408"]).is_none());
    }

    #[test]
    fn invalid_catalogs_are_refused() {
        let img = r#""image":{"url":"https://a.com/x.png","source_page":"https://a.com/"}"#;
        let mk = |body: &str| format!(r#"{{"version":1,"products":[{{"id":"p","kind":"fan","vendor":"V","name":"N",{body}}}]}}"#);
        assert!(parse(&mk(&format!(r#""match":{{"names":["modele"]}},{img}"#))).is_ok());
        assert!(parse(&mk(r#""match":{"names":["abc"]}"#)).is_err(), "motif trop court");
        assert!(parse(&mk(r#""match":{"names":["V"]}"#)).is_err(), "motif trop court");
        let only_vendor = mk(r#""match":{"names":["Maker Co"]}"#).replace(r#""vendor":"V""#, r#""vendor":"maker co""#);
        assert!(parse(&only_vendor).is_err(), "motif = fabricant seul");
        assert!(parse(&mk(r#""match":{}"#)).is_err(), "aucun critère");
        assert!(parse(&mk(r#""match":{"usb":["0416:5408 "]}"#)).is_err());
        assert!(parse(&mk(r#""match":{"usb":["0416:54G8"]}"#)).is_err());
        assert!(parse(&mk(r#""match":{"usb":["0416:54AB"]}"#)).is_err(), "majuscules refusées");
        let bad_url = |u: &str| {
            mk(&format!(r#""match":{{"names":["modele"]}},"image":{{"url":"{u}","source_page":"https://a.com/"}}"#))
        };
        for u in ["http://a.com/x.png", "https://127.0.0.1/x.png", "https://localhost/x.png", "https://a.com@evil.com/x"] {
            assert!(parse(&bad_url(u)).is_err(), "{u}");
        }
        assert!(parse(r#"{"version":2,"products":[]}"#).is_err());
        let one = r#"{"id":"p","kind":"fan","vendor":"V","name":"N","match":{"names":["modele"]}}"#;
        assert!(parse(&format!(r#"{{"version":1,"products":[{one},{one}]}}"#)).is_err(), "doublon");
    }
}
