//! Rapport de diagnostic LOCAL (affiché et copié par l'utilisateur, jamais envoyé automatiquement) :
//! une carte par sous-système avec un état OK / avertissement / erreur, et le même contenu en texte.
//! Lecture seule : aucune écriture matériel, aucune requête réseau. Chemins réduits au nom de fichier,
//! nom de compte retiré des messages. Ne touche pas à la télémétrie (`ExtraDiag` est inchangé).

use crate::backends::hid::native;
use crate::backends::hid::RawHidDevice;
use crate::conflicts::ConflictingSoftware;
use crate::diag_extra::{collect_from, context_with, scrub, ExtraDiag};
use crate::{conflicts, gamemode, inventory, AppState, HardwareDiagnostics};
use crate::screens::manager::ScreenManager;
use serde::Serialize;

/// Ordre = gravité croissante (`raise` garde le pire état).
#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    Ok,
    Warn,
    Error,
}

impl CardStatus {
    fn label(self) -> &'static str {
        match self {
            CardStatus::Ok => "OK",
            CardStatus::Warn => "AVERTISSEMENT",
            CardStatus::Error => "ERREUR",
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Card {
    pub id: &'static str,
    pub title: &'static str,
    pub status: CardStatus,
    pub rows: Vec<(String, String)>,
}

impl Card {
    fn new(id: &'static str, title: &'static str) -> Self {
        Card { id, title, status: CardStatus::Ok, rows: Vec::new() }
    }
    fn row(&mut self, k: &str, v: impl Into<String>) -> &mut Self {
        self.rows.push((k.to_string(), v.into()));
        self
    }
    fn raise(&mut self, s: CardStatus) -> &mut Self {
        self.status = self.status.max(s);
        self
    }
}

#[derive(Serialize)]
pub struct DiagReport {
    pub cards: Vec<Card>,
    pub text: String,
    pub hid_raw: Vec<RawHidDevice>,
}

/// Un canal ventilateur de la carte mère : nom, vitesse, commande en cours.
#[derive(Debug, Clone, PartialEq)]
pub struct FanRow {
    pub name: String,
    pub rpm: f64,
    /// Commande en % (capteur « Control » jumelé), si connue.
    pub duty: Option<f64>,
    pub controllable: bool,
}

/// Jumelle chaque capteur `Fan` (`/lpc/x/fan/0`) avec son `Control` (`/lpc/x/control/0`).
pub fn fan_rows(sensors: &[crate::sensors::Sensor]) -> Vec<FanRow> {
    sensors
        .iter()
        .filter(|s| s.kind == "Fan" && s.id.starts_with("/lpc/"))
        .map(|s| {
            let ctl = sensors.iter().find(|c| c.kind == "Control" && c.id == s.id.replace("/fan/", "/control/"));
            FanRow { name: s.name.clone(), rpm: s.value, duty: ctl.map(|c| c.value), controllable: ctl.is_some_and(|c| c.controllable) }
        })
        .collect()
}

fn fan_line(f: &FanRow) -> String {
    let duty = f.duty.map(|d| format!("commande {d:.0} %")).unwrap_or_else(|| "commande inconnue".into());
    format!("{:.0} tr/min · {duty} · {}", f.rpm, if f.controllable { "pilotable" } else { "lecture seule" })
}

/// Données locales hors `HardwareDiagnostics` / `ExtraDiag`.
pub struct Facts {
    pub os: String,
    pub cpu: String,
    pub ram_total_mb: u64,
    pub bios: String,
    pub app_version: String,
    pub usb_all: Vec<(String, String)>,
    pub conflicts: Vec<ConflictingSoftware>,
    pub fans: Vec<FanRow>,
    pub fans_measured: usize,
    pub fans_controllable: usize,
    pub game_enabled: bool,
    pub game_active: bool,
    pub game_name: Option<String>,
    pub game_grace: bool,
    pub vigem_ready: bool,
    pub vigem_version: String,
    pub hidhide_installed: bool,
    pub hidhide_version: String,
    pub smbus_writes: bool,
}

/// Nom de fichier seul (le chemin contient le nom de compte).
fn short(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

fn first_line(s: &str) -> String {
    scrub(s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim()).chars().take(200).collect()
}

fn yes(b: bool) -> &'static str {
    if b { "oui" } else { "non" }
}

fn parse_hex(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

/// Famille native qui pilote ce VID/PID (None = pas de pilote natif).
fn native_family(d: &RawHidDevice) -> Option<&'static str> {
    if !d.has_native_driver {
        return None;
    }
    let (v, p) = (parse_hex(&d.vid)?, parse_hex(&d.pid)?);
    Some(native::probe(v, p).map(|(f, _)| f.id).unwrap_or("corsair_node"))
}

pub fn build_cards(hw: &HardwareDiagnostics, x: &ExtraDiag, f: &Facts) -> Vec<Card> {
    use CardStatus::{Error, Warn};
    let mut cards = Vec::new();

    let mut c = Card::new("system", "Système");
    c.row("PureRGB", f.app_version.as_str()).row("Windows", f.os.as_str()).row("Processeur", f.cpu.as_str());
    c.row("Mémoire installée", format!("{} Go", (f.ram_total_mb + 512) / 1024));
    c.row("Carte mère", if x.board.is_empty() { "inconnue" } else { x.board.as_str() }).row("BIOS", if f.bios.is_empty() { "inconnu" } else { f.bios.as_str() });
    c.row("Carte(s) graphique(s)", if x.gpus.is_empty() { "aucune".to_string() } else { x.gpus.join(" ; ") });
    if let Some(e) = &x.inventory_error {
        c.raise(Warn).row("Inventaire Windows", first_line(e));
    }
    cards.push(c);

    let lc = &hw.liquidctl;
    let mut c = Card::new("liquidctl", "liquidctl");
    c.row("Binaire", lc.exe_path.as_deref().map(short).unwrap_or_else(|| "introuvable".into()));
    for (k, r) in [("version", &lc.version), ("list", &lc.list), ("initialize", &lc.initialize), ("status", &lc.status)] {
        match r {
            Ok(t) => c.row(k, first_line(t)),
            Err(e) => c.raise(if k == "version" { Error } else { Warn }).row(k, format!("échec : {}", first_line(e))),
        };
    }
    if lc.exe_path.is_none() {
        c.raise(Error);
    }
    cards.push(c);

    let mut c = Card::new("sensord", "sensord (capteurs)");
    c.row("Binaire", hw.sensord.exe_path.as_deref().map(short).unwrap_or_else(|| "introuvable".into()))
        .row("En cours", yes(hw.sensord.running))
        .row("Capteurs remontés", hw.sensord.sensor_count.to_string());
    if hw.sensord.exe_path.is_none() {
        c.raise(Error);
    } else if !hw.sensord.running {
        c.raise(Warn);
    }
    cards.push(c);

    let o = &hw.openrgb;
    let mut c = Card::new("openrgb", "OpenRGB");
    c.row("Binaire", o.exe_path.as_deref().map(short).unwrap_or_else(|| "introuvable".into()))
        .row("Serveur joignable", yes(o.server_reachable))
        .row("Géré par PureRGB", yes(o.managed))
        .row("PawnIO installé", yes(o.pawnio_installed))
        .row("PawnIO prêt", yes(o.pawnio_ready));
    if let Some(e) = &o.last_error {
        c.raise(Error).row("Dernière erreur", first_line(e));
    } else if !o.server_reachable || !o.pawnio_ready {
        c.raise(Warn);
    }
    cards.push(c);

    let unknown: Vec<&RawHidDevice> = hw.hid_raw.iter().filter(|d| !d.recognized).collect();
    let mut c = Card::new("hid", "HID brut");
    c.row("Périphériques HID", hw.hid_raw.len().to_string()).row("Non reconnus", unknown.len().to_string());
    for d in unknown.iter().take(12) {
        c.row(&format!("{}:{}", d.vid, d.pid), format!("{} {}", d.manufacturer, d.product).trim().to_string());
    }
    if hw.hid_raw.is_empty() {
        c.raise(Warn);
    }
    cards.push(c);

    let mut c = Card::new("usb", "USB");
    c.row("Périphériques USB vus par Windows", f.usb_all.len().to_string()).row("Dont marques RGB du catalogue", x.usb_rgb.len().to_string());
    for u in &x.usb_rgb {
        c.row(
            &format!("{}:{}", u.vid, u.pid),
            format!("{} ({})", u.windows_name, u.catalog_model.as_deref().unwrap_or("modèle absent du catalogue")),
        );
        if u.catalog_model.is_none() {
            c.raise(Warn);
        }
    }
    if !x.screens.plugged_but_idle.is_empty() {
        c.raise(Warn).row("Mini-écrans branchés sans pilote actif", x.screens.plugged_but_idle.join(", "));
    }
    cards.push(c);

    let with_native: Vec<&RawHidDevice> = hw.hid_raw.iter().filter(|d| d.has_native_driver).collect();
    let mut c = Card::new("native", "Pilotes natifs");
    c.row("Pilotes natifs", if x.native_drivers_enabled { "activés" } else { "désactivés" })
        .row("Familles disponibles", native::FAMILIES.len().to_string())
        .row("Périphériques couverts", with_native.len().to_string());
    for d in &with_native {
        c.row(&format!("{}:{}", d.vid, d.pid), format!("{} ({})", d.product, native_family(d).unwrap_or("?")));
    }
    if !with_native.is_empty() && !x.native_drivers_enabled {
        c.raise(Warn).row("Conseil", "des périphériques ont un pilote natif mais il est désactivé dans les réglages");
    }
    cards.push(c);

    let mut c = Card::new("smbus", "SMBus / mémoire RAM");
    c.row("Barrettes", if x.ram.is_empty() { "aucune lue".to_string() } else { x.ram.join(" ; ") })
        .row("Écriture SMBus autorisée", yes(f.smbus_writes))
        .row("PawnIO prêt", yes(o.pawnio_ready));
    let ram_findings: Vec<&inventory::Finding> =
        x.findings.iter().filter(|f| f.category.to_lowercase().contains("mémoire") || f.category.to_lowercase().contains("ram")).collect();
    for fi in ram_findings {
        if fi.status == inventory::Status::Warn {
            c.raise(Warn);
        }
        c.row(&fi.title, first_line(&fi.detail));
    }
    cards.push(c);

    let mut c = Card::new("gamepad", "ViGEmBus / HidHide");
    c.row("ViGEmBus", format!("{} (v{})", if f.vigem_ready { "prêt" } else { "absent ou suspendu" }, f.vigem_version))
        .row("HidHide", format!("{} (v{})", if f.hidhide_installed { "installé" } else { "non installé" }, f.hidhide_version));
    cards.push(c);

    let mut c = Card::new("fans", "Ventilateurs");
    c.row("Ventilateurs mesurés (carte mère)", f.fans_measured.to_string()).row("Canaux pilotables", f.fans_controllable.to_string());
    let fan_devs: Vec<String> = x.devices.iter().filter(|d| d.kind.contains("Fan")).map(|d| d.name.clone()).collect();
    if !fan_devs.is_empty() {
        c.row("Appareils ventilateurs", fan_devs.join(" ; "));
    }
    for fan in &f.fans {
        c.row(&fan.name, fan_line(fan));
    }
    let makers: Vec<&str> = f.conflicts.iter().filter(|s| s.active).map(|s| s.name.as_str()).collect();
    c.row(
        "Logiciels constructeur actifs (iCUE, L-Connect, MSI Center, Armoury…)",
        if makers.is_empty() { "aucun".to_string() } else { makers.join(", ") },
    );
    let mut fams: Vec<&str> = with_native.iter().filter_map(|d| native_family(d)).collect();
    fams.sort_unstable();
    fams.dedup();
    for fam in fams {
        c.row(fam, native::fan_restore_hint(fam));
    }
    if !hw.sensord.running {
        c.raise(Warn).row("Remarque", "sensord à l'arrêt : aucune mesure de ventilateur");
    }
    cards.push(c);

    let mut c = Card::new("conflicts", "Logiciels concurrents");
    let active: Vec<&ConflictingSoftware> = f.conflicts.iter().filter(|c| c.active).collect();
    c.row("Détectés actifs", active.len().to_string()).row("Famille(s) connues installées", f.conflicts.len().to_string());
    for s in &f.conflicts {
        let procs = if s.processes.is_empty() { "aucun processus".to_string() } else { s.processes.join(", ") };
        c.row(&s.name, format!("{} ; {}", if s.active { "ACTIF" } else { "inactif" }, procs));
    }
    for p in &x.suspect_processes {
        c.row("Processus suspect", p.as_str());
    }
    if !active.is_empty() {
        c.raise(Warn);
    }
    cards.push(c);

    let mut c = Card::new("gamemode", "Mode jeu");
    c.row("Réglage", if f.game_enabled { "activé" } else { "désactivé" }).row("Accès matériel suspendu", yes(f.game_active));
    if let Some(g) = &f.game_name {
        c.row("Jeu détecté", g.as_str());
    }
    if f.game_grace {
        c.row("Reprise", "dans quelques secondes");
    }
    if f.game_active {
        c.raise(Warn);
    }
    cards.push(c);

    let mut c = Card::new("errors", "Modules et dernières erreurs");
    c.row("PureRGB", f.app_version.as_str()).row("liquidctl", lc.version.as_ref().map(|v| first_line(v)).unwrap_or_else(|_| "indisponible".into()));
    c.row("ViGEmBus (embarqué)", f.vigem_version.as_str()).row("HidHide (embarqué)", f.hidhide_version.as_str());
    c.row("Erreur OpenRGB", o.last_error.as_deref().map(first_line).unwrap_or_else(|| "aucune".into()));
    c.row("Erreur liquidctl", [&lc.list, &lc.initialize, &lc.status].iter().find_map(|r| r.as_ref().err()).map(|e| first_line(e)).unwrap_or_else(|| "aucune".into()));
    c.row("Erreurs écrans", if x.screens.scan_errors.is_empty() { "aucune".to_string() } else { x.screens.scan_errors.join(" | ") });
    if o.last_error.is_some() || !x.screens.scan_errors.is_empty() {
        c.raise(Warn);
    }
    cards.push(c);

    let mut c = Card::new("findings", "Constats de l'inventaire");
    for fi in &x.findings {
        if fi.status == inventory::Status::Warn {
            c.raise(Warn);
        }
        c.row(&format!("[{}] {}", fi.category, fi.title), first_line(&fi.detail));
    }
    if x.findings.is_empty() {
        c.row("Constats", "aucun");
    }
    cards.push(c);

    let mut c = Card::new("log", "Journal des dernières 24 h (WARN/ERROR)");
    for (i, l) in x.log_tail.iter().rev().take(15).enumerate() {
        c.row(&format!("{}", i + 1), l.as_str());
    }
    if x.log_tail.is_empty() {
        c.row("Journal", "aucun avertissement récent");
    }
    cards.push(c);
    cards
}

/// Texte complet du rapport (copier / enregistrer).
pub fn render_text(version: &str, cards: &[Card]) -> String {
    let mut out = format!("Rapport de diagnostic PureRGB {version}\n");
    for c in cards {
        out.push_str(&format!("\n== {} [{}] ==\n", c.title, c.status.label()));
        for (k, v) in &c.rows {
            out.push_str(&format!("{k} : {v}\n"));
        }
    }
    out
}

fn gather_facts(state: &AppState, inv: &Result<inventory::Inventory, String>, found: Vec<ConflictingSoftware>) -> Facts {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    let ctx = context_with(state, &found);
    let drv = crate::gamepad_remap::driver::info();
    let (game_name, game_grace) = gamemode::current_game();
    let (game_enabled, smbus_writes) = {
        let s = state.settings.lock();
        (s.game_mode, s.smbus_writes_enabled)
    };
    let os = [sysinfo::System::name(), sysinfo::System::long_os_version(), sysinfo::System::kernel_version()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let (bios, usb_all) = match inv {
        Ok(i) => (i.bios.clone(), i.usb.iter().map(|u| (format!("{:04x}:{:04x}", u.vid, u.pid), u.name.clone())).collect()),
        Err(_) => (String::new(), Vec::new()),
    };
    Facts {
        os,
        cpu: sys.cpus().first().map(|c| format!("{} ({} threads)", c.brand().trim(), sys.cpus().len())).unwrap_or_default(),
        ram_total_mb: sys.total_memory() / (1024 * 1024),
        bios,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        usb_all,
        conflicts: found,
        fans: fan_rows(&state.sensors.snapshot()),
        fans_measured: ctx.fans_measured,
        fans_controllable: ctx.fans_controllable,
        game_enabled,
        game_active: gamemode::active(),
        game_name,
        game_grace,
        vigem_ready: drv.vigem_ready,
        vigem_version: drv.vigem_version.to_string(),
        hidhide_installed: drv.hidhide_installed,
        hidhide_version: drv.hidhide_version.to_string(),
        smbus_writes,
    }
}

/// Rapport complet : à appeler hors thread d'interface (PowerShell, scan des services).
pub fn local_report(state: &AppState, screens: &ScreenManager, hw: HardwareDiagnostics) -> DiagReport {
    let found = conflicts::scan().0;
    let ctx = context_with(state, &found);
    let inv = inventory::collect();
    let facts = gather_facts(state, &inv, found);
    let x = collect_from(state, screens, inv, ctx);
    let cards = build_cards(&hw, &x, &facts);
    DiagReport { text: render_text(&facts.app_version, &cards), cards, hid_raw: hw.hid_raw }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(status: CardStatus) -> Card {
        let mut c = Card::new("t", "Test");
        c.row("a", "1").raise(status);
        c
    }

    #[test]
    fn text_lists_every_card_with_its_status() {
        let t = render_text("0.49.0", &[card(CardStatus::Ok), card(CardStatus::Error)]);
        assert!(t.starts_with("Rapport de diagnostic PureRGB 0.49.0\n"));
        assert!(t.contains("== Test [OK] ==\na : 1\n"));
        assert!(t.contains("== Test [ERREUR] =="));
    }

    #[test]
    fn raise_keeps_the_worst_status() {
        let mut c = Card::new("t", "T");
        c.raise(CardStatus::Error).raise(CardStatus::Warn);
        assert_eq!(c.status, CardStatus::Error);
    }

    #[test]
    fn paths_are_reduced_to_file_names() {
        assert_eq!(short(r"C:\Users\Jean\AppData\liquidctl.exe"), "liquidctl.exe");
        assert_eq!(short("/opt/x/sensord"), "sensord");
    }

    #[test]
    fn first_line_skips_blank_lines_and_truncates() {
        assert_eq!(first_line("\n\n  hello \nworld"), "hello");
        assert_eq!(first_line(&"x".repeat(500)).chars().count(), 200);
    }

    #[test]
    fn native_family_needs_the_flag_and_a_known_pid() {
        let mut d = RawHidDevice { vid: "1b1c".into(), pid: "0c0b".into(), manufacturer: String::new(), product: String::new(), recognized: true, has_native_driver: false };
        assert_eq!(native_family(&d), None);
        d.has_native_driver = true;
        assert_eq!(native_family(&d), Some("corsair_node"));
    }

    #[test]
    fn fan_rows_pair_fan_and_control_sensors() {
        let s = |id: &str, name: &str, kind: &str, value: f64, controllable: bool| crate::sensors::Sensor {
            id: id.into(),
            hardware: "x".into(),
            name: name.into(),
            kind: kind.into(),
            value,
            controllable,
        };
        let rows = fan_rows(&[
            s("/lpc/nct/fan/0", "CPU Fan", "Fan", 1250.0, false),
            s("/lpc/nct/control/0", "CPU Fan", "Control", 45.0, true),
            s("/lpc/nct/fan/1", "Fan #2", "Fan", 0.0, false),
            s("/gpu/fan/0", "GPU", "Fan", 900.0, false),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(fan_line(&rows[0]), "1250 tr/min · commande 45 % · pilotable");
        assert_eq!(fan_line(&rows[1]), "0 tr/min · commande inconnue · lecture seule");
    }

    #[test]
    fn status_serializes_in_snake_case() {
        assert_eq!(serde_json::to_string(&CardStatus::Warn).unwrap(), "\"warn\"");
    }
}
