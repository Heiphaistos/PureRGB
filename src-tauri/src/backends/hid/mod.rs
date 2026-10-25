//! Backend HID natif : détection USB de tous les contrôleurs connus
//! + pilotage direct expérimental (Corsair Lighting Node, NZXT HUE2).
//!
//! Anti-conflit : aucun handle HID n'est ouvert au scan (énumération seule).
//! Un handle n'est ouvert que si les drivers natifs sont activés ET qu'une
//! couleur est réellement appliquée — iCUE/CAM gardent sinon l'accès exclusif.

pub mod corsair_node;
pub mod gamepads;
pub mod known;
pub mod known_remote;
pub mod native;
pub mod nzxt_hue2;

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, FanChannel, ZoneInfo};
use anyhow::{bail, Context, Result};
use corsair_node::CorsairLightingNode;
use hidapi::HidApi;
use known::NativeDriver;
use nzxt_hue2::NzxtHue2;
use serde::Serialize;
use std::collections::HashMap;

/// Appareil HID brut, tel qu'énuméré par Windows — diagnostic uniquement.
/// Permet de voir ce qui est branché même quand ce n'est reconnu par
/// aucune table (accessoires bas de gamme, clones, marques absentes de
/// `known::KNOWN_VENDORS`) pour signaler précisément le VID/PID manquant.
#[derive(Debug, Clone, Serialize)]
pub struct RawHidDevice {
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    /// true si présent dans KNOWN_DEVICES ou KNOWN_VENDORS.
    pub recognized: bool,
    /// true si un driver natif PureRGB existe pour ce couple VID/PID exact.
    pub has_native_driver: bool,
}

const CORSAIR_NODE_LEDS_PER_CHANNEL: u32 = 60;
const NZXT_HUE2_LEDS_PER_CHANNEL: u32 = 40;

struct Entry {
    path: std::ffi::CString,
    /// Chemin de la seconde collection HID (modèles à deux interfaces, voir `Model::aux`).
    aux_path: Option<std::ffi::CString>,
    driver: Option<NativeDriver>,
    /// Famille native de `native/` (prioritaire sur `driver`).
    native: Option<(&'static native::Family, native::Model)>,
    /// Appareil joint en Bluetooth (bus vu par hidapi).
    bluetooth: bool,
}

enum DriverInstance {
    Corsair(CorsairLightingNode),
    Nzxt(NzxtHue2),
    Native(Box<dyn native::NativeRgb>),
}

pub struct HidBackend {
    api: Option<HidApi>,
    native_enabled: bool,
    entries: HashMap<String, Entry>,
    open_drivers: HashMap<String, DriverInstance>,
    /// Tailles de zones choisies par l'utilisateur (clé « appareil|zone »), appliquées aux zones redimensionnables.
    zone_sizes: HashMap<String, u32>,
}

/// Applique les tailles enregistrées aux zones redimensionnables du modèle (bornées par leurs limites).
fn apply_zone_sizes(model: &mut native::Model, sizes: &HashMap<String, u32>) {
    for z in model.zones.iter_mut().filter(|z| z.resizable()) {
        if let Some(n) = sizes.get(&format!("{}|{}", model.name, z.name)) {
            z.led_count = (*n).clamp(z.leds_min, z.leds_max);
        }
    }
}

impl HidBackend {
    pub fn new(native_enabled: bool) -> Self {
        HidBackend {
            api: None,
            native_enabled,
            entries: HashMap::new(),
            open_drivers: HashMap::new(),
            zone_sizes: HashMap::new(),
        }
    }

    /// Remplace les tailles de zones enregistrées (prises en compte au prochain scan).
    pub fn set_zone_sizes(&mut self, sizes: HashMap<String, u32>) {
        self.zone_sizes = sizes;
    }

    /// Mémorise la taille d'une zone ; l'appelant relance un scan.
    pub fn set_zone_size(&mut self, key: String, size: u32) {
        self.zone_sizes.insert(key, size);
    }

    pub fn set_native_enabled(&mut self, enabled: bool) {
        self.native_enabled = enabled;
        if !enabled {
            // Fermer immédiatement tous les handles pour rendre la main
            // aux logiciels constructeur.
            for d in self.open_drivers.values_mut() {
                if let DriverInstance::Native(n) = d {
                    n.release();
                }
            }
            self.open_drivers.clear();
        }
    }

