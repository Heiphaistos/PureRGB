//! Pilote Lian Li HID : Galahad II LCD / Vision et HydroShift LCD (VID 0416, 480x480, JPEG).
//!
//! Sources (le protocole n'a pas de spécification écrite ; c'est du code MIT lu comme DONNÉES,
//! rien de copié, tout réécrit d'après la description des octets) :
//! - `sgtaziz/lian-li-linux` @f55f3e5 (MIT), `crates/lianli-devices/src/hydroshift_lcd/{protocol,controller}.rs`
//!   et `crates/lianli-shared/src/screen.rs` (`AIO_LCD_480` : 480x480, 24 i/s, JPEG, 153 600 octets max).
//! - Fiche `docs/screens-protocols/lian-li.md` (marqueurs [V]).
//!
//! Protocole implémenté (tout est [V] dans la source ci-dessus) :
//! - Rapports HID « B » : 1024 octets, identifiant de rapport 0x02 en premier octet.
//!   Entête de 11 octets `[02][cmd][taille totale BE32][n° de paquet BE24][longueur BE16]`, puis
//!   au plus 1013 octets de données, complétés de zéros.
//! - 0x0C « LCD control » (8 octets : mode, luminosité 0-100, rotation, 4 réservés, i/s) ;
//!   mode 1 = Application (images pilotées par l'hôte), mode 4 = réglage (luminosité).
//! - 0x0E « JPEG » : l'image JPEG complète découpée en rapports B.
//!
//! NON implémenté (INCONNU ou hors sujet écran) : rapports A (pompe, ventilateurs, firmware, température),
//! rapports C (512 o, firmware >= 1.2/1.5 seulement), H.264, rotation matérielle, bascule bureau,
//! récupération après perte du LCD (0x17), réinitialisation (0x8E).
//!
//! À VALIDER SUR MATÉRIEL : l'écriture hidapi sous Windows d'un rapport de 1024 octets vers un
//! périphérique à plusieurs identifiants de rapport (la source utilise rusb/hidraw), le choix de
//! l'interface HID quand plusieurs sont exposées, le délai de ~10 s que le dépôt de référence attend
//! après la mise sous tension avant de parler à l'AIO.

use super::encode::rgba_to_jpeg;
use super::transport::Transport;
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Context, Result};

const VID: u16 = 0x0416;
const REPORT_B: u8 = 0x02;
const PACKET_SIZE: usize = 1024;
const HEADER_LEN: usize = 11;
const MAX_CHUNK: usize = PACKET_SIZE - HEADER_LEN; // 1013
const CMD_LCD_CONTROL: u8 = 0x0C;
const CMD_SEND_JPEG: u8 = 0x0E;
/// Rapport d'état périodique de l'AIO (rapport A, commande 0x81) : à ignorer quand on attend un acquittement.
const REPORT_A: u8 = 0x01;
const CMD_STATUS: u8 = 0x81;
const MODE_APPLICATION: u8 = 1;
const MODE_LCD_SETTING: u8 = 4;
const SIDE: u32 = 480;
const FPS: u8 = 24;
/// Taille JPEG maximale (`AIO_LCD_480.max_payload`).
const MAX_JPEG_BYTES: usize = 153_600;
/// Qualités essayées dans l'ordre jusqu'à passer sous `MAX_JPEG_BYTES`.
const QUALITIES: [u8; 5] = [85, 70, 55, 40, 25];
/// Lectures d'acquittement : commande de contrôle (obligatoire) / image (facultatif).
const ACK_TRIES_CONTROL: usize = 40;
const ACK_TRIES_FRAME: usize = 4;
const READ_TIMEOUT_MS: i32 = 30;
const DEFAULT_BRIGHTNESS: u8 = 100;

/// Nom du produit pour un PID (fiche lian-li.md, tableau « Modèles d'écran »).
fn model_name(pid: u16) -> Option<&'static str> {
    match pid {
        0x7391 => Some("Galahad II LCD"),
        0x7395 => Some("Galahad II Vision"),
        0x7398 => Some("HydroShift LCD"),
        0x7399 => Some("HydroShift LCD RGB"),
        0x739A => Some("HydroShift LCD TL"),
        _ => None,
    }
}

/// Un rapport B complet (1024 octets). `total` = taille de toute la commande, `num` = rang du paquet.
fn build_packet(cmd: u8, total: u32, num: u32, chunk: &[u8]) -> Result<Vec<u8>> {
    if chunk.len() > MAX_CHUNK {
        bail!("Rapport HID trop grand : {} octets de données (maximum {MAX_CHUNK}).", chunk.len());
    }
    if num > 0x00FF_FFFF {
        bail!("Trop de paquets HID pour une seule commande ({num}).");
    }
    let mut p = vec![0u8; PACKET_SIZE];
    p[0] = REPORT_B;
    p[1] = cmd;
    p[2..6].copy_from_slice(&total.to_be_bytes());
    p[6..9].copy_from_slice(&num.to_be_bytes()[1..]);
    p[9..11].copy_from_slice(&(chunk.len() as u16).to_be_bytes());
    p[HEADER_LEN..HEADER_LEN + chunk.len()].copy_from_slice(chunk);
    Ok(p)
}

