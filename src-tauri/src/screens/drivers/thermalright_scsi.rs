//! Pilote Thermalright SCSI : 87CD:70DB et 0402:3922 (USB Mass Storage, fabricant « USBLCD »).
//!
//! Source : `doc/REFERENCE_TECHNICAL.md` (TRCC-Linux, documentation du protocole), sections
//! « SCSI Commands », « CDB Byte-Level Structure », « Initialization Sequence », « Poll Response »,
//! « Frame Transfer », « Pixel Format », « What SCSI Does NOT Control ». Les temporisations
//! (attente de démarrage 3 s, pause après poll) viennent de `scsi_lcd.py` (faits, pas de code copié).
//!
//! Résumé : CDB 16 octets = commande LE u32, 8 zéros, taille LE u32 (pas de CRC). Poll `0xF5`
//! en lecture de 0xE100 octets ; init `0x1F5` en écriture de 0xE100 zéros ; puis image RGB565
//! découpée en blocs (`0x101F5 | index << 24`), 64 Kio (0xE100 si l'écran a <= 76 800 pixels).
//! Pas de luminosité ni de rotation matérielles : tout est logiciel.
//!
//! Accès Windows : `IOCTL_SCSI_PASS_THROUGH_DIRECT` sur `\\.\PhysicalDriveN` (voir `scsi_win.rs`).

use super::catalog::{self, DeviceEntry, Profile};
use super::transport::ScsiTransport;
use super::wire;
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{bail, Result};
use std::time::Duration;

/// Taille de la réponse de poll et du bloc d'init (0xE100).
pub const POLL_SIZE: usize = 0xE100;
const POLL_CMD: u32 = 0xF5;
const INIT_CMD: u32 = 0x1F5;
const FRAME_CMD_BASE: u32 = 0x101F5;
const CHUNK_LARGE: u32 = 0x10000;
const CHUNK_SMALL: u32 = 0xE100;
/// Écrans de 76 800 pixels (320x240) ou moins : blocs de 0xE100.
const SMALL_PIXELS: u32 = 76_800;
/// Signature « en cours de démarrage » aux octets 4-7 de la réponse de poll.
const BOOT_SIGNATURE: [u8; 4] = [0xA1, 0xA2, 0xA3, 0xA4];
const BOOT_MAX_RETRIES: usize = 5;

/// Pauses du protocole (injectables pour les tests).
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub boot_wait: Duration,
    pub post_poll: Duration,
    pub post_init: Duration,
}

impl Timing {
    /// Valeurs de `scsi_lcd.py` : 3 s d'attente de boot, 1 s après le poll, 0,1 s après l'init.
    pub fn real() -> Self {
        Timing { boot_wait: Duration::from_secs(3), post_poll: Duration::from_secs(1), post_init: Duration::from_millis(100) }
    }

    #[cfg_attr(not(test), allow(dead_code))] // prêt et testé, volontairement non branché (écriture matériel non validée)
    pub fn none() -> Self {
        Timing { boot_wait: Duration::ZERO, post_poll: Duration::ZERO, post_init: Duration::ZERO }
    }
}

/// CDB de 16 octets : commande LE u32, 8 zéros, taille LE u32.
pub fn build_cdb(cmd: u32, size: u32) -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0..4].copy_from_slice(&cmd.to_le_bytes());
    cdb[12..16].copy_from_slice(&size.to_le_bytes());
    cdb
}

/// Liste (commande, taille) des blocs d'une image RGB565 de `width`x`height`.
pub fn frame_chunks(width: u32, height: u32) -> Vec<(u32, u32)> {
    let pixels = width * height;
    let chunk = if pixels <= SMALL_PIXELS { CHUNK_SMALL } else { CHUNK_LARGE };
    let total = pixels * 2;
    let (mut off, mut idx, mut out) = (0u32, 0u32, Vec::new());
    while off < total {
        let size = chunk.min(total - off);
        out.push((FRAME_CMD_BASE | (idx << 24), size));
        off += size;
        idx += 1;
    }
    out
}

/// Écran SCSI ouvert.
pub struct ScsiScreen<T: ScsiTransport> {
    info: ScreenInfo,
    profile: Profile,
    transport: T,
}

