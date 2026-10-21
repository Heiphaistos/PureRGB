//! Écrans USB-C « série » (Turing Smart Screen, XuanFang, UsbPCMonitor, Kipye Qiye) :
//! provider, liaison série mockable et logique commune aux révisions A/B/C/D.
//!
//! Source : wiki turing-smart-screen-python (« Hardware revisions ») et sa description du
//! protocole par révision (GPLv3 : documentation uniquement, aucun code copié). Tous les
//! écrans sont `verified: false` (aucun matériel disponible pour les tests).

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};

use super::turing_catalog::{classify, Detection, Revision};
use super::{turing_rev_a, turing_rev_b, turing_rev_c, turing_rev_d};
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};

/// Débit annoncé par la doc (115200 bauds, contrôle de flux matériel RTS/CTS).
const BAUD: u32 = 115_200;
const OPEN_TIMEOUT: Duration = Duration::from_secs(1);
const RECONNECT_TRIES: u32 = 3;
const WAKE_TRIES: u32 = 15;

/// Liaison série minimale ; un faux port l'implémente dans les tests.
pub trait SerialLink: Send {
    fn write_all(&mut self, data: &[u8]) -> Result<()>;
    /// Lit AU PLUS `len` octets, en attendant au maximum `timeout`. Une réponse courte
    /// (voire vide) n'est pas une erreur : c'est à l'appelant de valider la longueur.
    fn read(&mut self, len: usize, timeout: Duration) -> Result<Vec<u8>>;
    fn clear_input(&mut self) -> Result<()>;
    /// Délai imposé par le protocole (le faux port ne dort pas).
    fn pause(&mut self, d: Duration);
}

/// Pilote d'une révision : n'écrit que des octets sur la liaison.
pub(crate) trait Wire: Send {
    /// Poignée de main + orientation. Rejouée après une reconnexion.
    fn init(&mut self, link: &mut dyn SerialLink) -> Result<()>;
    /// Taille de l'image attendue (selon l'orientation, valable après `init`).
    fn size(&self) -> (u32, u32);
    fn push(&mut self, link: &mut dyn SerialLink, frame: &Frame) -> Result<()>;
    /// `Ok(true)` si la luminosité est gérée par le matériel.
    fn brightness(&mut self, link: &mut dyn SerialLink, percent: u8) -> Result<bool>;
}

/// Orientation (valeurs de la doc : portrait 0, portrait inversé 1, paysage 2, paysage inversé 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait = 0,
    ReversePortrait = 1,
    Landscape = 2,
    ReverseLandscape = 3,
}

impl Orientation {
    pub fn is_landscape(self) -> bool {
        matches!(self, Orientation::Landscape | Orientation::ReverseLandscape)
    }
    pub fn is_reverse(self) -> bool {
        matches!(self, Orientation::ReversePortrait | Orientation::ReverseLandscape)
    }
    /// Dimensions de l'image pour une résolution portrait donnée.
    pub fn oriented(self, portrait: (u32, u32)) -> (u32, u32) {
        if self.is_landscape() {
            (portrait.1, portrait.0)
        } else {
            portrait
        }
    }
}

pub(crate) fn check_frame(frame: &Frame, size: (u32, u32)) -> Result<()> {
    if !frame.is_valid() {
        bail!("image invalide : {} octets pour {}x{}", frame.rgba.len(), frame.width, frame.height);
    }
    if (frame.width, frame.height) != size {
        bail!("image {}x{} au lieu de {}x{} attendu par l'écran", frame.width, frame.height, size.0, size.1);
    }
    Ok(())
}

/// RGBA -> RGB565 (R sur les 5 bits de poids fort), alpha ignoré.
pub(crate) fn rgb565(rgba: &[u8], big_endian: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 2);
    for p in rgba.chunks_exact(4) {
        let v = ((u16::from(p[0]) >> 3) << 11) | ((u16::from(p[1]) >> 2) << 5) | (u16::from(p[2]) >> 3);
        out.extend_from_slice(&if big_endian { v.to_be_bytes() } else { v.to_le_bytes() });
    }
    out
}

