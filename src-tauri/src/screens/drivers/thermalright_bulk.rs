//! Pilote Thermalright Bulk : 87AD:70DB (GrandVision 360 AIO, Mjolnir Vision 360, Hyper Vision 360, ...).
//!
//! Sources : `doc/REFERENCE_TECHNICAL.md` « Bulk USB Protocol » (poignée de main, en-tête de trame,
//! écritures de 16 Kio, paquet de longueur nulle) et `doc/PROTOCOL_USBLCDNEW.md` « Protocol 1: Thermalright
//! LCD (87AD:70DB) » (EP01 OUT / EP01 IN, magic 0x12345678, PM = réponse[24], SUB = réponse[36]).
//!
//! Accès Windows : l'outil constructeur (USBLCDNEW) passe par LibUsbDotNet / WinUSB, et la doc
//! TRCC-Linux (GUIDE_INSTALL, section Windows) demande d'installer le pilote WinUSB avec Zadig pour ces
//! écrans. Ce pilote utilise `nusb` (WinUSB) : sans pilote WinUSB lié, l'erreur française de
//! `transport::winusb_hint` explique la marche à suivre. Non vérifié sur matériel.

use super::catalog::{self, DeviceEntry, Kind, Profile};
use super::transport::{self, NusbTransport, Transport};
use super::wire;
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Result};
use std::time::Duration;

pub const MAGIC: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
const HEADER_SIZE: usize = 64;
const REPLY_SIZE: usize = 1024;
/// Octets minimum de la réponse de poignée de main pour lire PM (24) et SUB (36).
const REPLY_MIN: usize = 41;
const WRITE_CHUNK: usize = 16 * 1024;
const CMD_JPEG: u32 = 2;
const CMD_RGB565: u32 = 3;
const JPEG_QUALITY: u8 = 80;

/// Poignée de main : 64 octets, magic, zéros, commande 01 à l'octet 56.
pub fn handshake_packet() -> [u8; HEADER_SIZE] {
    let mut p = [0u8; HEADER_SIZE];
    p[..4].copy_from_slice(&MAGIC);
    p[56] = 0x01;
    p
}

/// Trame : en-tête de 64 octets (u32 LE : commande en 4, largeur en 8, hauteur en 12, constante 2 en 56,
/// longueur de charge en 60) suivi de la charge.
pub fn build_frame(img: &wire::WireImage) -> Result<Vec<u8>> {
    let mut f = vec![0u8; HEADER_SIZE];
    f[..4].copy_from_slice(&MAGIC);
    f[4..8].copy_from_slice(&(if img.jpeg { CMD_JPEG } else { CMD_RGB565 }).to_le_bytes());
    f[8..12].copy_from_slice(&img.width.to_le_bytes());
    f[12..16].copy_from_slice(&img.height.to_le_bytes());
    f[56..60].copy_from_slice(&2u32.to_le_bytes());
    f[60..64].copy_from_slice(&u32::try_from(img.data.len())?.to_le_bytes());
    f.extend_from_slice(&img.data);
    Ok(f)
}

/// Écran bulk ouvert.
pub struct BulkScreen<T: Transport> {
    info: ScreenInfo,
    profile: Profile,
    transport: T,
}

impl<T: Transport> BulkScreen<T> {
    pub fn connect(mut transport: T, id: String, entry: &DeviceEntry) -> Result<Self> {
        let hs = handshake_packet();
        if transport.write(&hs)? < hs.len() {
            bail!("Écriture courte de la poignée de main USB.");
        }
        let resp = transport.read(REPLY_SIZE)?;
        if resp.len() < REPLY_MIN || resp[24] == 0 {
            bail!(
                "Poignée de main refusée (réponse de {} octets, PM = {}).",
                resp.len(),
                resp.get(24).copied().unwrap_or(0)
            );
        }
        let (pm, sub) = (resp[24], resp[36]);
        let profile = catalog::bulk_profile(pm, sub)
            .ok_or_else(|| anyhow!("Modèle d'écran inconnu (PM {pm}, SUB {sub}) : résolution non documentée."))?;
        let info = ScreenInfo {
            id,
            vendor: "Thermalright".into(),
            model: format!("{} (PM {pm})", catalog::display_name(entry)),
            protocol: Protocol::ThermalrightBulk,
            width: profile.width,
            height: profile.height,
            rotation: 0,
            resolution_confirmed: true,
            verified: false,
        };
        Ok(BulkScreen { info, profile, transport })
    }
}

impl<T: Transport> ScreenDriver for BulkScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        let img = wire::encode_frame(&self.profile, frame, self.info.rotation, JPEG_QUALITY)?;
        send_chunks(&mut self.transport, &build_frame(&img)?)
    }
}

/// Écritures de 16 Kio, puis paquet de longueur nulle si la trame est un multiple de 512 (délimiteur de transfert).
pub fn send_chunks(t: &mut impl Transport, data: &[u8]) -> Result<()> {
    for chunk in data.chunks(WRITE_CHUNK) {
        if t.write(chunk)? < chunk.len() {
            bail!("Écriture USB courte : l'écran n'a pas reçu toute l'image.");
        }
    }
    if data.len() % 512 == 0 {
        t.write(&[])?;
    }
    Ok(())
}

/// Détection des écrans bulk Thermalright.
pub struct BulkProvider;

