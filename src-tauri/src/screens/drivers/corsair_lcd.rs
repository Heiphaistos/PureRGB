//! Pilote Corsair iCUE LINK LCD 480x480 rond (HID), via `hidapi`.
//!
//! Sources (documentation, aucun code copié) :
//! - README de `zorrobyte/icue-link-lcd-reactor` (licence MIT), section « How the screen works » :
//!   rapports de sortie de 1024 octets, en-tête de 8 octets, luminosité `03 0B <0-100> 01`,
//!   rotation `03 0C <0-3> 01` (feature reports). [V pour 1B1C:0C4E]
//! - `python/lcd.py` du même dépôt : l'octet 0 (`02`) est l'identifiant de rapport et fait partie
//!   des 1024 octets écrits ; les feature reports commencent par l'identifiant `03`. [V]
//! - Protocole d'origine : OpenLinkHub (GPL-3.0, `src/devices/lsh/lsh.go`) : cité, jamais copié.
//! - Fiche : `docs/screens-protocols/corsair.md`.
//!
//! Non vérifié sur matériel. 0C43 / 0C57 / 0C55 sont SUPPOSÉS partager le protocole : ils ne sont
//! détectés que si le drapeau `experimental` est levé. 0C42 (XC7 Elite) : résolution INCONNUE,
//! non géré. Le service iCUE garde le périphérique : il faut le fermer.

use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use crate::screens::drivers::encode;
use anyhow::{anyhow, bail, Result};
use std::ffi::CString;

pub const VID: u16 = 0x1B1C;
/// Seul PID décrit par deux projets indépendants.
pub const PID_VERIFIED: u16 = 0x0C4E;
/// PID supposés (même pilote côté OpenLinkHub, protocole non décrit) : derrière `experimental`.
pub const PIDS_ASSUMED: [(u16, &str); 3] = [(0x0C43, "iCUE LINK XD5 LCD"), (0x0C57, "Nautilus LCD"), (0x0C55, "Nautilus LCD")];

const SIDE: u32 = 480;
const REPORT_SIZE: usize = 1024;
const HEADER_SIZE: usize = 8;
const CHUNK_SIZE: usize = REPORT_SIZE - HEADER_SIZE; // 1016
const REPORT_ID_OUT: u8 = 0x02;
const REPORT_ID_FEATURE: u8 = 0x03;
/// L'indice de morceau tient sur un octet ; au-delà le comportement est INCONNU : on refuse.
const MAX_CHUNKS: usize = 256;
const JPEG_QUALITY: u8 = 85;

const MSG_CLOSE_ICUE: &str = "Fermez iCUE (et son service Corsair) puis rebranchez l'écran : iCUE garde l'accès au module LCD.";

/// Liaison HID minimale (mockable) : rapport de sortie complet (identifiant inclus) et feature report.
pub trait HidLink: Send {
    /// Écrit un rapport de sortie dont l'octet 0 est l'identifiant. Renvoie les octets acceptés.
    fn write_report(&mut self, report: &[u8]) -> Result<usize>;
    fn send_feature(&mut self, report: &[u8]) -> Result<()>;
}

/// Découpe un JPEG en rapports de 1024 octets (en-tête `02 05 01 <dernier> <indice> 00 <len LE16>`).
pub fn build_reports(jpeg: &[u8]) -> Result<Vec<Vec<u8>>> {
    if jpeg.is_empty() {
        bail!("Image JPEG vide.");
    }
    let n = jpeg.len().div_ceil(CHUNK_SIZE);
    if n > MAX_CHUNKS {
        bail!("Image JPEG trop volumineuse pour l'écran Corsair ({} octets, {n} morceaux, maximum {MAX_CHUNKS}).", jpeg.len());
    }
    Ok(jpeg
        .chunks(CHUNK_SIZE)
        .enumerate()
        .map(|(i, chunk)| {
            let mut r = vec![0u8; REPORT_SIZE];
            r[..3].copy_from_slice(&[REPORT_ID_OUT, 0x05, 0x01]);
            r[3] = u8::from(i == n - 1);
            r[4] = i as u8; // i < 256 garanti plus haut
            r[6..8].copy_from_slice(&(chunk.len() as u16).to_le_bytes()); // <= 1016
            r[HEADER_SIZE..HEADER_SIZE + chunk.len()].copy_from_slice(chunk);
            r
        })
        .collect())
}