/// Découpe `data` en rapports B (au moins un, même pour une commande sans données).
fn packets(cmd: u8, data: &[u8]) -> Result<Vec<Vec<u8>>> {
    let total = u32::try_from(data.len()).map_err(|_| anyhow!("Commande HID trop volumineuse."))?;
    if data.is_empty() {
        return Ok(vec![build_packet(cmd, 0, 0, &[])?]);
    }
    data.chunks(MAX_CHUNK)
        .enumerate()
        .map(|(i, c)| build_packet(cmd, total, i as u32, c))
        .collect()
}

/// Charge utile de 0x0C : `[mode][luminosité][rotation=0][0 0 0 0][i/s]`.
fn control_payload(mode: u8, brightness: u8) -> [u8; 8] {
    [mode, brightness, 0, 0, 0, 0, 0, FPS]
}

/// Encode en JPEG sous la limite de l'appareil en baissant la qualité au besoin.
fn fit_jpeg(frame: &Frame, max_bytes: usize) -> Result<Vec<u8>> {
    for q in QUALITIES {
        let jpeg = rgba_to_jpeg(&frame.rgba, frame.width, frame.height, q)?;
        if jpeg.len() <= max_bytes {
            return Ok(jpeg);
        }
    }
    bail!("Image trop complexe : le JPEG dépasse {max_bytes} octets même à la qualité minimale.")
}

/// Écran Lian Li HID ouvert.
pub struct LianLiHidScreen<T: Transport> {
    info: ScreenInfo,
    transport: T,
}

impl<T: Transport> LianLiHidScreen<T> {
    /// Passe l'écran en mode Application (0x0C) et exige l'acquittement de l'appareil.
    pub fn connect(transport: T, id: String, pid: u16) -> Result<Self> {
        let name = model_name(pid).ok_or_else(|| anyhow!("PID Lian Li HID inconnu : {pid:04X}."))?;
        let info = ScreenInfo {
            id,
            vendor: "Lian Li".into(),
            model: name.into(),
            protocol: Protocol::LianLiHid,
            width: SIDE,
            height: SIDE,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        let mut s = LianLiHidScreen { info, transport };
        s.control(MODE_APPLICATION, DEFAULT_BRIGHTNESS).map_err(|e| {
            anyhow!(
                "{name} : l'écran ne répond pas ({e}). Fermez L-Connect 3, patientez une dizaine de secondes \
                 après la mise sous tension du PC, puis réessayez."
            )
        })?;
        Ok(s)
    }

    fn send(&mut self, cmd: u8, data: &[u8]) -> Result<()> {
        for p in packets(cmd, data)? {
            let n = self.transport.write(&p)?;
            if n < p.len() {
                bail!("Écriture HID courte ({n} octets sur {}) : l'écran n'a pas reçu toute la commande.", p.len());
            }
        }
        Ok(())
    }

    /// Attend l'écho `[02][cmd]` en ignorant les rapports d'état (A/0x81). `strict` : l'absence
    /// d'acquittement est une erreur (commandes de contrôle) ; sinon elle est tolérée, comme le fait
    /// le dépôt de référence pour les images.
    fn read_ack(&mut self, cmd: u8, strict: bool) -> Result<()> {
        let tries = if strict { ACK_TRIES_CONTROL } else { ACK_TRIES_FRAME };
        for _ in 0..tries {
            match self.transport.read(PACKET_SIZE) {
                Ok(r) if r.len() >= 2 && r[0] == REPORT_B && r[1] == cmd => return Ok(()),
                Ok(r) if r.len() >= 2 && r[0] == REPORT_A && r[1] == CMD_STATUS => {}
                Ok(r) => log::debug!("Rapport HID Lian Li ignoré en attente de {cmd:#04X} : {:02X?}", &r[..r.len().min(4)]),
                Err(e) if strict => log::debug!("Lecture HID Lian Li : {e:#}"),
                Err(_) => return Ok(()),
            }
        }
        if strict {
            bail!("aucun acquittement de la commande {cmd:#04X}");
        }
        Ok(())
    }

    fn control(&mut self, mode: u8, brightness: u8) -> Result<()> {
        self.send(CMD_LCD_CONTROL, &control_payload(mode, brightness))?;
        self.read_ack(CMD_LCD_CONTROL, true)
    }
}

impl<T: Transport> ScreenDriver for LianLiHidScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        if !frame.is_valid() || frame.width != SIDE || frame.height != SIDE {
            bail!("Image {}x{} invalide : cet écran attend {SIDE}x{SIDE}.", frame.width, frame.height);
        }
        let jpeg = fit_jpeg(frame, MAX_JPEG_BYTES)?;
        self.send(CMD_SEND_JPEG, &jpeg)?;
        self.read_ack(CMD_SEND_JPEG, false)
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        if percent > 100 {
            bail!("Luminosité {percent} hors bornes (0 à 100).");
        }
        self.control(MODE_LCD_SETTING, percent)?;
        Ok(true)
    }
}

