//! Pilote Thermalright HID : type 2 (0416:5302) et type 3 / F5 (0416:5406, 0418:5303, 0418:5304),
//! via `hidapi`.
//!
//! Sources (documentation du protocole TRCC-Linux, aucun code copié) :
//! - `doc/PROTOCOL_USBLCDNEW.md` : « Protocol 2: Nuvoton HID LCD » (poignée de main DA DB DC DD,
//!   envoi aligné sur 512) et « Protocol 3: ALi Corp LCD » (poignée de main F5, trame en un bloc + ACK 16 octets).
//! - `doc/REFERENCE_TECHNICAL.md` : « HID Devices », « FBL Detection », « PM -> FBL Mapping ».
//! - Octets absents de la doc (entête de trame type 2, tolérance de poignée de main courte) :
//!   `src/trcc/adapters/device/hid_lcd.py` et `_f5.py` (valeurs relevées, pas de code).
//!
//! ATTENTION : l'outil constructeur pilote ces écrans en WinUSB (EP02 OUT / EP01 IN) ; TRCC-Linux
//! n'utilise `hidapi` que « pour les périphériques HID purs sous Windows » (`transport.py`).
//! Si Windows lie le pilote `HidUsb`, ce chemin s'applique ; si un pilote WinUSB a été installé
//! (Zadig), l'écran n'est plus visible en HID. Non vérifié sur matériel.

use super::catalog::{self, DeviceEntry, Kind, Profile};
use super::transport::{HidTransport, Transport};
use super::wire;
use crate::screens::driver::{Frame, FramePreparer, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Result};
use std::sync::Arc;
use std::time::Duration;

const MAGIC: [u8; 4] = [0xDA, 0xDB, 0xDC, 0xDD];
const TYPE2_INIT_SIZE: usize = 512;
const TYPE2_REPLY_SIZE: usize = 512;
const ALIGN: usize = 512;
const F5_PREFIX_CMD: [u8; 8] = [0xF5, 0x00, 0x01, 0x00, 0xBC, 0xFF, 0xB6, 0xC8];
const F5_PREFIX_FRAME: [u8; 8] = [0xF5, 0x01, 0x01, 0x00, 0xBC, 0xFF, 0xB6, 0xC8];
const F5_HEADER: usize = 16;
const F5_REPLY_SIZE: usize = 1024;
const F5_ACK_SIZE: usize = 16;
/// Identités F5 acceptées en `resp[0]` : '6' (0x36), 'e' (0x65), 'f' (0x66).
const F5_IDENTITIES: [u8; 3] = [0x36, 0x65, 0x66];
const F5_PAYLOAD_DEFAULT: usize = 204_800;
const F5_PAYLOAD_SMALL: usize = 153_600;
const JPEG_QUALITY: u8 = 80;

/// Poignée de main type 2 : DA DB DC DD, 8 zéros, commande 01 à l'octet 12, complétée à 512.
pub fn type2_init_packet() -> Vec<u8> {
    let mut p = vec![0u8; TYPE2_INIT_SIZE];
    p[..4].copy_from_slice(&MAGIC);
    p[12] = 0x01;
    p
}

/// Poignée de main F5 : en-tête de 16 octets (taille 0x0400 à l'octet 12) + 1024 zéros.
pub fn f5_init_packet() -> Vec<u8> {
    let mut p = vec![0u8; F5_HEADER + F5_REPLY_SIZE];
    p[..8].copy_from_slice(&F5_PREFIX_CMD);
    p[12..16].copy_from_slice(&(F5_REPLY_SIZE as u32).to_le_bytes());
    p
}

/// En-tête de trame F5 : préfixe, zéros, taille LE u32 à l'octet 12.
pub fn f5_frame_header(payload: usize) -> [u8; 16] {
    let mut h = [0u8; 16];
    h[..8].copy_from_slice(&F5_PREFIX_FRAME);
    h[12..16].copy_from_slice(&(payload as u32).to_le_bytes());
    h
}