/// Feature report de luminosité : `03 0B <0-100> 01`.
pub fn brightness_report(percent: u8) -> [u8; 4] {
    [REPORT_ID_FEATURE, 0x0B, percent.min(100), 0x01]
}

/// Feature report de rotation : `03 0C <0-3> 01` (quarts de tour ; sens INCONNU).
pub fn rotation_report(quarter_turns: u8) -> Result<[u8; 4]> {
    if quarter_turns > 3 {
        bail!("Rotation Corsair hors bornes : {quarter_turns} (0 à 3 quarts de tour).");
    }
    Ok([REPORT_ID_FEATURE, 0x0C, quarter_turns, 0x01])
}

/// Écran Corsair ouvert.
pub struct CorsairScreen<L: HidLink> {
    info: ScreenInfo,
    link: L,
}

impl<L: HidLink> CorsairScreen<L> {
    pub fn new(link: L, id: String, model: String) -> Self {
        let info = ScreenInfo {
            id,
            vendor: "Corsair".into(),
            model,
            protocol: Protocol::CorsairLcd,
            width: SIDE,
            height: SIDE,
            rotation: 0,
            resolution_confirmed: false, // 480x480 vient de la doc, jamais lu sur l'appareil
            verified: false,
        };
        CorsairScreen { info, link }
    }

    /// Rotation matérielle en quarts de tour (0 à 3).
    pub fn set_rotation(&mut self, quarter_turns: u8) -> Result<()> {
        self.link.send_feature(&rotation_report(quarter_turns)?)
    }
}

impl<L: HidLink> ScreenDriver for CorsairScreen<L> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        if !frame.is_valid() || frame.width != SIDE || frame.height != SIDE {
            bail!("Image {}x{} refusée : l'écran Corsair attend {SIDE}x{SIDE}.", frame.width, frame.height);
        }
        let jpeg = encode::rgba_to_jpeg(&frame.rgba, frame.width, frame.height, JPEG_QUALITY)?;
        for report in build_reports(&jpeg)? {
            let n = self.link.write_report(&report)?;
            // hidapi/Windows peut compter l'octet d'identifiant : n >= 1024 - 0 accepté, moins = écriture courte.
            if n < REPORT_SIZE {
                bail!("Écriture HID courte vers l'écran Corsair ({n}/{REPORT_SIZE} octets). {MSG_CLOSE_ICUE}");
            }
        }
        Ok(())
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        self.link.send_feature(&brightness_report(percent))?;
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// HID réel (hidapi), ouverture paresseuse : la détection n'ouvre rien.
// ---------------------------------------------------------------------------

struct LazyHid {
    path: CString,
    dev: Option<hidapi::HidDevice>,
}

impl LazyHid {
    fn device(&mut self) -> Result<&hidapi::HidDevice> {
        if self.dev.is_none() {
            let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
            let dev = api
                .open_path(&self.path)
                .map_err(|e| anyhow!("Ouverture de l'écran Corsair impossible ({e}). {MSG_CLOSE_ICUE}"))?;
            self.dev = Some(dev);
        }
        self.dev.as_ref().ok_or_else(|| anyhow!("Écran Corsair non ouvert."))
    }

    /// Oublie le handle après une erreur : la prochaine image tente une réouverture.
    fn reset(&mut self) {
        self.dev = None;
    }
}

impl HidLink for LazyHid {
    fn write_report(&mut self, report: &[u8]) -> Result<usize> {
        let r = self.device()?.write(report);
        r.map_err(|e| {
            self.reset();
            anyhow!("Écriture HID vers l'écran Corsair échouée ({e}). {MSG_CLOSE_ICUE}")
        })
    }

    fn send_feature(&mut self, report: &[u8]) -> Result<()> {
        let r = self.device()?.send_feature_report(report);
        r.map_err(|e| {
            self.reset();
            anyhow!("Feature report refusé par l'écran Corsair ({e}). {MSG_CLOSE_ICUE}")
        })
    }
}