    /// Liste TOUS les périphériques HID vus par Windows (dédoublonnés),
    /// reconnus ou non — pour diagnostic, jamais utilisé pour piloter quoi
    /// que ce soit.
    pub fn list_raw(&mut self) -> Result<Vec<RawHidDevice>> {
        let api = self.ensure_api()?;
        let mut seen: HashMap<(u16, u16, String), (String, String)> = HashMap::new();
        for info in api.device_list() {
            let key = (
                info.vendor_id(),
                info.product_id(),
                info.serial_number().unwrap_or("").to_string(),
            );
            seen.entry(key).or_insert_with(|| {
                (
                    info.manufacturer_string().unwrap_or("").to_string(),
                    info.product_string().unwrap_or("").to_string(),
                )
            });
        }
        let mut out: Vec<RawHidDevice> = seen
            .into_iter()
            .map(|((vid, pid, _), (manufacturer, product))| RawHidDevice {
                vid: format!("{vid:04x}"),
                pid: format!("{pid:04x}"),
                manufacturer,
                product,
                recognized: known::find_known(vid, pid).is_some()
                    || known::find_vendor(vid).is_some()
                    || known_remote::is_known_remote(vid, pid)
                    || crate::rgb_catalog::known_without_rgb(vid, pid).is_some(),
                has_native_driver: known::find_known(vid, pid).and_then(|k| k.native_driver).is_some()
                    || native::probe(vid, pid).is_some(),
            })
            .collect();
        out.sort_by(|a, b| (a.recognized, &a.vid, &a.pid).cmp(&(b.recognized, &b.vid, &b.pid)));
        Ok(out)
    }

    fn ensure_api(&mut self) -> Result<&HidApi> {
        if self.api.is_none() {
            self.api = Some(HidApi::new().context("initialisation hidapi")?);
        } else if let Some(api) = self.api.as_mut() {
            api.refresh_devices().context("rafraîchissement HID")?;
        }
        Ok(self.api.as_ref().unwrap())
    }

    fn open_driver(&mut self, local_id: &str) -> Result<&mut DriverInstance> {
        if !self.open_drivers.contains_key(local_id) {
            let entry = self
                .entries
                .get(local_id)
                .with_context(|| format!("appareil HID inconnu: {local_id}"))?;
            let api = self.api.as_ref().context("hidapi non initialisé")?;
            if let Some((family, model)) = &entry.native {
                let device = api.open_path(&entry.path).context("ouverture HID refusée (logiciel constructeur actif ?)")?;
                let handle: Box<dyn native::Hid> = match (&model.aux, &entry.aux_path) {
                    (Some(_), Some(path)) => {
                        let second = api.open_path(path).context("ouverture de la seconde interface HID refusée")?;
                        Box::new(native::PairedHid { main: Box::new(device), second: Box::new(second) })
                    }
                    (Some(_), None) => bail!("seconde interface HID de l'appareil introuvable"),
                    (None, _) => Box::new(device),
                };
                let mut model = model.clone();
                model.bluetooth = entry.bluetooth;
                let dev = (family.open)(handle, &model)?;
                self.open_drivers.insert(local_id.to_string(), DriverInstance::Native(dev));
                return Ok(self.open_drivers.get_mut(local_id).unwrap());
            }
            let driver_kind = entry
                .driver
                .context("aucun driver natif pour cet appareil (utiliser OpenRGB)")?;
            let device = api
                .open_path(&entry.path)
                .context("ouverture HID refusée (logiciel constructeur actif ?)")?;
            let instance = match driver_kind {
                NativeDriver::CorsairLightingNode => {
                    DriverInstance::Corsair(CorsairLightingNode::new(device))
                }
                NativeDriver::NzxtHue2 => DriverInstance::Nzxt(NzxtHue2::new(device)),
            };
            self.open_drivers.insert(local_id.to_string(), instance);
        }
        Ok(self.open_drivers.get_mut(local_id).unwrap())
    }
}

/// Fabricant affiché : table de VID, sinon celui de la manette, sinon le premier mot du nom (« Sony DualSense » -> Sony),
/// pour que le registre reconnaisse le même appareil vu par OpenRGB.
fn vendor_of(vid: u16, name: &str, pad_vendor: Option<&str>, native: bool) -> String {
    if let Some((_, v, _)) = known::find_vendor(vid) {
        return v.to_string();
    }
    match pad_vendor {
        Some(v) => v.to_string(),
        None if native => name.split_whitespace().next().unwrap_or_default().to_string(),
        None => String::new(),
    }
}

impl Backend for HidBackend {
    fn name(&self) -> &'static str {
        "hid"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        let native_enabled = self.native_enabled;
        let api = self.ensure_api()?;

