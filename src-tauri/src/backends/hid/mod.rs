//! Backend HID natif : détection USB de tous les contrôleurs connus
//! + pilotage direct expérimental (Corsair Lighting Node, NZXT HUE2).
//!
//! Anti-conflit : aucun handle HID n'est ouvert au scan (énumération seule).
//! Un handle n'est ouvert que si les drivers natifs sont activés ET qu'une
//! couleur est réellement appliquée — iCUE/CAM gardent sinon l'accès exclusif.

pub mod corsair_node;
pub mod known;
pub mod known_remote;
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
    driver: Option<NativeDriver>,
}

enum DriverInstance {
    Corsair(CorsairLightingNode),
    Nzxt(NzxtHue2),
}

pub struct HidBackend {
    api: Option<HidApi>,
    native_enabled: bool,
    entries: HashMap<String, Entry>,
    open_drivers: HashMap<String, DriverInstance>,
}

impl HidBackend {
    pub fn new(native_enabled: bool) -> Self {
        HidBackend {
            api: None,
            native_enabled,
            entries: HashMap::new(),
            open_drivers: HashMap::new(),
        }
    }

    pub fn set_native_enabled(&mut self, enabled: bool) {
        self.native_enabled = enabled;
        if !enabled {
            // Fermer immédiatement tous les handles pour rendre la main
            // aux logiciels constructeur.
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
                    || known_remote::is_known_remote(vid, pid),
                has_native_driver: known::find_known(vid, pid)
                    .and_then(|k| k.native_driver)
                    .is_some(),
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
            let driver_kind = entry
                .driver
                .context("aucun driver natif pour cet appareil (utiliser OpenRGB)")?;
            let api = self.api.as_ref().context("hidapi non initialisé")?;
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

impl Backend for HidBackend {
    fn name(&self) -> &'static str {
        "hid"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        let native_enabled = self.native_enabled;
        let api = self.ensure_api()?;

        // Dédoublonnage par (vid, pid, serial) : Windows expose une entrée
        // par interface HID du même périphérique physique.
        let mut seen: HashMap<(u16, u16, String), std::ffi::CString> = HashMap::new();
        for info in api.device_list() {
            let key = (
                info.vendor_id(),
                info.product_id(),
                info.serial_number().unwrap_or("").to_string(),
            );
            seen.entry(key).or_insert_with(|| info.path().to_owned());
        }

        self.entries.clear();
        self.open_drivers.clear();
        let mut devices = Vec::new();
        let mut counters: HashMap<(u16, u16), u32> = HashMap::new();

        for ((vid, pid, _serial), path) in seen {
            let (name, dtype, driver) = if let Some(k) = known::find_known(vid, pid) {
                (k.name.to_string(), k.device_type, k.native_driver)
            } else if let Some((_, vendor_name, dtype)) = known::find_vendor(vid) {
                (format!("Appareil {vendor_name}"), *dtype, None)
            } else {
                continue; // matériel sans rapport avec le RGB/refroidissement
            };

            let n = counters.entry((vid, pid)).or_insert(0);
            let local_id = format!("{vid:04x}:{pid:04x}:{n}");
            *n += 1;

            let (zones, led_count, fan_channels, controllable, note) = match driver {
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
            };

            self.entries.insert(local_id.clone(), Entry { path, driver });

            devices.push(DeviceInfo {
                id: local_id,
                name,
                vendor: known::find_vendor(vid).map(|(_, v, _)| v.to_string()).unwrap_or_default(),
                backend: String::new(),
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