/// Trame type 2 : en-tête de 20 octets + image, complétée de zéros à un multiple de 512.
/// JPEG : octet 6 = 0, dimensions réelles en 8..12. RGB565 : octet 6 = 1, dimensions 240x320
/// (valeur figée de l'outil constructeur, cf. hid_lcd.py).
pub fn type2_frame(img: &wire::WireImage) -> Result<Vec<u8>> {
    let mut h = Vec::with_capacity(20);
    h.extend_from_slice(&MAGIC);
    h.extend_from_slice(&[0x02, 0x00]); // commande « image »
    if img.jpeg {
        let (w, hh) = (u16::try_from(img.width)?, u16::try_from(img.height)?);
        h.extend_from_slice(&[0x00, 0x00]);
        h.extend_from_slice(&w.to_le_bytes());
        h.extend_from_slice(&hh.to_le_bytes());
    } else {
        h.extend_from_slice(&[0x01, 0x00]);
        h.extend_from_slice(&240u16.to_le_bytes());
        h.extend_from_slice(&320u16.to_le_bytes());
    }
    h.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);
    h.extend_from_slice(&u32::try_from(img.data.len())?.to_le_bytes());
    debug_assert_eq!(h.len(), 20);
    h.extend_from_slice(&img.data);
    h.resize(h.len().div_ceil(ALIGN) * ALIGN, 0);
    Ok(h)
}

/// Résultat du handshake : profil + octets d'identité.
struct Hello {
    profile: Profile,
    f5_payload: usize,
    model_suffix: String,
}

fn parse_type2(resp: &[u8]) -> Result<Hello> {
    // Magic obligatoire. Les firmwares « courts » répondent 8 octets (hid_lcd.py) : seul le magic est exigé.
    if resp.len() < 6 || resp[..4] != MAGIC {
        bail!("Réponse de l'écran HID invalide (poignée de main type 2).");
    }
    if resp.len() >= 20 && resp[12] != 0x01 {
        bail!("L'écran HID n'a pas confirmé la poignée de main (octet 12 = {:#04X}).", resp[12]);
    }
    let (sub, pm) = (resp[4], resp[5]);
    if pm == 0 {
        bail!("L'écran HID n'annonce aucun modèle (PM = 0).");
    }
    let fbl = catalog::pm_to_fbl(pm, sub);
    let profile = catalog::fbl_profile(fbl, pm, sub)
        .ok_or_else(|| anyhow!("Modèle d'écran inconnu (PM {pm}, SUB {sub}, FBL {fbl}) : résolution non documentée."))?;
    Ok(Hello { profile, f5_payload: 0, model_suffix: format!("PM {pm}") })
}

fn parse_f5(resp: &[u8]) -> Result<Hello> {
    if resp.len() < 14 || !F5_IDENTITIES.contains(&resp[0]) {
        bail!("Réponse de l'écran HID invalide (poignée de main F5, octet 0 = {:#04X}).", resp.first().copied().unwrap_or(0));
    }
    let fbl = resp[0] - 1; // 0x36 -> 53, 0x65 -> 100, 0x66 -> 101
    let profile = catalog::fbl_profile(fbl, fbl, 0).ok_or_else(|| anyhow!("FBL {fbl} inconnu."))?;
    let f5_payload = if resp[0] == 0x36 { F5_PAYLOAD_SMALL } else { F5_PAYLOAD_DEFAULT };
    Ok(Hello { profile, f5_payload, model_suffix: format!("FBL {fbl}") })
}

/// Écran HID ouvert (type 2 ou F5).
pub struct HidScreen<T: Transport> {
    info: ScreenInfo,
    profile: Profile,
    kind: Kind,
    f5_payload: usize,
    transport: T,
}

impl<T: Transport> HidScreen<T> {
    /// Poignée de main puis création de l'écran. `kind` : `Kind::HidType2` ou `Kind::HidF5`.
    pub fn connect(mut transport: T, id: String, entry: &DeviceEntry) -> Result<Self> {
        let hello = match entry.kind {
            Kind::HidType2 => {
                let n = transport.write(&type2_init_packet())?;
                if n < TYPE2_INIT_SIZE {
                    bail!("Écriture courte de la poignée de main HID ({n} octets).");
                }
                parse_type2(&transport.read(TYPE2_REPLY_SIZE)?)?
            }
            Kind::HidF5 => {
                let init = f5_init_packet();
                if transport.write(&init)? < init.len() {
                    bail!("Écriture courte de la poignée de main HID F5.");
                }
                parse_f5(&transport.read(F5_REPLY_SIZE)?)?
            }
            _ => bail!("Famille de protocole non HID."),
        };
        let info = ScreenInfo {
            id,
            vendor: "Thermalright".into(),
            model: format!("{} ({})", catalog::display_name(entry), hello.model_suffix),
            protocol: entry.kind.protocol(),
            width: hello.profile.width,
            height: hello.profile.height,
            rotation: 0,
            resolution_confirmed: true,
            verified: false,
        };
        Ok(HidScreen { info, profile: hello.profile, kind: entry.kind, f5_payload: hello.f5_payload, transport })
    }

