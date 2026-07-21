//! Appareils réseau / maison connectée (ampoules, bandeaux Wi-Fi, panneaux…)
//! pilotés via les détecteurs réseau d'OpenRGB. PureRGB écrit leur
//! configuration dans %APPDATA%\OpenRGB\OpenRGB.json (sections et champs
//! vérifiés dans le binaire 1.0rc3 embarqué + source officiel), puis
//! redémarre le serveur pour qu'ils apparaissent comme n'importe quel
//! contrôleur RGB.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Appareil réseau déclaré par l'utilisateur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NetworkDevice {
    /// Pont Philips Hue. `username`/`clientkey` sont créés par OpenRGB après
    /// appui sur le bouton du pont — la synchro les préserve.
    Hue {
        ip: String,
        mac: String,
        #[serde(default)]
        entertainment: bool,
    },
    /// Panneaux Nanoleaf (Shapes, Canvas, Light Panels…). Token obtenu en
    /// mode appairage (bouton power 5-7 s).
    Nanoleaf {
        ip: String,
        port: u16,
        auth_token: String,
    },
    /// Ampoules/bandeaux Yeelight (« Contrôle LAN » activé dans l'app).
    Yeelight {
        ip: String,
        #[serde(default)]
        music_mode: bool,
    },
    /// Ampoules/bandeaux LIFX.
    Lifx {
        ip: String,
        name: String,
        #[serde(default)]
        multizone: bool,
        #[serde(default)]
        extended_multizone: bool,
    },
    /// Éclairages Govee (« API LAN » activée dans l'app Govee Home).
    Govee { ip: String },
    /// Ampoules Philips Wiz.
    Wiz { ip: String },
    /// Elgato Key Light / Key Light Air.
    ElgatoKeyLight { ip: String },
    /// Elgato Light Strip.
    ElgatoLightStrip { ip: String },
    /// Prises/ampoules TP-Link Kasa.
    Kasa { ip: String, name: String },
    /// WLED et tout récepteur E1.31/sACN (contrôleurs de bandeaux DIY).
    E131 {
        name: String,
        ip: String,
        num_leds: u32,
        start_universe: u32,
        start_channel: u32,
        universe_size: u32,
        keepalive_time: u32,
    },
}

impl NetworkDevice {
    pub fn ip(&self) -> &str {
        match self {
            NetworkDevice::Hue { ip, .. }
            | NetworkDevice::Nanoleaf { ip, .. }
            | NetworkDevice::Yeelight { ip, .. }
            | NetworkDevice::Lifx { ip, .. }
            | NetworkDevice::Govee { ip }
            | NetworkDevice::Wiz { ip }
            | NetworkDevice::ElgatoKeyLight { ip }
            | NetworkDevice::ElgatoLightStrip { ip }
            | NetworkDevice::Kasa { ip, .. }
            | NetworkDevice::E131 { ip, .. } => ip,
        }
    }

    /// Nom du détecteur OpenRGB correspondant (clé REGISTER_DETECTOR).
    fn detector_name(&self) -> &'static str {
        match self {
            NetworkDevice::Hue { .. } => "Philips Hue",
            NetworkDevice::Nanoleaf { .. } => "Nanoleaf",
            NetworkDevice::Yeelight { .. } => "Yeelight",
            NetworkDevice::Lifx { .. } => "LIFX",
            NetworkDevice::Govee { .. } => "Govee",
            NetworkDevice::Wiz { .. } => "Philips Wiz",
            NetworkDevice::ElgatoKeyLight { .. } => "ElgatoKeyLight",
            NetworkDevice::ElgatoLightStrip { .. } => "ElgatoLightStrip",
            NetworkDevice::Kasa { .. } => "KasaSmart",
            NetworkDevice::E131 { .. } => "E1.31",
        }
    }

    pub fn validate(&self) -> Result<()> {
        validate_host(self.ip())?;
        match self {
            NetworkDevice::Nanoleaf { auth_token, .. } if auth_token.trim().is_empty() => {
                bail!("token Nanoleaf manquant — lancer l'appairage d'abord")
            }
            NetworkDevice::Hue { mac, .. } if mac.trim().is_empty() => {
                bail!("adresse MAC du pont Hue manquante — lancer l'appairage d'abord")
            }
            NetworkDevice::E131 { num_leds, .. } if *num_leds == 0 || *num_leds > 4096 => {
                bail!("nombre de LEDs E1.31 invalide (1-4096)")
            }
            _ => Ok(()),
        }
    }
}

/// Hôte sûr pour une URL/un argument : IPv4, IPv6 ou nom d'hôte simple.
fn validate_host(host: &str) -> Result<()> {
    let h = host.trim();
    if h.is_empty() || h.len() > 253 {
        bail!("adresse invalide");
    }
    if h.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    if h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !h.starts_with('-')
    {
        return Ok(());
    }
    bail!("adresse invalide: {h}")
}

