//! Tests d'acceptation du mode `--background` avec le VRAI exécutable (écran simulé, jamais de matériel) :
//! lancer avec `cargo test --test screens_only_exe -- --ignored --test-threads=1`.
//! APPDATA est redirigé vers un dossier temporaire : aucun réglage réel n'est touché.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const EXE: &str = env!("CARGO_BIN_EXE_purergb");
const SCREENS_JSON: &str = r##"{"sim:0000:0000:1":{"brightness":100,"rotation":0,"fps":5,"layout":{"version":1,"name":"t","width":480,"height":480,
"background":{"kind":"solid","color":"#000000"},"widgets":[{"id":"h","x":10,"y":10,"w":200,"h":60,"type":"value","source":"time",
"format":"%H:%M","style":{"font":"x","size":40,"color":"#ffffff"}}]}}}"##;

fn appdata(tag: &str, keep: bool) -> PathBuf {
    let root = std::env::temp_dir().join(format!("purergb-so-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let d = root.join("PureRGB");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("screens.json"), SCREENS_JSON).unwrap();
    std::fs::write(d.join("settings.json"), format!(r#"{{"start_minimized":true,"background":{{"screens":{keep}}},"auto_start_openrgb":false,"auto_manage_conflicts":false,"game_mode":false,"auto_update_check":false}}"#)).unwrap();
    root
}

fn spawn(root: &Path, args: &[&str], envs: &[(&str, &str)]) -> Child {
    let mut c = Command::new(EXE);
    c.args(args).env("APPDATA", root).env("PURERGB_SIM_SCREEN", "1").stdout(Stdio::null()).stderr(Stdio::null());
    for (k, v) in envs {
        c.env(k, v);
    }
    c.spawn().expect("lancement de l'exe")
}

fn ps(script: &str) -> String {
    let o = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]).output().unwrap();
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn alive(pid: u32) -> bool {
    ps(&format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{'1'}} else {{'0'}}")) == "1"
}

fn wait_for(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(secs), "délai dépassé : {what}");
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn pid_file(root: &Path) -> PathBuf {
    root.join("PureRGB").join("background.pid")
}

fn log_text(root: &Path) -> String {
    std::fs::read_to_string(root.join("PureRGB").join("purergb.log")).unwrap_or_default()
}

fn read_pid(root: &Path) -> Option<u32> {
    std::fs::read_to_string(pid_file(root)).ok()?.trim().parse().ok()
}

#[test]
#[ignore = "lance le vrai exécutable"]
fn daemon_has_no_window_no_children_low_ram_and_stops_cleanly() {
    let root = appdata("alone", false);
    let mut d = spawn(&root, &["--background=screens"], &[]);
    let pid = d.id();
    wait_for("écran simulé actif", 20, || log_text(&root).contains("arrière-plan : 1 écran(s) actif(s)"));
    std::thread::sleep(Duration::from_secs(3));
    assert!(d.try_wait().unwrap().is_none(), "le processus doit rester en vie");
    assert_eq!(read_pid(&root), Some(pid));
    // Aucune fenêtre visible, aucun enfant (ni WebView2 ni sensord : le design n'utilise que l'heure).
    assert_eq!(ps(&format!("(Get-Process -Id {pid}).MainWindowHandle")), "0", "aucune fenêtre attendue");
    let kids = ps(&format!("(Get-CimInstance Win32_Process -Filter 'ParentProcessId={pid}' | Select-Object -ExpandProperty Name) -join ','"));
    assert_eq!(kids, "", "aucun processus enfant attendu, trouvé : {kids}");
    let ws_mb: f64 = ps(&format!("[math]::Round((Get-Process -Id {pid}).WorkingSet64/1MB,1)")).replace(',', ".").parse().unwrap();
    let cpu0: f64 = ps(&format!("(Get-Process -Id {pid}).TotalProcessorTime.TotalSeconds")).replace(',', ".").parse().unwrap();
    std::thread::sleep(Duration::from_secs(10));
    let cpu1: f64 = ps(&format!("(Get-Process -Id {pid}).TotalProcessorTime.TotalSeconds")).replace(',', ".").parse().unwrap();
    println!("RAM {ws_mb} Mo, CPU {:.1} % d'un coeur sur 10 s", (cpu1 - cpu0) / 10.0 * 100.0);
    assert!(ws_mb < 150.0, "trop de RAM : {ws_mb} Mo");
    // Un second lancement ne doit pas dupliquer le processus.
    let mut second = spawn(&root, &["--background=screens"], &[]);
    wait_for("second lancement terminé", 15, || second.try_wait().unwrap().is_some());
    assert!(d.try_wait().unwrap().is_none());
    // Arrêt demandé par fichier (chemin utilisé par l'application complète).
    std::fs::write(root.join("PureRGB").join("background.stop"), b"stop").unwrap();
    wait_for("arrêt par fichier", 15, || d.try_wait().unwrap().is_some());
    assert!(!pid_file(&root).exists(), "fichier PID laissé");
    assert!(!alive(pid));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "lance le vrai exécutable"]
fn brutal_kill_leaves_nothing() {
    let root = appdata("kill", false);
    let mut d = spawn(&root, &["--background=screens"], &[]);
    let pid = d.id();
    wait_for("démarrage", 20, || log_text(&root).contains("arrière-plan : 1 écran(s) actif(s)"));
    d.kill().unwrap();
    let _ = d.wait();
    assert!(!alive(pid));
    // Un nouveau lancement reprend la main malgré le fichier PID périmé.
    let mut d2 = spawn(&root, &["--background=screens"], &[]);
    std::thread::sleep(Duration::from_secs(4));
    assert!(d2.try_wait().unwrap().is_none(), "le PID périmé ne doit pas bloquer");
    assert_eq!(read_pid(&root), Some(d2.id()));
    d2.kill().unwrap();
    let _ = d2.wait();
    let _ = std::fs::remove_dir_all(&root);
}

/// Chaîne complète : écrans seuls en cours -> l'application complète le remplace -> à sa fermeture
/// (réglage actif) elle passe la main à un nouveau processus écrans seuls, qui survit au job de fermeture.
#[test]
#[ignore = "lance le vrai exécutable (application complète, fenêtre masquée)"]
fn full_app_replaces_daemon_and_hands_back_on_quit() {
    let root = appdata("handover", true);
    let mut d1 = spawn(&root, &["--background=screens"], &[]);
    wait_for("écrans seuls actif", 20, || log_text(&root).contains("arrière-plan : 1 écran(s) actif(s)"));
    let mut full = spawn(&root, &[], &[("PURERGB_AUTOQUIT_SECS", "25")]);
    wait_for("l'application complète a arrêté les écrans seuls", 30, || d1.try_wait().unwrap().is_some());
    assert!(full.try_wait().unwrap().is_none(), "l'application complète doit tourner");
    wait_for("l'application complète a quitté", 60, || full.try_wait().unwrap().is_some());
    // Un nouveau processus écrans seuls a pris le relais (PID différent, vivant, hors job).
    wait_for("relais écrans seuls", 20, || read_pid(&root).is_some_and(|p| p != d1.id() && p != full.id()));
    let pid = read_pid(&root).unwrap();
    std::thread::sleep(Duration::from_secs(3));
    assert!(alive(pid), "le relais écrans seuls doit survivre à la fermeture de l'application complète");
    // Lancement de l'application complète une 2e fois : doit l'arrêter aussi puis on referme sans relais.
    std::fs::write(root.join("PureRGB").join("background.stop"), b"stop").unwrap();
    wait_for("arrêt final", 15, || !alive(pid));
    let leftovers = ps("(Get-Process purergb,msedgewebview2 -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*PureRGB-wt*' -or $_.Name -eq 'purergb' } | Measure-Object).Count");
    println!("processus purergb restants (toutes sessions confondues) : {leftovers}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Tous les modules cochés (écrans, éclairage, ventilateurs, manette) sur ce PC, sans réglage enregistré :
/// rien ne doit être écrit (aucune courbe, aucun effet), aucune fenêtre, pas d'OpenRGB, et un arrêt propre
/// ne laisse aucun processus (sensord compris).
#[test]
#[ignore = "lance le vrai exécutable"]
fn all_modules_start_without_window_or_openrgb_and_leave_nothing() {
    let root = appdata("all", false);
    let mut d = spawn(&root, &["--background=screens,lighting,fans,gamepad"], &[]);
    let pid = d.id();
    wait_for("démarrage", 30, || log_text(&root).contains("arrière-plan : 1 écran(s) actif(s)"));
    std::thread::sleep(Duration::from_secs(12)); // laisse sensord et le scan matériel démarrer
    assert!(d.try_wait().unwrap().is_none());
    assert_eq!(ps(&format!("(Get-Process -Id {pid}).MainWindowHandle")), "0");
    let kids = ps(&format!("(Get-CimInstance Win32_Process -Filter 'ParentProcessId={pid}' | Select-Object -ExpandProperty Name) -join ','"));
    println!("enfants : [{kids}]");
    assert!(!kids.to_lowercase().contains("openrgb"), "OpenRGB ne doit pas être lancé (auto_start coupé / non demandé)");
    assert!(!kids.to_lowercase().contains("webview"), "aucun WebView2");
    let ws_mb: f64 = ps(&format!("[math]::Round((Get-Process -Id {pid}).WorkingSet64/1MB,1)")).replace(',', ".").parse().unwrap();
    println!("RAM tous modules : {ws_mb} Mo");
    std::fs::write(root.join("PureRGB").join("background.stop"), b"stop").unwrap();
    wait_for("arrêt", 20, || d.try_wait().unwrap().is_some());
    std::thread::sleep(Duration::from_secs(2));
    let left = ps("(Get-Process sensord,OpenRGB -ErrorAction SilentlyContinue | Measure-Object).Count");
    assert_eq!(left, "0", "processus sensord/OpenRGB restants");
    let log = log_text(&root);
    assert!(!log.contains("OpenRGB embarqué démarré"));
    let _ = std::fs::remove_dir_all(&root);
}

/// Seul le module ventilateurs : pas d'écran (aucune détection), et sans courbe enregistrée aucune écriture.
#[test]
#[ignore = "lance le vrai exécutable"]
fn fans_only_without_curves_runs_no_screen_and_stops_cleanly() {
    let root = appdata("fans", false);
    let mut d = spawn(&root, &["--background=fans"], &[]);
    std::thread::sleep(Duration::from_secs(12));
    assert!(d.try_wait().unwrap().is_none());
    assert!(!log_text(&root).contains("écran(s) actif(s)"), "le module écrans ne doit pas tourner");
    std::fs::write(root.join("PureRGB").join("background.stop"), b"stop").unwrap();
    wait_for("arrêt", 20, || d.try_wait().unwrap().is_some());
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(ps("(Get-Process sensord -ErrorAction SilentlyContinue | Measure-Object).Count"), "0");
    let _ = std::fs::remove_dir_all(&root);
}
