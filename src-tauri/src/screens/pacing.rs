//! Outils de cadence du pipeline d'affichage : échéances absolues, boîte aux lettres à une
//! image (on jette l'ancienne), mesures de performance et qualité JPEG adaptative.
//! Tout est pur (les instants sont passés en paramètre) pour être testé sans horloge réelle.

use parking_lot::{Condvar, Mutex};
use serde::Serialize;
use std::time::{Duration, Instant};

/// Planificateur à échéances absolues : la grille `t0 + k × période` ne dérive jamais.
#[derive(Debug, Clone)]
pub struct Pacer {
    period: Duration,
    next: Instant,
}

impl Pacer {
    pub fn new(period: Duration, now: Instant) -> Pacer {
        Pacer { period: period.max(Duration::from_millis(1)), next: now }
    }

    pub fn due(&self) -> Instant {
        self.next
    }

    /// Repart de `now` (changement de cadence ou de design, reprise après erreur).
    pub fn reset(&mut self, period: Duration, now: Instant) {
        *self = Pacer::new(period, now);
    }

    /// Clôt l'échéance courante, constatée à `now`. En retard de moins d'une période : rattrapage
    /// immédiat (la grille est conservée). En retard d'une période ou plus : les échéances
    /// manquées sont sautées (pas de spirale), la grille reste alignée. Renvoie le nombre sautées.
    pub fn advance(&mut self, now: Instant) -> u32 {
        self.next += self.period;
        let lag = now.saturating_duration_since(self.next);
        if lag < self.period {
            return 0;
        }
        let skipped = (lag.as_nanos() / self.period.as_nanos()) as u32;
        self.next += self.period * skipped;
        skipped
    }
}

struct MailState<T> {
    item: Option<T>,
    closed: bool,
}

/// Case unique entre deux étages : `put` remplace l'image en attente (la plus ancienne est jetée).
pub struct Mailbox<T> {
    state: Mutex<MailState<T>>,
    cv: Condvar,
}

impl<T> Mailbox<T> {
    pub fn new() -> Mailbox<T> {
        Mailbox { state: Mutex::new(MailState { item: None, closed: false }), cv: Condvar::new() }
    }

    /// Dépose `v` ; vrai si une image non consommée a été jetée.
    pub fn put(&self, v: T) -> bool {
        let mut s = self.state.lock();
        let replaced = s.item.replace(v).is_some();
        self.cv.notify_one();
        replaced
    }

    /// Prend l'image en attente, en attendant au plus `timeout` ; `None` si rien ou si fermé et vide.
    pub fn take(&self, timeout: Duration) -> Option<T> {
        let mut s = self.state.lock();
        if s.item.is_none() && !s.closed {
            self.cv.wait_for(&mut s, timeout);
        }
        s.item.take()
    }

    pub fn close(&self) {
        self.state.lock().closed = true;
        self.cv.notify_all();
    }

    pub fn is_closed(&self) -> bool {
        self.state.lock().closed
    }
}

/// Mesures exposées à l'interface (Options > cadence réelle).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Perf {
    /// Images réellement envoyées par seconde (0 si rien n'est envoyé : image fixe ou arrêt).
    pub fps: f32,
    /// Cadence visée par le planificateur.
    pub target_fps: f32,
    pub render_ms: f32,
    pub encode_ms: f32,
    pub send_ms: f32,
    /// Images abandonnées (retard de rendu, d'encodage ou d'envoi) depuis le démarrage.
    pub dropped: u64,
    /// Qualité JPEG courante (0 si l'écran n'est pas en JPEG).
    pub quality: u8,
}

#[derive(Default)]
struct PerfInner {
    p: Perf,
    seen: [bool; 3],
    win_start: Option<Instant>,
    win_count: u32,
    last_send: Option<Instant>,
}

/// Moyennes glissantes et cadence réelle, partagées entre les trois étages.
#[derive(Default)]
pub struct PerfStats(Mutex<PerfInner>);

fn ema(old: f32, new: f32, seen: &mut bool) -> f32 {
    let v = if *seen { old * 0.8 + new * 0.2 } else { new };
    *seen = true;
    v
}

impl PerfStats {
    pub fn render(&self, d: Duration) {
        let mut s = self.0.lock();
        let v = ema(s.p.render_ms, d.as_secs_f32() * 1e3, &mut s.seen[0]);
        s.p.render_ms = v;
    }

    pub fn encode(&self, d: Duration, quality: u8) {
        let mut s = self.0.lock();
        let v = ema(s.p.encode_ms, d.as_secs_f32() * 1e3, &mut s.seen[1]);
        s.p.encode_ms = v;
        s.p.quality = quality;
    }

    /// Une image a été envoyée à `now` en `d`.
    pub fn sent(&self, d: Duration, now: Instant) {
        let mut s = self.0.lock();
        let v = ema(s.p.send_ms, d.as_secs_f32() * 1e3, &mut s.seen[2]);
        s.p.send_ms = v;
        s.last_send = Some(now);
        let start = *s.win_start.get_or_insert(now);
        if start != now {
            s.win_count += 1; // l'image qui ouvre la fenêtre n'est pas comptée : N images = N-1 intervalles
        }
        let span = now.saturating_duration_since(start);
        if span >= Duration::from_secs(1) {
            s.p.fps = s.win_count as f32 / span.as_secs_f32();
            s.win_start = Some(now);
            s.win_count = 0;
        }
    }

    pub fn dropped(&self, n: u64) {
        self.0.lock().p.dropped += n;
    }

    pub fn set_target(&self, fps: f32) {
        self.0.lock().p.target_fps = fps;
    }

    pub fn send_ms(&self) -> f32 {
        self.0.lock().p.send_ms
    }

