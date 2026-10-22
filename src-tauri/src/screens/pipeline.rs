//! Étages aval du pipeline d'un écran : encodage (hors du thread d'envoi) puis envoi USB.
//! Le rendu (étage amont) est dans `runner.rs`. Entre deux étages : une `Mailbox` à une image.

use super::driver::{Frame, FramePreparer, ScreenDriver};
use super::drivers::wire::WireImage;
use super::pacing::{Mailbox, QualityCtl};
use super::runner::Shared;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Délai de scrutation des arrêts quand aucune image n'arrive.
const POLL: Duration = Duration::from_millis(50);

pub type SharedDriver = Arc<Mutex<Box<dyn ScreenDriver>>>;

/// Une image rendue, en attente d'encodage.
pub struct Job {
    pub frame: Frame,
    /// Contenu animé : la qualité JPEG s'adapte ; sinon qualité maximale du pilote.
    pub animated: bool,
}

/// Une image prête à partir sur le câble.
pub enum Payload {
    Frame(Frame),
    Wire(WireImage),
}

/// Encode les images rendues (si le pilote a un encodeur autonome), les passe telles quelles sinon.
pub fn encode_stage(
    shared: &Shared,
    id: &str,
    preparer: Option<Arc<dyn FramePreparer>>,
    input: &Mailbox<Job>,
    output: &Mailbox<Payload>,
) {
    let mut ctl = QualityCtl::default();
    loop {
        let Some(job) = input.take(POLL) else {
            if input.is_closed() || shared.stop.load(Ordering::Acquire) {
                return;
            }
            continue;
        };
        let t = Instant::now();
        let animated = job.animated;
        let (payload, quality) = match &preparer {
            Some(p) => {
                let q = if animated { ctl.quality() } else { 100 };
                match p.prepare(&job.frame, q) {
                    Ok(w) => (Payload::Wire(w), q),
                    Err(e) => {
                        shared.fail(id, &e);
                        continue;
                    }
                }
            }
            None => (Payload::Frame(job.frame), 0),
        };
        shared.perf.encode(t.elapsed(), quality);
        let replaced = output.put(payload);
        if replaced {
            shared.perf.dropped(1);
        }
        if preparer.is_some() && animated {
            let period_ms = shared.period_us.load(Ordering::Relaxed) as f32 / 1000.0;
            let send = shared.perf.send_ms();
            ctl.update(replaced || send > 0.9 * period_ms, !replaced && send < 0.5 * period_ms);
        }
    }
}