/// Chemin de la config du serveur OpenRGB (dossier par défaut, le manager
/// lance OpenRGB sans --config).
pub fn openrgb_config_path() -> Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA").context("APPDATA introuvable")?;
    Ok(PathBuf::from(appdata).join("OpenRGB").join("OpenRGB.json"))
}

/// Sections d'OpenRGB.json possédées par PureRGB : réécrites entièrement à
/// chaque synchro (un appareil supprimé côté PureRGB disparaît de la config).
const OWNED_SECTIONS: [&str; 10] = [
    "PhilipsHueDevices",
    "NanoleafDevices",
    "YeelightDevices",
    "LIFXDevices",
    "GoveeDevices",
    "PhilipsWizDevices",
    "ElgatoKeyLightDevices",
    "ElgatoLightStripDevices",
    "KasaSmartDevices",
    "E131Devices",
];

/// Écrit les appareils réseau dans la config OpenRGB, en préservant tout le
/// reste du fichier ainsi que les identifiants Hue déjà appairés.
pub fn sync_openrgb_config(devices: &[NetworkDevice], path: &Path) -> Result<()> {
    let mut root: Value = match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };
    if !root.is_object() {
        root = json!({});
    }

    // Identifiants Hue existants (username/clientkey), indexés par IP.
    let mut hue_creds: Map<String, Value> = Map::new();
    if let Some(bridges) = root
        .get("PhilipsHueDevices")
        .and_then(|s| s.get("bridges"))
        .and_then(|b| b.as_array())
    {
        for b in bridges {
            if let Some(ip) = b.get("ip").and_then(|v| v.as_str()) {
                hue_creds.insert(ip.to_string(), b.clone());
            }
        }
    }

    let obj = root.as_object_mut().expect("root objet");
    for section in OWNED_SECTIONS {
        obj.remove(section);
    }

    let mut hue_bridges: Vec<Value> = Vec::new();
    let mut lists: Map<String, Value> = Map::new();
    let mut push = |section: &str, entry: Value| {
        lists
            .entry(section.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("liste")
            .push(entry);
    };

    for d in devices {
        match d {
            NetworkDevice::Hue {
                ip,
                mac,
                entertainment,
            } => {
                let mut entry = json!({
                    "ip": ip,
                    "mac": mac,
                    "autoconnect": true,
                    "entertainment": entertainment,
                });
                if let Some(prev) = hue_creds.get(ip.as_str()) {
                    for key in ["username", "clientkey"] {
                        if let Some(v) = prev.get(key) {
                            entry[key] = v.clone();
                        }
                    }
                }
                hue_bridges.push(entry);
            }
            NetworkDevice::Nanoleaf {
                ip,
                port,
                auth_token,
            } => push(
                "NanoleafDevices",
                json!({ "ip": ip, "port": port, "auth_token": auth_token }),
            ),
            NetworkDevice::Yeelight { ip, music_mode } => push(
                "YeelightDevices",
                json!({ "ip": ip, "music_mode": music_mode }),
            ),
            NetworkDevice::Lifx {
                ip,
                name,
                multizone,
                extended_multizone,
            } => push(
                "LIFXDevices",
                json!({
                    "ip": ip,
                    "name": name,
                    "multizone": multizone,
                    "extended_multizone": extended_multizone,
                }),
            ),
            NetworkDevice::Govee { ip } => push("GoveeDevices", json!({ "ip": ip })),
            NetworkDevice::Wiz { ip } => push("PhilipsWizDevices", json!({ "ip": ip })),
            NetworkDevice::ElgatoKeyLight { ip } => {
                push("ElgatoKeyLightDevices", json!({ "ip": ip }))
            }
            NetworkDevice::ElgatoLightStrip { ip } => {
                push("ElgatoLightStripDevices", json!({ "ip": ip }))
            }
            NetworkDevice::Kasa { ip, name } => {
                push("KasaSmartDevices", json!({ "ip": ip, "name": name }))
            }
            NetworkDevice::E131 {
                name,
                ip,
                num_leds,
                start_universe,
                start_channel,
                universe_size,
                keepalive_time,
            } => push(
                "E131Devices",
                json!({
                    "name": name,
                    "ip": ip,
                    "num_leds": num_leds,
                    "start_universe": start_universe,
                    "start_channel": start_channel,
                    "universe_size": universe_size,
                    "keepalive_time": keepalive_time,
                }),
            ),
        }
    }

    if !hue_bridges.is_empty() {
        obj.insert("PhilipsHueDevices".into(), json!({ "bridges": hue_bridges }));
    }
    for (section, list) in lists {
        obj.insert(section, json!({ "devices": list }));
    }

    // S'assurer que les détecteurs concernés ne sont pas désactivés.
    {
        let detectors = obj
            .entry("Detectors")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .context("section Detectors invalide")?
            .entry("detectors")
            .or_insert_with(|| json!({}));
        if let Some(map) = detectors.as_object_mut() {
            for d in devices {
                map.insert(d.detector_name().to_string(), json!(true));
            }
        }
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("création du dossier OpenRGB")?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&root)?)
        .context("écriture OpenRGB.json")
}