impl<T: ScsiTransport> ScsiScreen<T> {
    /// Poll (avec attente de boot) puis init. La résolution vient de l'octet de poll ; si l'écran
    /// répond court ou pas du tout (cas documenté de TRCC), on retombe sur le catalogue
    /// (`resolution_confirmed = false`).
    pub fn connect(mut transport: T, id: String, entry: &DeviceEntry, timing: Timing) -> Result<Self> {
        let poll = build_cdb(POLL_CMD, POLL_SIZE as u32);
        let mut resp: Option<Vec<u8>> = None;
        for attempt in 0..BOOT_MAX_RETRIES {
            match transport.read_cdb(&poll, POLL_SIZE) {
                Ok(r) if r.len() >= 8 && r[4..8] == BOOT_SIGNATURE => {
                    if attempt + 1 == BOOT_MAX_RETRIES {
                        bail!("L'écran est toujours en cours de démarrage après {BOOT_MAX_RETRIES} essais ; réessayez dans quelques secondes.");
                    }
                    std::thread::sleep(timing.boot_wait);
                }
                Ok(r) => {
                    resp = Some(r);
                    break;
                }
                Err(e) => {
                    log::warn!("Poll SCSI sans réponse exploitable ({e}) : résolution du catalogue utilisée.");
                    break;
                }
            }
        }
        std::thread::sleep(timing.post_poll);

        let (profile, confirmed) = match resp.as_deref().and_then(|r| r.first().copied()) {
            Some(byte) => {
                let mut p = catalog::fbl_profile(byte, byte, 0).ok_or_else(|| {
                    anyhow::anyhow!("Octet de sondage SCSI inconnu : 0x{byte:02X} (résolution non documentée).")
                })?;
                // « big-endian ... on SCSI FBL 51 » (REFERENCE_TECHNICAL.md « Pixel Format »).
                p.big_endian |= byte == 51;
                (p, true)
            }
            None => {
                let (w, h) = entry.default_resolution;
                if w == 0 || h == 0 {
                    bail!("Résolution de l'écran SCSI inconnue et aucune valeur par défaut au catalogue."); // INCONNU
                }
                let p = catalog::fbl_profile(100, 100, 0).filter(|p| (p.width, p.height) == (w, h));
                (p.ok_or_else(|| anyhow::anyhow!("Résolution par défaut {w}x{h} sans profil au catalogue."))?, false)
            }
        };

        let init = build_cdb(INIT_CMD, POLL_SIZE as u32);
        transport.write_cdb(&init, &vec![0u8; POLL_SIZE])?;
        std::thread::sleep(timing.post_init);

        let info = ScreenInfo {
            id,
            vendor: "Thermalright".into(),
            model: catalog::display_name(entry),
            protocol: Protocol::ThermalrightScsi,
            width: profile.width,
            height: profile.height,
            rotation: 0,
            resolution_confirmed: confirmed,
            verified: false,
        };
        Ok(ScsiScreen { info, profile, transport })
    }
}

impl<T: ScsiTransport> ScreenDriver for ScsiScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        let img = wire::encode_frame(&self.profile, frame, self.info.rotation, 0)?;
        let total = (self.profile.width * self.profile.height * 2) as usize;
        if img.jpeg || img.data.len() != total {
            bail!("Charge utile SCSI incohérente : {} octets (attendu {total} en RGB565).", img.data.len());
        }
        let mut off = 0usize;
        for (cmd, size) in frame_chunks(self.profile.width, self.profile.height) {
            let end = off + size as usize;
            self.transport.write_cdb(&build_cdb(cmd, size), &img.data[off..end])?;
            off = end;
        }
        Ok(())
    }
}

/// Détection des écrans SCSI Thermalright.
pub struct ScsiProvider;

