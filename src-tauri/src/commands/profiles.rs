//! Réglages, démarrage avec Windows et profils exportables.

use crate::backends::hid::HidBackend;
use crate::backends::openrgb::OpenRgbBackend;
use crate::backends::smbus::SmbusBackend;
use crate::settings::Settings;
use tauri::State;
use crate::settings;
use crate::{restore_saved_state, AppState};

/// Active/désactive le lancement au démarrage de Windows via une tâche
/// planifiée niveau Highest (pas de prompt UAC à chaque boot).
#[tauri::command(async)]
pub(crate) fn set_autostart(state: State<AppState>, enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // Tâche HIGHEST = admin sans UAC : refusée si l'exe est remplaçable par
    // un processus non élevé (hors %ProgramFiles%).
    if enabled && !crate::trust::is_trusted(&exe) {
        return Err("Démarrage automatique indisponible : installez PureRGB avec             l'installeur (dossier Program Files). Cette copie est dans un dossier             modifiable sans droits administrateur.".into());
    }
    let run = |args: &[&str]| -> Result<std::process::Output, String> {
        let mut cmd = std::process::Command::new("schtasks.exe");
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.output().map_err(|e| e.to_string())
    };
    if enabled {
        let tr = format!("\"{}\"", exe.display());
        let out = run(&[
            "/Create", "/TN", "PureRGB Autostart", "/TR", &tr, "/SC", "ONLOGON",
            "/RL", "HIGHEST", "/F",
        ])?;
        if !out.status.success() {
            return Err(format!(
                "schtasks: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    } else {
        let _ = run(&["/Delete", "/TN", "PureRGB Autostart", "/F"]);
    }
    let mut s = state.settings.lock();
    s.autostart = enabled;
    settings::save(&s).map_err(|e| e.to_string())
}

/// L'application tourne en administrateur : un chemin venu de la WebView ne
/// doit ni lire ni écrire hors du dossier de l'utilisateur. Seuls les `.json`
/// sous `home` sont acceptés, après résolution des liens (le dossier home est
/// canonicalisé lui aussi, sinon le préfixe `\\?\` de `canonicalize` fait
/// échouer toute comparaison).
pub(crate) fn confine_profile_path(path: &str, home: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);
    let is_json = p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !p.is_absolute() || !is_json {
        return Err("chemin de profil refusé (fichier .json attendu)".into());
    }
    let home = home.canonicalize().map_err(|e| format!("dossier utilisateur : {e}"))?;
    let name = p.file_name().ok_or("chemin de profil refusé")?;
    let parent = p
        .parent()
        .ok_or("chemin de profil refusé")?
        .canonicalize()
        .map_err(|e| format!("dossier du profil : {e}"))?;
    let target = if p.exists() {
        p.canonicalize().map_err(|e| e.to_string())?
    } else {
        parent.join(name)
    };
    if !target.starts_with(&home) {
        return Err("chemin de profil refusé : hors du dossier utilisateur".into());
    }
    Ok(target)
}

pub(crate) fn user_home() -> Result<std::path::PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "dossier utilisateur introuvable".to_string())
}

/// Exporte la configuration complète (effets, courbes, modes, réglages).
#[tauri::command]
pub(crate) fn profile_export(state: State<AppState>, path: String) -> Result<(), String> {
    let path = confine_profile_path(&path, &user_home()?)?;
    let s = state.settings.lock();
    let json = serde_json::to_string_pretty(&*s).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Importe un profil et l'applique immédiatement.
#[tauri::command(async)]
pub(crate) fn profile_import(state: State<AppState>, path: String) -> Result<(), String> {
    let path = confine_profile_path(&path, &user_home()?)?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: Settings = serde_json::from_str(&text).map_err(|e| format!("profil invalide: {e}"))?;
    {
        let mut s = state.settings.lock();
        *s = imported.clone();
        settings::save(&s).map_err(|e| e.to_string())?;
    }
    state.engine.set_fps(imported.fps);
    state.curve_engine.set_curves(imported.curves.clone());
    restore_saved_state(&state.registry, &state.engine, &imported);
    Ok(())
}

#[tauri::command]
pub(crate) fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
pub(crate) fn update_settings(
    state: State<AppState>,
    openrgb_host: String,
    openrgb_port: u16,
    auto_start_openrgb: bool,
    native_drivers_enabled: bool,
    smbus_writes_enabled: bool,
    fps: u32,
    start_minimized: bool,
    auto_manage_conflicts: bool,
    telemetry_opt_in: bool,
    auto_update_check: bool,
) -> Result<(), String> {
    // Validation input : host non vide, fps borné.
    let host = openrgb_host.trim().to_string();
    if host.is_empty() || host.len() > 253 {
        return Err("hôte OpenRGB invalide".into());
    }
    let fps = fps.clamp(5, 144);

    {
        let mut reg = state.registry.lock();
        for b in reg.backends_mut() {
            if b.name() == "openrgb" {
                if let Some(orgb) = b.as_any_mut().downcast_mut::<OpenRgbBackend>() {
                    orgb.set_endpoint(host.clone(), openrgb_port);
                }
            } else if b.name() == "hid" {
                if let Some(hid) = b.as_any_mut().downcast_mut::<HidBackend>() {
                    hid.set_native_enabled(native_drivers_enabled);
                }
            } else if b.name() == "smbus" {
                if let Some(sm) = b.as_any_mut().downcast_mut::<SmbusBackend>() {
                    sm.set_native_enabled(native_drivers_enabled);
                    sm.set_writes_enabled(smbus_writes_enabled);
                }
            }
        }
    }
    state.engine.set_fps(fps);

    let mut s = state.settings.lock();
    s.openrgb_host = host;
    s.openrgb_port = openrgb_port;
    s.auto_start_openrgb = auto_start_openrgb;
    s.native_drivers_enabled = native_drivers_enabled;
    s.smbus_writes_enabled = smbus_writes_enabled;
    s.fps = fps;
    s.start_minimized = start_minimized;
    s.auto_manage_conflicts = auto_manage_conflicts;
    s.telemetry_opt_in = telemetry_opt_in;
    s.auto_update_check = auto_update_check;
    settings::save(&s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod profile_path_tests {
    use super::confine_profile_path;

    #[test]
    fn profile_path_must_be_a_json_inside_home() {
        let base = std::env::temp_dir().join(format!("purergb-prof-{}", std::process::id()));
        let home = base.join("home");
        let outside = base.join("outside");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let ok = confine_profile_path(&home.join("p.json").to_string_lossy(), &home);
        let other_dir = confine_profile_path(&outside.join("p.json").to_string_lossy(), &home);
        let not_json = confine_profile_path(&home.join("p.exe").to_string_lossy(), &home);
        let traversal = confine_profile_path(&home.join("..").join("outside").join("p.json").to_string_lossy(), &home);
        let relative = confine_profile_path("p.json", &home);
        std::fs::remove_dir_all(&base).ok();
        assert!(ok.is_ok(), "{ok:?}");
        assert!(other_dir.is_err());
        assert!(not_json.is_err());
        assert!(traversal.is_err());
        assert!(relative.is_err());
    }
}
