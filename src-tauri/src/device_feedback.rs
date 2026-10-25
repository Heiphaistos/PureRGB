//! Catalogue communautaire : fiche appareil envoyée à la demande de l'utilisateur (jamais en arrière-plan) et
//! retours agrégés lus depuis le serveur de télémétrie. Vie privée : la fiche ne contient que VID:PID, noms USB
//! du constructeur, famille de pilote, version de l'app, verdict et une note courte nettoyée. Aucune IP, aucun
//! chemin, aucun numéro de série, aucune capture USB.

use crate::backends::hid::{known, native};
use crate::netdev::curl;
use crate::settings::dirs_dir;
use crate::telemetry::{ensure_accepted, TELEMETRY_BASE_URL};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const NOTE_MAX: usize = 280;
const NAME_MAX: usize = 64;
const COUNT_MAX: u64 = 10_000_000;
const ENTRIES_MAX: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Works,
    Partial,
    Broken,
}

impl Verdict {
    pub fn parse(s: &str) -> Option<Verdict> {
        match s {
            "works" => Some(Verdict::Works),
            "partial" => Some(Verdict::Partial),
            "broken" => Some(Verdict::Broken),
            _ => None,
        }
    }
}

/// Contenu EXACT envoyé au serveur (c'est aussi ce que l'aperçu affiche).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Fiche {
    pub vid: String,
    pub pid: String,
    pub app_version: String,
    pub driver_family: String,
    pub verdict: Verdict,
    pub manufacturer: String,
    pub product: String,
    pub note: String,
}

fn is_invisible_format(c: char) -> bool {
    matches!(c, '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}' | '\u{E000}'..='\u{F8FF}')
}

/// Texte libre sûr : sans caractères de contrôle ni de formatage invisibles (bidi, zéro-largeur), espaces
/// réduits, longueur bornée en caractères.
pub fn sanitize_text(s: &str, max: usize) -> String {
    let spaced: String = s.chars().map(|c| if c.is_control() || is_invisible_format(c) { ' ' } else { c }).collect();
    let joined = spaced.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(max).collect::<String>().trim().to_string()
}

fn hex4(s: &str) -> Option<String> {
    let t = s.trim().to_ascii_lowercase();
    (t.len() == 4 && t.bytes().all(|b| b.is_ascii_hexdigit())).then_some(t)
}

/// Famille de pilote natif de ce VID/PID ("" si aucun).
fn driver_family(vid: &str, pid: &str) -> String {
    let (Ok(v), Ok(p)) = (u16::from_str_radix(vid, 16), u16::from_str_radix(pid, 16)) else { return String::new() };
    if let Some((f, _)) = native::probe(v, p) {
        return f.id.to_string();
    }
    known::find_known(v, p)
        .and_then(|k| k.native_driver)
        .map(|d| format!("{d:?}").to_ascii_lowercase())
        .unwrap_or_default()
}

/// Construit la fiche. Sans consentement explicite : refus. Entrées invalides : refus (rien n'est deviné).
#[allow(clippy::too_many_arguments)]
pub fn build_fiche(vid: &str, pid: &str, manufacturer: &str, product: &str, verdict: &str, note: &str, consent: bool, app_version: &str) -> Result<Fiche, String> {
    if !consent {
        return Err("Consentement requis avant l'envoi de la fiche".into());
    }
    let vid = hex4(vid).ok_or("VID invalide")?;
    let pid = hex4(pid).ok_or("PID invalide")?;
    let verdict = Verdict::parse(verdict).ok_or("Résultat invalide")?;
    Ok(Fiche {
        driver_family: driver_family(&vid, &pid),
        vid,
        pid,
        app_version: app_version.to_string(),
        verdict,
        manufacturer: sanitize_text(manufacturer, NAME_MAX),
        product: sanitize_text(product, NAME_MAX),
        note: sanitize_text(note, NOTE_MAX),
    })
}