impl ScreenProvider for ScsiProvider {
    fn name(&self) -> &'static str {
        "Thermalright SCSI"
    }

    #[cfg(windows)]
    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        use super::scsi_win;
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for entry in catalog::DEVICES.iter().filter(|e| e.kind == catalog::Kind::Scsi) {
            for drive in scsi_win::list_drives(entry.vid, entry.pid)? {
                let id = format!("{:04x}:{:04x}:{}", entry.vid, entry.pid, drive.serial);
                let res = scsi_win::WinScsi::open(drive.number).and_then(|t| ScsiScreen::connect(t, id, entry, Timing::real()));
                match res {
                    Ok(s) => found.push(Box::new(s)),
                    Err(e) => {
                        log::warn!("Écran SCSI {:04X}:{:04X} ignoré : {e}", entry.vid, entry.pid);
                        first_err.get_or_insert(e);
                    }
                }
            }
        }
        match (found.is_empty(), first_err) {
            (true, Some(e)) => Err(e),
            _ => Ok(found),
        }
    }

    #[cfg(not(windows))]
    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        bail!("Les écrans Thermalright SCSI ne sont pilotés que sous Windows.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::MockScsi;

    fn entry() -> &'static DeviceEntry {
        catalog::find(0x0402, 0x3922).unwrap()
    }

    fn poll_reply(first: u8) -> Vec<u8> {
        let mut r = vec![0u8; POLL_SIZE];
        r[0] = first;
        r
    }

    #[test]
    fn cdb_bytes() {
        // Poll : F5 00 00 00, 8 zéros, taille 0xE100 LE = 00 E1 00 00.
        assert_eq!(build_cdb(0xF5, 0xE100), [0xF5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xE1, 0x00, 0x00]);
        // Init : F5 01 00 00.
        assert_eq!(build_cdb(0x1F5, 0xE100)[..4], [0xF5, 0x01, 0x00, 0x00]);
        // Bloc 3 de 0x2000 : F5 01 01 03 ... 00 20 00 00.
        let c = build_cdb(0x101F5 | (3 << 24), 0x2000);
        assert_eq!(c, [0xF5, 0x01, 0x01, 0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x20, 0x00, 0x00]);
    }

    #[test]
    fn chunk_tables_match_doc() {
        // « Frame Transfer » : 240x240 -> 2 x 0xE100 ; 320x240 -> 0xE100, 0xE100, 0x9600 ; 320x320 -> 3 x 0x10000 + 0x2000.
        assert_eq!(frame_chunks(240, 240), vec![(0x101F5, 0xE100), (0x1_01_01F5, 0xE100)]);
        assert_eq!(frame_chunks(320, 240), vec![(0x101F5, 0xE100), (0x1_01_01F5, 0xE100), (0x2_01_01F5, 0x9600)]);
        assert_eq!(
            frame_chunks(320, 320),
            vec![(0x101F5, 0x10000), (0x01_01_01F5, 0x10000), (0x02_01_01F5, 0x10000), (0x03_01_01F5, 0x2000)]
        );
    }

    #[test]
    fn handshake_and_full_frame_320x320() {
        let (t, log) = MockScsi::new(vec![Ok(poll_reply(b'd'))]);
        let mut s = ScsiScreen::connect(t, "0402:3922:test".into(), entry(), Timing::none()).unwrap();
        assert_eq!((s.info().width, s.info().height, s.info().resolution_confirmed, s.info().verified), (320, 320, true, false));
        s.push_frame(&Frame { width: 320, height: 320, rgba: [255u8, 0, 0, 255].repeat(320 * 320) }).unwrap();
        let l = log.lock().unwrap();
        // poll, init, 4 blocs.
        assert_eq!(l.cdbs.len(), 6);
        assert_eq!(l.cdbs[0], build_cdb(0xF5, 0xE100));
        assert_eq!(l.cdbs[1], build_cdb(0x1F5, 0xE100));
        assert_eq!(l.writes[0], vec![0u8; 0xE100]);
        let sizes: Vec<usize> = l.writes[1..].iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![0x10000, 0x10000, 0x10000, 0x2000]);
        assert_eq!(l.cdbs[2][..4], [0xF5, 0x01, 0x01, 0x00]);
        assert_eq!(l.cdbs[5][..4], [0xF5, 0x01, 0x01, 0x03]);
        // Rouge pur en RGB565 big-endian = F8 00.
        assert_eq!(l.writes[1][..4], [0xF8, 0x00, 0xF8, 0x00]);
    }

    #[test]
    fn boot_signature_is_retried_then_frame_320x240_rotated() {
        let mut booting = poll_reply(b'2');
        booting[4..8].copy_from_slice(&[0xA1, 0xA2, 0xA3, 0xA4]);
        let (t, log) = MockScsi::new(vec![Ok(booting), Ok(poll_reply(b'2'))]);
        let mut s = ScsiScreen::connect(t, "x".into(), entry(), Timing::none()).unwrap();
        assert_eq!((s.info().width, s.info().height), (320, 240));
        s.push_frame(&Frame::new(320, 240)).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.cdbs.len(), 2 + 1 + 3); // 2 polls, init, 3 blocs
        let sizes: Vec<usize> = l.writes[1..].iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![0xE100, 0xE100, 0x9600]);
    }

    #[test]
    fn poll_failure_falls_back_to_catalog_unconfirmed() {
        let (t, _) = MockScsi::new(vec![Err(anyhow::anyhow!("WinError 121"))]);
        let s = ScsiScreen::connect(t, "x".into(), entry(), Timing::none()).unwrap();
        assert_eq!((s.info().width, s.info().height, s.info().resolution_confirmed), (320, 320, false));
    }

    #[test]
    fn unknown_poll_byte_and_stuck_boot_are_errors() {
        let (t, _) = MockScsi::new(vec![Ok(poll_reply(0x01))]);
        assert!(ScsiScreen::connect(t, "x".into(), entry(), Timing::none()).is_err());
        let mut booting = poll_reply(b'd');
        booting[4..8].copy_from_slice(&BOOT_SIGNATURE);
        let (t, _) = MockScsi::new((0..5).map(|_| Ok(booting.clone())).collect());
        let err = ScsiScreen::connect(t, "x".into(), entry(), Timing::none()).err().unwrap().to_string();
        assert!(err.contains("démarrage"));
    }

    #[test]
    fn wrong_frame_size_is_refused() {
        let (t, _) = MockScsi::new(vec![Ok(poll_reply(b'd'))]);
        let mut s = ScsiScreen::connect(t, "x".into(), entry(), Timing::none()).unwrap();
        assert!(s.push_frame(&Frame::new(240, 240)).is_err());
    }
}
