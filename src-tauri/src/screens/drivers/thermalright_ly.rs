//! Pilote Thermalright LY : 0416:5408 (LY, Trofeo Vision 9.16) et 0416:5409 (LY1).
//!
//! Sources : `doc/REFERENCE_TECHNICAL.md` « LY Bulk Protocol » (poignée de main, chunks de 512 octets,
//! formules PM/SUB par PID) et `doc/PROTOCOL_USBLCDNEW.md` « Protocol 4: LY LCD » / « Protocol 5: LY1 LCD »
//! (tailles d'écriture/lecture, ACK, points d'accès EP09 OUT / EP02 OUT, EP01 IN).
//!
//! VÉRIFIÉ contre USBLCDNEW.dll 2.3.0 du setup TRCC 2.1.8 (`docs/screens-protocols/REA-trcc.md`) : les deux
//! variantes ci-dessous sont confirmées octet pour octet (poignée de main, PM/SUB, chunks, tailles, ACK, points d'accès).
//! Écarts entre les deux documents publics, tranchés ainsi :
//! - LY1 : on suit l'outil constructeur (poignée de main 512 octets, réponse 511, trame en une écriture,
//!   ACK 511) ; TRCC-Linux envoie la même poignée de main de 2048 octets que LY.
//! - LY : poignée de main 2048 / réponse 512, écritures de 4096 (2048 pour la dernière), ACK 512.
//!
//! Accès Windows : WinUSB via `nusb` (voir `transport::winusb_hint` si le pilote n'est pas lié).

use super::catalog::{self, DeviceEntry, Kind, Profile};
use super::transport::{self, NusbTransport, Transport};
use super::wire;
use crate::screens::driver::{Frame, FramePreparer, ScreenDriver, ScreenInfo, ScreenProvider};
use std::sync::Arc;
use anyhow::{anyhow, bail, Result};
use std::time::Duration;

const CHUNK: usize = 512;
const CHUNK_HEADER: usize = 16;
const CHUNK_DATA: usize = CHUNK - CHUNK_HEADER; // 496
const HANDSHAKE_HEADER: [u8; 16] = [0x02, 0xFF, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
const WRITE_SIZE: usize = 4096;
/// Qualité de départ de TRCC.exe 2.1.8 (`FormCZTVInit(..., ysl = 95)`), baissée de 5 en 5 sous le plafond.
const JPEG_QUALITY: u8 = 95;

/// Variante LY / LY1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Ly,
    Ly1,
}

impl Variant {
    fn from_kind(kind: Kind) -> Result<Self> {
        match kind {
            Kind::Ly => Ok(Variant::Ly),
            Kind::Ly1 => Ok(Variant::Ly1),
            _ => bail!("Famille de protocole non LY."),
        }
    }

    /// Taille de la requête de poignée de main (16 + 2032 pour LY, 16 + 496 pour LY1).
    fn handshake_size(self) -> usize {
        match self {
            Variant::Ly => 2048,
            Variant::Ly1 => 512,
        }
    }

    /// Taille lue en réponse (et pour l'ACK de trame).
    fn reply_size(self) -> usize {
        match self {
            Variant::Ly => 512,
            Variant::Ly1 => 511,
        }
    }

    /// Octet 8 de chaque chunk.
    fn chunk_flag(self) -> u8 {
        match self {
            Variant::Ly => 1,
            Variant::Ly1 => 2,
        }
    }

    /// (point d'accès OUT préféré, IN préféré) d'après PROTOCOL_USBLCDNEW.md.
    pub fn endpoints(self) -> (u8, u8) {
        match self {
            Variant::Ly => (0x09, 0x81),
            Variant::Ly1 => (0x02, 0x81),
        }
    }
}

/// Requête de poignée de main (en-tête de 16 octets + zéros).
pub fn handshake_packet(v: Variant) -> Vec<u8> {
    let mut p = vec![0u8; v.handshake_size()];
    p[..16].copy_from_slice(&HANDSHAKE_HEADER);
    p
}

