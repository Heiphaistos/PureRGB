use crate::backends::Backend;
use crate::core::{Color, DeviceInfo};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Registre central : agrège les appareils de tous les backends
/// sous des ids globaux "<backend>:<id local>".
pub struct DeviceRegistry {
    backends: Vec<Box<dyn Backend>>,
    devices: HashMap<String, DeviceInfo>,
}

pub type SharedRegistry = Arc<Mutex<DeviceRegistry>>;

impl DeviceRegistry {
    pub fn new(backends: Vec<Box<dyn Backend>>) -> Self {
        DeviceRegistry {
            backends,
            devices: HashMap::new(),
        }
    }

    pub fn shared(backends: Vec<Box<dyn Backend>>) -> SharedRegistry {
        Arc::new(Mutex::new(Self::new(backends)))
    }

    /// Re-scanne tous les backends. Les erreurs d'un backend n'empêchent pas les autres.
    pub fn scan_all(&mut self) -> Vec<DeviceInfo> {
        self.devices.clear();
        for backend in &mut self.backends {
            let bname = backend.name();
            match backend.scan() {
                Ok(devs) => {
                    for mut d in devs {
                        d.backend = bname.to_string();
                        d.id = format!("{}:{}", bname, d.id);
                        self.devices.insert(d.id.clone(), d);
                    }
                }
                Err(e) => log::warn!("scan backend {bname} en échec: {e:#}"),
            }
        }
        self.device_list()
    }

    pub fn device_list(&self) -> Vec<DeviceInfo> {
        let mut list: Vec<DeviceInfo> = self.devices.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub fn get(&self, id: &str) -> Option<&DeviceInfo> {
        self.devices.get(id)
    }

    fn split_id<'a>(&self, id: &'a str) -> Result<(&'a str, &'a str)> {
        id.split_once(':')
            .ok_or_else(|| anyhow!("id d'appareil invalide: {id}"))
    }

    pub fn set_colors(&mut self, id: &str, colors: &[Color]) -> Result<()> {
        let (bname, local) = self.split_id(id)?;
        let (bname, local) = (bname.to_string(), local.to_string());
        let backend = self
            .backends
            .iter_mut()
            .find(|b| b.name() == bname)
            .ok_or_else(|| anyhow!("backend inconnu: {bname}"))?;
        backend.set_colors(&local, colors)
    }

    pub fn set_fan_duty(&mut self, id: &str, channel: u8, percent: u8) -> Result<()> {
        let (bname, local) = self.split_id(id)?;
        let (bname, local) = (bname.to_string(), local.to_string());
        let backend = self
            .backends
            .iter_mut()
            .find(|b| b.name() == bname)
            .ok_or_else(|| anyhow!("backend inconnu: {bname}"))?;
        backend.set_fan_duty(&local, channel, percent.min(100))
    }

    pub fn backend_status(&self) -> Vec<(String, bool)> {
        self.backends
            .iter()
            .map(|b| (b.name().to_string(), b.is_available()))
            .collect()
    }

    pub fn backends_mut(&mut self) -> &mut Vec<Box<dyn Backend>> {
        &mut self.backends
    }
}