        // Dédoublonnage par (vid, pid, serial) : Windows expose une entrée
        // par interface HID du même périphérique physique.
        // Pour une famille native, on garde l'interface demandée par le modèle (interface / page d'usage HID).
        let mut seen: HashMap<(u16, u16, String), std::ffi::CString> = HashMap::new();
        // Seconde collection des modèles à deux interfaces (contrôle + LEDs).
        let mut aux_seen: HashMap<(u16, u16, String), std::ffi::CString> = HashMap::new();
        // Manettes vues (collection Joystick/Gamepad) : (VID, PID, série) -> chaîne produit ; et appareils joints en Bluetooth.
        let mut pads: HashMap<(u16, u16, String), String> = HashMap::new();
        let mut bluetooth: std::collections::HashSet<(u16, u16, String)> = Default::default();
        for info in api.device_list() {
            let key = (info.vendor_id(), info.product_id(), info.serial_number().unwrap_or("").to_string());
            if gamepads::is_pad_collection(info.usage_page(), info.usage()) {
                pads.entry(key.clone()).or_insert_with(|| info.product_string().unwrap_or("").to_string());
            }
            if matches!(info.bus_type(), hidapi::BusType::Bluetooth) {
                bluetooth.insert(key.clone());
            }
            match native::probe(key.0, key.1) {
                Some((_, m)) => {
                    let ok = m.interface.is_none_or(|i| i == info.interface_number())
                        && m.usage_page.is_none_or(|u| u == info.usage_page())
                        && m.usage.is_none_or(|u| u == info.usage());
                    if ok {
                        seen.entry(key.clone()).or_insert_with(|| info.path().to_owned());
                    }
                    if let Some(a) = m.aux {
                        let second = a.interface.is_none_or(|i| i == info.interface_number())
                            && a.usage_page.is_none_or(|u| u == info.usage_page())
                            && a.usage.is_none_or(|u| u == info.usage());
                        if second {
                            aux_seen.entry(key).or_insert_with(|| info.path().to_owned());
                        }
                    }
                }
                None => {
                    seen.entry(key).or_insert_with(|| info.path().to_owned());
                }
            }
        }

        self.entries.clear();
        self.open_drivers.clear();
        let mut devices = Vec::new();
        let mut counters: HashMap<(u16, u16), u32> = HashMap::new();

        for ((vid, pid, _serial), path) in seen {
            let mut nat = native::probe(vid, pid);
            if let Some((_, m)) = nat.as_mut() {
                apply_zone_sizes(m, &self.zone_sizes);
            }
            let native_model = nat.is_some();
            let pad = pads.get(&(vid, pid, _serial.clone())).map(|p| gamepads::classify(vid, pid, p));
            let (name, dtype, driver) = if let Some((_, m)) = &nat {
                (m.name.clone(), m.device_type, None)
            } else if let Some(k) = known::find_known(vid, pid) {
                (k.name.to_string(), k.device_type, k.native_driver)
            } else if let Some((_, vendor_name, dtype)) = known::find_vendor(vid) {
                (format!("Appareil {vendor_name}"), *dtype, None)
            } else if let Some(p) = &pad {
                (p.name.clone(), DeviceType::Gamepad, None)
            } else {
                continue; // matériel sans rapport avec le RGB/refroidissement
            };

            let n = counters.entry((vid, pid)).or_insert(0);
            let local_id = format!("{vid:04x}:{pid:04x}:{n}");
            *n += 1;

            let (zones, led_count, fan_channels, controllable, note) = if let Some((fam, m)) = &nat {
                (
                    m.zones.clone(),
                    m.led_count(),
                    (0..m.fans)
                        .map(|i| FanChannel { index: i, name: format!("Ventilateur {}", i + 1), duty_percent: None, rpm: None })
                        .collect(),
                    native_enabled,
                    if native_enabled {
                        format!("driver natif expérimental (famille {}, d'après {})", fam.id, fam.doc)
                    } else {
                        "driver natif disponible (désactivé) — sinon via OpenRGB".to_string()
                    },
                )
            } else { match driver {
                Some(NativeDriver::CorsairLightingNode) => (
                    vec![
                        ZoneInfo::fixed("Canal 1", CORSAIR_NODE_LEDS_PER_CHANNEL),
                        ZoneInfo::fixed("Canal 2", CORSAIR_NODE_LEDS_PER_CHANNEL),
                    ],
                    CORSAIR_NODE_LEDS_PER_CHANNEL * 2,
                    Vec::new(),
                    native_enabled,
                    if native_enabled {
                        "driver natif expérimental".to_string()
                    } else {
                        "driver natif disponible (désactivé) — sinon via OpenRGB".to_string()
                    },
                ),
                Some(NativeDriver::NzxtHue2) => (
                    vec![
                        ZoneInfo::fixed("LED 1", NZXT_HUE2_LEDS_PER_CHANNEL),
                        ZoneInfo::fixed("LED 2", NZXT_HUE2_LEDS_PER_CHANNEL),
                    ],
                    NZXT_HUE2_LEDS_PER_CHANNEL * 2,
                    (0..3)
                        .map(|i| FanChannel {
                            index: i,
                            name: format!("Ventilateur {}", i + 1),
                            duty_percent: None,
                            rpm: None,
                        })
                        .collect(),
                    native_enabled,
                    if native_enabled {
                        "driver natif expérimental".to_string()
                    } else {
                        "driver natif disponible (désactivé) — sinon via OpenRGB".to_string()
                    },
                ),
                None if pad.is_some() && known::find_known(vid, pid).is_none() => (
                    Vec::new(),
                    0,
                    Vec::new(),
                    false,
                    pad.as_ref().map(|p| p.note.to_string()).unwrap_or_default(),
                ),
                None => (
                    Vec::new(),
                    0,
                    Vec::new(),
                    false,
                    "vu en USB (info) — s'il n'apparaît pas aussi côté OpenRGB : \
                     logiciel constructeur en conflit (onglet Conflits), droits admin \
                     ou matériel non supporté"
                        .to_string(),
                ),
            } };

            let aux_path = aux_seen.get(&(vid, pid, _serial.clone())).cloned();
            let bt = bluetooth.contains(&(vid, pid, _serial.clone()));
            self.entries.insert(local_id.clone(), Entry { path, aux_path, driver, native: nat, bluetooth: bt });

            devices.push(DeviceInfo {
                id: local_id,
                vendor: vendor_of(vid, &name, pad.as_ref().map(|p| p.vendor), native_model),
                backend: String::new(),
                name,
                device_type: if dtype == DeviceType::Unknown { DeviceType::Accessory } else { dtype },
                zones,
                led_count,
                fan_channels,
                controllable,
                has_lcd: false,
                modes: Vec::new(),
                active_mode: -1,
                note,
            });
        }