impl ScreenProvider for BulkProvider {
    fn name(&self) -> &'static str {
        "Thermalright USB bulk"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for (entry, info, tag) in transport::enumerate_usb(&[Kind::Bulk])? {
            let id = format!("{:04x}:{:04x}:{tag}", entry.vid, entry.pid);
            // EP01 OUT (0x01) et EP01 IN (0x81) d'après la doc ; sinon premiers points d'accès bulk.
            let res = NusbTransport::open(&info, 0x01, 0x81, Duration::from_secs(5)).and_then(|t| BulkScreen::connect(t, id, entry));
            match res {
                Ok(s) => found.push(Box::new(s)),
                Err(e) => {
                    log::warn!("Écran USB {:04X}:{:04X} ignoré : {e}", entry.vid, entry.pid);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::MockTransport;

    fn entry() -> &'static DeviceEntry {
        catalog::find(0x87AD, 0x70DB).unwrap()
    }

    fn reply(pm: u8, sub: u8) -> Vec<u8> {
        let mut r = vec![0u8; 1024];
        r[24] = pm;
        r[36] = sub;
        r
    }

    #[test]
    fn handshake_bytes() {
        // « Handshake » : 64 octets, 12 34 56 78, zéros, octet 56 = 01.
        let p = handshake_packet();
        assert_eq!(p[..8], [0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0]);
        assert_eq!(p[56], 0x01);
        assert_eq!(p.iter().filter(|&&b| b != 0).count(), 5);
    }

    #[test]
    fn frame_header_bytes() {
        let img = wire::WireImage { data: vec![7; 10], width: 480, height: 480, jpeg: true };
        let f = build_frame(&img).unwrap();
        // 480 = 0x1E0 -> E0 01 00 00 ; commande 2 = JPEG ; constante 2 en 56 ; longueur 10 en 60.
        assert_eq!(f[..16], [0x12, 0x34, 0x56, 0x78, 2, 0, 0, 0, 0xE0, 0x01, 0, 0, 0xE0, 0x01, 0, 0]);
        assert!(f[16..56].iter().all(|&b| b == 0));
        assert_eq!(f[56..64], [2, 0, 0, 0, 10, 0, 0, 0]);
        assert_eq!(f.len(), 74);
        let rgb = wire::WireImage { data: vec![0; 4], width: 320, height: 320, jpeg: false };
        assert_eq!(build_frame(&rgb).unwrap()[4..8], [3, 0, 0, 0]);
    }

    #[test]
    fn handshake_resolves_resolution_and_sends_16k_chunks() {
        // PM 3 hors table -> FBL 72 : 480x480 JPEG.
        let (t, log) = MockTransport::new(vec![reply(3, 1)]);
        let mut s = BulkScreen::connect(t, "87ad:70db:x".into(), entry()).unwrap();
        assert_eq!((s.info().width, s.info().height, s.info().verified), (480, 480, false));
        // Bruit pseudo-aléatoire : le JPEG dépasse 16 Kio et passe en plusieurs écritures.
        let mut seed = 12345u32;
        let rgba: Vec<u8> = (0..480 * 480 * 4)
            .map(|i| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                if i % 4 == 3 { 255 } else { (seed >> 24) as u8 }
            })
            .collect();
        s.push_frame(&Frame { width: 480, height: 480, rgba }).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0], handshake_packet());
        let frame: Vec<u8> = l.writes[1..].iter().flatten().copied().collect();
        let mut sizes: Vec<usize> = l.writes[1..].iter().map(Vec::len).collect();
        assert!(sizes.len() >= 2, "doit être découpé");
        let last = sizes.pop().unwrap();
        assert!(sizes.iter().all(|&s| s == 16384));
        // Pas de ZLP sauf si la taille est un multiple de 512 ; le dernier morceau est vide dans ce cas.
        assert_eq!(last == 0, frame.len() % 512 == 0);
        assert!(last <= 16384);
        assert_eq!(frame[..4], MAGIC);
        let len = u32::from_le_bytes([frame[60], frame[61], frame[62], frame[63]]) as usize;
        let sent = frame.len();
        assert_eq!(sent, 64 + len);
        assert_eq!(frame[64..66], [0xFF, 0xD8]);
    }

    #[test]
    fn zero_length_packet_only_on_512_alignment() {
        // 40 000 octets = 16384 + 16384 + 7232, non aligné : pas de ZLP.
        let (mut t, log) = MockTransport::new(vec![]);
        send_chunks(&mut t, &vec![1; 40_000]).unwrap();
        assert_eq!(log.lock().unwrap().writes.iter().map(Vec::len).collect::<Vec<_>>(), vec![16384, 16384, 7232]);
        // 33 280 = 65 x 512 : 16384 + 16384 + 512, puis un paquet vide.
        let (mut t, log) = MockTransport::new(vec![]);
        send_chunks(&mut t, &vec![1; 33_280]).unwrap();
        assert_eq!(log.lock().unwrap().writes.iter().map(Vec::len).collect::<Vec<_>>(), vec![16384, 16384, 512, 0]);
    }

    #[test]
    fn rgb565_pm32_uses_command_3() {
        let (t, log) = MockTransport::new(vec![reply(32, 0)]);
        let mut s = BulkScreen::connect(t, "x".into(), entry()).unwrap();
        assert_eq!((s.info().width, s.info().height), (320, 320));
        s.push_frame(&Frame::new(320, 320)).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[1][4..8], [3, 0, 0, 0]);
        let total: usize = l.writes[1..].iter().map(Vec::len).sum();
        assert_eq!(total, 64 + 204_800);
        assert_eq!(l.writes[1].len(), 16384);
    }

    #[test]
    fn bad_handshakes_are_refused() {
        for bad in [vec![], vec![0u8; 40], reply(0, 0)] {
            let (t, _) = MockTransport::new(vec![bad]);
            assert!(BulkScreen::connect(t, "x".into(), entry()).is_err());
        }
    }
}