/// Envoie les images au pilote ; une erreur est exposée et relance le rendu à froid.
pub fn send_stage(shared: &Shared, id: &str, driver: &SharedDriver, input: &Mailbox<Payload>) {
    loop {
        let Some(payload) = input.take(POLL) else {
            if input.is_closed() || shared.stop.load(Ordering::Acquire) {
                return;
            }
            continue;
        };
        let t = Instant::now();
        let r = {
            let mut d = driver.lock();
            match &payload {
                Payload::Frame(f) => d.push_frame(f),
                Payload::Wire(w) => d.send_prepared(w),
            }
        };
        match r {
            Ok(()) => {
                shared.perf.sent(t.elapsed(), Instant::now());
                *shared.error.lock() = None;
            }
            Err(e) => shared.fail(id, &e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::driver::{Protocol, ScreenInfo};
    use crate::screens::state::ScreenConfig;

    /// Pilote lent : chaque envoi dure `delay` ; mémorise le numéro (1er octet) des images reçues.
    struct Slow {
        info: ScreenInfo,
        delay: Duration,
        got: Arc<Mutex<Vec<u8>>>,
        fail_on: Option<u8>,
    }

    impl ScreenDriver for Slow {
        fn info(&self) -> &ScreenInfo {
            &self.info
        }
        fn push_frame(&mut self, f: &Frame) -> anyhow::Result<()> {
            std::thread::sleep(self.delay);
            if self.fail_on == Some(f.rgba[0]) {
                anyhow::bail!("câble débranché");
            }
            self.got.lock().push(f.rgba[0]);
            Ok(())
        }
    }

    fn slow(delay_ms: u64, fail_on: Option<u8>) -> (SharedDriver, Arc<Mutex<Vec<u8>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let info = ScreenInfo {
            id: "slow".into(),
            vendor: "t".into(),
            model: "t".into(),
            protocol: Protocol::Simulated,
            width: 1,
            height: 1,
            rotation: 0,
            resolution_confirmed: true,
            verified: true,
        };
        let d: Box<dyn ScreenDriver> = Box::new(Slow { info, delay: Duration::from_millis(delay_ms), got: got.clone(), fail_on });
        (Arc::new(Mutex::new(d)), got)
    }

    fn job(n: u8) -> Job {
        Job { frame: Frame { width: 1, height: 1, rgba: vec![n, 0, 0, 255] }, animated: true }
    }

    /// Lance les deux étages, injecte `n` images toutes les 10 ms, puis ferme et attend la fin.
    fn run(shared: &Arc<Shared>, driver: &SharedDriver, n: u8) {
        let (a, b) = (Arc::new(Mailbox::new()), Arc::new(Mailbox::new()));
        let enc = {
            let (s, a, b) = (shared.clone(), a.clone(), b.clone());
            std::thread::spawn(move || encode_stage(&s, "t", None, &a, &b))
        };
        let snd = {
            let (s, d, b) = (shared.clone(), driver.clone(), b.clone());
            std::thread::spawn(move || send_stage(&s, "t", &d, &b))
        };
        for i in 1..=n {
            a.put(job(i));
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(250)); // laisse l'aval finir la dernière image
        a.close();
        b.close();
        enc.join().expect("encodeur");
        snd.join().expect("envoi");
    }

    /// Comme `run`, mais attend la livraison de chaque image avant d'injecter la suivante (au plus 2 s par image).
    fn run_paced(shared: &Arc<Shared>, driver: &SharedDriver, got: &Arc<Mutex<Vec<u8>>>, n: u8) {
        let (a, b) = (Arc::new(Mailbox::new()), Arc::new(Mailbox::new()));
        let enc = {
            let (s, a, b) = (shared.clone(), a.clone(), b.clone());
            std::thread::spawn(move || encode_stage(&s, "t", None, &a, &b))
        };
        let snd = {
            let (s, d, b) = (shared.clone(), driver.clone(), b.clone());
            std::thread::spawn(move || send_stage(&s, "t", &d, &b))
        };
        for i in 1..=n {
            a.put(job(i));
            let deadline = Instant::now() + Duration::from_secs(2);
            while got.lock().len() < i as usize && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        a.close();
        b.close();
        enc.join().expect("encodeur");
        snd.join().expect("envoi");
    }

    #[test]
    fn slow_screen_never_accumulates_and_keeps_order() {
        let shared = Arc::new(Shared::new(ScreenConfig::default(), true));
        let (driver, got) = slow(60, None);
        let t = Instant::now();
        run(&shared, &driver, 30); // 30 images en 300 ms, l'écran en absorbe ~1 toutes les 60 ms
        let got = got.lock().clone();
        assert!(got.len() < 15, "trop d'images livrées ({}) : l'écran lent aurait dû en faire jeter", got.len());
        assert!(got.len() >= 4, "trop peu d'images : {got:?}");
        assert!(got.windows(2).all(|w| w[0] < w[1]), "ordre non conservé : {got:?}");
        assert_eq!(got.last(), Some(&30), "la dernière image doit arriver (on jette les anciennes)");
        assert!(t.elapsed() < Duration::from_millis(900), "l'envoi a pris du retard cumulé : {:?}", t.elapsed());
        assert!(shared.perf.snapshot(Instant::now()).dropped >= 10, "les abandons doivent être comptés");
    }

    #[test]
    fn fast_screen_gets_every_frame() {
        let shared = Arc::new(Shared::new(ScreenConfig::default(), true));
        let (driver, got) = slow(1, None);
        // Chaque image n'est injectée qu'une fois la précédente livrée : le test ne dépend plus de la charge de la machine.
        run_paced(&shared, &driver, &got, 20);
        assert_eq!(*got.lock(), (1..=20).collect::<Vec<u8>>());
    }

    #[test]
    fn send_error_is_exposed_and_requests_a_restart() {
        let shared = Arc::new(Shared::new(ScreenConfig::default(), true));
        let (driver, got) = slow(1, Some(3));
        run(&shared, &driver, 5);
        assert!(shared.failed.load(Ordering::Acquire));
        assert!(shared.error.lock().as_deref().is_some_and(|e| e.contains("câble débranché")) || got.lock().contains(&5));
        assert!(!got.lock().contains(&3));
    }
}