    fn push_type2(&mut self, img: &wire::WireImage) -> Result<()> {
        for chunk in type2_frame(img)?.chunks(ALIGN) {
            // Windows compte l'octet d'identifiant en plus ; HidTransport le retire. Écriture courte = échec.
            if self.transport.write(chunk)? < chunk.len() {
                bail!("Écriture HID courte : l'écran n'a pas reçu toute l'image.");
            }
        }
        Ok(())
    }

    fn push_f5(&mut self, img: &wire::WireImage) -> Result<()> {
        if img.jpeg || img.data.len() != self.f5_payload {
            bail!("Charge utile F5 incohérente : {} octets (attendu {}).", img.data.len(), self.f5_payload);
        }
        let mut pkt = Vec::with_capacity(F5_HEADER + img.data.len());
        pkt.extend_from_slice(&f5_frame_header(img.data.len()));
        pkt.extend_from_slice(&img.data);
        if self.transport.write(&pkt)? < pkt.len() {
            bail!("Écriture HID F5 courte : l'écran n'a pas reçu toute l'image.");
        }
        if self.transport.read(F5_ACK_SIZE)?.is_empty() {
            bail!("L'écran n'a pas acquitté l'image (ACK vide).");
        }
        Ok(())
    }
}

impl<T: Transport> ScreenDriver for HidScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn preparer(&self) -> Option<Arc<dyn FramePreparer>> {
        wire::jpeg_preparer(&self.profile, self.info.rotation, JPEG_QUALITY)
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        let img = wire::encode_frame(&self.profile, frame, self.info.rotation, JPEG_QUALITY)?;
        self.send_prepared(&img)
    }

    fn send_prepared(&mut self, img: &wire::WireImage) -> Result<()> {
        match self.kind {
            Kind::HidType2 => self.push_type2(img),
            _ => self.push_f5(img),
        }
    }
}

/// Détection des écrans HID Thermalright.
pub struct HidProvider;

impl ScreenProvider for HidProvider {
    fn name(&self) -> &'static str {
        "Thermalright HID"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
        // Énumération seule (aucun handle) : un candidat par (VID, PID, série, interface).
        let mut seen: Vec<(u16, u16, String, i32)> = Vec::new();
        let mut candidates = Vec::new();
        for d in api.device_list() {
            let Some(entry) = catalog::find(d.vendor_id(), d.product_id()).filter(|e| matches!(e.kind, Kind::HidType2 | Kind::HidF5)) else {
                continue;
            };
            let serial = d.serial_number().unwrap_or("").to_string();
            let key = (d.vendor_id(), d.product_id(), serial.clone(), d.interface_number());
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            candidates.push((entry, serial, d.path().to_owned()));
        }
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for (entry, serial, path) in candidates {
            let tag = if serial.is_empty() { path.to_string_lossy().into_owned() } else { serial };
            let id = format!("{:04x}:{:04x}:{tag}", entry.vid, entry.pid);
            match open_with_retry(&api, &path, entry).and_then(|t| HidScreen::connect(t, id, entry)) {
                Ok(s) => found.push(Box::new(s)),
                Err(e) => {
                    log::warn!("Écran HID {:04X}:{:04X} ignoré : {e}", entry.vid, entry.pid);
                    first_err.get_or_insert(e);
                }
            }
        }
        match (found.is_empty(), first_err) {
            (true, Some(e)) => Err(e),
            _ => Ok(found),
        }
    }
}

