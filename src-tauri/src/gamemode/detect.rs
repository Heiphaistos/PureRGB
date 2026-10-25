//! Mode jeu, partie pure (sans E/S) : liste d'anti-cheat versionnée (`data/gamemode_anticheat.json`),
//! décision « un jeu protégé tourne-t-il ? » et machine à états avec délai de grâce.

use serde::Deserialize;
use std::time::{Duration, Instant};

/// Délai entre la fermeture du jeu et la reprise de l'accès matériel.
pub const GRACE: Duration = Duration::from_secs(10);
/// Toujours suspendus : serveur OpenRGB, bus SMBus/RAM, capteurs et ventilateurs carte mère (sensord).
const LOW_LEVEL: [&str; 3] = ["openrgb", "smbus", "mobo"];
/// Suspendus sauf option « garder l'éclairage HID ».
const HID_LIKE: [&str; 2] = ["hid", "liquidctl"];

#[derive(Deserialize)]
struct Entry {
    name: String,
    exes: Vec<String>,
}

#[derive(Deserialize)]
struct Data {
    anticheat: Vec<Entry>,
}

/// Nom de processus normalisé : minuscules, sans `.exe`.
pub fn norm(name: &str) -> String {
    let n = name.trim().to_lowercase();
    n.strip_suffix(".exe").unwrap_or(&n).to_string()
}

pub struct Rules {
    /// (nom normalisé, libellé de l'anti-cheat).
    known: Vec<(String, String)>,
    user: Vec<String>,
}

impl Rules {
    pub fn load(user_games: &[String]) -> Rules {
        let data: Data = serde_json::from_str(include_str!("../../data/gamemode_anticheat.json")).unwrap_or_else(|e| {
            log::error!("liste anti-cheat illisible : {e}");
            Data { anticheat: Vec::new() }
        });
        let known = data.anticheat.iter().flat_map(|a| a.exes.iter().map(|e| (norm(e), a.name.clone()))).collect();
        Rules { known, user: user_games.iter().map(|g| norm(g)).filter(|g| !g.is_empty()).collect() }
    }

    /// Premier processus protégé trouvé (libellé affichable), s'il y en a un.
    pub fn detect<'a>(&self, process_names: impl IntoIterator<Item = &'a str>) -> Option<String> {
        for p in process_names {
            let p = norm(p);
            if let Some((_, label)) = self.known.iter().find(|(k, _)| *k == p) {
                return Some(label.clone());
            }
            if self.user.contains(&p) {
                return Some(p);
            }
        }
        None
    }
}

/// Backends à ne plus toucher quand le mode jeu est actif.
pub fn suspended_backends(keep_hid: bool) -> Vec<&'static str> {
    let mut v = LOW_LEVEL.to_vec();
    if !keep_hid {
        v.extend(HID_LIKE);
    }
    v
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    Active(String),
    /// Jeu fermé, reprise dans `GRACE` : (dernier jeu, début du délai).
    Grace(String, Instant),
}

#[derive(Debug, PartialEq)]
pub enum Transition {
    None,
    Enter(String),
    Exit,
}

impl Phase {
    /// Avance d'un relevé. `enabled` faux (réglage coupé) ramène tout de suite à l'état normal.
    pub fn step(&mut self, detected: Option<String>, enabled: bool, now: Instant) -> Transition {
        let detected = detected.filter(|_| enabled);
        match (&*self, detected) {
            (Phase::Idle, Some(g)) => {
                *self = Phase::Active(g.clone());
                Transition::Enter(g)
            }
            (Phase::Idle, None) => Transition::None,
            (Phase::Active(_) | Phase::Grace(..), Some(g)) => {
                *self = Phase::Active(g);
                Transition::None
            }
            (Phase::Active(g), None) if enabled => {
                *self = Phase::Grace(g.clone(), now);
                Transition::None
            }
            (Phase::Grace(_, since), None) if enabled && now.duration_since(*since) < GRACE => Transition::None,
            _ => {
                *self = Phase::Idle;
                Transition::Exit
            }
        }
    }

