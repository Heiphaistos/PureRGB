//! Pilote DeepCool LM240 / LM360 (USB 3633:0026), bulk USB via `nusb` (pilote WinUSB requis).
//!
//! Sources (documentation, aucun code copié) :
//! - `docs/PROTOCOL.md` de `zazazaxky-m/deepcool-lcd` (MIT) et
//!   `docs/protocol/lm-series.md` de `siam-parvez/opencreative` (MIT/Apache-2.0) : paquet
//!   `AA | len u16 LE | payload | CRC-16/XMODEM u16 LE`, commandes, trame RGB565 LE 320x240.
//! - 5 paquets capturés du logiciel constructeur (dépôt `daedlock/deepcool-lm`, MIT, cité par les
//!   deux docs) : utilisés comme vecteurs de test.
//! - Fiche : `docs/screens-protocols/deepcool.md`.
//!
//! INCONNU : sens de `2c 01` dans l'en-tête de trame (= 300 = 153600/512 selon OpenCreative,
//! déduit, non prouvé), contenu des réponses sur l'endpoint IN 0x81 (jamais décodées : on ne lit
//! rien), luminosité sous 10, rotation, LM240 vs LM360. Mystique 240/360 (PID 9, 640x480) : non géré.
//! Non vérifié sur matériel. Le logiciel DeepCreative garde l'interface : il faut le fermer.

use super::transport::{NusbTransport, Transport};
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use crate::screens::drivers::encode;
use anyhow::{anyhow, bail, Result};
use std::time::Duration;

pub const VID: u16 = 0x3633;
pub const PID: u16 = 0x0026;
const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const FRAME_BYTES: usize = (WIDTH as usize) * (HEIGHT as usize) * 2; // 153 600
const EP_OUT: u8 = 0x01;
const EP_IN: u8 = 0x81;
const TIMEOUT: Duration = Duration::from_secs(5);
/// Le logiciel constructeur attend ~500 ms après la poignée de main (OpenCreative).
const INIT_DELAY: Duration = Duration::from_millis(500);
/// deepcool-lcd n'envoie que 10 à 100 ; en dessous : INCONNU, on borne.
const MIN_BRIGHTNESS: u8 = 10;

const MSG_CLOSE_DEEPCREATIVE: &str = "Fermez DeepCreative (le logiciel DeepCool) puis rebranchez l'écran.";

/// CRC-16/XMODEM (poly 0x1021, init 0, sans réflexion ni XOR final).
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Paquet `AA | len LE16 | payload | CRC LE16`.
pub fn packet(payload: &[u8]) -> Result<Vec<u8>> {
    let len = u16::try_from(payload.len()).map_err(|_| anyhow!("Charge utile DeepCool trop longue ({} octets).", payload.len()))?;
    let mut p = Vec::with_capacity(payload.len() + 5);
    p.push(0xAA);
    p.extend_from_slice(&len.to_le_bytes());
    p.extend_from_slice(payload);
    p.extend_from_slice(&crc16_xmodem(payload).to_le_bytes());
    Ok(p)
}

/// Poignée de main : payload `09`.
pub fn init_packet() -> Vec<u8> {
    packet(&[0x09]).unwrap_or_default() // 1 octet : ne peut pas échouer
}

/// Luminosité : payload `06 03` + niveau LE16.
pub fn brightness_packet(level: u8) -> Vec<u8> {
    packet(&[0x06, 0x03, level, 0x00]).unwrap_or_default()
}

/// Bascule écran allumé/éteint (« zen ») : payload `03 00 00 00`.
pub fn zen_toggle_packet() -> Vec<u8> {
    packet(&[0x03, 0x00, 0x00, 0x00]).unwrap_or_default()
}

/// En-tête de trame : payload `00 01` + taille LE32 + `2c 01` (constante, sens INCONNU).
pub fn frame_header_packet() -> Vec<u8> {
    let mut payload = vec![0x00, 0x01];
    payload.extend_from_slice(&(FRAME_BYTES as u32).to_le_bytes());
    payload.extend_from_slice(&[0x2C, 0x01]);
    packet(&payload).unwrap_or_default()
}

/// Écran LM ouvert. L'init part au premier envoi (la détection n'ouvre rien).
pub struct DeepcoolScreen<T: Transport> {
    info: ScreenInfo,
    transport: T,
    initialized: bool,
    init_delay: Duration,
}

fn write_all<T: Transport>(t: &mut T, data: &[u8], what: &str) -> Result<()> {
    let n = t.write(data)?;
    if n < data.len() {
        bail!("Écriture USB courte vers l'écran DeepCool ({what} : {n}/{} octets). {MSG_CLOSE_DEEPCREATIVE}", data.len());
    }
    Ok(())
}

