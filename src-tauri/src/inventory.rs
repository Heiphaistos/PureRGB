//! Inventaire du PC (carte mère, mémoire, USB, ventilateurs) comparé à ce que PureRGB sait piloter.
//!
//! Windows (WMI) dit ce qui est branché ; le registre de PureRGB dit ce qui est pilotable. L'écart
//! est expliqué à l'utilisateur (driver SMBus absent, logiciel constructeur en conflit, connecteurs
//! ARGB à 0 LED…) au lieu d'un simple « rien trouvé ». L'analyse est pure (testée sans matériel).

use crate::core::{DeviceInfo, DeviceType};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Board {
    #[serde(default, rename = "Manufacturer")]
    pub manufacturer: String,
    #[serde(default, rename = "Product")]
    pub product: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RamStick {
    #[serde(default, rename = "Manufacturer")]
    pub manufacturer: String,
    #[serde(default, rename = "PartNumber")]
    pub part: String,
    #[serde(default, rename = "DeviceLocator")]
    pub slot: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsbDev {
    pub vid: u16,
    pub pid: u16,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub board: Board,
    pub ram: Vec<RamStick>,
    pub gpus: Vec<String>,
    pub usb: Vec<UsbDev>,
    /// Version et date du BIOS (lecture seule, affichage du diagnostic).
    pub bios: String,
}

/// Ce que PureRGB sait par ailleurs (hors WMI).
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub openrgb_up: bool,
    pub pawnio_ready: bool,
    pub sensord_up: bool,
    pub fans_measured: usize,
    pub fans_controllable: usize,
    /// Logiciels constructeur réellement en cours d'exécution (pas simplement installés).
    pub active_conflicts: Vec<String>,
    /// Dernière raison de l'échec du démarrage d'OpenRGB.
    pub openrgb_error: Option<String>,
}

impl Context {
    /// Phrase à ajouter aux explications : vide si aucun logiciel constructeur n'est actif.
    fn conflict_hint(&self) -> String {
        if self.active_conflicts.is_empty() {
            String::new()
        } else {
            format!(" Logiciel(s) en cours d'exécution qui peuvent verrouiller le matériel : {} (voir « Logiciels en conflit »).", self.active_conflicts.join(", "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Info,
    Warn,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub category: String,
    pub status: Status,
    pub title: String,
    pub detail: String,
}

fn f(category: &str, status: Status, title: impl Into<String>, detail: impl Into<String>) -> Finding {
    Finding { category: category.into(), status, title: title.into(), detail: detail.into() }
}

fn count(devices: &[DeviceInfo], t: DeviceType) -> usize {
    devices.iter().filter(|d| d.device_type == t && d.controllable).count()
}

/// Compare l'inventaire Windows au registre PureRGB et produit les constats affichés.
pub fn analyze(inv: &Inventory, devices: &[DeviceInfo], ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    if !ctx.openrgb_up {
        let why = ctx.openrgb_error.as_deref().map(|e| format!(" Cause : {e}")).unwrap_or_default();
        out.push(f("OpenRGB", Status::Warn, "Serveur OpenRGB injoignable",
            format!("Sans lui, ni la carte mère, ni la mémoire, ni la plupart des périphériques ne sont détectés. Utilisez « Démarrer OpenRGB ».{why}")));
    }
    board(inv, devices, ctx, &mut out);
    board_spec(inv, &mut out);
    memory(inv, devices, ctx, &mut out);
    usb(inv, devices, ctx, &mut out);
    fans(ctx, &mut out);
    if !inv.gpus.is_empty() {
        let n = count(devices, DeviceType::Gpu);
        let names = inv.gpus.join(", ");
        out.push(if n > 0 {
            f("Carte graphique", Status::Ok, format!("Éclairage de la carte graphique pilotable ({n})"), names)
        } else {
            f("Carte graphique", Status::Info, "Éclairage de la carte graphique non pilotable",
                format!("{names} : normal si le modèle n'a pas de RGB ou n'est pas encore pris en charge par OpenRGB."))
        });
    }
    out
}

fn board(inv: &Inventory, devices: &[DeviceInfo], ctx: &Context, out: &mut Vec<Finding>) {
    let name = format!("{} {}", inv.board.manufacturer, inv.board.product).trim().to_string();
    let boards: Vec<&DeviceInfo> = devices.iter().filter(|d| d.device_type == DeviceType::Motherboard && d.controllable).collect();
    if boards.is_empty() {
        out.push(f("Carte mère", Status::Info, format!("Aucun éclairage de carte mère pilotable ({name})"),
            format!("Normal si la carte n'a pas de RGB. Sinon, « Redémarrer OpenRGB ».{}", ctx.conflict_hint())));
        return;
    }
    out.push(f("Carte mère", Status::Ok, format!("Éclairage de la carte mère pilotable ({name})"),
        format!("{} contrôleur(s).", boards.len())));
    for d in boards {
        let empty: Vec<&str> = d.zones.iter().filter(|z| z.led_count == 0 && z.resizable()).map(|z| z.name.as_str()).collect();
        if !empty.is_empty() {
            out.push(f("Carte mère", Status::Warn, format!("Connecteurs ARGB à 0 LED : {}", empty.join(", ")),
                "Un ventilateur ou un bandeau ARGB branché dessus reste invisible tant que son nombre de LEDs n'est pas indiqué (Éclairage, zone, nombre de LEDs)."));
        }
    }
}

/// Ce que le constructeur documente pour cette carte : connecteurs ARGB/RGB et limite de LEDs par connecteur.
fn board_spec(inv: &Inventory, out: &mut Vec<Finding>) {
    use crate::led_catalog as lc;
    let (m, p) = (inv.board.manufacturer.as_str(), inv.board.product.as_str());
    let limit = lc::vendor_rule(m).and_then(|r| r.argb_header.as_ref()).map(|h| {
        let volt = h.voltage.as_deref().map(|v| format!("{v}, ")).unwrap_or_default();
        match (h.max_leds, h.max_power_a) {
            (Some(n), Some(a)) => format!("limite documentée par le constructeur : {n} LEDs par connecteur ARGB ({volt}{a} A)"),
            (Some(n), None) => format!("limite documentée par le constructeur : {n} LEDs par connecteur ARGB ({})", volt.trim_end_matches(", ")),
            _ => "limite de LEDs non publiée par le constructeur".to_string(),
        }
    });
    match (lc::board(m, p), limit) {
        (Some(b), l) => out.push(f("Carte mère", Status::Info, format!("Connecteurs de la carte ({})", b.model),
            format!("ARGB 5 V : {} ; RGB 12 V : {}{}. Un connecteur ne peut pas détecter ce qui y est branché : choisissez le modèle de ventilateur (Éclairage, zone).",
                if b.argb_headers.is_empty() { "—".into() } else { b.argb_headers.join(", ") },
                if b.rgb_headers.is_empty() { "—".into() } else { b.rgb_headers.join(", ") },
                l.map(|l| format!(" ; {l}")).unwrap_or_default()))),
        (None, Some(l)) => out.push(f("Carte mère", Status::Info, "Connecteurs ARGB de la carte",
            format!("Modèle absent du catalogue ; règle du constructeur : {l}."))),
        (None, None) => {}
    }
}

fn memory(inv: &Inventory, devices: &[DeviceInfo], ctx: &Context, out: &mut Vec<Finding>) {
    let sticks = inv.ram.len();
    if sticks == 0 {
        return;
    }
    let names: Vec<String> = inv.ram.iter().map(|r| format!("{} {}", r.manufacturer, r.part.trim()).trim().to_string()).collect();
    let driven = count(devices, DeviceType::Dram);
    if driven > 0 {
        let st = if driven < sticks { Status::Info } else { Status::Ok };
        out.push(f("Mémoire", st, format!("{driven} barrette(s) sur {sticks} pilotable(s)"), names.join(" · ")));
    } else if !ctx.pawnio_ready {
        out.push(f("Mémoire", Status::Warn, format!("{sticks} barrette(s) vue(s) par Windows, aucune pilotable : pilote PawnIO absent"),
            "L'éclairage de la mémoire passe par le bus SMBus, inaccessible sans le pilote PawnIO (Réglages, installer PawnIO), puis redémarrer OpenRGB."));
    } else {
        out.push(f("Mémoire", Status::Info, format!("{sticks} barrette(s) vue(s), aucune pilotable"),
            format!("{} : barrettes sans RGB ou modèle non supporté.{}", names.join(" · "), ctx.conflict_hint())));
    }
}

fn usb(inv: &Inventory, devices: &[DeviceInfo], ctx: &Context, out: &mut Vec<Finding>) {
    use crate::rgb_catalog::catalog;
    let cat = catalog();
    for u in &inv.usb {
        if let Some(name) = crate::rgb_catalog::known_without_rgb(u.vid, u.pid) {
            out.push(f("USB", Status::Info, format!("{name} (USB {:04X}:{:04X}) : sans éclairage RGB", u.vid, u.pid),
                "Cet appareil n'a pas d'éclairage : rien à piloter, aucune action requise."));
            continue;
        }
        let Some(vendor) = cat.vendor(u.vid) else { continue };
        let key = vendor.split([' ', '/']).next().unwrap_or(vendor).to_lowercase();
        let model_name = cat.device(u.vid, u.pid);
        let driven = is_driven(devices, &key, model_name);
        match model_name {
            Some(model) if driven => out.push(f("USB", Status::Ok, format!("{model} pilotable"), format!("{} (USB {:04X}:{:04X})", u.name, u.vid, u.pid))),
            Some(model) if !ctx.openrgb_up => out.push(f("USB", Status::Info, format!("{model} reconnu, en attente d'OpenRGB"),
                format!("USB {:04X}:{:04X} sera pilotable dès que le serveur OpenRGB tourne.", u.vid, u.pid))),
            Some(model) => out.push(f("USB", Status::Warn, format!("{model} reconnu mais non détecté"),
                format!("USB {:04X}:{:04X} est un contrôleur RGB connu mais OpenRGB ne l'a pas pris : rebranchez-le puis « Redémarrer OpenRGB ».{}", u.vid, u.pid, ctx.conflict_hint()))),
            None => out.push(f("USB", Status::Info, format!("{vendor} : {} (USB {:04X}:{:04X}) hors catalogue", u.name, u.vid, u.pid),
                "Fabricant RGB connu mais ce modèle précis n'est pas encore pris en charge (ou n'a pas d'éclairage). Signalez-le via le diagnostic matériel.")),
        }
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().replace('_', "-")
}

/// Un périphérique pilotable correspond-il à ce modèle du catalogue ? Par le nom complet du modèle, par son
/// identifiant (dernier mot avec un chiffre, ex. `ms-7d93`), ou à défaut par le 1er mot du fabricant.
fn is_driven(devices: &[DeviceInfo], vendor_key: &str, model: Option<&str>) -> bool {
    let model = model.map(norm);
    let ident = model.as_deref().and_then(|m| m.rsplit(' ').next()).filter(|t| t.len() >= 4 && t.chars().any(|c| c.is_ascii_digit()));
    devices.iter().filter(|d| d.controllable).any(|d| {
        let hay = norm(&format!("{} {}", d.vendor, d.name));
        hay.contains(vendor_key) || model.as_deref().is_some_and(|m| hay.contains(m)) || ident.is_some_and(|i| hay.contains(i))
    })
}

fn fans(ctx: &Context, out: &mut Vec<Finding>) {
    if !ctx.sensord_up {
        out.push(f("Ventilateurs", Status::Warn, "Capteurs matériel indisponibles",
            "Sans le service de capteurs (sensord), les ventilateurs de la carte mère ne sont ni mesurés ni pilotables. Voir Réglages, diagnostic matériel."));
    } else if ctx.fans_measured == 0 {
        out.push(f("Ventilateurs", Status::Info, "Aucun ventilateur mesuré sur la carte mère",
            "Soit aucun ventilateur n'est branché sur les connecteurs de la carte, soit son circuit (Super I/O) n'est pas encore pris en charge."));
    } else if ctx.fans_controllable == 0 {
        out.push(f("Ventilateurs", Status::Info, format!("{} ventilateur(s) mesuré(s), aucun pilotable", ctx.fans_measured),
            "Le circuit de la carte ne permet que la lecture : réglez les courbes dans le BIOS, ou passez par un hub USB (Corsair, NZXT…)."));
    } else {
        out.push(f("Ventilateurs", Status::Ok, format!("{} ventilateur(s) mesuré(s), {} pilotable(s)", ctx.fans_measured, ctx.fans_controllable),
            "Les canaux pilotables sont dans l'onglet Ventilateurs."));
    }
}

/// Un objet ou une liste, PowerShell ne fait pas de liste à un seul élément.
fn one_or_many<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum V<T> {
        Many(Vec<T>),
        One(T),
        Nothing(()),
    }
    Ok(match V::<T>::deserialize(d)? {
        V::Many(v) => v,
        V::One(x) => vec![x],
        V::Nothing(()) => Vec::new(),
    })
}

#[derive(Deserialize)]
struct RawGpu {
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Deserialize)]
struct RawUsb {
    #[serde(rename = "InstanceId")]
    id: String,
    #[serde(default, rename = "FriendlyName")]
    name: Option<String>,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    board: Option<Board>,
    #[serde(default, deserialize_with = "one_or_many")]
    ram: Vec<RamStick>,
    #[serde(default, deserialize_with = "one_or_many")]
    gpu: Vec<RawGpu>,
    #[serde(default, deserialize_with = "one_or_many")]
    usb: Vec<RawUsb>,
    #[serde(default)]
    bios: Option<RawBios>,
}

#[derive(Deserialize)]
struct RawBios {
    #[serde(default, rename = "SMBIOSBIOSVersion")]
    version: Option<String>,
    #[serde(default, rename = "Manufacturer")]
    vendor: Option<String>,
}

/// `USB\VID_1B1C&PID_0C3F\...` -> (0x1B1C, 0x0C3F).
fn parse_vid_pid(instance: &str) -> Option<(u16, u16)> {
    let up = instance.to_uppercase();
    let hex = |tag: &str| {
        let at = up.find(tag)? + tag.len();
        u16::from_str_radix(up.get(at..at + 4)?, 16).ok()
    };
    Some((hex("VID_")?, hex("PID_")?))
}

pub fn parse_inventory(json: &str) -> Result<Inventory, String> {
    let raw: Raw = serde_json::from_str(json.trim_start_matches('\u{feff}')).map_err(|e| format!("inventaire illisible : {e}"))?;
    let mut usb: BTreeMap<(u16, u16), String> = BTreeMap::new();
    for u in raw.usb {
        if let Some(k) = parse_vid_pid(&u.id) {
            let name = u.name.unwrap_or_default();
            // Une interface (MI_xx) ou un nom générique ne remplace pas un vrai nom déjà vu.
            usb.entry(k).and_modify(|n| if n.is_empty() { *n = name.clone() }).or_insert(name);
        }
    }
    Ok(Inventory {
        board: raw.board.unwrap_or_default(),
        ram: raw.ram,
        gpus: raw.gpu.into_iter().map(|g| g.name).collect(),
        usb: usb.into_iter().map(|((vid, pid), name)| UsbDev { vid, pid, name: if name.is_empty() { format!("USB {vid:04X}:{pid:04X}") } else { name } }).collect(),
        bios: raw.bios.map(|b| format!("{} {}", b.vendor.unwrap_or_default(), b.version.unwrap_or_default()).trim().to_string()).unwrap_or_default(),
    })
}

const QUERY: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; $ErrorActionPreference='SilentlyContinue'; \
[ordered]@{ \
 board = Get-CimInstance Win32_BaseBoard | Select-Object Manufacturer,Product; \
 ram = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Manufacturer,PartNumber,DeviceLocator); \
 gpu = @(Get-CimInstance Win32_VideoController | Select-Object Name);  bios = Get-CimInstance Win32_BIOS | Select-Object Manufacturer,SMBIOSBIOSVersion; \
 usb = @(Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -like 'USB\\VID_*' } | Select-Object InstanceId,FriendlyName) \
} | ConvertTo-Json -Compress -Depth 4";

/// Interroge Windows (une commande PowerShell, quelques secondes) : à appeler hors thread d'interface.
#[cfg(windows)]
pub fn collect() -> Result<Inventory, String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", QUERY])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| format!("PowerShell indisponible : {e}"))?;
    if !out.status.success() {
        return Err(format!("inventaire en échec : {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    parse_inventory(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(not(windows))]
pub fn collect() -> Result<Inventory, String> {
    Err("inventaire disponible uniquement sous Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ZoneInfo;

    fn dev(name: &str, vendor: &str, t: DeviceType, zones: Vec<(&str, u32)>) -> DeviceInfo {
        DeviceInfo {
            id: format!("openrgb:{name}"),
            name: name.into(),
            vendor: vendor.into(),
            backend: "openrgb".into(),
            device_type: t,
            zones: zones.into_iter().map(|(n, c)| ZoneInfo { name: n.into(), led_count: c, zone_type: 1, leds_min: if c == 0 { 0 } else { c }, leds_max: if c == 0 { 60 } else { c } }).collect(),
            led_count: 0,
            fan_channels: vec![],
            controllable: true,
            has_lcd: false,
            modes: vec![],
            active_mode: -1,
            note: String::new(),
        }
    }

    const SAMPLE: &str = r#"{"board":{"Manufacturer":"ASUSTeK","Product":"ROG STRIX B650-F"},
        "ram":[{"Manufacturer":"Corsair","PartNumber":"CMH32 ","DeviceLocator":"DIMM_A1"},{"Manufacturer":"Corsair","PartNumber":"CMH32 ","DeviceLocator":"DIMM_B1"}],
        "gpu":{"Name":"NVIDIA GeForce RTX 4080"},
        "usb":[{"InstanceId":"USB\\VID_1532&PID_0043\\5&1","FriendlyName":"Razer DeathAdder"},
               {"InstanceId":"USB\\VID_1532&PID_0043&MI_01\\6&2","FriendlyName":""},
               {"InstanceId":"USB\\VID_8087&PID_0033\\X","FriendlyName":"Intel BT"}]}"#;

    #[test]
    fn openrgb_devices_count_as_driving_catalog_models() {
        let mobo = dev("MSI Z790 GAMING PRO WIFI (MS-7D93)", "Micro-Star", DeviceType::Motherboard, vec![]);
        assert!(is_driven(&[mobo], "msi_zz", Some("MSI Mystic Light MS_7D93")));
        let kb = dev("Cougar 700K EVO", "Cougar Gaming", DeviceType::Keyboard, vec![]);
        assert!(is_driven(&[kb], "zzz", Some("Cougar 700K EVO")));
        let other = dev("Razer Huntsman", "Razer", DeviceType::Keyboard, vec![]);
        assert!(!is_driven(&[other], "cougar", Some("Cougar 700K EVO")));
    }

    #[test]
    fn parses_powershell_json_with_single_and_multiple_items() {
        let inv = parse_inventory(SAMPLE).unwrap();
        assert_eq!(inv.board.product, "ROG STRIX B650-F");
        assert_eq!(inv.ram.len(), 2);
        assert_eq!(inv.gpus, vec!["NVIDIA GeForce RTX 4080"]);
        assert_eq!(inv.usb.len(), 2, "un VID/PID = une entrée, interfaces MI_xx fusionnées");
        assert_eq!(inv.usb[0].name, "Razer DeathAdder");
        assert_eq!((inv.usb[0].vid, inv.usb[0].pid), (0x1532, 0x0043));
        assert!(parse_inventory("{}").unwrap().ram.is_empty());
        assert!(parse_inventory("pas du json").is_err());
    }

    #[test]
    fn missing_ram_driver_and_unseen_razer_are_explained() {
        let inv = parse_inventory(SAMPLE).unwrap();
        let ctx = Context { openrgb_up: true, pawnio_ready: false, sensord_up: true, fans_measured: 3, fans_controllable: 0, ..Default::default() };
        let r = analyze(&inv, &[], &ctx);
        let by = |c: &str| r.iter().find(|x| x.category == c).unwrap_or_else(|| panic!("constat {c} absent"));
        assert_eq!(by("Mémoire").status, Status::Warn);
        assert!(by("Mémoire").title.contains("PawnIO"));
        assert_eq!(by("USB").status, Status::Warn, "Razer Deathadder Chroma est au catalogue");
        assert_eq!(by("Ventilateurs").status, Status::Info);
        assert!(r.iter().all(|x| !x.title.contains("Intel")), "un fabricant sans RGB n'est pas listé");
    }

    #[test]
    fn controllable_hardware_is_reported_ok_and_empty_argb_headers_warn() {
        let inv = parse_inventory(SAMPLE).unwrap();
        let ctx = Context { openrgb_up: true, pawnio_ready: true, sensord_up: true, fans_measured: 2, fans_controllable: 2, ..Default::default() };
        let devices = vec![
            dev("ROG STRIX B650-F", "ASUS", DeviceType::Motherboard, vec![("Addressable 1", 0), ("Mainboard", 8)]),
            dev("Corsair Vengeance 1", "Corsair", DeviceType::Dram, vec![]),
            dev("Corsair Vengeance 2", "Corsair", DeviceType::Dram, vec![]),
            dev("Razer DeathAdder", "Razer", DeviceType::Mouse, vec![]),
        ];
        let r = analyze(&inv, &devices, &ctx);
        assert!(r.iter().any(|x| x.category == "Mémoire" && x.status == Status::Ok && x.title.starts_with("2 barrette")));
        assert!(r.iter().any(|x| x.category == "USB" && x.status == Status::Ok));
        let w = r.iter().find(|x| x.status == Status::Warn).expect("ARGB à 0 LED signalé");
        assert!(w.title.contains("Addressable 1") && !w.title.contains("Mainboard"));
        assert!(r.iter().any(|x| x.category == "Ventilateurs" && x.status == Status::Ok));
    }

    #[test]
    fn installed_but_stopped_software_is_never_called_a_conflict() {
        let inv = parse_inventory(SAMPLE).unwrap();
        let calm = Context { openrgb_up: true, pawnio_ready: true, sensord_up: true, ..Default::default() };
        let r = analyze(&inv, &[], &calm);
        assert!(r.iter().all(|x| !x.detail.contains("en cours d'exécution") && !x.detail.contains("fermez")));
        let busy = Context { active_conflicts: vec!["Corsair iCUE".into()], ..calm };
        let r = analyze(&inv, &[], &busy);
        assert!(r.iter().any(|x| x.category == "USB" && x.detail.contains("Corsair iCUE")));
    }

    #[test]
    fn usb_devices_wait_for_openrgb_instead_of_blaming_software() {
        let inv = parse_inventory(SAMPLE).unwrap();
        let down = Context { openrgb_error: Some("port pris".into()), ..Default::default() };
        let r = analyze(&inv, &[], &down);
        let o = r.iter().find(|x| x.category == "OpenRGB").expect("constat OpenRGB");
        assert!(o.detail.contains("port pris"));
        assert!(r.iter().filter(|x| x.category == "USB").all(|x| x.status != Status::Warn));
    }

    /// Vrai appel WMI sur cette machine : `cargo test real_inventory -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_inventory() {
        let inv = collect().expect("inventaire");
        println!("{inv:#?}");
        assert!(!inv.board.product.is_empty() || !inv.gpus.is_empty());
    }

    #[test]
    fn headsets_without_rgb_are_never_reported_as_unsupported() {
        let mut inv = Inventory::default();
        for pid in [0x0ABA, 0x0AF7] {
            inv.usb.push(UsbDev { vid: 0x046D, pid, name: "PRO X".into() });
        }
        let r = analyze(&inv, &[], &Context::default());
        let usb: Vec<_> = r.iter().filter(|x| x.category == "USB").collect();
        assert_eq!(usb.len(), 2);
        assert!(usb.iter().all(|x| x.status == Status::Info && x.title.contains("sans éclairage RGB")));
        assert!(!r.iter().any(|x| x.title.contains("hors catalogue") || x.detail.contains("Signalez")));
    }

    #[test]
    fn down_services_are_flagged() {
        let r = analyze(&Inventory::default(), &[], &Context::default());
        assert!(r.iter().any(|x| x.category == "OpenRGB" && x.status == Status::Warn));
        assert!(r.iter().any(|x| x.category == "Ventilateurs" && x.status == Status::Warn));
    }
}