/// Envoi unique, délai court, aucun nouvel essai. Le serveur répond `{"ok":true}`.
pub fn send(f: &Fiche) -> Result<()> {
    let payload = serde_json::to_string(f).context("sérialisation de la fiche")?;
    let tmp = std::env::temp_dir().join(format!("purergb_feedback_{}.json", std::process::id()));
    std::fs::write(&tmp, &payload).context("écriture de la fiche temporaire")?;
    let url = format!("{TELEMETRY_BASE_URL}/device-feedback");
    let res = curl(&["-X", "POST", "--max-time", "5", "-H", "Content-Type: application/json", "--data-binary", &format!("@{}", tmp.display()), &url]);
    let _ = std::fs::remove_file(&tmp);
    ensure_accepted(&res?)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityEntry {
    pub vid: String,
    pub pid: String,
    pub works: u64,
    pub partial: u64,
    pub broken: u64,
}

/// Réponse du serveur -> entrées valides. Tout ce qui est aberrant (VID/PID mal formé, compte négatif, absurde ou
/// non entier, doublon) est ignoré ; une réponse qui n'est pas un tableau est une erreur.
pub fn parse_community(body: &str) -> Result<Vec<CommunityEntry>> {
    let v: serde_json::Value = serde_json::from_str(body.trim()).context("réponse communautaire illisible")?;
    let arr = v.as_array().context("réponse communautaire : tableau attendu")?;
    let count = |e: &serde_json::Value, k: &str| e.get(k).and_then(|n| n.as_u64()).filter(|n| *n <= COUNT_MAX);
    let mut out: Vec<CommunityEntry> = Vec::new();
    for e in arr.iter().take(ENTRIES_MAX * 2) {
        let (Some(vid), Some(pid)) = (e.get("vid").and_then(|s| s.as_str()).and_then(hex4), e.get("pid").and_then(|s| s.as_str()).and_then(hex4)) else { continue };
        let (Some(works), Some(partial), Some(broken)) = (count(e, "works"), count(e, "partial"), count(e, "broken")) else { continue };
        if out.iter().any(|x| x.vid == vid && x.pid == pid) {
            continue;
        }
        out.push(CommunityEntry { vid, pid, works, partial, broken });
        if out.len() >= ENTRIES_MAX {
            break;
        }
    }
    Ok(out)
}

#[derive(Debug, Serialize)]
pub struct CommunityList {
    pub entries: Vec<CommunityEntry>,
    /// Vrai si le serveur était injoignable (liste lue dans le cache local, éventuellement vide).
    pub offline: bool,
}

fn cache_path() -> Option<std::path::PathBuf> {
    dirs_dir().map(|d| d.join("community_feedback_cache.json"))
}

/// Réseau d'abord (3 s), sinon cache local : fonctionne hors-ligne.
pub fn community_list() -> CommunityList {
    let url = format!("{TELEMETRY_BASE_URL}/device-feedback");
    match curl(&["--max-time", "3", &url]).and_then(|b| parse_community(&b)) {
        Ok(entries) => {
            if let (Some(p), Ok(json)) = (cache_path(), serde_json::to_string(&entries)) {
                if let Some(dir) = p.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = std::fs::write(&p, json) {
                    log::debug!("cache communautaire non écrit : {e}");
                }
            }
            CommunityList { entries, offline: false }
        }
        Err(e) => {
            log::debug!("retours communautaires injoignables : {e:#}");
            let cached = cache_path()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .and_then(|t| parse_community(&t).ok())
                .unwrap_or_default();
            CommunityList { entries: cached, offline: true }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fiche(consent: bool, note: &str) -> Result<Fiche, String> {
        build_fiche("1B1C", "0c0b", "Corsair", "Lighting Node Pro", "works", note, consent, "0.45.0")
    }

    #[test]
    fn refuses_without_consent() {
        assert!(fiche(false, "ok").unwrap_err().contains("Consentement"));
    }

    #[test]
    fn builds_exact_payload() {
        let f = fiche(true, "ok").unwrap();
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            r#"{"vid":"1b1c","pid":"0c0b","app_version":"0.45.0","driver_family":"corsairlightingnode","verdict":"works","manufacturer":"Corsair","product":"Lighting Node Pro","note":"ok"}"#
        );
    }

    #[test]
    fn rejects_bad_ids_and_verdicts() {
        assert!(build_fiche("zz", "0c0b", "", "", "works", "", true, "1.0.0").is_err());
        assert!(build_fiche("1b1c", "0c0b0", "", "", "works", "", true, "1.0.0").is_err());
        assert!(build_fiche("1b1c", "0c0b", "", "", "great", "", true, "1.0.0").is_err());
    }

    #[test]
    fn note_is_cleaned_and_bounded() {
        let evil = format!("  a\u{0}b\u{202E}c\u{200B}d\n\n\te {}", "x".repeat(500));
        let n = sanitize_text(&evil, NOTE_MAX);
        assert!(n.starts_with("a b c d e "));
        assert_eq!(n.chars().count(), NOTE_MAX);
        assert!(!n.chars().any(|c| c.is_control() || is_invisible_format(c)));
        assert_eq!(sanitize_text(" \t\n ", 10), "");
        assert_eq!(sanitize_text(&"é".repeat(400), NOTE_MAX).chars().count(), NOTE_MAX);
    }

    #[test]
    fn markup_stays_inert_text() {
        assert_eq!(sanitize_text("<img src=x onerror=alert(1)>", 100), "<img src=x onerror=alert(1)>");
    }

    #[test]
    fn parse_keeps_valid_and_drops_aberrant() {
        let body = r#"[
            {"vid":"1b1c","pid":"0c0b","works":3,"partial":1,"broken":0},
            {"vid":"1B1C","pid":"0c0b","works":9,"partial":9,"broken":9},
            {"vid":"zzzz","pid":"0c0b","works":1,"partial":0,"broken":0},
            {"vid":"aaaa","pid":"bbbb","works":-1,"partial":0,"broken":0},
            {"vid":"aaaa","pid":"bbbb","works":1.5,"partial":0,"broken":0},
            {"vid":"aaaa","pid":"bbbb","works":99999999999,"partial":0,"broken":0},
            {"vid":"aaaa","pid":"bbbb","works":"3","partial":0,"broken":0},
            {"vid":"cccc","pid":"dddd","works":0,"partial":0},
            "x", 5, null,
            {"vid":"1234","pid":"5678","works":0,"partial":2,"broken":4,"note":"<script>"}
        ]"#;
        let got = parse_community(body).unwrap();
        assert_eq!(
            got,
            vec![
                CommunityEntry { vid: "1b1c".into(), pid: "0c0b".into(), works: 3, partial: 1, broken: 0 },
                CommunityEntry { vid: "1234".into(), pid: "5678".into(), works: 0, partial: 2, broken: 4 },
            ]
        );
    }

    #[test]
    fn parse_rejects_non_array() {
        assert!(parse_community(r#"{"error":"x"}"#).is_err());
        assert!(parse_community("").is_err());
        assert!(parse_community("<html>").is_err());
    }
}