impl<T: Transport> DeepcoolScreen<T> {
    pub fn new(transport: T, id: String, init_delay: Duration) -> Self {
        let info = ScreenInfo {
            id,
            vendor: "DeepCool".into(),
            model: "LM240 / LM360".into(),
            protocol: Protocol::DeepcoolLm,
            width: WIDTH,
            height: HEIGHT,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        DeepcoolScreen { info, transport, initialized: false, init_delay }
    }

    fn ensure_init(&mut self) -> Result<()> {
        if !self.initialized {
            write_all(&mut self.transport, &init_packet(), "init")?;
            std::thread::sleep(self.init_delay);
            self.initialized = true;
        }
        Ok(())
    }

    /// Bascule l'écran (« zen »). État courant INCONNU : c'est une bascule, pas un réglage.
    pub fn toggle_screen(&mut self) -> Result<()> {
        self.ensure_init()?;
        write_all(&mut self.transport, &zen_toggle_packet(), "zen")
    }
}

impl<T: Transport> ScreenDriver for DeepcoolScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        if !frame.is_valid() || frame.width != WIDTH || frame.height != HEIGHT {
            bail!("Image {}x{} refusée : l'écran DeepCool LM attend {WIDTH}x{HEIGHT}.", frame.width, frame.height);
        }
        let pixels = encode::rgba_to_rgb565(&frame.rgba, false);
        if pixels.len() != FRAME_BYTES {
            bail!("Trame RGB565 incohérente ({} octets, attendu {FRAME_BYTES}).", pixels.len());
        }
        self.ensure_init()?;
        write_all(&mut self.transport, &frame_header_packet(), "en-tête de trame")?;
        write_all(&mut self.transport, &pixels, "trame")
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        self.ensure_init()?;
        write_all(&mut self.transport, &brightness_packet(percent.clamp(MIN_BRIGHTNESS, 100)), "luminosité")?;
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// USB réel : ouverture paresseuse (la détection n'ouvre ni ne réclame rien).
// ---------------------------------------------------------------------------

struct LazyNusb {
    info: nusb::DeviceInfo,
    inner: Option<NusbTransport>,
}

impl LazyNusb {
    fn open(&mut self) -> Result<&mut NusbTransport> {
        if self.inner.is_none() {
            let t = NusbTransport::open(&self.info, EP_OUT, EP_IN, TIMEOUT)
                .map_err(|e| anyhow!("{e} {MSG_CLOSE_DEEPCREATIVE}"))?;
            self.inner = Some(t);
        }
        self.inner.as_mut().ok_or_else(|| anyhow!("Écran DeepCool non ouvert."))
    }
}

impl Transport for LazyNusb {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let r = self.open()?.write(data);
        if r.is_err() {
            self.inner = None; // réouverture à la prochaine image
        }
        r
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        self.open()?.read(len)
    }
}

/// Détection des DeepCool LM240/LM360.
pub struct DeepcoolLmProvider;

