pub mod effects;

use crate::core::registry::SharedRegistry;
use effects::{render, EffectConfig};
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Moteur d'effets : thread unique, tick adaptatif.
/// - Effets animés => boucle à `fps` images/s.
/// - Uniquement des effets statiques => une application puis sommeil complet
///   (réveil par condvar au prochain changement). 0% CPU au repos.
pub struct EffectsEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    /// device_id -> (config, led_count)
    assignments: Mutex<HashMap<String, (EffectConfig, u32)>>,
    dirty: Condvar,
    running: AtomicBool,
    fps: AtomicU32,
}

impl EffectsEngine {
    pub fn start(registry: SharedRegistry) -> Self {
        let inner = Arc::new(EngineInner {
            assignments: Mutex::new(HashMap::new()),
            dirty: Condvar::new(),
            running: AtomicBool::new(true),
            fps: AtomicU32::new(30),
        });
        let thread_inner = inner.clone();
        std::thread::Builder::new()
            .name("effects-engine".into())
            .spawn(move || engine_loop(thread_inner, registry))
            .expect("spawn effects engine");
        EffectsEngine { inner }
    }

    /// Assigne un effet à un appareil et réveille le moteur.
    pub fn set_effect(&self, device_id: String, cfg: EffectConfig, led_count: u32) {
        let mut map = self.inner.assignments.lock();
        map.insert(device_id, (cfg, led_count));
        self.inner.dirty.notify_all();
    }

    pub fn set_fps(&self, fps: u32) {
        self.inner.fps.store(fps.clamp(5, 60), Ordering::Relaxed);
    }

    pub fn shutdown(&self) {
        self.inner.running.store(false, Ordering::Relaxed);
        self.inner.dirty.notify_all();
    }
}

fn engine_loop(inner: Arc<EngineInner>, registry: SharedRegistry) {
    let start = Instant::now();
    // Ids déjà appliqués en statique (éviter le re-envoi à chaque réveil).
    let mut applied_static: HashMap<String, EffectConfig> = HashMap::new();

    while inner.running.load(Ordering::Relaxed) {
        let snapshot: Vec<(String, EffectConfig, u32)> = {
            let map = inner.assignments.lock();
            map.iter()
                .map(|(k, (c, n))| (k.clone(), c.clone(), *n))
                .collect()
        };

        let t = start.elapsed().as_secs_f32();
        let mut any_animated = false;

        for (id, cfg, led_count) in &snapshot {
            if cfg.is_static() {
                // N'appliquer qu'une fois tant que la config ne change pas.
                if applied_static.get(id) == Some(cfg) {
                    continue;
                }
            } else {
                any_animated = true;
            }
            let colors = render(cfg, t, *led_count as usize);
            let result = registry.lock().set_colors(id, &colors);
            match result {
                Ok(()) => {
                    if cfg.is_static() {
                        applied_static.insert(id.clone(), cfg.clone());
                    }
                }
                Err(e) => log::debug!("set_colors {id}: {e:#}"),
            }
        }

        // Purge des statiques retirés.
        applied_static.retain(|id, _| snapshot.iter().any(|(sid, _, _)| sid == id));

        if any_animated {
            let fps = inner.fps.load(Ordering::Relaxed).max(1);
            std::thread::sleep(Duration::from_millis(1000 / fps as u64));
        } else {
            // Rien d'animé : sommeil jusqu'au prochain set_effect / shutdown.
            let mut guard = inner.assignments.lock();
            inner.dirty.wait(&mut guard);
        }
    }
}