    pub fn snapshot(&self, now: Instant) -> Perf {
        let s = self.0.lock();
        let mut p = s.p;
        if s.last_send.is_none_or(|t| now.saturating_duration_since(t) > Duration::from_secs(2)) {
            p.fps = 0.0;
        }
        p
    }
}

pub const QUALITY_MAX: u8 = 95;
pub const QUALITY_MIN: u8 = 50;
const QUALITY_UP_AFTER: u32 = 20;

/// Qualité JPEG adaptative d'un contenu animé : baisse vite quand l'envoi n'arrive pas à tenir
/// la cadence, remonte lentement quand il a de la marge. Le plafond de taille TRCC reste appliqué
/// ensuite par `wire::encode_frame`.
#[derive(Debug, Clone)]
pub struct QualityCtl {
    q: u8,
    calm: u32,
}

impl Default for QualityCtl {
    fn default() -> Self {
        QualityCtl { q: QUALITY_MAX, calm: 0 }
    }
}

impl QualityCtl {
    pub fn quality(&self) -> u8 {
        self.q
    }

    /// `late` : l'envoi a jeté une image ou dépasse ~90 % de la période ; `roomy` : envoi sous 50 %.
    pub fn update(&mut self, late: bool, roomy: bool) -> u8 {
        if late {
            self.q = self.q.saturating_sub(10).max(QUALITY_MIN);
            self.calm = 0;
        } else if roomy && self.q < QUALITY_MAX {
            self.calm += 1;
            if self.calm >= QUALITY_UP_AFTER {
                self.q = (self.q + 5).min(QUALITY_MAX);
                self.calm = 0;
            }
        }
        self.q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Duration = Duration::from_millis(100);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn pacer_keeps_an_absolute_grid_despite_jitter() {
        let t0 = Instant::now();
        let mut p = Pacer::new(P, t0);
        // Chaque tick est constaté avec un peu de gigue : l'échéance suivante reste k × 100 ms.
        for (k, jitter) in [(1u64, 7u64), (2, 0), (3, 30), (4, 12)] {
            assert_eq!(p.advance(t0 + ms((k - 1) * 100 + jitter)), 0);
            assert_eq!(p.due(), t0 + ms(k * 100), "pas de dérive au tick {k}");
        }
    }

    #[test]
    fn pacer_catches_up_when_slightly_late_without_dropping() {
        let t0 = Instant::now();
        let mut p = Pacer::new(P, t0);
        // Tick fini à 160 ms : échéance suivante 100 ms déjà passée de 60 ms (< période) : rattrapage.
        assert_eq!(p.advance(t0 + ms(160)), 0);
        assert_eq!(p.due(), t0 + ms(100));
    }

    #[test]
    fn pacer_skips_missed_ticks_without_spiral() {
        let t0 = Instant::now();
        let mut p = Pacer::new(P, t0);
        // Rendu de 450 ms : l'échéance 100 est en retard de 350 ms -> 3 échéances sautées (100,200,300),
        // la suivante est 400 (grille conservée, pas de rafale pour rattraper).
        let skipped = p.advance(t0 + ms(450));
        assert_eq!(skipped, 3);
        assert_eq!(p.due(), t0 + ms(400));
        // Le tick suivant, fini à l'heure, repart normalement.
        assert_eq!(p.advance(t0 + ms(410)), 0);
        assert_eq!(p.due(), t0 + ms(500));
    }

    #[test]
    fn mailbox_drops_the_old_frame_and_keeps_order() {
        let m = Mailbox::new();
        assert!(!m.put(1));
        assert!(m.put(2), "l'image 1 non consommée est jetée");
        assert_eq!(m.take(ms(1)), Some(2));
        assert_eq!(m.take(ms(1)), None);
        assert!(!m.put(3));
        m.close();
        assert!(m.is_closed());
        assert_eq!(m.take(ms(1)), Some(3), "fermé mais pas vide : on livre encore");
        assert_eq!(m.take(ms(1)), None);
    }

    #[test]
    fn perf_measures_real_rate_and_idles_to_zero() {
        let s = PerfStats::default();
        let t0 = Instant::now();
        for i in 0..=32u64 {
            s.sent(ms(5), t0 + ms(i * 62)); // ~16 i/s
        }
        let p = s.snapshot(t0 + ms(32 * 62));
        assert!((p.fps - 16.0).abs() < 0.8, "fps mesuré {}", p.fps);
        assert!((p.send_ms - 5.0).abs() < 0.01);
        assert_eq!(s.snapshot(t0 + ms(32 * 62) + Duration::from_secs(3)).fps, 0.0, "plus d'envoi : 0 i/s");
        s.dropped(2);
        s.dropped(1);
        assert_eq!(s.snapshot(t0).dropped, 3);
    }

    #[test]
    fn quality_drops_fast_and_recovers_slowly_within_bounds() {
        let mut c = QualityCtl::default();
        assert_eq!(c.quality(), 95);
        assert_eq!(c.update(true, false), 85);
        for _ in 0..10 {
            c.update(true, false);
        }
        assert_eq!(c.quality(), QUALITY_MIN, "plancher");
        for _ in 0..19 {
            c.update(false, true);
        }
        assert_eq!(c.quality(), QUALITY_MIN, "pas de remontée avant 20 images calmes");
        assert_eq!(c.update(false, true), 55);
        for _ in 0..(20 * 20) {
            c.update(false, true);
        }
        assert_eq!(c.quality(), QUALITY_MAX, "plafond");
        c.update(false, false); // ni en retard ni de marge : on garde
        assert_eq!(c.quality(), QUALITY_MAX);
    }
}