/// Ouvre le périphérique (3 essais espacés de 0,5 s : certains firmwares se ré-énumèrent, cf. transport.py).
fn open_with_retry(api: &hidapi::HidApi, path: &std::ffi::CStr, entry: &DeviceEntry) -> Result<HidTransport> {
    let mut last = None;
    for attempt in 0..3 {
        match api.open_path(path) {
            Ok(dev) => return Ok(HidTransport::new(dev, 1000)),
            Err(e) => {
                last = Some(e);
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }
    Err(anyhow!(
        "Impossible d'ouvrir l'écran HID {:04X}:{:04X} ({}). Fermez TRCC (ou tout logiciel constructeur) puis rebranchez l'écran.",
        entry.vid,
        entry.pid,
        last.map_or_else(String::new, |e| e.to_string())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::MockTransport;

    fn e(vid: u16, pid: u16) -> &'static DeviceEntry {
        catalog::find(vid, pid).unwrap()
    }

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Frame {
        Frame { width: w, height: h, rgba: px.repeat((w * h) as usize) }
    }

    #[test]
    fn type2_init_bytes() {
        // PROTOCOL_USBLCDNEW.md « Handshake (DA/DB/DC/DD) » : 512 octets, magic, 8 zéros, 01 à l'octet 12.
        let p = type2_init_packet();
        assert_eq!(p.len(), 512);
        assert_eq!(p[..16], [0xDA, 0xDB, 0xDC, 0xDD, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0]);
        assert!(p[16..].iter().all(|&b| b == 0));
    }

    #[test]
    fn f5_init_and_frame_header_bytes() {
        // « Protocol 3 » : F5 00 01 00 BC FF B6 C8, 00 00 00 00, 00 04 00 00 puis 1024 zéros = 1040.
        let p = f5_init_packet();
        assert_eq!(p.len(), 1040);
        assert_eq!(p[..16], [0xF5, 0x00, 0x01, 0x00, 0xBC, 0xFF, 0xB6, 0xC8, 0, 0, 0, 0, 0x00, 0x04, 0x00, 0x00]);
        // 204800 = 0x32000 -> 00 20 03 00 en LE.
        assert_eq!(f5_frame_header(204_800), [0xF5, 0x01, 0x01, 0x00, 0xBC, 0xFF, 0xB6, 0xC8, 0, 0, 0, 0, 0x00, 0x20, 0x03, 0x00]);
        assert_eq!(f5_frame_header(153_600)[12..], [0x00, 0x58, 0x02, 0x00]);
    }

    #[test]
    fn type2_frame_layout_and_padding() {
        let rgb = wire::WireImage { data: vec![0xAB; 100], width: 240, height: 320, jpeg: false };
        let f = type2_frame(&rgb).unwrap();
        // 20 + 100 = 120 -> 512.
        assert_eq!(f.len(), 512);
        assert_eq!(f[..20], [0xDA, 0xDB, 0xDC, 0xDD, 0x02, 0x00, 0x01, 0x00, 0xF0, 0x00, 0x40, 0x01, 0x02, 0, 0, 0, 100, 0, 0, 0]);
        assert_eq!(f[20], 0xAB);
        assert_eq!(f[120], 0);
        let jpg = wire::WireImage { data: vec![0xCD; 600], width: 1280, height: 480, jpeg: true };
        let f = type2_frame(&jpg).unwrap();
        // 620 -> 1024 ; largeur 1280 = 0x0500, hauteur 480 = 0x01E0.
        assert_eq!(f.len(), 1024);
        assert_eq!(f[..20], [0xDA, 0xDB, 0xDC, 0xDD, 0x02, 0x00, 0x00, 0x00, 0x00, 0x05, 0xE0, 0x01, 0x02, 0, 0, 0, 0x58, 0x02, 0, 0]);
        // Taille exactement alignée : pas de bloc ajouté.
        let exact = wire::WireImage { data: vec![1; 492], width: 1, height: 1, jpeg: true };
        assert_eq!(type2_frame(&exact).unwrap().len(), 512);
    }

    fn type2_reply(sub: u8, pm: u8) -> Vec<u8> {
        let mut r = vec![0u8; 512];
        r[..4].copy_from_slice(&MAGIC);
        r[4] = sub;
        r[5] = pm;
        r[12] = 1;
        r
    }

    #[test]
    fn type2_handshake_and_jpeg_frame_with_fake_device() {
        // PM 68 -> FBL 192 -> 1280x480 JPEG.
        let (t, log) = MockTransport::new(vec![type2_reply(0, 68)]);
        let mut s = HidScreen::connect(t, "0416:5302:x".into(), e(0x0416, 0x5302)).unwrap();
        assert_eq!((s.info().width, s.info().height, s.info().resolution_confirmed, s.info().verified), (1280, 480, true, false));
        s.push_frame(&solid(1280, 480, [10, 20, 30, 255])).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0], type2_init_packet());
        assert_eq!(l.reads[0], 512);
        // Toutes les écritures d'image font exactement 512 octets, la première porte l'en-tête.
        assert!(l.writes[1..].iter().all(|w| w.len() == 512));
        assert_eq!(l.writes[1][..8], [0xDA, 0xDB, 0xDC, 0xDD, 0x02, 0x00, 0x00, 0x00]);
        assert_eq!(l.writes[1][8..12], [0x00, 0x05, 0xE0, 0x01]); // 1280x480
        let total_len = u32::from_le_bytes([l.writes[1][16], l.writes[1][17], l.writes[1][18], l.writes[1][19]]) as usize;
        assert_eq!((l.writes.len() - 1) * 512, (total_len + 20).div_ceil(512) * 512);
        assert_eq!(l.writes[1][20..22], [0xFF, 0xD8]); // début du JPEG
    }

    #[test]
    fn type2_rgb565_frame_320x240() {
        // PM 58 -> FBL 58 -> 320x240 RGB565, tourné en 240x320 (153 600 octets).
        let (t, log) = MockTransport::new(vec![type2_reply(0, 58)]);
        let mut s = HidScreen::connect(t, "x".into(), e(0x0416, 0x5302)).unwrap();
        s.push_frame(&solid(320, 240, [0, 0, 0, 255])).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[1][6..12], [0x01, 0x00, 0xF0, 0x00, 0x40, 0x01]);
        assert_eq!(l.writes.len() - 1, (20 + 153_600usize).div_ceil(512));
    }

    #[test]
    fn type2_short_reply_accepted_bad_replies_refused() {
        // Réponse courte de 8 octets (firmware 4.07) : magic suffit.
        let (t, _) = MockTransport::new(vec![vec![0xDA, 0xDB, 0xDC, 0xDD, 0x00, 0x3A, 0, 0]]);
        let s = HidScreen::connect(t, "x".into(), e(0x0416, 0x5302)).unwrap();
        assert_eq!((s.info().width, s.info().height), (320, 240)); // PM 58
        for bad in [vec![], vec![0u8; 512], type2_reply(0, 0), type2_reply(0, 99)] {
            let (t, _) = MockTransport::new(vec![bad]);
            assert!(HidScreen::connect(t, "x".into(), e(0x0416, 0x5302)).is_err());
        }
        let mut wrong12 = type2_reply(0, 58);
        wrong12[12] = 0;
        let (t, _) = MockTransport::new(vec![wrong12]);
        assert!(HidScreen::connect(t, "x".into(), e(0x0416, 0x5302)).is_err());
    }

    fn f5_reply(b0: u8) -> Vec<u8> {
        let mut r = vec![0u8; 1024];
        r[0] = b0;
        r
    }

    #[test]
    fn f5_handshake_and_single_write_frame() {
        let (t, log) = MockTransport::new(vec![f5_reply(0x65), vec![0u8; 16]]);
        let mut s = HidScreen::connect(t, "x".into(), e(0x0416, 0x5406)).unwrap();
        assert_eq!((s.info().width, s.info().height), (320, 320));
        s.push_frame(&solid(320, 320, [255, 255, 255, 255])).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0].len(), 1040);
        assert_eq!(l.writes.len(), 2); // init + une seule écriture de trame
        assert_eq!(l.writes[1].len(), 16 + 204_800);
        assert_eq!(l.writes[1][..16], f5_frame_header(204_800));
        assert_eq!(l.writes[1][16..20], [0xFF, 0xFF, 0xFF, 0xFF]); // blanc RGB565
        assert_eq!(l.reads, vec![1024, 16]); // réponse de poignée de main, puis ACK 16 octets
    }

    #[test]
    fn f5_small_panel_and_validation() {
        let (t, log) = MockTransport::new(vec![f5_reply(0x36), vec![1u8; 16]]);
        let mut s = HidScreen::connect(t, "x".into(), e(0x0418, 0x5303)).unwrap();
        assert_eq!((s.info().width, s.info().height), (320, 240)); // « 240x320 » tourné
        s.push_frame(&solid(320, 240, [0, 0, 0, 255])).unwrap();
        assert_eq!(log.lock().unwrap().writes[1].len(), 16 + 153_600);
        for bad in [f5_reply(0x41), vec![0x65; 10], vec![]] {
            let (t, _) = MockTransport::new(vec![bad]);
            assert!(HidScreen::connect(t, "x".into(), e(0x0416, 0x5406)).is_err());
        }
        // ACK vide = erreur.
        let (t, _) = MockTransport::new(vec![f5_reply(0x65), vec![]]);
        let mut s = HidScreen::connect(t, "x".into(), e(0x0416, 0x5406)).unwrap();
        assert!(s.push_frame(&Frame::new(320, 320)).is_err());
    }

    #[test]
    fn short_write_is_an_error() {
        let (mut t, _) = MockTransport::new(vec![f5_reply(0x65)]);
        t.short_write = Some(100);
        assert!(HidScreen::connect(t, "x".into(), e(0x0416, 0x5406)).is_err());
    }
}
