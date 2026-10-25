//! Couches d'E/S matérielles des pilotes Thermalright, derrière deux petits traits
//! pour pouvoir tester handshake + envoi de frame avec un faux périphérique.
//!
//! - [`Transport`] : écritures / lectures d'octets (bulk USB via `nusb`, HID via `hidapi`).
//! - [`ScsiTransport`] : une commande SCSI (CDB 16 octets) avec ou sans données.

use anyhow::{anyhow, bail, Context, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, Wake, Waker};
use std::time::{Duration, Instant};

/// Flux d'octets vers un point d'accès sortant et depuis un point d'accès entrant.
pub trait Transport: Send {
    /// Écrit `data` d'un bloc. Renvoie le nombre d'octets acceptés (`data.len()` si tout est parti).
    /// Une écriture vide est légale (paquet de longueur nulle).
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    /// Lit au plus `len` octets. Peut renvoyer moins ; erreur sur délai dépassé.
    fn read(&mut self, len: usize) -> Result<Vec<u8>>;
}

/// Transport SCSI : un CDB de 16 octets, données en lecture ou en écriture.
pub trait ScsiTransport: Send {
    fn read_cdb(&mut self, cdb: &[u8; 16], len: usize) -> Result<Vec<u8>>;
    fn write_cdb(&mut self, cdb: &[u8; 16], data: &[u8]) -> Result<()>;
}

// ---------------------------------------------------------------------------
// block_on avec délai (nusb 0.1 est asynchrone, sans délai intégré)
// ---------------------------------------------------------------------------