/// Requête HTTP locale via curl.exe (présent sur Windows 10/11) — évite
/// d'embarquer un client HTTP complet pour deux appels d'appairage LAN.
pub(crate) fn curl(args: &[&str]) -> Result<String> {
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.args(["-s", "--max-time", "8"]).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().context("lancement de curl.exe")?;
    if !out.status.success() && out.stdout.is_empty() {
        bail!(
            "requête échouée: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Interroge un pont Hue et retourne son adresse MAC (nécessaire à OpenRGB).
/// Les ponts récents ne répondent qu'en HTTPS (certificat auto-signé).
pub fn hue_fetch_mac(ip: &str) -> Result<String> {
    validate_host(ip)?;
    let body = curl(&["-k", &format!("https://{ip}/api/config")])
        .or_else(|_| curl(&[&format!("http://{ip}/api/config")]))?;
    let v: Value = serde_json::from_str(body.trim())
        .with_context(|| format!("réponse du pont illisible: {}", body.trim()))?;
    v.get("mac")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
        .context("le pont n'a pas renvoyé d'adresse MAC — vérifier l'IP")
}

/// Demande un auth_token à un Nanoleaf en mode appairage (maintenir le
/// bouton power 5-7 s jusqu'au clignotement, puis appeler dans les 30 s).
pub fn nanoleaf_request_token(ip: &str, port: u16) -> Result<String> {
    validate_host(ip)?;
    let body = curl(&[
        "-X",
        "POST",
        &format!("http://{ip}:{port}/api/v1/new"),
    ])?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        bail!("pas de réponse — appareil pas en mode appairage ? (bouton power 5-7 s)");
    }
    let v: Value = serde_json::from_str(trimmed)
        .with_context(|| format!("réponse Nanoleaf illisible: {trimmed}"))?;
    v.get("auth_token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .context("token refusé — activer le mode appairage puis réessayer")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("purergb_netdev_{name}.json"))
    }

    #[test]
    fn sync_writes_sections_and_cleans_removed() {
        let path = tmp_path("sections");
        let _ = std::fs::remove_file(&path);
        let devices = vec![
            NetworkDevice::Govee { ip: "192.168.1.50".into() },
            NetworkDevice::E131 {
                name: "WLED salon".into(),
                ip: "192.168.1.60".into(),
                num_leds: 120,
                start_universe: 1,
                start_channel: 1,
                universe_size: 510,
                keepalive_time: 0,
            },
        ];
        sync_openrgb_config(&devices, &path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["GoveeDevices"]["devices"][0]["ip"], "192.168.1.50");
        assert_eq!(v["E131Devices"]["devices"][0]["num_leds"], 120);
        assert_eq!(v["Detectors"]["detectors"]["Govee"], true);
        assert_eq!(v["Detectors"]["detectors"]["E1.31"], true);

        // Suppression du Govee : sa section doit disparaître, l'autre rester.
        sync_openrgb_config(&devices[1..], &path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v.get("GoveeDevices").is_none());
        assert_eq!(v["E131Devices"]["devices"][0]["name"], "WLED salon");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sync_preserves_hue_credentials_and_foreign_sections() {
        let path = tmp_path("hue");
        std::fs::write(
            &path,
            r#"{
                "PhilipsHueDevices": { "bridges": [
                    { "ip": "10.0.0.2", "mac": "aa:bb", "username": "secretuser", "clientkey": "secretkey" }
                ]},
                "LEDStripDevices": { "keep": "me" }
            }"#,
        )
        .unwrap();
        let devices = vec![NetworkDevice::Hue {
            ip: "10.0.0.2".into(),
            mac: "aa:bb".into(),
            entertainment: false,
        }];
        sync_openrgb_config(&devices, &path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let b = &v["PhilipsHueDevices"]["bridges"][0];
        assert_eq!(b["username"], "secretuser");
        assert_eq!(b["clientkey"], "secretkey");
        assert_eq!(b["autoconnect"], true);
        // Section non gérée par PureRGB : intacte.
        assert_eq!(v["LEDStripDevices"]["keep"], "me");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn validate_rejects_bad_hosts() {
        assert!(validate_host("192.168.1.10").is_ok());
        assert!(validate_host("wled-salon.local").is_ok());
        assert!(validate_host("").is_err());
        assert!(validate_host("http://evil").is_err());
        assert!(validate_host("host;rm -rf").is_err());
        assert!(validate_host("-flag").is_err());
    }

    #[test]
    fn validate_device_requirements() {
        assert!(NetworkDevice::Nanoleaf {
            ip: "1.2.3.4".into(),
            port: 16021,
            auth_token: "".into()
        }
        .validate()
        .is_err());
        assert!(NetworkDevice::Hue {
            ip: "1.2.3.4".into(),
            mac: "".into(),
            entertainment: false
        }
        .validate()
        .is_err());
        assert!(NetworkDevice::Govee { ip: "1.2.3.4".into() }.validate().is_ok());
    }
}