/// Rotation antihoraire (sens de `rotate` de PIL, utilisé par la doc) de 0/90/180/270 degrés.
pub(crate) fn rotate_rgba(rgba: &[u8], w: u32, h: u32, ccw: u16) -> (Vec<u8>, u32, u32) {
    let (w, h) = (w as usize, h as usize);
    let (nw, nh) = if ccw == 90 || ccw == 270 { (h, w) } else { (w, h) };
    if ccw % 360 == 0 {
        return (rgba.to_vec(), w as u32, h as u32);
    }
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = match ccw {
                90 => (y, w - 1 - x),
                180 => (w - 1 - x, h - 1 - y),
                _ => (h - 1 - y, x), // 270
            };
            let (s, d) = ((y * w + x) * 4, (ny * nw + nx) * 4);
            out[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    (out, nw as u32, nh as u32)
}

// ---------------------------------------------------------------- port réel

struct RealLink(Box<dyn serialport::SerialPort>);

impl SerialLink for RealLink {
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        self.0.write_all(data).context("écriture série")?;
        self.0.flush().context("vidage série")
    }

    fn read(&mut self, len: usize, timeout: Duration) -> Result<Vec<u8>> {
        use std::io::Read;
        let deadline = Instant::now() + timeout;
        let mut out = vec![0u8; len];
        let mut got = 0;
        while got < len {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            self.0.set_timeout(left).context("délai de lecture série")?;
            match self.0.read(&mut out[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(anyhow!(e).context("lecture série")),
            }
        }
        out.truncate(got);
        Ok(out)
    }

    fn clear_input(&mut self) -> Result<()> {
        self.0.clear(serialport::ClearBuffer::Input).context("purge série")
    }

    fn pause(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

type Opener = Arc<dyn Fn(&str) -> Result<Box<dyn SerialLink>> + Send + Sync>;

fn real_opener() -> Opener {
    Arc::new(|name: &str| {
        let port = serialport::new(name, BAUD)
            .timeout(OPEN_TIMEOUT)
            .flow_control(serialport::FlowControl::Hardware)
            .open()
            .with_context(|| format!("ouverture de {name}"))?;
        Ok(Box::new(RealLink(port)) as Box<dyn SerialLink>)
    })
}

// ---------------------------------------------------------------- écran

/// Un écran ouvert. Sur erreur d'envoi : réouverture du port (bornée) puis un seul rejeu.
pub struct TuringScreen {
    info: ScreenInfo,
    wire: Box<dyn Wire>,
    link: Box<dyn SerialLink>,
    port: String,
    opener: Opener,
    retry_delay: Duration,
}

impl TuringScreen {
    fn connect(
        mut info: ScreenInfo,
        mut wire: Box<dyn Wire>,
        port: String,
        opener: Opener,
        retry_delay: Duration,
    ) -> Result<Self> {
        let mut link = opener(&port)?;
        wire.init(link.as_mut())?;
        let (w, h) = wire.size();
        info.width = w;
        info.height = h;
        Ok(TuringScreen { info, wire, link, port, opener, retry_delay })
    }

    fn reconnect(&mut self) -> Result<()> {
        let mut last = anyhow!("reconnexion impossible");
        for _ in 0..RECONNECT_TRIES {
            std::thread::sleep(self.retry_delay);
            let attempt = (self.opener)(&self.port).and_then(|mut l| {
                self.wire.init(l.as_mut())?;
                Ok(l)
            });
            match attempt {
                Ok(l) => {
                    self.link = l;
                    return Ok(());
                }
                Err(e) => last = e,
            }
        }
        Err(last.context(format!("reconnexion à {} échouée", self.port)))
    }
}

impl ScreenDriver for TuringScreen {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        check_frame(frame, self.wire.size())?;
        if self.wire.push(self.link.as_mut(), frame).is_ok() {
            return Ok(());
        }
        self.reconnect()?;
        self.wire.push(self.link.as_mut(), frame)
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        if percent > 100 {
            bail!("luminosité {percent} hors 0-100");
        }
        self.wire.brightness(self.link.as_mut(), percent)
    }
}

// ---------------------------------------------------------------- provider

/// Port série vu par l'énumération (VID/PID/série USB).
#[derive(Debug, Clone)]
pub struct PortMeta {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub serial: Option<String>,
}

/// Surcharge pour un port ambigu (VID/PID partagé) ou dont la taille est inconnue.
#[derive(Debug, Clone, Copy)]
pub struct Override {
    pub revision: Revision,
    /// Résolution portrait.
    pub portrait: (u32, u32),
}

type Lister = Box<dyn Fn() -> Vec<PortMeta> + Send>;

pub struct TuringProvider {
    orientation: Orientation,
    overrides: Vec<(String, Override)>,
    lister: Lister,
    opener: Opener,
    retry_delay: Duration,
    /// Raisons des ports catalogués mais non ouverts (pour le diagnostic).
    pub notes: Vec<String>,
}

fn list_real_ports() -> Vec<PortMeta> {
    let Ok(ports) = serialport::available_ports() else { return Vec::new() };
    ports
        .into_iter()
        .filter_map(|p| match p.port_type {
            serialport::SerialPortType::UsbPort(u) => {
                Some(PortMeta { name: p.port_name, vid: u.vid, pid: u.pid, serial: u.serial_number })
            }
            _ => None,
        })
        .collect()
}

impl Default for TuringProvider {
    fn default() -> Self {
        TuringProvider {
            orientation: Orientation::Portrait,
            overrides: Vec::new(),
            lister: Box::new(list_real_ports),
            opener: real_opener(),
            retry_delay: Duration::from_secs(1),
            notes: Vec::new(),
        }
    }
}

fn is_sleeping(p: &PortMeta) -> bool {
    matches!(classify(p.vid, p.pid, p.serial.as_deref()), Some(Detection::Sleeping(_)))
}

impl TuringProvider {
    pub fn with_orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }

    /// Impose révision et taille pour un port (ex. `COM5`), seulement s'il est au catalogue.
    pub fn with_override(mut self, port: &str, ov: Override) -> Self {
        self.overrides.push((port.to_string(), ov));
        self
    }

    /// Ouvre brièvement les ports en veille (révision C) jusqu'à ce qu'ils se ré-énumèrent éveillés.
    fn wake_sleeping(&self, ports: Vec<PortMeta>) -> Vec<PortMeta> {
        let sleeping: Vec<PortMeta> = ports.iter().filter(|p| is_sleeping(p)).cloned().collect();
        if sleeping.is_empty() {
            return ports;
        }
        let mut current = ports;
        for _ in 0..WAKE_TRIES {
            for p in &sleeping {
                // L'ouverture seule réveille l'écran ; son échec éventuel est sans importance ici.
                drop((self.opener)(&p.name));
            }
            std::thread::sleep(self.retry_delay);
            current = (self.lister)();
            if !current.iter().any(is_sleeping) {
                break;
            }
        }
        current
    }

    fn build(&mut self, p: &PortMeta) -> Option<Box<dyn ScreenDriver>> {
        let det = classify(p.vid, p.pid, p.serial.as_deref())?;
        let ov = self.overrides.iter().find(|(n, _)| *n == p.name).map(|(_, o)| *o);
        let (rev, portrait, vendor, model) = match (&det, ov) {
            (_, Some(o)) => (o.revision, o.portrait, "Turing/compatible", "Écran série (révision imposée)".to_string()),
            (Detection::Known(e), None) => match e.portrait {
                Some(s) => (e.revision, s, e.vendor, e.model.to_string()),
                None => {
                    self.notes.push(format!("{} : taille inconnue, surcharge requise", p.name));
                    return None;
                }
            },
            (Detection::Ambiguous(revs), None) => {
                self.notes.push(format!("{} : VID/PID partagé entre {revs:?}, surcharge requise", p.name));
                return None;
            }
            (Detection::Sleeping(_), None) => {
                self.notes.push(format!("{} : écran resté en veille", p.name));
                return None;
            }
        };
        let wire: Box<dyn Wire> = match rev {
            Revision::A => Box::new(turing_rev_a::RevA::new(portrait, self.orientation)),
            Revision::B => Box::new(turing_rev_b::RevB::new(portrait, self.orientation)),
            Revision::C => Box::new(turing_rev_c::RevC::new(portrait, self.orientation)),
            Revision::D => Box::new(turing_rev_d::RevD::new(portrait, self.orientation)),
        };
        let info = ScreenInfo {
            id: format!("{:04x}:{:04x}:{}", p.vid, p.pid, p.serial.as_deref().unwrap_or(&p.name)),
            vendor: vendor.to_string(),
            model,
            protocol: Protocol::TuringSerial,
            width: portrait.0,
            height: portrait.1,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        match TuringScreen::connect(info, wire, p.name.clone(), self.opener.clone(), self.retry_delay) {
            Ok(s) => Some(Box::new(s)),
            Err(e) => {
                self.notes.push(format!("{} : {e:#}", p.name));
                None
            }
        }
    }
}

impl ScreenProvider for TuringProvider {
    fn name(&self) -> &'static str {
        "turing_serial"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        self.notes.clear();
        let ports = self.wake_sleeping((self.lister)());
        Ok(ports.iter().filter_map(|p| self.build(p)).collect())
    }
}

// ---------------------------------------------------------------- faux port (tests)

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct State {
        pub writes: Vec<Vec<u8>>,
        pub rx: VecDeque<u8>,
        /// Nombre d'écritures à faire échouer avant de fonctionner.
        pub fail_writes: usize,
    }

    #[derive(Clone, Default)]
    pub struct Mock(pub Arc<Mutex<State>>);

    impl Mock {
        pub fn with_rx(bytes: &[u8]) -> Self {
            let m = Mock::default();
            if let Ok(mut s) = m.0.lock() {
                s.rx.extend(bytes);
            }
            m
        }
        pub fn writes(&self) -> Vec<Vec<u8>> {
            self.0.lock().map(|s| s.writes.clone()).unwrap_or_default()
        }
    }

    impl SerialLink for Mock {
        fn write_all(&mut self, data: &[u8]) -> Result<()> {
            let mut s = self.0.lock().map_err(|_| anyhow!("mutex"))?;
            if s.fail_writes > 0 {
                s.fail_writes -= 1;
                bail!("port déconnecté");
            }
            s.writes.push(data.to_vec());
            Ok(())
        }
        fn read(&mut self, len: usize, _t: Duration) -> Result<Vec<u8>> {
            let mut s = self.0.lock().map_err(|_| anyhow!("mutex"))?;
            let n = len.min(s.rx.len());
            Ok(s.rx.drain(..n).collect())
        }
        fn clear_input(&mut self) -> Result<()> {
            Ok(())
        }
        fn pause(&mut self, _d: Duration) {}
    }
}