/// Détection des modules LCD Corsair iCUE LINK. `experimental` ajoute les PID supposés.
#[derive(Default)]
pub struct CorsairProvider {
    pub experimental: bool,
}

/// Nom du modèle pour un PID, selon le drapeau expérimental.
fn model_for(pid: u16, experimental: bool) -> Option<String> {
    if pid == PID_VERIFIED {
        return Some("iCUE LINK AIO LCD (480x480)".into());
    }
    PIDS_ASSUMED
        .iter()
        .find(|(p, _)| *p == pid && experimental)
        .map(|(_, n)| format!("{n} (expérimental, protocole supposé)"))
}

impl ScreenProvider for CorsairProvider {
    fn name(&self) -> &'static str {
        "Corsair iCUE LINK LCD"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
        let mut seen: Vec<(u16, String)> = Vec::new();
        let mut out: Vec<Box<dyn ScreenDriver>> = Vec::new();
        for d in api.device_list().filter(|d| d.vendor_id() == VID) {
            let Some(model) = model_for(d.product_id(), self.experimental) else { continue };
            let serial = d.serial_number().unwrap_or("").to_string();
            // Plusieurs collections HID par appareil : INCONNU laquelle reçoit le rapport 0x02 ;
            // on retient la première énumérée par (PID, série).
            let key = (d.product_id(), serial.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let tag = if serial.is_empty() { d.path().to_string_lossy().into_owned() } else { serial };
            let id = format!("{:04x}:{:04x}:{tag}", VID, d.product_id());
            out.push(Box::new(CorsairScreen::new(LazyHid { path: d.path().to_owned(), dev: None }, id, model)));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Log {
        reports: Vec<Vec<u8>>,
        features: Vec<Vec<u8>>,
    }

    struct MockHid {
        log: Arc<Mutex<Log>>,
        short: Option<usize>,
    }

    fn mock() -> (MockHid, Arc<Mutex<Log>>) {
        let log = Arc::new(Mutex::new(Log::default()));
        (MockHid { log: log.clone(), short: None }, log)
    }

    impl HidLink for MockHid {
        fn write_report(&mut self, report: &[u8]) -> Result<usize> {
            self.log.lock().unwrap().reports.push(report.to_vec());
            Ok(self.short.unwrap_or(report.len()))
        }
        fn send_feature(&mut self, report: &[u8]) -> Result<()> {
            self.log.lock().unwrap().features.push(report.to_vec());
            Ok(())
        }
    }

    #[test]
    fn single_chunk_header_bytes() {
        // README icue-link-lcd-reactor : octets 0-2 = 02 05 01 ; octet 3 = 01 (dernier) ; octet 4 = indice 0 ;
        // octet 5 = 00 ; octets 6-7 = longueur LE ; données dès l'octet 8 ; rapport de 1024 octets.
        let r = build_reports(&[0xAA, 0xBB]).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].len(), 1024);
        assert_eq!(r[0][..10], [0x02, 0x05, 0x01, 0x01, 0x00, 0x00, 0x02, 0x00, 0xAA, 0xBB]);
        assert!(r[0][10..].iter().all(|&b| b == 0));
    }