        devices.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(devices)
    }

    fn set_colors(&mut self, local_id: &str, colors: &[Color]) -> Result<()> {
        if !self.native_enabled {
            bail!("drivers natifs désactivés — activer dans Réglages ou passer par OpenRGB");
        }
        let driver = self.open_driver(local_id)?;
        match driver {
            DriverInstance::Native(d) => d.set_colors(colors),
            DriverInstance::Corsair(d) => {
                let per = CORSAIR_NODE_LEDS_PER_CHANNEL as usize;
                for ch in 0..2u8 {
                    let start = ch as usize * per;
                    let end = (start + per).min(colors.len());
                    if start >= colors.len() {
                        break;
                    }
                    d.set_channel_colors(ch, &colors[start..end])?;
                }
                Ok(())
            }
            DriverInstance::Nzxt(d) => {
                let per = NZXT_HUE2_LEDS_PER_CHANNEL as usize;
                for ch in 0..2u8 {
                    let start = ch as usize * per;
                    let end = (start + per).min(colors.len());
                    if start >= colors.len() {
                        break;
                    }
                    d.set_channel_colors(ch, &colors[start..end])?;
                }
                Ok(())
            }
        }
    }

    fn set_fan_duty(&mut self, local_id: &str, channel: u8, percent: u8) -> Result<()> {
        if !self.native_enabled {
            bail!("drivers natifs désactivés — activer dans Réglages");
        }
        let driver = self.open_driver(local_id)?;
        match driver {
            DriverInstance::Nzxt(d) => d.set_fan_duty(channel, percent),
            DriverInstance::Native(d) => d.set_fan_duty(channel, percent),
            _ => bail!("contrôle ventilateur non supporté sur cet appareil"),
        }
    }

    fn is_available(&self) -> bool {
        self.api.is_some()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn saved_sizes_apply_only_to_resizable_zones_within_bounds() {
        let (_, mut m) = native::probe(0x1462, 0x7D93).unwrap();
        let sizes: HashMap<String, u32> = [
            (format!("{}|JRAINBOW 1", m.name), 40),
            (format!("{}|JRAINBOW 2", m.name), 9999),
            (format!("{}|JRGB 1", m.name), 12),
        ]
        .into();
        apply_zone_sizes(&mut m, &sizes);
        let n = |name: &str| m.zones.iter().find(|z| z.name == name).unwrap().led_count;
        assert_eq!(n("JRAINBOW 1"), 40);
        assert_eq!(n("JRAINBOW 2"), 75);
        assert_eq!(n("JRGB 1"), 1);
    }
}
