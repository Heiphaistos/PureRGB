//! Commandes Tauri des photos de produits (identification sans réseau ; photo = téléchargement
//! borné, voir `product_images`). Désactivable dans les réglages : alors aucune requête réseau.

use crate::product_images::{extract_usb, fetch_image, identify, product_by_id};
use crate::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
pub struct ProductView {
    id: String,
    kind: crate::product_images::Kind,
    vendor: String,
    name: String,
    has_image: bool,
    source_page: Option<String>,
}

/// Identifie un appareil. `usb` : `vid:pid` ou un id d'appareil (`hid:vid:pid:…`, `vid:pid:série`).
#[tauri::command]
pub fn product_lookup(vendor: String, name: String, usb: Option<String>) -> Option<ProductView> {
    let usb = usb.as_deref().and_then(extract_usb);
    let ids: Vec<&str> = usb.iter().map(String::as_str).collect();
    identify(&vendor, &name, &ids).map(|p| ProductView {
        id: p.id.clone(),
        kind: p.kind,
        vendor: p.vendor.clone(),
        name: p.name.clone(),
        has_image: p.image.is_some(),
        source_page: p.image.as_ref().map(|i| i.source_page.clone()),
    })
}

/// Photo du produit en data URL PNG (téléchargée au besoin, mise en cache sur disque).
#[tauri::command]
pub async fn product_image(state: State<'_, AppState>, id: String) -> Result<String, String> {
    if !state.settings.lock().product_photos {
        return Err("L'affichage des photos de produits est désactivé dans les réglages".into());
    }
    let product = product_by_id(&id).ok_or("Produit inconnu du catalogue")?;
    let path = tauri::async_runtime::spawn_blocking(move || fetch_image(product))
        .await
        .map_err(|e| format!("Tâche photo interrompue : {e}"))?
        .map_err(|e| format!("Photo indisponible : {e:#}"))?;
    let bytes = std::fs::read(&path).map_err(|e| format!("Lecture de la photo impossible : {e}"))?;
    Ok(format!("data:image/png;base64,{}", crate::screens::render::base64(&bytes)))
}

/// Ouvre la page du fabricant (URL du catalogue embarqué uniquement).
#[tauri::command]
pub fn product_open_source(app: AppHandle, id: String) -> Result<(), String> {
    let url = product_by_id(&id)
        .and_then(|p| p.image.as_ref())
        .map(|i| i.source_page.clone())
        .ok_or("Aucune page source pour ce produit")?;
    app.opener().open_url(url, None::<&str>).map_err(|e| format!("Ouverture impossible : {e}"))
}

#[tauri::command]
pub fn set_product_photos(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut s = state.settings.lock();
    s.product_photos = enabled;
    crate::settings::save(&s).map_err(|e| e.to_string())
}