/// (PM, SUB) d'après la réponse. LY : PM = 64 + octet20 (≤ 3 lu comme 1), SUB = octet22 + 1.
/// LY1 : PM = 49 + octet20, SUB = octet22.
fn pm_sub(v: Variant, resp: &[u8]) -> Result<(u8, u8)> {
    if resp.len() < 37 || resp[0] != 0x03 || resp[1] != 0xFF || resp[8] != 0x01 {
        bail!("Poignée de main LY invalide (réponse de {} octets).", resp.len());
    }
    // USBLCDNEW.dll calcule PM et SUB dans des octets (`(byte)(64 + b20)`) : le débordement s'enroule.
    let (b20, b22) = (resp[20], resp[22]);
    Ok(match v {
        Variant::Ly => (64u8.wrapping_add(if b20 <= 3 { 1 } else { b20 }), b22.wrapping_add(1)),
        Variant::Ly1 => (49u8.wrapping_add(b20), b22),
    })
}

/// Découpe `payload` en chunks de 512 octets (en-tête de 16 + 496 de données). Nombre de chunks =
/// longueur / 496 + 1 ; LY complète à un multiple de 4 avec des chunks vides, LY1 non.
pub fn build_chunks(v: Variant, payload: &[u8]) -> Result<Vec<u8>> {
    if payload.is_empty() {
        bail!("Image vide.");
    }
    let total = u32::try_from(payload.len())?;
    let count = payload.len() / CHUNK_DATA + 1;
    let count16 = u16::try_from(count).map_err(|_| anyhow!("Image trop grande ({} chunks).", count))?;
    let padded = if v == Variant::Ly { count.div_ceil(4) * 4 } else { count };
    let mut out = vec![0u8; padded * CHUNK];
    for i in 0..count {
        let off = i * CHUNK;
        let data = &payload[(i * CHUNK_DATA).min(payload.len())..((i + 1) * CHUNK_DATA).min(payload.len())];
        out[off] = 0x01;
        out[off + 1] = 0xFF;
        out[off + 2..off + 6].copy_from_slice(&total.to_le_bytes());
        out[off + 6..off + 8].copy_from_slice(&(data.len() as u16).to_le_bytes());
        out[off + 8] = v.chunk_flag();
        out[off + 9..off + 11].copy_from_slice(&count16.to_le_bytes());
        out[off + 11..off + 13].copy_from_slice(&(i as u16).to_le_bytes());
        out[off + CHUNK_HEADER..off + CHUNK_HEADER + data.len()].copy_from_slice(data);
    }
    Ok(out)
}

/// Écran LY ouvert.
pub struct LyScreen<T: Transport> {
    info: ScreenInfo,
    profile: Profile,
    variant: Variant,
    transport: T,
}

impl<T: Transport> LyScreen<T> {
    pub fn connect(mut transport: T, id: String, entry: &DeviceEntry) -> Result<Self> {
        let variant = Variant::from_kind(entry.kind)?;
        let hs = handshake_packet(variant);
        if transport.write(&hs)? < hs.len() {
            bail!("Écriture courte de la poignée de main USB.");
        }
        let (pm, sub) = pm_sub(variant, &transport.read(variant.reply_size())?)?;
        let profile = catalog::bulk_profile(pm, sub)
            .ok_or_else(|| anyhow!("Modèle d'écran inconnu (PM {pm}, SUB {sub}) : résolution non documentée."))?;
        let info = ScreenInfo {
            id,
            vendor: "Thermalright".into(),
            model: format!("{} (PM {pm})", catalog::display_name(entry)),
            protocol: entry.kind.protocol(),
            width: profile.width,
            height: profile.height,
            rotation: 0,
            resolution_confirmed: true,
            verified: false,
        };
        Ok(LyScreen { info, profile, variant, transport })
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let mut pos = 0usize;
        while pos < buf.len() {
            let rest = buf.len() - pos;
            let size = match self.variant {
                // LY : 4096 octets, ou 2048 pour la dernière écriture incomplète.
                Variant::Ly if rest >= WRITE_SIZE => WRITE_SIZE,
                Variant::Ly => rest.min(2048),
                // LY1 : une écriture du tampon entier, répétée jusqu'à tout envoyer.
                Variant::Ly1 => rest,
            };
            let n = self.transport.write(&buf[pos..pos + size])?;
            if n == 0 {
                bail!("Écriture USB bloquée : l'écran n'accepte plus de données.");
            }
            pos += n.min(size);
        }
        Ok(())
    }
}