/// Transport hidapi « brut » : le premier octet est déjà l'identifiant de rapport
/// (contrairement à `HidTransport`, qui ajoute 0x00).
struct RawHidTransport {
    dev: hidapi::HidDevice,
}

impl Transport for RawHidTransport {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.dev.write(data).context("Écriture HID échouée")
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let n = self.dev.read_timeout(&mut buf, READ_TIMEOUT_MS).context("Lecture HID échouée")?;
        if n == 0 {
            bail!("Aucune réponse de l'écran (délai dépassé).");
        }
        buf.truncate(n);
        Ok(buf)
    }
}

/// Détection des écrans Lian Li HID (0416:7391/7395/7398/7399/739A).
pub struct LianLiHidProvider;

impl ScreenProvider for LianLiHidProvider {
    fn name(&self) -> &'static str {
        "Lian Li HID"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
        // Énumération seule : un candidat par (PID, série ou chemin), interface « fabricant » d'abord.
        let mut devs: Vec<_> = api.device_list().filter(|d| d.vendor_id() == VID && model_name(d.product_id()).is_some()).collect();
        devs.sort_by_key(|d| std::cmp::Reverse(d.usage_page() >= 0xFF00));
        let mut seen: Vec<(u16, String)> = Vec::new();
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for d in devs {
            let tag = match d.serial_number() {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => d.path().to_string_lossy().into_owned(),
            };
            if seen.contains(&(d.product_id(), tag.clone())) {
                continue;
            }
            seen.push((d.product_id(), tag.clone()));
            let id = format!("{VID:04x}:{:04x}:{tag}", d.product_id());
            let opened = api.open_path(d.path()).map_err(|e| {
                anyhow!("Impossible d'ouvrir l'écran Lian Li {VID:04X}:{:04X} ({e}). Fermez L-Connect 3 puis rebranchez l'écran.", d.product_id())
            });
            match opened.and_then(|dev| LianLiHidScreen::connect(RawHidTransport { dev }, id, d.product_id())) {
                Ok(s) => found.push(Box::new(s)),
                Err(e) => {
                    log::warn!("Écran Lian Li HID ignoré : {e:#}");
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

    fn ack(cmd: u8) -> Vec<u8> {
        let mut r = vec![0u8; 1024];
        r[0] = 0x02;
        r[1] = cmd;
        r
    }

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Frame {
        Frame { width: w, height: h, rgba: px.repeat((w * h) as usize) }
    }

    #[test]
    fn jpeg_packet_headers_match_the_documented_layout() {
        // lian-li.md : `[id][cmd][taille totale BE32][n paquet BE24][len BE16]`, 1013 o de données max.
        // 2000 octets = 0x000007D0 ; 1013 = 0x03F5 ; reste 987 = 0x03DB.
        let data: Vec<u8> = (0..2000u32).map(|i| (i % 251) as u8).collect();
        let p = packets(0x0E, &data).unwrap();
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|x| x.len() == 1024));
        assert_eq!(p[0][..11], [0x02, 0x0E, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00, 0x00, 0x03, 0xF5]);
        assert_eq!(p[1][..11], [0x02, 0x0E, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00, 0x01, 0x03, 0xDB]);
        assert_eq!(p[0][11..11 + 1013], data[..1013]);
        assert_eq!(p[1][11..11 + 987], data[1013..]);
        assert!(p[1][11 + 987..].iter().all(|&b| b == 0));
    }

    #[test]
    fn lcd_control_application_bytes() {
        // Mode 1 (Application), luminosité 100 = 0x64, rotation 0, i/s 24 = 0x18 à l'octet 7.
        let p = packets(0x0C, &control_payload(1, 100)).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0][..19], [0x02, 0x0C, 0, 0, 0, 8, 0, 0, 0, 0, 8, 0x01, 0x64, 0, 0, 0, 0, 0, 0x18]);
        assert!(p[0][19..].iter().all(|&b| b == 0));
    }

    #[test]
    fn oversized_chunk_and_packet_counter_are_rejected() {
        assert!(build_packet(0x0E, 10, 0, &[0u8; 1014]).is_err());
        assert!(build_packet(0x0E, 10, 0x0100_0000, &[1]).is_err());
    }

    #[test]
    fn connect_sets_application_mode_and_skips_status_reports() {
        let mut status = vec![0u8; 512];
        status[0] = 0x01;
        status[1] = 0x81;
        let (t, log) = MockTransport::new(vec![status, ack(0x0C)]);
        let s = LianLiHidScreen::connect(t, "0416:7395:x".into(), 0x7395).unwrap();
        assert_eq!(s.info().model, "Galahad II Vision");
        assert_eq!((s.info().width, s.info().height, s.info().verified), (480, 480, false));
        let l = log.lock().unwrap();
        assert_eq!(l.writes.len(), 1);
        assert_eq!(l.writes[0][..13], [0x02, 0x0C, 0, 0, 0, 8, 0, 0, 0, 0, 8, 0x01, 0x64]);
    }

    #[test]
    fn connect_fails_without_ack_or_with_wrong_echo() {
        let (t, _) = MockTransport::new(vec![]);
        let e = LianLiHidScreen::connect(t, "x".into(), 0x7391).err().unwrap().to_string();
        assert!(e.contains("L-Connect 3"), "{e}");
        let (t, _) = MockTransport::new(vec![ack(0x0E); 3]);
        assert!(LianLiHidScreen::connect(t, "x".into(), 0x7391).is_err());
        let (t, _) = MockTransport::new(vec![]);
        assert!(LianLiHidScreen::connect(t, "x".into(), 0x1234).is_err());
    }

    #[test]
    fn push_frame_sends_a_complete_jpeg_in_b_reports() {
        let (t, log) = MockTransport::new(vec![ack(0x0C), ack(0x0E)]);
        let mut s = LianLiHidScreen::connect(t, "x".into(), 0x7391).unwrap();
        s.push_frame(&solid(480, 480, [200, 30, 30, 255])).unwrap();
        let l = log.lock().unwrap();
        let reports = &l.writes[1..];
        assert!(!reports.is_empty() && reports.iter().all(|r| r.len() == 1024 && r[0] == 2 && r[1] == 0x0E));
        let total = u32::from_be_bytes([reports[0][2], reports[0][3], reports[0][4], reports[0][5]]) as usize;
        let mut jpeg = Vec::new();
        for (i, r) in reports.iter().enumerate() {
            assert_eq!(u32::from_be_bytes([0, r[6], r[7], r[8]]) as usize, i);
            let len = u16::from_be_bytes([r[9], r[10]]) as usize;
            assert_eq!(len, if i + 1 < reports.len() { 1013 } else { total - 1013 * i });
            jpeg.extend_from_slice(&r[11..11 + len]);
        }
        assert_eq!(jpeg.len(), total);
        assert_eq!(jpeg[..2], [0xFF, 0xD8]);
        assert_eq!(jpeg[total - 2..], [0xFF, 0xD9]);
    }

    #[test]
    fn push_frame_validates_size_and_short_writes() {
        let (t, _) = MockTransport::new(vec![ack(0x0C)]);
        let mut s = LianLiHidScreen::connect(t, "x".into(), 0x7391).unwrap();
        assert!(s.push_frame(&solid(100, 100, [0, 0, 0, 255])).is_err());
        let mut bad = solid(480, 480, [0, 0, 0, 255]);
        bad.rgba.pop();
        assert!(s.push_frame(&bad).is_err());
        let (mut t, _) = MockTransport::new(vec![ack(0x0C)]);
        let mut s = LianLiHidScreen::connect(t, "x".into(), 0x7391).unwrap();
        s.transport.short_write = Some(10);
        let e = s.push_frame(&solid(480, 480, [9, 9, 9, 255])).unwrap_err().to_string();
        assert!(e.contains("courte"), "{e}");
    }

    #[test]
    fn brightness_uses_lcd_setting_mode_and_rejects_out_of_range() {
        let (t, log) = MockTransport::new(vec![ack(0x0C), ack(0x0C)]);
        let mut s = LianLiHidScreen::connect(t, "x".into(), 0x7398).unwrap();
        assert!(s.set_brightness(30).unwrap());
        assert_eq!(log.lock().unwrap().writes[1][11..13], [0x04, 30]);
        assert!(s.set_brightness(101).is_err());
    }

    #[test]
    fn fit_jpeg_errors_when_limit_is_unreachable() {
        assert!(fit_jpeg(&solid(480, 480, [1, 2, 3, 255]), 100).is_err());
        assert!(fit_jpeg(&solid(480, 480, [1, 2, 3, 255]), MAX_JPEG_BYTES).is_ok());
    }

    #[test]
    fn catalog_covers_the_five_pids() {
        for pid in [0x7391, 0x7395, 0x7398, 0x7399, 0x739A] {
            assert!(model_name(pid).is_some());
        }
        assert!(model_name(0x7372).is_none());
    }
}