#[cfg(test)]
mod tests {
    use super::mock::Mock;
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn rgb565_known_colors() {
        // Rouge pur 0xF800, vert 0x07E0, bleu 0x001F (définition RGB565).
        let px = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
        assert_eq!(rgb565(&px, true), [0xF8, 0x00, 0x07, 0xE0, 0x00, 0x1F]);
        assert_eq!(rgb565(&px, false), [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00]);
    }

    #[test]
    fn rotate_matches_hand_computed() {
        // 3x2 : a b c / d e f (1 octet répété 4 fois par pixel).
        let img: Vec<u8> = b"abcdef".iter().flat_map(|&c| [c; 4]).collect();
        let tag = |v: &[u8]| v.chunks(4).map(|p| p[0]).collect::<Vec<_>>();
        // Antihoraire 90 : c f / b e / a d (2x3).
        let (r, w, h) = rotate_rgba(&img, 3, 2, 90);
        assert_eq!((w, h, tag(&r)), (2, 3, b"cfbead".to_vec()));
        let (r, ..) = rotate_rgba(&img, 3, 2, 180);
        assert_eq!(tag(&r), b"fedcba");
        // Antihoraire 270 (= horaire 90) : d a / e b / f c.
        let (r, w, h) = rotate_rgba(&img, 3, 2, 270);
        assert_eq!((w, h, tag(&r)), (2, 3, b"daebfc".to_vec()));
    }