impl<T: Transport> ScreenDriver for LyScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    /// TRCC n'envoie jamais plus de 16 images/s à l'écran LY (`nowTimerVal += 62.5`, REA-trcc.md §4).
    fn max_fps(&self) -> u8 {
        16
    }

    fn preparer(&self) -> Option<Arc<dyn FramePreparer>> {
        wire::jpeg_preparer(&self.profile, self.info.rotation, JPEG_QUALITY)
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        let img = wire::encode_frame(&self.profile, frame, self.info.rotation, JPEG_QUALITY)?;
        self.send_prepared(&img)
    }

    fn send_prepared(&mut self, img: &wire::WireImage) -> Result<()> {
        let chunks = build_chunks(self.variant, &img.data)?;
        self.write_all(&chunks)?;
        // ACK : lu mais son contenu n'est pas documenté (INCONNU) ; un délai dépassé est une erreur.
        self.transport.read(self.variant.reply_size())?;
        Ok(())
    }
}

/// Détection des écrans LY / LY1.
pub struct LyProvider;

impl ScreenProvider for LyProvider {
    fn name(&self) -> &'static str {
        "Thermalright LY"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for (entry, info, tag) in transport::enumerate_usb(&[Kind::Ly, Kind::Ly1])? {
            let id = format!("{:04x}:{:04x}:{tag}", entry.vid, entry.pid);
            let res = Variant::from_kind(entry.kind).and_then(|v| {
                let (out, inp) = v.endpoints();
                NusbTransport::open(&info, out, inp, Duration::from_secs(5))
            });
            match res.and_then(|t| LyScreen::connect(t, id, entry)) {
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

    fn entry(pid: u16) -> &'static DeviceEntry {
        catalog::find(0x0416, pid).unwrap()
    }

    fn reply(b20: u8, b22: u8) -> Vec<u8> {
        let mut r = vec![0u8; 512];
        r[0] = 0x03;
        r[1] = 0xFF;
        r[8] = 0x01;
        r[20] = b20;
        r[22] = b22;
        r
    }

    #[test]
    fn handshake_bytes() {
        // REFERENCE_TECHNICAL.md : 02 FF 00 00 00 00 00 00 01 00 ... puis zéros ; LY = 2048, LY1 = 512.
        let ly = handshake_packet(Variant::Ly);
        assert_eq!(ly.len(), 2048);
        assert_eq!(ly[..16], [0x02, 0xFF, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0]);
        assert!(ly[16..].iter().all(|&b| b == 0));
        assert_eq!(handshake_packet(Variant::Ly1).len(), 512);
    }

    #[test]
    fn pm_sub_formulas() {
        // LY : PM = 64 + b20 (<=3 lu comme 1), SUB = b22 + 1.
        assert_eq!(pm_sub(Variant::Ly, &reply(1, 4)).unwrap(), (65, 5));
        assert_eq!(pm_sub(Variant::Ly, &reply(0, 0)).unwrap(), (65, 1));
        assert_eq!(pm_sub(Variant::Ly, &reply(3, 0)).unwrap(), (65, 1));
        assert_eq!(pm_sub(Variant::Ly, &reply(5, 0)).unwrap(), (69, 1));
        // LY1 : PM = 49 + b20, SUB = b22.
        assert_eq!(pm_sub(Variant::Ly1, &reply(16, 4)).unwrap(), (65, 4));
        // Débordement d'octet enroulé comme `(byte)(49 + 255)` = 48 dans USBLCDNEW.dll ; réponses invalides refusées.
        assert_eq!(pm_sub(Variant::Ly1, &reply(255, 0)).unwrap(), (48, 0));
        assert_eq!(pm_sub(Variant::Ly, &reply(1, 255)).unwrap(), (65, 0));
        let mut bad = reply(1, 1);
        bad[1] = 0;
        assert!(pm_sub(Variant::Ly, &bad).is_err());
        assert!(pm_sub(Variant::Ly, &[3, 0xFF]).is_err());
    }

    #[test]
    fn chunk_layout_ly_padded_to_four() {
        // 1000 octets : 1000/496 + 1 = 3 chunks (496, 496, 8), complétés à 4 chunks vides (LY).
        let payload: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let out = build_chunks(Variant::Ly, &payload).unwrap();
        assert_eq!(out.len(), 4 * 512);
        // 1000 = 0x03E8 ; données 496 = 0x01F0 ; 3 chunks.
        assert_eq!(out[..16], [0x01, 0xFF, 0xE8, 0x03, 0, 0, 0xF0, 0x01, 0x01, 0x03, 0x00, 0x00, 0, 0, 0, 0]);
        assert_eq!(out[16..512], payload[..496]);
        assert_eq!(out[512 + 11..512 + 13], [1, 0]); // index 1
        assert_eq!(out[1024 + 6..1024 + 8], [8, 0]); // dernier chunk : 8 octets
        assert_eq!(out[1024 + 11..1024 + 13], [2, 0]);
        assert_eq!(out[1024 + 16..1024 + 24], payload[992..]);
        assert!(out[1024 + 24..1024 + 512].iter().all(|&b| b == 0));
        assert!(out[1536..].iter().all(|&b| b == 0)); // chunk de bourrage totalement vide
    }

    #[test]
    fn chunk_layout_ly1_not_padded_flag_2() {
        let out = build_chunks(Variant::Ly1, &[9u8; 600]).unwrap();
        assert_eq!(out.len(), 2 * 512); // 600/496 + 1 = 2
        assert_eq!(out[8], 0x02);
        assert_eq!(out[9..11], [2, 0]);
        assert_eq!(out[512 + 6..512 + 8], [104, 0]); // 600 - 496
        // Taille multiple de 496 : dernier chunk de 0 octet de données (formule du doc).
        let exact = build_chunks(Variant::Ly1, &[1u8; 992]).unwrap();
        assert_eq!(exact.len(), 3 * 512);
        assert_eq!(exact[1024 + 6..1024 + 8], [0, 0]);
        assert!(build_chunks(Variant::Ly, &[]).is_err());
    }

    #[test]
    fn full_frame_ly_with_fake_device() {
        // b20 = 1 -> PM 65 -> 1920x462 JPEG ; SUB = 1.
        let (t, log) = MockTransport::new(vec![reply(1, 0), vec![0u8; 512]]);
        let mut s = LyScreen::connect(t, "0416:5408:x".into(), entry(0x5408)).unwrap();
        assert_eq!((s.info().width, s.info().height, s.info().verified), (1920, 462, false));
        s.push_frame(&Frame { width: 1920, height: 462, rgba: [30u8, 60, 90, 255].repeat(1920 * 462) }).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0].len(), 2048);
        let all: Vec<u8> = l.writes[1..].iter().flatten().copied().collect();
        assert_eq!(all.len() % 2048, 0); // multiple de 4 chunks
        let sizes: Vec<usize> = l.writes[1..].iter().map(Vec::len).collect();
        assert!(sizes[..sizes.len() - 1].iter().all(|&s| s == 4096));
        assert!(matches!(sizes[sizes.len() - 1], 4096 | 2048));
        let total = u32::from_le_bytes([all[2], all[3], all[4], all[5]]) as usize;
        assert_eq!(all.len(), (total / 496 + 1).div_ceil(4) * 4 * 512);
        assert_eq!(all[16..18], [0xFF, 0xD8]);
        assert_eq!(l.reads, vec![512, 512]); // réponse de poignée de main, puis ACK
    }

    #[test]
    fn full_frame_ly1_single_write_and_ack_511() {
        // b20 = 16 -> PM 65 ; poignée de main 512 / réponse 511.
        let (t, log) = MockTransport::new(vec![reply(16, 3), vec![0u8; 511]]);
        let mut s = LyScreen::connect(t, "x".into(), entry(0x5409)).unwrap();
        assert_eq!((s.info().width, s.info().height), (1920, 462));
        s.push_frame(&Frame::new(1920, 462)).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes[0].len(), 512);
        assert_eq!(l.writes.len(), 2); // trame en UNE écriture
        assert_eq!(l.writes[1][8], 0x02);
        assert_eq!(l.writes[1].len() % 512, 0);
        assert_eq!(l.reads, vec![511, 511]);
    }

    #[test]
    fn zero_byte_write_fails_instead_of_looping() {
        let (t, _) = MockTransport::new(vec![reply(1, 0)]);
        let mut s = LyScreen::connect(t, "x".into(), entry(0x5408)).unwrap();
        s.transport.short_write = Some(0);
        assert!(s.write_all(&[1u8; 2048]).is_err());
    }
}
