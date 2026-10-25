//! Détection d'un OpenRGB.exe résiduel (lancé par PureRGB, figé) qui garde le
//! port du serveur SDK sans répondre au protocole. Seules NOS copies
//! (`%ProgramFiles%\PureRGB\openrgb` ou ressources du bundle) sont terminées :
//! jamais un OpenRGB de l'utilisateur installé ailleurs.

use super::manager::CreationFlagsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Processus qui écoute sur le port.
#[derive(Debug, Clone)]
pub struct PortOwner {
    pub pid: u32,
    pub name: String,
    pub exe: Option<PathBuf>,
    /// Ancienneté du processus en secondes.
    pub age_secs: u64,
}

/// Décision face au port du serveur.
#[derive(Debug, PartialEq, Eq)]
pub enum PortAction {
    /// Rien n'écoute : on peut lancer OpenRGB.
    Launch,
    /// Notre OpenRGB figé tient le port : le terminer puis relancer.
    KillStale(u32),
    /// Un autre programme (ou l'OpenRGB de l'utilisateur) tient le port.
    Foreign,
}

/// Fonction pure : que faire d'un port non servi (le cas « déjà servi » est traité avant).
pub fn decide(owner: Option<&PortOwner>, our_dirs: &[PathBuf]) -> PortAction {
    let Some(o) = owner else { return PortAction::Launch };
    let ours = o.exe.as_deref().is_some_and(|e| {
        crate::trust::is_under(e, our_dirs)
            && e.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("OpenRGB.exe"))
    });
    if ours { PortAction::KillStale(o.pid) } else { PortAction::Foreign }
}

/// Texte lisible : « OpenRGB.exe (PID 12, lancé il y a 3 min 5 s) ».
pub fn describe(o: &PortOwner) -> String {
    let age = if o.age_secs >= 60 { format!("{} min {} s", o.age_secs / 60, o.age_secs % 60) } else { format!("{} s", o.age_secs) };
    format!("« {} » (PID {}, lancé il y a {age})", o.name, o.pid)
}

/// Programme qui écoute sur ce port TCP local, d'après `netstat`.
pub fn port_owner(port: u16) -> Option<PortOwner> {
    let out = Command::new("netstat.exe").args(["-ano", "-p", "TCP"]).creation_flags_no_window().output().ok()?;
    let pid = parse_listener(&String::from_utf8_lossy(&out.stdout), port)?;
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing()
            .with_processes(sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::Always)),
    );
    let p = sys.process(sysinfo::Pid::from_u32(pid));
    Some(PortOwner {
        pid,
        name: p.map(|p| p.name().to_string_lossy().into_owned()).unwrap_or_else(|| "inconnu".into()),
        exe: p.and_then(|p| p.exe().map(Path::to_path_buf)),
        age_secs: p.map(|p| p.run_time()).unwrap_or(0),
    })
}

/// PID du processus en écoute sur `port` dans la sortie de `netstat -ano`.
fn parse_listener(netstat: &str, port: u16) -> Option<u32> {
    let suffix = format!(":{port}");
    netstat.lines().filter(|l| l.contains("LISTENING")).find_map(|l| {
        let cols: Vec<&str> = l.split_whitespace().collect();
        if cols.get(1)?.ends_with(&suffix) { cols.last()?.parse::<u32>().ok() } else { None }
    })
}

/// Termine le processus (OpenRGB n'a pas de fermeture douce hors fenêtre) et
/// attend jusqu'à 4 s que le port soit libéré.
pub fn kill_and_wait(pid: u32, port: u16) -> bool {
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    );
    if let Some(p) = sys.process(sysinfo::Pid::from_u32(pid)) {
        p.kill();
    }
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if port_owner(port).is_none() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(exe: Option<&str>) -> PortOwner {
        PortOwner { pid: 42, name: "OpenRGB.exe".into(), exe: exe.map(PathBuf::from), age_secs: 200 }
    }
    fn dirs() -> Vec<PathBuf> {
        vec![PathBuf::from(r"C:\Program Files\PureRGB\openrgb")]
    }

    #[test]
    fn free_port_launches() {
        assert_eq!(decide(None, &dirs()), PortAction::Launch);
    }

    #[test]
    fn our_stale_copy_is_killed() {
        let o = owner(Some(r"C:\Program Files\PureRGB\openrgb\OpenRGB.exe"));
        assert_eq!(decide(Some(&o), &dirs()), PortAction::KillStale(42));
    }

    #[test]
    fn users_openrgb_is_never_killed() {
        let o = owner(Some(r"C:\Program Files\OpenRGB\OpenRGB.exe"));
        assert_eq!(decide(Some(&o), &dirs()), PortAction::Foreign);
        let o = owner(Some(r"C:\Users\x\Downloads\OpenRGB\OpenRGB.exe"));
        assert_eq!(decide(Some(&o), &dirs()), PortAction::Foreign);
    }

    #[test]
    fn unknown_path_or_other_exe_is_foreign() {
        assert_eq!(decide(Some(&owner(None)), &dirs()), PortAction::Foreign);
        let o = owner(Some(r"C:\Program Files\PureRGB\openrgb\autre.exe"));
        assert_eq!(decide(Some(&o), &dirs()), PortAction::Foreign);
        let o = owner(Some(r"C:\Program Files\PureRGB\openrgb\..\..\x\OpenRGB.exe"));
        assert_eq!(decide(Some(&o), &dirs()), PortAction::Foreign);
    }

    #[test]
    fn describes_age() {
        assert_eq!(describe(&owner(None)), "« OpenRGB.exe » (PID 42, lancé il y a 3 min 20 s)");
    }

    #[test]
    fn parses_netstat() {
        let s = "  TCP    127.0.0.1:6800   0.0.0.0:0   LISTENING   1234\n  TCP    0.0.0.0:68000   0.0.0.0:0   LISTENING   9\n  TCP  127.0.0.1:6800 127.0.0.1:5 ESTABLISHED 7";
        assert_eq!(parse_listener(s, 6800), Some(1234));
        assert_eq!(parse_listener(s, 80), None);
    }
}