struct ThreadWaker(std::thread::Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Attend `fut` au plus `timeout`. `None` = délai dépassé ; le futur est alors abandonné
/// (nusb annule le transfert au `drop`).
pub(crate) fn block_on_timeout<F: Future>(fut: F, timeout: Duration) -> Option<F::Output> {
    let mut fut = Box::pin(fut);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = TaskContext::from_waker(&waker);
    let deadline = Instant::now() + timeout;
    loop {
        if let Poll::Ready(v) = Pin::as_mut(&mut fut).poll(&mut cx) {
            return Some(v);
        }
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        std::thread::park_timeout(deadline - now);
    }
}

// ---------------------------------------------------------------------------
// Bulk USB (nusb / WinUSB)
// ---------------------------------------------------------------------------

/// Transport bulk sur une interface USB déjà réclamée.
pub struct NusbTransport {
    iface: nusb::Interface,
    ep_out: u8,
    ep_in: u8,
    timeout: Duration,
}

/// Message FR commun « pas de pilote WinUSB » (conseil à l'utilisateur).
pub fn winusb_hint(vid: u16, pid: u16, driver: Option<&str>) -> String {
    let actuel = driver.map_or("aucun pilote".to_string(), |d| format!("pilote actuel : {d}"));
    format!(
        "L'écran USB {vid:04X}:{pid:04X} n'utilise pas le pilote WinUSB ({actuel}). \
         Installez WinUSB pour ce périphérique avec Zadig (https://zadig.akeo.ie : choisir le périphérique \
         {vid:04X}:{pid:04X}, pilote « WinUSB », « Replace Driver »), puis rebranchez-le. \
         Fermez aussi TRCC s'il est lancé."
    )
}

/// Le pilote est bon mais l'accès est refusé : un autre programme tient l'écran. On nomme les suspects.
pub fn busy_hint() -> String {
    let holders = crate::conflicts::running_screen_software();
    let who = if holders.is_empty() {
        "aucun logiciel d'écran connu n'est repéré parmi les processus (vérifiez le Gestionnaire des tâches, onglet Détails)".to_string()
    } else {
        format!("processus suspects : {}", holders.join(", "))
    };
    format!("L'écran est déjà utilisé par un autre programme ({who}). Fermez-le complètement, y compris dans la zone de notification, puis « Rechercher ».")
}

impl NusbTransport {
    /// Ouvre le périphérique, trouve la première interface portant un point d'accès bulk OUT et IN
    /// (en préférant `pref_out` / `pref_in` quand ils existent), et la réclame.
    pub fn open(info: &nusb::DeviceInfo, pref_out: u8, pref_in: u8, timeout: Duration) -> Result<Self> {
        let (vid, pid) = (info.vendor_id(), info.product_id());
        #[cfg(windows)]
        {
            // Détection SANS ouvrir : le pilote lié est connu de l'énumération.
            let drv = info.driver();
            if !drv.is_some_and(|d| d.eq_ignore_ascii_case("winusb")) {
                bail!(winusb_hint(vid, pid, drv));
            }
        }
        let device = info.open().map_err(|e| {
            anyhow!("Ouverture de l'écran USB {vid:04X}:{pid:04X} impossible ({e}). Fermez TRCC (ou tout logiciel constructeur) et rebranchez l'écran. {}", super::usb_service::describe(vid, pid))
        })?;
        let config = device.active_configuration().map_err(|e| anyhow!("Configuration USB illisible : {e}"))?;
        let mut found: Option<(u8, u8, u8)> = None;
        for group in config.interfaces() {
            let (mut outs, mut ins) = (Vec::new(), Vec::new());
            if let Some(alt) = group.alt_settings().next() {
                for ep in alt.endpoints() {
                    if ep.transfer_type() != nusb::transfer::EndpointType::Bulk {
                        continue;
                    }
                    match ep.direction() {
                        nusb::transfer::Direction::Out => outs.push(ep.address()),
                        nusb::transfer::Direction::In => ins.push(ep.address()),
                    }
                }
            }
            let pick = |list: &[u8], pref: u8| list.contains(&pref).then_some(pref).or_else(|| list.first().copied());
            if let (Some(o), Some(i)) = (pick(&outs, pref_out), pick(&ins, pref_in)) {
                found = Some((group.interface_number(), o, i));
                break;
            }
        }
        let (num, ep_out, ep_in) = found.ok_or_else(|| {
            anyhow!("Aucun point d'accès bulk entrée/sortie sur {vid:04X}:{pid:04X} : l'écran n'est pas dans le mode attendu.")
        })?;
        let iface = device.claim_interface(num).map_err(|e| {
            #[cfg(windows)]
            let extra = if info.driver().is_some_and(|d| d.eq_ignore_ascii_case("winusb")) {
                busy_hint()
            } else {
                winusb_hint(vid, pid, info.driver())
            };
            #[cfg(not(windows))]
            let extra = String::new();
            anyhow!("Impossible de réclamer l'interface {num} de {vid:04X}:{pid:04X} ({e}). {extra} {}", super::usb_service::describe(vid, pid))
        })?;
        Ok(NusbTransport { iface, ep_out, ep_in, timeout })
    }
}

impl Transport for NusbTransport {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let fut = self.iface.bulk_out(self.ep_out, data.to_vec());
        let done = block_on_timeout(fut, self.timeout).ok_or_else(|| anyhow!("Délai dépassé à l'écriture USB ({} octets).", data.len()))?;
        done.status.map_err(|e| anyhow!("Écriture USB échouée : {e}"))?;
        Ok(data.len())
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let fut = self.iface.bulk_in(self.ep_in, nusb::transfer::RequestBuffer::new(len));
        let done = block_on_timeout(fut, self.timeout).ok_or_else(|| anyhow!("Délai dépassé à la lecture USB."))?;
        done.status.map_err(|e| anyhow!("Lecture USB échouée : {e}"))?;
        Ok(done.data)
    }
}

