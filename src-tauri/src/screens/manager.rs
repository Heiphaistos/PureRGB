//! Gestionnaire des écrans : détection, un `Runner` par écran, réglages persistés.

use super::bindings::ValueStore;
use super::designs::DesignStore;
use super::driver::{ScreenDriver, ScreenInfo, ScreenProvider};
use super::drivers::{
    thermalright_bulk::BulkProvider, thermalright_hid::HidProvider, thermalright_ly::LyProvider,
    thermalright_scsi::ScsiProvider, turing_serial::TuringProvider,
};
use super::layout::Layout;
use super::media::MediaStore;
use super::runner::Runner;
use super::sim::SimProvider;
use super::state::{self, ConfigMap, ScreenConfig};
use crate::sensors::SensorHub;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct ScreenState {
    pub layout: Option<Layout>,
    pub brightness: u8,
    pub rotation: u16,
    pub fps: u8,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScreenOptions {
    pub brightness: u8,
    pub rotation: u16,
    pub fps: u8,
}

pub struct ScreenManager {
    pub store: Arc<ValueStore>,
    pub media: Arc<MediaStore>,
    pub designs: DesignStore,
    runners: Mutex<Vec<Runner>>,
    configs: Mutex<ConfigMap>,
    /// Une détection à la fois (elle ouvre les périphériques).
    scanning: Mutex<()>,
}

fn providers(with_sim: bool) -> Vec<Box<dyn ScreenProvider>> {
    let mut v: Vec<Box<dyn ScreenProvider>> = vec![
        Box::new(ScsiProvider),
        Box::new(HidProvider),
        Box::new(BulkProvider),
        Box::new(LyProvider),
        Box::new(TuringProvider::default()),
    ];
    if with_sim {
        v.push(Box::new(SimProvider));
    }
    v
}

impl ScreenManager {
    pub fn new(hub: Arc<SensorHub>) -> Arc<ScreenManager> {
        let root = state::data_dir().unwrap_or_else(|| std::env::temp_dir().join("PureRGB-screens"));
        let store = ValueStore::new(hub);
        let ticker = store.clone();
        let spawned = std::thread::Builder::new().name("screens-values".into()).spawn(move || loop {
            ticker.tick();
            std::thread::sleep(Duration::from_secs(1));
        });
        if let Err(e) = spawned {
            log::error!("thread des valeurs d'écran: {e}");
        }
        Arc::new(ScreenManager {
            store,
            media: Arc::new(MediaStore::new(root.join("media"))),
            designs: DesignStore::new(root),
            runners: Mutex::new(Vec::new()),
            configs: Mutex::new(state::load()),
            scanning: Mutex::new(()),
        })
    }

    /// Arrête les écrans en cours, détecte à nouveau et relance avec les réglages sauvegardés.
    pub fn scan(&self, with_sim: bool) -> Vec<ScreenInfo> {
        let _one_at_a_time = self.scanning.lock();
        let old: Vec<Runner> = std::mem::take(&mut *self.runners.lock());
        for r in old {
            r.stop();
        }
        let mut found: Vec<Box<dyn ScreenDriver>> = Vec::new();
        for mut p in providers(with_sim) {
            match p.detect() {
                Ok(mut d) => found.append(&mut d),
                Err(e) => log::warn!("détection d'écrans ({}) : {e:#}", p.name()),
            }
        }
        let mut runners = Vec::new();
        for d in found {
            let cfg = self.configs.lock().get(&d.info().id).cloned().unwrap_or_default().clamped();
            runners.push(Runner::start(d, cfg, self.store.clone(), self.media.clone()));
        }
        let infos = runners.iter().map(|r| r.info.clone()).collect();
        *self.runners.lock() = runners;
        infos
    }

    pub fn list(&self) -> Vec<ScreenInfo> {
        self.runners.lock().iter().map(|r| r.info.clone()).collect()
    }

    fn with_runner<T>(&self, id: &str, f: impl FnOnce(&Runner) -> T) -> Result<T> {
        let runners = self.runners.lock();
        let r = runners.iter().find(|r| r.info.id == id).ok_or_else(|| anyhow!("écran introuvable : {id}"))?;
        Ok(f(r))
    }

    pub fn state(&self, id: &str) -> Result<ScreenState> {
        self.with_runner(id, |r| {
            let c = r.shared.cfg.lock();
            ScreenState {
                layout: c.layout.clone(),
                brightness: c.brightness,
                rotation: c.rotation,
                fps: c.fps,
                running: r.is_running(),
                error: r.shared.error.lock().clone(),
            }
        })
    }

    fn update(&self, id: &str, change: impl FnOnce(&mut ScreenConfig)) -> Result<()> {
        let cfg = self.with_runner(id, |r| {
            let mut c = r.shared.cfg.lock();
            change(&mut c);
            *c = c.clone().clamped();
            r.touch();
            c.clone()
        })?;
        let mut all = self.configs.lock();
        all.insert(id.to_string(), cfg);
        state::save(&all)
    }

    pub fn set_layout(&self, id: &str, layout: Layout) -> Result<()> {
        layout.validate().map_err(anyhow::Error::msg)?;
        self.update(id, |c| c.layout = Some(layout))
    }

    pub fn set_options(&self, id: &str, o: ScreenOptions) -> Result<()> {
        self.update(id, |c| {
            c.brightness = o.brightness;
            c.rotation = o.rotation;
            c.fps = o.fps;
        })
    }

    pub fn clear(&self, id: &str) -> Result<()> {
        self.update(id, |c| c.layout = None)
    }

    /// Libère tous les périphériques (fermeture de l'application).
    pub fn shutdown(&self) {
        let old: Vec<Runner> = std::mem::take(&mut *self.runners.lock());
        for r in old {
            r.stop();
        }
    }
}