    #[test]
    fn multi_chunk_split_2000_bytes() {
        // 2000 octets = 1016 + 984 ; 1016 = 0x03F8, 984 = 0x03D8 (little endian : F8 03 / D8 03).
        let jpeg: Vec<u8> = (0..2000u32).map(|i| (i % 251) as u8).collect();
        let r = build_reports(&jpeg).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0][..8], [0x02, 0x05, 0x01, 0x00, 0x00, 0x00, 0xF8, 0x03]);
        assert_eq!(r[1][..8], [0x02, 0x05, 0x01, 0x01, 0x01, 0x00, 0xD8, 0x03]);
        assert_eq!(r[0][8..1024], jpeg[..1016]);
        assert_eq!(r[1][8..992], jpeg[1016..]);
        assert!(r[1][992..].iter().all(|&b| b == 0));
    }

    #[test]
    fn exact_multiple_marks_only_last() {
        let r = build_reports(&vec![7u8; 1016 * 3]).unwrap();
        assert_eq!(r.iter().map(|x| x[3]).collect::<Vec<_>>(), [0, 0, 1]);
        assert_eq!(r.iter().map(|x| x[4]).collect::<Vec<_>>(), [0, 1, 2]);
    }

    #[test]
    fn rejects_empty_and_oversized() {
        assert!(build_reports(&[]).is_err());
        assert!(build_reports(&vec![0u8; 1016 * 256 + 1]).is_err());
        assert!(build_reports(&vec![0u8; 1016 * 256]).is_ok());
    }

    #[test]
    fn feature_reports_bytes() {
        // README : luminosité `03 0B <0-100> 01`, rotation `03 0C <0-3> 01`.
        assert_eq!(brightness_report(80), [0x03, 0x0B, 0x50, 0x01]);
        assert_eq!(brightness_report(250), [0x03, 0x0B, 0x64, 0x01]); // borné à 100
        assert_eq!(rotation_report(2).unwrap(), [0x03, 0x0C, 0x02, 0x01]);
        assert!(rotation_report(4).is_err());
    }

    #[test]
    fn push_frame_sends_only_full_reports_with_last_flag() {
        let (m, log) = mock();
        let mut s = CorsairScreen::new(m, "1b1c:0c4e:x".into(), "t".into());
        let mut f = Frame::new(480, 480);
        f.rgba.chunks_exact_mut(4).for_each(|p| p.copy_from_slice(&[200, 30, 90, 255]));
        s.push_frame(&f).unwrap();
        let l = log.lock().unwrap();
        assert!(!l.reports.is_empty());
        assert!(l.reports.iter().all(|r| r.len() == 1024 && r[..3] == [2, 5, 1]));
        let last = l.reports.len() - 1;
        for (i, r) in l.reports.iter().enumerate() {
            assert_eq!(r[3], u8::from(i == last));
            assert_eq!(r[4] as usize, i);
        }
        // Les données concaténées forment un JPEG (SOI FF D8 ... EOI FF D9).
        let data: Vec<u8> = l.reports.iter().flat_map(|r| r[8..8 + u16::from_le_bytes([r[6], r[7]]) as usize].to_vec()).collect();
        assert_eq!(data[..2], [0xFF, 0xD8]);
        assert_eq!(data[data.len() - 2..], [0xFF, 0xD9]);
    }

    #[test]
    fn push_frame_validates_size_and_short_write() {
        let (m, _) = mock();
        let mut s = CorsairScreen::new(m, "i".into(), "t".into());
        assert!(s.push_frame(&Frame::new(320, 240)).is_err());
        assert!(s.push_frame(&Frame { width: 480, height: 480, rgba: vec![0; 10] }).is_err());
        let (mut m2, _) = mock();
        m2.short = Some(100);
        let mut s2 = CorsairScreen::new(m2, "i".into(), "t".into());
        let e = s2.push_frame(&Frame::new(480, 480)).unwrap_err().to_string();
        assert!(e.contains("iCUE"), "{e}");
    }

    #[test]
    fn brightness_and_rotation_go_through_feature_reports() {
        let (m, log) = mock();
        let mut s = CorsairScreen::new(m, "i".into(), "t".into());
        assert!(s.set_brightness(50).unwrap());
        s.set_rotation(1).unwrap();
        assert!(s.set_rotation(9).is_err());
        assert_eq!(log.lock().unwrap().features, vec![vec![3, 0x0B, 50, 1], vec![3, 0x0C, 1, 1]]);
    }

    #[test]
    fn info_is_unverified_and_models_gated() {
        let (m, _) = mock();
        let s = CorsairScreen::new(m, "i".into(), "t".into());
        assert!(!s.info().verified && !s.info().resolution_confirmed);
        assert_eq!((s.info().width, s.info().height), (480, 480));
        assert!(model_for(0x0C4E, false).is_some());
        assert!(model_for(0x0C43, false).is_none() && model_for(0x0C57, false).is_none() && model_for(0x0C55, false).is_none());
        assert!(model_for(0x0C43, true).unwrap().contains("expérimental"));
        assert!(model_for(0x0C42, true).is_none()); // XC7 : INCONNU
    }
}