/// Écrans USB présents (VID/PID du catalogue pour les `kinds` demandés), SANS ouvrir de handle.
/// Renvoie (entrée catalogue, fiche USB, étiquette stable : série ou identifiant d'instance).
pub fn enumerate_usb(
    kinds: &[super::catalog::Kind],
) -> Result<Vec<(&'static super::catalog::DeviceEntry, nusb::DeviceInfo, String)>> {
    let list = nusb::list_devices().map_err(|e| anyhow!("Énumération USB impossible : {e}"))?;
    let mut out = Vec::new();
    for info in list {
        let Some(entry) = super::catalog::find(info.vendor_id(), info.product_id()).filter(|e| kinds.contains(&e.kind)) else {
            continue;
        };
        let tag = match info.serial_number() {
            Some(s) if !s.is_empty() => s.to_string(),
            #[cfg(windows)]
            _ => info.instance_id().to_string_lossy().rsplit('\\').next().unwrap_or("0").to_string(),
            #[cfg(not(windows))]
            _ => "0".to_string(),
        };
        out.push((entry, info, tag));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// HID (hidapi)
// ---------------------------------------------------------------------------

/// Transport HID : chaque écriture part en un rapport, précédé de l'identifiant 0x00
/// (rapport par défaut), comme l'exige hidapi.
pub struct HidTransport {
    dev: hidapi::HidDevice,
    timeout_ms: i32,
}

impl HidTransport {
    pub fn new(dev: hidapi::HidDevice, timeout_ms: i32) -> Self {
        HidTransport { dev, timeout_ms }
    }
}

impl Transport for HidTransport {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(0x00); // report ID
        buf.extend_from_slice(data);
        let n = self.dev.write(&buf).context("Écriture HID échouée")?;
        // Windows compte l'octet d'identifiant : n = len + 1 pour une écriture complète.
        Ok(n.saturating_sub(1).min(data.len()))
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let n = self.dev.read_timeout(&mut buf, self.timeout_ms).context("Lecture HID échouée")?;
        if n == 0 {
            bail!("Aucune réponse de l'écran (délai dépassé).");
        }
        buf.truncate(n);
        Ok(buf)
    }
}

// ---------------------------------------------------------------------------
// Faux périphérique pour les tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Journal partagé entre le test et le faux périphérique.
    #[derive(Default)]
    pub struct Log {
        pub writes: Vec<Vec<u8>>,
        pub reads: Vec<usize>,
        pub cdbs: Vec<[u8; 16]>,
    }

    /// Répond aux lectures avec les réponses préparées, enregistre tout ce qui est écrit.
    pub struct MockTransport {
        pub log: Arc<Mutex<Log>>,
        pub replies: VecDeque<Result<Vec<u8>>>,
        /// Si Some(n) : les écritures n'acceptent que n octets (écriture courte).
        pub short_write: Option<usize>,
    }

    impl MockTransport {
        pub fn new(replies: Vec<Vec<u8>>) -> (Self, Arc<Mutex<Log>>) {
            let log = Arc::new(Mutex::new(Log::default()));
            let t = MockTransport { log: log.clone(), replies: replies.into_iter().map(Ok).collect(), short_write: None };
            (t, log)
        }
    }

    impl Transport for MockTransport {
        fn write(&mut self, data: &[u8]) -> Result<usize> {
            if let Ok(mut l) = self.log.lock() {
                l.writes.push(data.to_vec());
            }
            Ok(self.short_write.map_or(data.len(), |n| n.min(data.len())))
        }

        fn read(&mut self, len: usize) -> Result<Vec<u8>> {
            if let Ok(mut l) = self.log.lock() {
                l.reads.push(len);
            }
            self.replies.pop_front().unwrap_or_else(|| Err(anyhow!("pas de réponse préparée")))
        }
    }

    /// Faux SCSI : réponses de poll préparées, journal des CDB et des données.
    pub struct MockScsi {
        pub log: Arc<Mutex<Log>>,
        pub poll: VecDeque<Result<Vec<u8>>>,
    }

    impl MockScsi {
        pub fn new(poll: Vec<Result<Vec<u8>>>) -> (Self, Arc<Mutex<Log>>) {
            let log = Arc::new(Mutex::new(Log::default()));
            (MockScsi { log: log.clone(), poll: poll.into() }, log)
        }
    }

    impl ScsiTransport for MockScsi {
        fn read_cdb(&mut self, cdb: &[u8; 16], len: usize) -> Result<Vec<u8>> {
            if let Ok(mut l) = self.log.lock() {
                l.cdbs.push(*cdb);
                l.reads.push(len);
            }
            self.poll.pop_front().unwrap_or_else(|| Err(anyhow!("pas de réponse préparée")))
        }

        fn write_cdb(&mut self, cdb: &[u8; 16], data: &[u8]) -> Result<()> {
            if let Ok(mut l) = self.log.lock() {
                l.cdbs.push(*cdb);
                l.writes.push(data.to_vec());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_on_ready_and_timeout() {
        assert_eq!(block_on_timeout(async { 7 }, Duration::from_millis(50)), Some(7));
        let pending = std::future::pending::<()>();
        assert!(block_on_timeout(pending, Duration::from_millis(30)).is_none());
    }

    #[test]
    fn winusb_hint_is_french_and_actionable() {
        let m = winusb_hint(0x87AD, 0x70DB, Some("usbccgp"));
        assert!(m.contains("87AD:70DB") && m.contains("Zadig") && m.contains("WinUSB") && m.contains("usbccgp"));
        assert!(winusb_hint(1, 2, None).contains("aucun pilote"));
    }
}
