//! Logiciels constructeur en conflit.

use serde::Serialize;
use tauri::State;
use crate::{conflicts, settings};
use crate::AppState;

#[derive(Serialize)]
pub(crate) struct ConflictReport {
    conflicts: Vec<conflicts::ConflictingSoftware>,
    openrgb_running: bool,
    guarded_families: std::collections::HashSet<String>,
}

/// Énumère processus ET services : passe par PowerShell, donc async.
#[tauri::command(async)]
pub(crate) fn check_conflicts(state: State<AppState>) -> ConflictReport {
    let (conflicts, openrgb_running) = conflicts::scan();
    ConflictReport {
        conflicts,
        openrgb_running,
        guarded_families: state.settings.lock().guarded_families.clone(),
    }
}

/// Stoppe une famille de logiciels en conflit (services + processus).
/// `disable` : désactive aussi le démarrage automatique des services
/// (mode d'origine sauvegardé pour `conflict_restore`).
#[tauri::command(async)]
pub(crate) fn conflict_stop(state: State<AppState>, family: String, disable: bool) -> Result<(), String> {
    let modes = conflicts::stop_family(&family, disable).map_err(|e| format!("{e:#}"))?;
    if !modes.is_empty() {
        let mut s = state.settings.lock();
        s.disabled_services.extend(modes);
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Réactive une famille : StartupType restauré + services relancés.
#[tauri::command(async)]
pub(crate) fn conflict_restore(state: State<AppState>, family: String) -> Result<(), String> {
    let saved = state.settings.lock().disabled_services.clone();
    let restored = conflicts::restore_family(&family, &saved).map_err(|e| format!("{e:#}"))?;
    let mut s = state.settings.lock();
    for name in restored {
        s.disabled_services.remove(&name);
    }
    // Réactiver = incohérent avec la garde active : la retirer.
    s.guarded_families.remove(&family);
    settings::save(&s).map_err(|e| e.to_string())
}

/// Active/désactive la garde continue d'une famille (re-tue le processus en
/// boucle tant que l'app tourne) — pour les logiciels qui se relancent
/// malgré service désactivé + tâche planifiée neutralisée.
#[tauri::command(async)]
pub(crate) fn conflict_guard_set(state: State<AppState>, family: String, enabled: bool) -> Result<(), String> {
    let mut s = state.settings.lock();
    if enabled {
        s.guarded_families.insert(family);
    } else {
        s.guarded_families.remove(&family);
    }
    settings::save(&s).map_err(|e| e.to_string())
}