impl ScreenProvider for DeepcoolLmProvider {
    fn name(&self) -> &'static str {
        "DeepCool LM"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let list = nusb::list_devices().map_err(|e| anyhow!("Énumération USB impossible : {e}"))?;
        let mut out: Vec<Box<dyn ScreenDriver>> = Vec::new();
        for info in list.filter(|d| d.vendor_id() == VID && d.product_id() == PID) {
            let tag = match info.serial_number() {
                Some(s) if !s.is_empty() => s.to_string(),
                #[cfg(windows)]
                _ => info.instance_id().to_string_lossy().rsplit('\\').next().unwrap_or("0").to_string(),
                #[cfg(not(windows))]
                _ => format!("{}-{}", info.bus_number(), info.device_address()),
            };
            let id = format!("{VID:04x}:{PID:04x}:{tag}");
            out.push(Box::new(DeepcoolScreen::new(LazyNusb { info, inner: None }, id, INIT_DELAY)));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::MockTransport;

    // Les 5 paquets capturés (deepcool-lm via PROTOCOL.md), octets écrits à la main.
    const CAP_INIT: [u8; 6] = [0xAA, 0x01, 0x00, 0x09, 0x29, 0x91];
    const CAP_BRIGHT_97: [u8; 9] = [0xAA, 0x04, 0x00, 0x06, 0x03, 0x61, 0x00, 0xD2, 0x46];
    const CAP_ZEN: [u8; 9] = [0xAA, 0x04, 0x00, 0x03, 0x00, 0x00, 0x00, 0xDC, 0x9B];
    const CAP_FRAME_HDR: [u8; 13] = [0xAA, 0x08, 0x00, 0x00, 0x01, 0x00, 0x58, 0x02, 0x00, 0x2C, 0x01, 0xBC, 0x11];

    #[test]
    fn crc_known_vectors() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3); // vecteur standard CRC-16/XMODEM
        assert_eq!(crc16_xmodem(&[]), 0);
        assert_eq!(crc16_xmodem(&[0x09]), 0x9129);
    }

    #[test]
    fn captured_packets_match() {
        assert_eq!(init_packet(), CAP_INIT);
        assert_eq!(brightness_packet(97), CAP_BRIGHT_97);
        assert_eq!(zen_toggle_packet(), CAP_ZEN);
        assert_eq!(frame_header_packet(), CAP_FRAME_HDR);
    }

    #[test]
    fn brightness_29_crc_independent() {
        // 5e capture (niveau 29 = 0x1D) : paquet non reproduit dans les docs ; octets CRC E6 0B calculés
        // séparément (script Python indépendant), pas relevé d'une capture.
        assert_eq!(brightness_packet(29), [0xAA, 0x04, 0x00, 0x06, 0x03, 0x1D, 0x00, 0xE6, 0x0B]);
    }

    #[test]
    fn oversized_payload_rejected() {
        assert!(packet(&vec![0u8; 70_000]).is_err());
    }

    fn solid(r: u8, g: u8, b: u8) -> Frame {
        Frame { width: 320, height: 240, rgba: [r, g, b, 255].repeat(320 * 240) }
    }

    #[test]
    fn push_frame_sequence_and_rgb565_le() {
        let (t, log) = MockTransport::new(vec![]);
        let mut s = DeepcoolScreen::new(t, "i".into(), Duration::ZERO);
        // Rouge pur 0xF800 -> LE : 00 F8 (doc : octet bas GGGBBBBB, octet haut RRRRRGGG).
        s.push_frame(&solid(255, 0, 0)).unwrap();
        s.push_frame(&solid(0, 0, 255)).unwrap(); // bleu pur 0x001F -> 1F 00
        let l = log.lock().unwrap();
        assert_eq!(l.writes.len(), 5); // init, (en-tête, trame) x2 : init une seule fois
        assert_eq!(l.writes[0], CAP_INIT);
        assert_eq!(l.writes[1], CAP_FRAME_HDR);
        assert_eq!(l.writes[2].len(), 153_600);
        assert_eq!(l.writes[2][..4], [0x00, 0xF8, 0x00, 0xF8]);
        assert_eq!(l.writes[3], CAP_FRAME_HDR);
        assert_eq!(l.writes[4][..4], [0x1F, 0x00, 0x1F, 0x00]);
        assert!(l.reads.is_empty()); // réponses IN non décodées : aucune lecture
    }

    #[test]
    fn brightness_clamped_and_sent_after_init() {
        let (t, log) = MockTransport::new(vec![]);
        let mut s = DeepcoolScreen::new(t, "i".into(), Duration::ZERO);
        assert!(s.set_brightness(97).unwrap());
        s.set_brightness(0).unwrap(); // borné à 10
        s.set_brightness(200).unwrap(); // borné à 100
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0], CAP_INIT);
        assert_eq!(l.writes[1], CAP_BRIGHT_97);
        assert_eq!(l.writes[2], brightness_packet(10));
        assert_eq!(l.writes[3], brightness_packet(100));
    }

    #[test]
    fn toggle_screen_sends_zen() {
        let (t, log) = MockTransport::new(vec![]);
        let mut s = DeepcoolScreen::new(t, "i".into(), Duration::ZERO);
        s.toggle_screen().unwrap();
        assert_eq!(log.lock().unwrap().writes, vec![CAP_INIT.to_vec(), CAP_ZEN.to_vec()]);
    }

    #[test]
    fn validates_frame_and_short_write() {
        let (t, log) = MockTransport::new(vec![]);
        let mut s = DeepcoolScreen::new(t, "i".into(), Duration::ZERO);
        assert!(s.push_frame(&Frame::new(480, 480)).is_err());
        assert!(s.push_frame(&Frame { width: 320, height: 240, rgba: vec![0; 8] }).is_err());
        assert!(log.lock().unwrap().writes.is_empty()); // rien d'envoyé sur une image invalide

        let (mut t2, _) = MockTransport::new(vec![]);
        t2.short_write = Some(3);
        let mut s2 = DeepcoolScreen::new(t2, "i".into(), Duration::ZERO);
        let e = s2.push_frame(&solid(1, 2, 3)).unwrap_err().to_string();
        assert!(e.contains("DeepCreative"), "{e}");
    }

    #[test]
    fn info_unverified() {
        let (t, _) = MockTransport::new(vec![]);
        let s = DeepcoolScreen::new(t, "i".into(), Duration::ZERO);
        assert!(!s.info().verified && !s.info().resolution_confirmed);
        assert_eq!((s.info().width, s.info().height), (320, 240));
    }
}