    pub fn game(&self) -> Option<&str> {
        match self {
            Phase::Idle => None,
            Phase::Active(g) | Phase::Grace(g, _) => Some(g),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        Rules::load(&["MonJeu.exe".to_string(), "  ".to_string()])
    }

    #[test]
    fn data_file_parses_and_names_are_unique() {
        let r = Rules::load(&[]);
        assert!(r.known.len() >= 15);
        let mut names: Vec<&String> = r.known.iter().map(|(k, _)| k).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len(), "doublon dans la liste");
    }

    #[test]
    fn detects_known_anticheat_ignoring_case_and_extension() {
        let r = rules();
        assert_eq!(r.detect(["explorer.exe", "EASYANTICHEAT.EXE"]).as_deref(), Some("Easy Anti-Cheat"));
        assert_eq!(r.detect(["beservice"]).as_deref(), Some("BattlEye"));
        assert_eq!(r.detect(["vgc.exe"]).as_deref(), Some("Riot Vanguard"));
        assert_eq!(r.detect(["EAAntiCheat.GameService.exe"]).as_deref(), Some("EA Javelin"));
        assert_eq!(r.detect(["npggsvc.exe"]).as_deref(), Some("nProtect GameGuard"));
    }

    #[test]
    fn detects_user_games_and_ignores_the_rest() {
        let r = rules();
        assert_eq!(r.detect(["monjeu.EXE"]).as_deref(), Some("monjeu"));
        assert_eq!(r.detect(["chrome.exe", "purergb.exe", "easyanticheat_setup.exe", ""]), None);
    }

    #[test]
    fn hid_is_suspended_unless_kept() {
        assert!(suspended_backends(false).contains(&"hid") && suspended_backends(false).contains(&"liquidctl"));
        let keep = suspended_backends(true);
        assert!(!keep.contains(&"hid") && keep.contains(&"smbus") && keep.contains(&"openrgb") && keep.contains(&"mobo"));
    }

    #[test]
    fn game_start_then_stop_waits_for_the_grace_delay() {
        let t0 = Instant::now();
        let mut p = Phase::Idle;
        assert_eq!(p.step(None, true, t0), Transition::None);
        assert_eq!(p.step(Some("EAC".into()), true, t0), Transition::Enter("EAC".into()));
        assert_eq!(p.step(Some("EAC".into()), true, t0 + Duration::from_secs(4)), Transition::None);
        assert_eq!(p.step(None, true, t0 + Duration::from_secs(8)), Transition::None);
        assert!(matches!(p, Phase::Grace(..)));
        assert_eq!(p.step(None, true, t0 + Duration::from_secs(17)), Transition::None);
        assert_eq!(p.step(None, true, t0 + Duration::from_secs(18)), Transition::Exit);
        assert_eq!(p, Phase::Idle);
    }

    #[test]
    fn relaunch_during_grace_keeps_the_mode_active() {
        let t0 = Instant::now();
        let mut p = Phase::Active("BattlEye".into());
        p.step(None, true, t0);
        assert_eq!(p.step(Some("BattlEye".into()), true, t0 + Duration::from_secs(5)), Transition::None);
        assert_eq!(p, Phase::Active("BattlEye".into()));
        assert_eq!(p.step(None, true, t0 + Duration::from_secs(6)), Transition::None);
    }

    #[test]
    fn disabling_the_setting_resumes_immediately_and_blocks_new_entries() {
        let t0 = Instant::now();
        let mut p = Phase::Active("FACEIT".into());
        assert_eq!(p.step(Some("FACEIT".into()), false, t0), Transition::Exit);
        assert_eq!(p.step(Some("FACEIT".into()), false, t0), Transition::None);
        assert_eq!(p, Phase::Idle);
    }
}