    #[test]
    fn rejects_bad_frames() {
        let f = Frame { width: 2, height: 2, rgba: vec![0; 3] };
        assert!(check_frame(&f, (2, 2)).is_err());
        assert!(check_frame(&Frame::new(2, 2), (3, 2)).is_err());
        assert!(check_frame(&Frame::new(2, 2), (2, 2)).is_ok());
    }

    fn provider(ports: Vec<PortMeta>, mock: Mock, opened: Arc<Mutex<Vec<String>>>) -> TuringProvider {
        let mut p = TuringProvider::default();
        p.lister = Box::new(move || ports.clone());
        p.retry_delay = Duration::ZERO;
        p.opener = Arc::new(move |name: &str| {
            if let Ok(mut o) = opened.lock() {
                o.push(name.to_string());
            }
            Ok(Box::new(mock.clone()) as Box<dyn SerialLink>)
        });
        p
    }

    fn meta(name: &str, vid: u16, pid: u16, serial: Option<&str>) -> PortMeta {
        PortMeta { name: name.into(), vid, pid, serial: serial.map(String::from) }
    }

    #[test]
    fn detect_opens_only_catalogued_ports() {
        let opened = Arc::new(Mutex::new(Vec::new()));
        let ports = vec![
            meta("COM3", 0x0403, 0x6001, Some("FTDI")), // hors catalogue
            meta("COM4", 0x1a86, 0x5722, None),         // ambigu A/B
            meta("COM5", 0x454d, 0x4e41, None),         // Kipye (rév. D)
        ];
        let mut p = provider(ports, Mock::default(), opened.clone());
        let found = p.detect().unwrap_or_default();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].info().vendor, "Kipye");
        assert!(!found[0].info().verified);
        assert_eq!(found[0].info().id, "454d:4e41:COM5");
        assert_eq!(opened.lock().map(|o| o.clone()).unwrap_or_default(), vec!["COM5".to_string()]);
        assert!(p.notes.iter().any(|n| n.contains("COM4")));
    }

    #[test]
    fn override_resolves_ambiguous_port_and_orientation_swaps_size() {
        let opened = Arc::new(Mutex::new(Vec::new()));
        let mut p = provider(vec![meta("COM4", 0x1a86, 0x5722, None)], Mock::default(), opened)
            .with_orientation(Orientation::Landscape)
            .with_override("COM4", Override { revision: Revision::D, portrait: (320, 480) });
        let found = p.detect().unwrap_or_default();
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].info().width, found[0].info().height), (480, 320));
    }

    #[test]
    fn push_reconnects_once_after_write_failure() {
        let mock = Mock::default();
        let m2 = mock.clone();
        let opener: Opener = Arc::new(move |_| Ok(Box::new(m2.clone()) as Box<dyn SerialLink>));
        let info = ScreenInfo {
            id: "t".into(),
            vendor: "t".into(),
            model: "t".into(),
            protocol: Protocol::TuringSerial,
            width: 320,
            height: 480,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        let wire = Box::new(turing_rev_d::RevD::new((320, 480), Orientation::Portrait));
        let Ok(mut s) = TuringScreen::connect(info, wire, "COM9".into(), opener, Duration::ZERO) else {
            panic!("connect")
        };
        let before = mock.writes().len();
        if let Ok(mut st) = mock.0.lock() {
            st.fail_writes = 1;
        }
        assert!(s.push_frame(&Frame::new(320, 480)).is_ok());
        // reconnexion (orientation) + trame complète rejouée : > 4800 écritures de 64 octets
        assert!(mock.writes().len() > before + 4800);
    }
}
