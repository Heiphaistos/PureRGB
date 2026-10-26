//! Gestionnaire des écrans : détection, un `Runner` par écran, réglages persistés.

use super::bindings::ValueStore;
use super::designs::DesignStore;
use super::driver::{ScreenDriver, ScreenInfo, ScreenProvider};
use super::drivers::{
    asus_ryujin::RyujinProvider, lianli_hid::LianLiHidProvider, turzx_usb::TurzxUsbProvider, corsair_lcd::CorsairProvider, deepcool_lm::DeepcoolLmProvider,
    thermaltake_lcd::ThermaltakeProvider, thermalright_bulk::BulkProvider, thermalright_hid::HidProvider, thermalright_ly::LyProvider,
    thermalright_scsi::ScsiProvider, turing_serial::TuringProvider,
};
use super::layout::Layout;
use super::media::MediaStore;
use super::pacing::Perf;
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
    /// Faux : l'écran n'accepte que des images fixes (bouton « Envoyer comme image fixe »).
    pub live: bool,
    pub error: Option<String>,
    /// Cadence réelle et temps moyens rendu/encodage/envoi.
    pub perf: Perf,
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
    /// Raisons des échecs de la dernière détection (affichées quand aucun écran n'apparaît).
    errors: Mutex<Vec<String>>,
}

fn providers(with_sim: bool) -> Vec<Box<dyn ScreenProvider>> {
    let mut v: Vec<Box<dyn ScreenProvider>> = vec![
        Box::new(ScsiProvider),
        Box::new(HidProvider),
        Box::new(BulkProvider),
        Box::new(LyProvider),
        Box::new(TuringProvider::default()),
        Box::new(CorsairProvider::default()),
        Box::new(DeepcoolLmProvider),
        Box::new(RyujinProvider),
        Box::new(ThermaltakeProvider),
        Box::new(LianLiHidProvider),
        Box::new(TurzxUsbProvider),
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
            media: {
                let media = Arc::new(MediaStore::new(root.join("media")));
                super::builtin_media::install(&media);
                media
            },
            designs: DesignStore::new(root),
            runners: Mutex::new(Vec::new()),
            configs: Mutex::new(state::load()),
            scanning: Mutex::new(()),
            errors: Mutex::new(Vec::new()),
        })
    }

    /// Arrête les écrans en cours, détecte à nouveau et relance avec les réglages sauvegardés.
    pub fn scan(&self, with_sim: bool) -> Vec<ScreenInfo> {
        let _one_at_a_time = self.scanning.lock();
        if crate::gamemode::active() {
            return Vec::new(); // mode jeu : l'interface USB des écrans reste libre
        }
        let old: Vec<Runner> = std::mem::take(&mut *self.runners.lock());
        for r in old {
            r.stop();
        }
        let mut found: Vec<Box<dyn ScreenDriver>> = Vec::new();
        let mut errors = Vec::new();
        for mut p in providers(with_sim) {
            match p.detect() {
                Ok(mut d) => found.append(&mut d),
                Err(e) => {
                    log::warn!("détection d'écrans ({}) : {e:#}", p.name());
                    errors.push(format!("{} : {e:#}", p.name()));
                }
            }
        }
        *self.errors.lock() = errors;
        let mut runners = Vec::new();
        for d in found {
            let cfg = self.configs.lock().get(&d.info().id).cloned().unwrap_or_default().clamped();
            runners.push(Runner::start(d, cfg, self.store.clone(), self.media.clone()));
        }
        let infos = runners.iter().map(|r| r.info.clone()).collect();
        *self.runners.lock() = runners;
        infos
    }

    pub fn scan_errors(&self) -> Vec<String> {
        self.errors.lock().clone()
    }

    /// Écrans du catalogue branchés en USB mais sans pilote actif (détection ratée ou branchement tardif).
    pub fn plugged_but_idle(&self) -> Vec<String> {
        let Ok(devices) = nusb::list_devices() else { return Vec::new() };
        let running: Vec<String> = self.runners.lock().iter().map(|r| r.info.id.clone()).collect();
        let mut idle: Vec<String> = devices
            .filter(|d| super::drivers::catalog::find(d.vendor_id(), d.product_id()).is_some())
            .map(|d| format!("{:04x}:{:04x}", d.vendor_id(), d.product_id()))
            .filter(|key| !running.iter().any(|id| id.starts_with(key.as_str())))
            .collect();
        idle.sort();
        idle
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
                live: r.shared.live,
                error: r.shared.error.lock().clone(),
                perf: r.perf(),
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

    /// Envoie le design courant comme image fixe (écrans sans flux direct).
    pub fn send_still(&self, id: &str) -> Result<()> {
        self.with_runner(id, |r| {
            if r.shared.live {
                return Err(anyhow!("cet écran affiche déjà le design en direct"));
            }
            r.request_still();
            Ok(())
        })?
    }

    pub fn clear(&self, id: &str) -> Result<()> {
        self.update(id, |c| c.layout = None)
    }

    /// Un écran tourne avec un design (c'est lui que « garder les écrans allumés » doit préserver).
    pub fn has_active_design(&self) -> bool {
        self.runners.lock().iter().any(|r| r.is_running() && r.shared.cfg.lock().layout.is_some())
    }

    /// Designs des écrans en cours (pour savoir si les capteurs matériels sont nécessaires).
    pub fn active_layouts(&self) -> Vec<Layout> {
        self.runners.lock().iter().filter_map(|r| r.shared.cfg.lock().layout.clone()).collect()
    }

    /// Faux si aucun écran ne tourne ou si le fil d'un écran s'est arrêté (débranché, erreur d'envoi) : il faut re-détecter.
    pub fn all_running(&self) -> bool {
        let r = self.runners.lock();
        !r.is_empty() && r.iter().all(Runner::is_running)
    }

    /// Libère tous les périphériques (fermeture de l'application).
    pub fn shutdown(&self) {
        let old: Vec<Runner> = std::mem::take(&mut *self.runners.lock());
        for r in old {
            r.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Détection réelle sur la machine de test (aucun écran attendu) : ne doit ni
    /// bloquer ni paniquer, et l'écran simulé doit toujours être trouvé.
    #[test]
    fn scan_with_all_providers_finds_the_simulator() {
        let m = ScreenManager::new(SensorHub::new());
        let started = std::time::Instant::now();
        let infos = m.scan(true);
        assert!(started.elapsed() < Duration::from_secs(20), "détection trop lente");
        assert!(infos.iter().any(|i| i.protocol == super::super::driver::Protocol::Simulated));
        let st = m.state("sim:0000:0000:1").unwrap();
        assert!(st.layout.is_none() && st.brightness == 100);
        m.shutdown();
        assert!(m.list().is_empty());
    }
}
