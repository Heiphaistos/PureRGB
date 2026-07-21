//! Télémétrie matériel opt-in : envoie un snapshot diagnostic au service
//! VPS et récupère la table de reconnaissance étendue. Best-effort partout
//! — aucune erreur réseau ne doit bloquer le démarrage ni l'usage normal.

use crate::backends::hid::known_remote::{self, RemoteDevice};
use crate::netdev::curl;
use crate::settings::dirs_dir;
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const TELEMETRY_BASE_URL: &str = "https://telemetry-purergb.heiphaistos.org";

/// Hash non cryptographique (zéro dépendance) — sert uniquement à éviter
/// de renvoyer un rapport identique à chaque lancement, pas à la sécurité.
pub fn hash_diagnostics(diagnostics_json: &str) -> String {
    let mut hasher = DefaultHasher::new();
    diagnostics_json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cache_dir() -> Result<PathBuf> {
    let dir = dirs_dir().context("répertoire de config introuvable")?;
    std::fs::create_dir_all(&dir).context("création du dossier de config")?;
    Ok(dir)
}

fn report_id_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("telemetry_report_id.txt"))
}

fn last_hash_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("telemetry_last_hash.txt"))
}

fn known_devices_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("known_devices_cache.json"))
}

/// Identifiant local pseudo-aléatoire (128 bits), généré une fois et mis
/// en cache — sert uniquement à regrouper les rapports d'une même
/// installation côté dashboard ("vu N fois"), jamais à identifier une
/// personne. `RandomState` puise dans l'aléa système (protection HashDoS
/// de la std), suffisant ici et évite une dépendance `uuid` complète.
fn generate_report_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

/// Charge le report_id depuis le cache, ou en génère un et le persiste.
pub fn report_id() -> String {
    if let Ok(path) = report_id_path() {
        if let Ok(existing) = std::fs::read_to_string(&path) {
            let trimmed = existing.trim();
            if trimmed.len() == 32 {
                return trimmed.to_string();
            }
        }
        let id = generate_report_id();
        let _ = std::fs::write(&path, &id);
        return id;
    }
    generate_report_id()
}

/// Envoie le snapshot diagnostic si l'opt-in est actif ET que son contenu
/// a changé depuis le dernier envoi. Best-effort : toute erreur réseau est
/// retournée à l'appelant pour log uniquement, jamais de panique.
pub fn maybe_send_report(diagnostics_json: &str, app_version: &str) -> Result<bool> {
    if !crate::settings::load().telemetry_opt_in {
        return Ok(false);
    }
    let hash = hash_diagnostics(diagnostics_json);
    let last_hash_path = last_hash_path()?;
    if let Ok(previous) = std::fs::read_to_string(&last_hash_path) {
        if previous.trim() == hash {
            return Ok(false); // rien de nouveau à envoyer
        }
    }
    send_report_now(diagnostics_json, app_version)?;
    std::fs::write(&last_hash_path, &hash).context("écriture du cache de hash télémétrie")?;
    Ok(true)
}

/// Envoi immédiat, sans vérification de hash — utilisé par le bouton
/// "Envoyer maintenant".
pub fn send_report_now(diagnostics_json: &str, app_version: &str) -> Result<()> {
    let payload = format!(
        r#"{{"report_id":"{}","app_version":"{}","diagnostics":{}}}"#,
        report_id(),
        app_version,
        diagnostics_json
    );
    let tmp_path = std::env::temp_dir().join(format!(
        "purergb_telemetry_payload_{}.json",
        std::process::id()
    ));
    std::fs::write(&tmp_path, &payload).context("écriture du payload temporaire")?;
    let url = format!("{TELEMETRY_BASE_URL}/report");
    let result = curl(&[
        "-X",
        "POST",
        "--max-time",
        "5",
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        &format!("@{}", tmp_path.display()),
        &url,
    ]);
    let _ = std::fs::remove_file(&tmp_path);
    result.map(|_| ())
}

/// Récupère la table distante, la fusionne en mémoire (`known_remote`) et
/// la met en cache localement. Hors-ligne : réutilise le cache existant.
/// Best-effort total — ne bloque jamais le démarrage.
pub fn refresh_known_devices() {
    match fetch_known_devices() {
        Ok(devices) => {
            known_remote::set_remote(devices.clone());
            if let Ok(path) = known_devices_cache_path() {
                if let Ok(json) = serde_json::to_string(&devices) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
        Err(e) => {
            log::warn!("known-devices distant injoignable ({e:#}), utilisation du cache local");
            if let Some(cached) = load_known_devices_cache() {
                known_remote::set_remote(cached);
            }
        }
    }
}

fn fetch_known_devices() -> Result<Vec<RemoteDevice>> {
    let url = format!("{TELEMETRY_BASE_URL}/known-devices");
    let body = curl(&["--max-time", "3", &url])?;
    serde_json::from_str(&body).context("réponse known-devices illisible")
}

fn load_known_devices_cache() -> Option<Vec<RemoteDevice>> {
    let path = known_devices_cache_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable_pour_le_meme_contenu() {
        let a = hash_diagnostics(r#"{"hid_raw":[]}"#);
        let b = hash_diagnostics(r#"{"hid_raw":[]}"#);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_different_pour_contenu_different() {
        let a = hash_diagnostics(r#"{"hid_raw":[]}"#);
        let b = hash_diagnostics(r#"{"hid_raw":[{"vid":"dead"}]}"#);
        assert_ne!(a, b);
    }
}
