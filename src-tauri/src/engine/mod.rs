pub mod curves;
pub mod effects;

use crate::core::registry::SharedRegistry;
use crate::core::Color;
use effects::{render, EffectConfig};
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Effets d'un appareil : un effet global optionnel + des effets par zone
/// (offset/longueur en LEDs) qui se superposent au global.
#[derive(Clone, PartialEq, Default)]
struct DeviceAssignment {
    total_leds: u32,
    whole: Option<EffectConfig>,
    /// zone index -> (config, offset LED, longueur)
    zones: HashMap<u32, (EffectConfig, u32, u32)>,
}

impl DeviceAssignment {
    fn is_static(&self) -> bool {
        self.whole.as_ref().map_or(true, |c| c.is_static())
            && self.zones.values().all(|(c, _, _)| c.is_static())
    }

    fn is_empty(&self) -> bool {
        self.whole.is_none() && self.zones.is_empty()
    }
}

/// Moteur d'effets : thread unique, tick adaptatif.
/// - Effets animés => boucle à `fps` images/s.
/// - Uniquement des effets statiques => une application puis sommeil complet
///   (réveil par condvar au prochain changement). 0% CPU au repos.
#[derive(Clone)]
pub struct EffectsEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    assignments: Mutex<HashMap<String, DeviceAssignment>>,
    dirty: Condvar,
    running: AtomicBool,
    fps: AtomicU32,
    /// Incrémentée par invalidate() : purge le cache des statiques appliqués.
    generation: AtomicU32,
}

impl EffectsEngine {
    pub fn start(registry: SharedRegistry) -> Self {
        let inner = Arc::new(EngineInner {
            assignments: Mutex::new(HashMap::new()),
            dirty: Condvar::new(),
            running: AtomicBool::new(true),
            fps: AtomicU32::new(30),
            generation: AtomicU32::new(0),
        });
        let thread_inner = inner.clone();
        std::thread::Builder::new()
            .name("effects-engine".into())
            .spawn(move || engine_loop(thread_inner, registry))
            .expect("spawn effects engine");
        EffectsEngine { inner }
    }

    /// Assigne un effet global à un appareil (remplace les effets de zone).
    pub fn set_effect(&self, device_id: String, cfg: EffectConfig, led_count: u32) {
        let mut map = self.inner.assignments.lock();
        let entry = map.entry(device_id).or_default();
        entry.total_leds = led_count;
        entry.whole = Some(cfg);
        entry.zones.clear();
        self.inner.dirty.notify_all();
    }

    /// Assigne un effet à une zone (se superpose à l'effet global éventuel).
    pub fn set_zone_effect(
        &self,
        device_id: String,
        zone: u32,
        cfg: EffectConfig,
        offset: u32,
        len: u32,
        total_leds: u32,
    ) {
        let mut map = self.inner.assignments.lock();
        let entry = map.entry(device_id).or_default();
        entry.total_leds = total_leds;
        entry.zones.insert(zone, (cfg, offset, len));
        self.inner.dirty.notify_all();
    }

    /// Retire tous les effets d'un appareil (ex: passage en mode matériel).
    pub fn remove_effect(&self, device_id: &str) {
        self.inner.assignments.lock().remove(device_id);
        self.inner.dirty.notify_all();
    }

    pub fn set_fps(&self, fps: u32) {
        self.inner.fps.store(fps.clamp(5, 144), Ordering::Relaxed);
    }

    /// Force la ré-application de tous les effets (y compris statiques) au
    /// prochain tick. À appeler après un re-scan : le matériel a pu être
    /// réinitialisé ou reconnecté.
    pub fn invalidate(&self) {
        self.inner.generation.fetch_add(1, Ordering::Relaxed);
        self.inner.dirty.notify_all();
    }

    pub fn shutdown(&self) {
        self.inner.running.store(false, Ordering::Relaxed);
        self.inner.dirty.notify_all();
    }
}

/// Rend le buffer complet d'un appareil : effet global (ou noir) puis zones.
fn render_device(a: &DeviceAssignment, t: f32) -> Vec<Color> {
    let total = a.total_leds as usize;
    let mut buf = match &a.whole {
        Some(cfg) => render(cfg, t, total),
        None => vec![Color::BLACK; total],
    };
    for (cfg, offset, len) in a.zones.values() {
        let zone_colors = render(cfg, t, *len as usize);
        let start = (*offset as usize).min(total);
        let end = (start + *len as usize).min(total);
        buf[start..end].copy_from_slice(&zone_colors[..end - start]);
    }
    buf
}

fn engine_loop(inner: Arc<EngineInner>, registry: SharedRegistry) {
    let start = Instant::now();
    // Assignations statiques déjà appliquées (éviter le re-envoi à chaque réveil).
    let mut applied_static: HashMap<String, DeviceAssignment> = HashMap::new();
    let mut seen_generation = 0u32;
    // Cadence à échéance fixe : évite la dérive d'un sleep(render_time +
    // frame_time) répété, qui ralentit visiblement l'animation quand le
    // rendu/l'écriture réseau prend du temps (plusieurs appareils OpenRGB).
    let mut next_tick = Instant::now();

    while inner.running.load(Ordering::Relaxed) {
        let generation = inner.generation.load(Ordering::Relaxed);
        if generation != seen_generation {
            seen_generation = generation;
            applied_static.clear();
        }
        let snapshot: Vec<(String, DeviceAssignment)> = {
            let mut map = inner.assignments.lock();
            map.retain(|_, a| !a.is_empty());
            map.iter().map(|(k, a)| (k.clone(), a.clone())).collect()
        };

        let t = start.elapsed().as_secs_f32();
        let mut any_animated = false;

        for (id, assignment) in &snapshot {
            if assignment.is_static() {
                if applied_static.get(id) == Some(assignment) {
                    continue;
                }
            } else {
                any_animated = true;
            }
            let colors = render_device(assignment, t);
            let result = registry.lock().set_colors(id, &colors);
            match result {
                Ok(()) => {
                    if assignment.is_static() {
                        applied_static.insert(id.clone(), assignment.clone());
                    }
                }
                Err(e) => log::debug!("set_colors {id}: {e:#}"),
            }
        }

        // Purge des statiques retirés.
        applied_static.retain(|id, _| snapshot.iter().any(|(sid, _)| sid == id));

        if any_animated {
            let fps = inner.fps.load(Ordering::Relaxed).max(1);
            let frame = Duration::from_micros(1_000_000 / fps as u64);
            next_tick += frame;
            let now = Instant::now();
            if next_tick > now {
                std::thread::sleep(next_tick - now);
            } else {
                // Rendu plus lent que la cadence visée (matériel lent / trop
                // d'appareils) : ne pas accumuler de retard, repartir de now.
                next_tick = now;
            }
        } else {
            // Rien d'animé : sommeil jusqu'au prochain set_effect / shutdown.
            let mut guard = inner.assignments.lock();
            inner.dirty.wait(&mut guard);
            next_tick = Instant::now();
        }
    }
}
