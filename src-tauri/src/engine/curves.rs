//! Courbes ventilateurs par température : un thread lit les capteurs (sensord)
//! toutes les 3 s et applique le duty interpolé aux canaux configurés.
//! Hystérésis : on n'écrit sur le matériel que si le duty change de ≥ 3 points
//! (évite l'usure et le bruit de commutation).

use crate::core::registry::SharedRegistry;
use crate::sensors::SensorHub;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub temp: f32,
    pub duty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveConfig {
    pub sensor_id: String,
    /// Points triés par température croissante (2 à 8 points).
    pub points: Vec<CurvePoint>,
    pub enabled: bool,
}

impl CurveConfig {
    /// Duty interpolé linéairement ; borné par le premier/dernier point.
    pub fn duty_for(&self, temp: f32) -> u8 {
        let pts = &self.points;
        match pts.iter().position(|p| temp < p.temp) {
            Some(0) => pts[0].duty,
            None => pts.last().map(|p| p.duty).unwrap_or(0),
            Some(i) => {
                let (a, b) = (&pts[i - 1], &pts[i]);
                let span = (b.temp - a.temp).max(0.001);
                let t = (temp - a.temp) / span;
                (a.duty as f32 + (b.duty as f32 - a.duty as f32) * t).round() as u8
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.points.len() < 2 || self.points.len() > 8 {
            return Err("2 à 8 points requis".into());
        }
        if !self.points.windows(2).all(|w| w[0].temp < w[1].temp) {
            return Err("températures non croissantes".into());
        }
        if self.points.iter().any(|p| p.duty > 100 || p.temp < 0.0 || p.temp > 120.0) {
            return Err("valeurs hors bornes (0-120 °C, 0-100 %)".into());
        }
        Ok(())
    }
}

/// Clé de courbe : "<device_id_global>|<index_canal>".
pub type CurveMap = HashMap<String, CurveConfig>;

pub struct CurveEngine {
    curves: Arc<Mutex<CurveMap>>,
    last_duty: Mutex<HashMap<String, u8>>,
    running: AtomicBool,
}

impl CurveEngine {
    pub fn start(
        registry: SharedRegistry,
        sensors: Arc<SensorHub>,
        initial: CurveMap,
    ) -> Arc<Self> {
        let engine = Arc::new(CurveEngine {
            curves: Arc::new(Mutex::new(initial)),
            last_duty: Mutex::new(HashMap::new()),
            running: AtomicBool::new(true),
        });
        let e = engine.clone();
        std::thread::Builder::new()
            .name("fan-curves".into())
            .spawn(move || {
                while e.running.load(Ordering::Relaxed) {
                    e.tick(&registry, &sensors);
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            })
            .expect("spawn fan-curves");
        engine
    }

    pub fn set_curves(&self, curves: CurveMap) {
        *self.curves.lock() = curves;
        self.last_duty.lock().clear(); // forcer la ré-application
    }

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn tick(&self, registry: &SharedRegistry, sensors: &Arc<SensorHub>) {
        let curves = self.curves.lock().clone();
        for (key, cfg) in curves.iter().filter(|(_, c)| c.enabled) {
            let Some((device_id, chan)) = key.rsplit_once('|') else {
                continue;
            };
            let Ok(chan) = chan.parse::<u8>() else { continue };
            let Some(temp) = sensors.value(&cfg.sensor_id) else {
                continue; // capteur indisponible : ne rien changer
            };
            let duty = cfg.duty_for(temp as f32);
            let apply = {
                let last = self.last_duty.lock();
                last.get(key).map_or(true, |d| (*d as i16 - duty as i16).abs() >= 3)
            };
            if apply {
                match registry.lock().set_fan_duty(device_id, chan, duty) {
                    Ok(()) => {
                        self.last_duty.lock().insert(key.clone(), duty);
                    }
                    Err(e) => log::warn!("courbe {key}: {e:#}"),
                }
            }
        }
    }
}
