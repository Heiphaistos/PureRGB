//! Commandes Tauri du module Écrans (contrat côté interface : `src/screens/api.ts`).

use super::designs::DesignMeta;
use super::driver::ScreenInfo;
use super::layout::Layout;
use super::manager::{ScreenManager, ScreenOptions, ScreenState};
use super::media::MediaItem;
use super::render;
use super::theme_pack;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

type Mgr<'a> = State<'a, Arc<ScreenManager>>;
type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// Un aperçu ne dépasse pas cette taille (protège la mémoire contre une demande absurde).
const MAX_PREVIEW: u32 = 2048;

#[tauri::command]
pub fn screens_list(m: Mgr) -> Vec<ScreenInfo> {
    m.list()
}

#[tauri::command]
pub async fn screens_scan(m: Mgr<'_>, with_simulated: bool) -> Res<Vec<ScreenInfo>> {
    let mgr = m.inner().clone();
    tauri::async_runtime::spawn_blocking(move || mgr.scan(with_simulated))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn screen_state(m: Mgr, id: String) -> Res<ScreenState> {
    m.state(&id).map_err(err)
}

#[tauri::command]
pub fn screen_set_layout(m: Mgr, id: String, layout: Layout) -> Res<()> {
    m.set_layout(&id, layout).map_err(err)
}

#[tauri::command]
pub fn screen_set_options(m: Mgr, id: String, options: ScreenOptions) -> Res<()> {
    m.set_options(&id, options).map_err(err)
}

#[tauri::command]
pub fn screen_send_still(m: Mgr, id: String) -> Res<()> {
    m.send_still(&id).map_err(err)
}

#[tauri::command]
pub fn screen_clear(m: Mgr, id: String) -> Res<()> {
    m.clear(&id).map_err(err)
}

#[tauri::command]
pub fn screen_preview(m: Mgr, layout: Layout, width: u32, height: u32, t_ms: u64) -> Res<String> {
    layout.validate()?;
    if width == 0 || height == 0 || width > MAX_PREVIEW || height > MAX_PREVIEW {
        return Err("taille d'aperçu invalide".into());
    }
    let frame = render::render(&layout, &m.store, &m.media, width, height, t_ms);
    render::frame_to_png_base64(&frame).map_err(err)
}

#[tauri::command]
pub fn screen_values(m: Mgr) -> HashMap<String, f64> {
    m.store.snapshot()
}

#[tauri::command]
pub fn designs_list(m: Mgr) -> Vec<DesignMeta> {
    m.designs.list()
}

#[tauri::command]
pub fn design_get(m: Mgr, id: String) -> Res<Layout> {
    m.designs.get(&id).map_err(err)
}

#[tauri::command]
pub fn design_user_save(m: Mgr, layout: Layout) -> Res<String> {
    m.designs.save_user(&layout).map_err(err)
}

#[tauri::command]
pub fn design_user_delete(m: Mgr, id: String) -> Res<()> {
    m.designs.delete_user(&id).map_err(err)
}

#[tauri::command]
pub fn design_export(m: Mgr, id: String, path: String) -> Res<()> {
    m.designs.export(&id, Path::new(&path)).map_err(err)
}

#[tauri::command]
pub fn design_import(m: Mgr, path: String) -> Res<String> {
    m.designs.import(Path::new(&path)).map_err(err)
}

#[tauri::command]
pub fn media_list(m: Mgr) -> Vec<MediaItem> {
    m.media.list()
}

#[tauri::command]
pub fn media_import(m: Mgr, path: String) -> Res<MediaItem> {
    m.media.import(Path::new(&path)).map_err(err)
}

#[tauri::command]
pub fn media_delete(m: Mgr, id: String) -> Res<()> {
    m.media.delete(&id).map_err(err)
}

/// Exporte un thème (design + médias) : `id` d'un design de la bibliothèque, ou `layout` (éditeur, non enregistré).
#[tauri::command]
pub async fn theme_export(m: Mgr<'_>, id: Option<String>, layout: Option<Layout>, path: String) -> Res<()> {
    let layout = match (id, layout) {
        (_, Some(l)) => l,
        (Some(id), None) => m.designs.get(&id).map_err(err)?,
        (None, None) => return Err("aucun design à exporter".into()),
    };
    let media = m.media.clone();
    tauri::async_runtime::spawn_blocking(move || theme_pack::export(&layout, &media, Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// Importe un `.purergb-theme` : médias dans la médiathèque, design dans « Mes designs » ; retourne l'id du design.
#[tauri::command]
pub async fn theme_import(m: Mgr<'_>, path: String) -> Res<String> {
    let mgr = m.inner().clone();
    tauri::async_runtime::spawn_blocking(move || theme_pack::import(Path::new(&path), &mgr.designs, &mgr.media, theme_pack::LIMITS))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}
