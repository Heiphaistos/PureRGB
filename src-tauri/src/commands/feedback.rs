//! Commandes du catalogue communautaire (voir `device_feedback.rs`).

use crate::device_feedback::{self, CommunityList};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Aperçu : le JSON EXACT qui serait envoyé (même constructeur que l'envoi). Le consentement n'est pas requis
/// pour regarder, il l'est pour envoyer.
#[tauri::command(async)]
pub(crate) fn device_feedback_preview(vid: String, pid: String, manufacturer: String, product: String, verdict: String, note: String) -> Result<String, String> {
    let f = device_feedback::build_fiche(&vid, &pid, &manufacturer, &product, &verdict, &note, true, VERSION)?;
    serde_json::to_string_pretty(&f).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) fn device_feedback_send(vid: String, pid: String, manufacturer: String, product: String, verdict: String, note: String, consent: bool) -> Result<(), String> {
    let f = device_feedback::build_fiche(&vid, &pid, &manufacturer, &product, &verdict, &note, consent, VERSION)?;
    device_feedback::send(&f).map_err(|e| format!("{e:#}"))
}

#[tauri::command(async)]
pub(crate) fn community_feedback_list() -> CommunityList {
    device_feedback::community_list()
}
