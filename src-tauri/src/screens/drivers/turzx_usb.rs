//! Pilote TURZX / Turing USB (VID 1CBE) : écrans « Turing » génériques (PID 00xx) et écrans
//! Lian Li OEM (HydroShift II, Lancool 207, Universal Screen 8,8", Vision 9,2", TL/SL Flex : PID A0xx).
//!
//! Sources (aucun code copié ; octets et étapes réécrits d'après leur description, contenus lus comme DONNÉES) :
//! - `sgtaziz/lian-li-linux` @f55f3e5 (MIT) : `crates/lianli-devices/src/crypto.rs` (en-tête chiffré, commandes),
//!   `winusb/lcd/{mod,core,slv3,base,h2_lcd}.rs` (séquence d'init, envoi d'image), `lianli-shared/src/screen.rs`
//!   (résolutions, limites de charge utile). Recoupé par `andyjusa/hydroshift2-rs` (MIT, README « Protocol notes »).
//! - `mathoudebine/turing-smart-screen-python` @2b33ab4 (GPL-3.0, lu comme description du protocole seulement),
//!   `library/lcd/lcd_comm_turing_usb.py` : table des PID Turing, commandes 101 (JPEG) / 10 / 14, échelle de luminosité.
//! - Fiches `docs/screens-protocols/{lian-li,generiques-usb}.md` (marqueurs [V]).
//!
//! Trame [V] : 512 octets de commande + charge utile, en UNE écriture bulk (EP 0x01 OUT / 0x81 IN).
//! Commande en clair de 500 octets : `[0]=cmd, [2]=0x1A, [3]=0x6D, [4..8]=horloge ms LE u32, [8..]=paramètres`
//! (taille de l'image en BE u32 à l'octet 8), complétée à 504 octets, chiffrée en DES-CBC (clé = IV = `slv3tuzx`),
//! puis `[504..510]=0` et `[510..512]=A1 1A`. Complément de l'en-tête : zéros côté Turing (Python),
//! PKCS#7 côté Lian Li (Rust) : les deux familles sont testées séparément par leurs auteurs, d'où `Padding`.
//!
//! Crate `des` (RustCrypto, épinglée) : DES n'est pas dans la bibliothèque standard et ne se réimplémente pas
//! à la légère ; le mode CBC est écrit ici (quelques lignes) pour éviter une seconde dépendance (`cbc`).
//!
//! NON implémenté (INCONNU / hors sujet) : H.264 (0x79/0x7B/0x11), PNG et calque de superposition (0x66/0x67,
//! OLED Curve A068 = 1080x2288 exclu), horloge interne (0x33/0x34), rotation (0x0D), cadence (0x0F), pompe/ventilateurs
//! (0xFB), anneau LED (0xFC), bascule mode bureau 1A86:AD** (commande 0x96), vidage de la file de réponses après lecture.
//! À VALIDER SUR MATÉRIEL : tout (aucun test réel), notamment le contenu exact des acquittements (seule la réponse
//! à GetVer, `[0]=0x0A` + version ASCII en `[8..40]`, est décrite) et la compatibilité des deux compléments.

use super::encode::rgba_to_jpeg;
use super::transport::{NusbTransport, Transport};
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Result};
use des::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use des::Des;
use std::time::{Duration, Instant};

const VID: u16 = 0x1CBE;
const DESKTOP_VID: u16 = 0x1A86;
const KEY: [u8; 8] = *b"slv3tuzx";
const CMD_GET_VER: u8 = 0x0A;
const CMD_BRIGHTNESS: u8 = 0x0E;
const CMD_PUSH_JPG: u8 = 0x65;
const CMD_QUERY_BLOCK: u8 = 0x7A;
const CMD_STOP_PLAY: u8 = 0x7B;
const PLAIN_LEN: usize = 500;
const HEADER_LEN: usize = 512;
const REPLY_LEN: usize = 512;
const IO_TIMEOUT: Duration = Duration::from_millis(2000);
/// Au-dessus de ce niveau de tampon (octet 8 de l'acquittement), on attend avant d'envoyer la suite.
const BUFFER_HIGH: u8 = 3;
const BUFFER_TARGET: u8 = 2;
const BUFFER_POLLS: usize = 20;
const POLL_DELAY: Duration = Duration::from_millis(if cfg!(test) { 1 } else { 50 });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Padding {
    Zero,
    Pkcs7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Turing,
    LianLi,
}

/// Une ligne du catalogue (dimensions natives = orientation portrait envoyée à l'appareil).
struct Model {
    pid: u16,
    name: &'static str,
    width: u32,
    height: u32,
    family: Family,
    /// Qualité JPEG de départ (`screen.rs` : 85 AIO, 95 grands écrans ; Python : 90).
    quality: u8,
    /// Taille maximale du JPEG (`screen.rs` `max_payload` ; Python : 1 Mio).
    max_jpeg: usize,
    /// StopPlay (0x7B) accepté avant les images (`supports_stop_play`).
    stop_play: bool,
    /// L'AIO H2 ne répond pas toujours à GetVer (commentaire de `h2_control_init`).
    getver_optional: bool,
}

const fn turing(pid: u16, name: &'static str, width: u32, height: u32) -> Model {
    Model { pid, name, width, height, family: Family::Turing, quality: 90, max_jpeg: 1 << 20, stop_play: false, getver_optional: false }
}

const fn lianli(pid: u16, name: &'static str, (width, height): (u32, u32), quality: u8, max_jpeg: usize, stop_play: bool, getver_optional: bool) -> Model {
    Model { pid, name, width, height, family: Family::LianLi, quality, max_jpeg, stop_play, getver_optional }
}

const MODELS: &[Model] = &[
    turing(0x0028, "Turing 2,8\" rond", 480, 480),
    turing(0x0046, "Turing 4,6\"", 320, 960),
    turing(0x0050, "Turing 5,2\"", 720, 1280),
    turing(0x0080, "Turing 8,0\"", 800, 1280),
    turing(0x0088, "Turing 8,8\"", 480, 1920),
    turing(0x0092, "Turing 9,2\"", 462, 1920),
    turing(0x0123, "Turing 12,3\"", 720, 1920),
    lianli(0xA021, "HydroShift II LCD Circle", (480, 480), 85, 153_600, true, true),
    lianli(0xA034, "HydroShift II LCD Square", (480, 480), 85, 153_600, true, true),
    lianli(0xA065, "Lancool 207 Digital", (720, 1472), 95, 512_000, true, false),
    lianli(0xA088, "Universal Screen 8,8\"", (480, 1920), 95, 512_000, true, false),
    lianli(0xA092, "Vision 9,2\"", (464, 1920), 95, 512_000, true, false),
    lianli(0xA018, "TL Flex LCD", (400, 400), 95, 512_000, false, false),
    lianli(0xA019, "SL Infinity Flex LCD", (400, 400), 95, 512_000, false, false),
];

fn model(pid: u16) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.pid == pid)
}

/// DES-CBC sans bourrage : `data.len()` doit être un multiple de 8.
fn des_cbc_encrypt(key: &[u8; 8], iv: &[u8; 8], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() % 8 != 0 {
        bail!("Données DES non alignées sur 8 octets ({}).", data.len());
    }
    let cipher = Des::new(GenericArray::from_slice(key));
    let mut prev = *iv;
    let mut out = Vec::with_capacity(data.len());
    for block in data.chunks_exact(8) {
        let mut ga = GenericArray::clone_from_slice(block);
        for (v, p) in ga.iter_mut().zip(prev) {
            *v ^= p;
        }
        cipher.encrypt_block(&mut ga);
        prev.copy_from_slice(&ga);
        out.extend_from_slice(&ga);
    }
    Ok(out)
}

/// En-tête chiffré de 512 octets (voir l'en-tête du fichier).
fn build_header(cmd: u8, params: &[u8], timestamp: u32, padding: Padding) -> Result<[u8; HEADER_LEN]> {
    if params.len() > PLAIN_LEN - 8 {
        bail!("Paramètres de commande trop longs ({} octets).", params.len());
    }
    let mut plain = [0u8; PLAIN_LEN + 4];
    plain[0] = cmd;
    plain[2] = 0x1A;
    plain[3] = 0x6D;
    plain[4..8].copy_from_slice(&timestamp.to_le_bytes());
    plain[8..8 + params.len()].copy_from_slice(params);
    if padding == Padding::Pkcs7 {
        plain[PLAIN_LEN..].fill(4);
    }
    let enc = des_cbc_encrypt(&KEY, &KEY, &plain)?;
    let mut out = [0u8; HEADER_LEN];
    out[..PLAIN_LEN + 4].copy_from_slice(&enc);
    out[510] = 0xA1;
    out[511] = 0x1A;
    Ok(out)
}

/// Version firmware d'une réponse GetVer : `[0]=0x0A`, texte ASCII imprimable non vide en `[8..40]`.
fn parse_get_ver(reply: &[u8]) -> Result<String> {
    if reply.first() != Some(&CMD_GET_VER) || reply.len() < 9 {
        bail!("réponse GetVer invalide ({} octets)", reply.len());
    }
    let field = &reply[8..reply.len().min(40)];
    let text = field.split(|&b| b == 0).next().unwrap_or_default();
    if text.is_empty() || !text.iter().all(|&b| b.is_ascii_graphic() || b == b' ') {
        bail!("version firmware illisible dans la réponse GetVer");
    }
    Ok(String::from_utf8_lossy(text).into_owned())
}

/// Écran TURZX ouvert.
pub struct TurzxScreen<T: Transport> {
    info: ScreenInfo,
    transport: T,
    model: &'static Model,
    start: Instant,
    last_ts: u32,
    firmware: Option<String>,
}

impl<T: Transport> TurzxScreen<T> {
    /// Ouvre la session : StopPlay (si géré) puis GetVer. Une réponse présente doit être valide.
    pub fn connect(transport: T, id: String, pid: u16) -> Result<Self> {
        let model = model(pid).ok_or_else(|| anyhow!("PID TURZX inconnu : 1CBE:{pid:04X}."))?;
        let info = ScreenInfo {
            id,
            vendor: if model.family == Family::LianLi { "Lian Li" } else { "Turing" }.into(),
            model: model.name.into(),
            protocol: Protocol::TurzxUsb,
            width: model.width,
            height: model.height,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        let mut s = TurzxScreen { info, transport, model, start: Instant::now(), last_ts: 0, firmware: None };
        if model.stop_play {
            // La source ignore l'absence de réponse à StopPlay (lecture seulement tracée).
            if let Err(e) = s.exchange(CMD_STOP_PLAY, &[]) {
                log::debug!("StopPlay sans réponse sur 1CBE:{pid:04X} : {e:#}");
            }
        }
        match s.exchange(CMD_GET_VER, &[]).and_then(|r| parse_get_ver(&r)) {
            Ok(v) => s.firmware = Some(v),
            Err(e) if model.getver_optional => log::debug!("GetVer sans réponse exploitable sur 1CBE:{pid:04X} : {e:#}"),
            Err(e) => bail!(
                "{} : l'écran ne répond pas correctement ({e:#}). Fermez L-Connect 3 / le logiciel TURZX, rebranchez l'écran et réessayez.",
                model.name
            ),
        }
        Ok(s)
    }

    pub fn firmware(&self) -> Option<&str> {
        self.firmware.as_deref()
    }

    fn padding(&self) -> Padding {
        if self.model.family == Family::Turing { Padding::Zero } else { Padding::Pkcs7 }
    }

    /// Horloge en ms, strictement croissante (comme le dépôt de référence).
    fn next_ts(&mut self) -> u32 {
        let raw = u32::try_from(self.start.elapsed().as_millis()).unwrap_or(u32::MAX);
        self.last_ts = raw.max(self.last_ts.saturating_add(1));
        self.last_ts
    }

    /// Envoie `header + payload` en une écriture et lit une réponse non vide.
    fn exchange_with(&mut self, cmd: u8, params: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let ts = self.next_ts();
        let mut packet = build_header(cmd, params, ts, self.padding())?.to_vec();
        packet.extend_from_slice(payload);
        let n = self.transport.write(&packet)?;
        if n < packet.len() {
            bail!("Écriture USB courte ({n} octets sur {}) : l'écran n'a pas reçu toute la commande.", packet.len());
        }
        let reply = self.transport.read(REPLY_LEN)?;
        if reply.is_empty() {
            bail!("réponse vide à la commande {cmd:#04X}");
        }
        Ok(reply)
    }

    fn exchange(&mut self, cmd: u8, params: &[u8]) -> Result<Vec<u8>> {
        self.exchange_with(cmd, params, &[])
    }

    /// Attend que le tampon de l'écran redescende (QueryBlock 0x7A, niveau à l'octet 8).
    fn wait_buffer(&mut self, mut level: u8) -> Result<()> {
        for _ in 0..BUFFER_POLLS {
            if level <= BUFFER_TARGET {
                return Ok(());
            }
            std::thread::sleep(POLL_DELAY);
            let r = self.exchange(CMD_QUERY_BLOCK, &[])?;
            level = *r.get(8).ok_or_else(|| anyhow!("réponse QueryBlock trop courte ({} octets)", r.len()))?;
        }
        if level <= BUFFER_TARGET {
            return Ok(());
        }
        bail!("L'écran n'absorbe plus les images (tampon saturé, niveau {level}).")
    }
}

impl<T: Transport> ScreenDriver for TurzxScreen<T> {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        let m = self.model;
        if !frame.is_valid() || frame.width != m.width || frame.height != m.height {
            bail!("Image {}x{} invalide : cet écran attend {}x{}.", frame.width, frame.height, m.width, m.height);
        }
        let mut jpeg = None;
        for q in [m.quality, 75, 60, 45, 30].into_iter().filter(|&q| q <= m.quality) {
            let data = rgba_to_jpeg(&frame.rgba, frame.width, frame.height, q)?;
            if data.len() <= m.max_jpeg {
                jpeg = Some(data);
                break;
            }
        }
        let jpeg = jpeg.ok_or_else(|| anyhow!("Image trop complexe : le JPEG dépasse {} octets même à la qualité minimale.", m.max_jpeg))?;
        let size = u32::try_from(jpeg.len())?.to_be_bytes();
        let reply = self.exchange_with(CMD_PUSH_JPG, &size, &jpeg)?;
        // Niveau de tampon : décrit pour Lian Li seulement ([V] core.rs `buffer_level`).
        match reply.get(8) {
            Some(&lvl) if m.family == Family::LianLi && lvl > BUFFER_HIGH => self.wait_buffer(lvl),
            _ => Ok(()),
        }
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        if percent > 100 {
            bail!("Luminosité {percent} hors bornes (0 à 100).");
        }
        // Turing : échelle 0-102 (`int(niveau / 100 * 102)`) ; Lian Li : pourcentage direct.
        let value = if self.model.family == Family::Turing { (u32::from(percent) * 102 / 100) as u8 } else { percent };
        self.exchange(CMD_BRIGHTNESS, &[value])?;
        Ok(true)
    }
}

/// Identifiant stable : série USB, sinon identifiant d'instance Windows.
fn usb_tag(info: &nusb::DeviceInfo) -> String {
    match info.serial_number() {
        Some(s) if !s.is_empty() => s.to_string(),
        #[cfg(windows)]
        _ => info.instance_id().to_string_lossy().rsplit('\\').next().unwrap_or("0").to_string(),
        #[cfg(not(windows))]
        _ => "0".to_string(),
    }
}

/// Écran Lian Li resté en « mode bureau » (CH340 1A86:AD10..AD3F, ACD1, ACE1).
fn is_desktop_mode(vid: u16, pid: u16) -> bool {
    vid == DESKTOP_VID && ((0xAD10..=0xAD3F).contains(&pid) || matches!(pid, 0xACD1 | 0xACE1))
}

/// Détection des écrans TURZX / Turing USB et Lian Li 1CBE.
pub struct TurzxUsbProvider;

impl ScreenProvider for TurzxUsbProvider {
    fn name(&self) -> &'static str {
        "TURZX / Lian Li USB"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let list: Vec<_> = nusb::list_devices().map_err(|e| anyhow!("Énumération USB impossible : {e}"))?.collect();
        let desktop = list.iter().find(|d| is_desktop_mode(d.vendor_id(), d.product_id())).map(|d| d.product_id());
        let (mut found, mut first_err) = (Vec::<Box<dyn ScreenDriver>>::new(), None::<anyhow::Error>);
        for info in list.iter().filter(|d| d.vendor_id() == VID && model(d.product_id()).is_some()) {
            let pid = info.product_id();
            let id = format!("{VID:04x}:{pid:04x}:{}", usb_tag(info));
            let opened = NusbTransport::open(info, 0x01, 0x81, IO_TIMEOUT)
                .map_err(|e| anyhow!("{e:#} Fermez aussi L-Connect 3 ou le logiciel TURZX s'ils sont lancés."))
                .and_then(|t| TurzxScreen::connect(t, id, pid));
            match opened {
                Ok(s) => found.push(Box::new(s)),
                Err(e) => {
                    log::warn!("Écran TURZX 1CBE:{pid:04X} ignoré : {e:#}");
                    first_err.get_or_insert(e);
                }
            }
        }
        match (found.is_empty(), first_err, desktop) {
            (true, Some(e), _) => Err(e),
            (true, None, Some(pid)) => Err(anyhow!(
                "Un écran Lian Li est en mode bureau (1A86:{pid:04X}) : PureRGB ne sait pas encore le passer en mode LCD. \
                 Basculez-le en mode LCD depuis le logiciel du constructeur, puis relancez la détection."
            )),
            _ => Ok(found),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::{Log, MockTransport};
    use des::cipher::BlockDecrypt;
    use std::sync::{Arc, Mutex};

    fn des_cbc_decrypt(data: &[u8]) -> Vec<u8> {
        let cipher = Des::new(GenericArray::from_slice(&KEY));
        let mut prev = KEY;
        let mut out = Vec::new();
        for block in data.chunks_exact(8) {
            let mut ga = GenericArray::clone_from_slice(block);
            cipher.decrypt_block(&mut ga);
            out.extend(ga.iter().zip(prev).map(|(a, b)| a ^ b));
            prev.copy_from_slice(block);
        }
        out
    }

    fn ver_reply() -> Vec<u8> {
        let mut r = vec![0u8; 512];
        r[0] = 0x0A;
        r[8..12].copy_from_slice(b"V1.7");
        r
    }

    fn level_reply(level: u8) -> Vec<u8> {
        let mut r = vec![0u8; 16];
        r[8] = level;
        r
    }

    fn solid(w: u32, h: u32) -> Frame {
        Frame { width: w, height: h, rgba: [10u8, 120, 240, 255].repeat((w * h) as usize) }
    }

    fn screen(pid: u16, replies: Vec<Vec<u8>>) -> (TurzxScreen<MockTransport>, Arc<Mutex<Log>>) {
        let (t, log) = MockTransport::new(replies);
        (TurzxScreen::connect(t, "x".into(), pid).unwrap(), log)
    }

    #[test]
    fn des_known_answer() {
        // Vecteur DES classique (Stallings) : clé 133457799BBCDFF1, clair 0123456789ABCDEF -> 85E813540F0AB405.
        let c = des_cbc_encrypt(&[0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1], &[0; 8], &[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]).unwrap();
        assert_eq!(c, [0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05]);
        assert!(des_cbc_encrypt(&KEY, &KEY, &[0; 7]).is_err());
    }

    #[test]
    fn header_plaintext_and_trailer() {
        // crypto.rs : [0]=cmd, [2]=1A, [3]=6D, [4..8]=horloge LE (100001 = 0x000186A1), [8..12]=taille BE (0x00012345).
        let h = build_header(0x65, &0x0001_2345u32.to_be_bytes(), 100_001, Padding::Pkcs7).unwrap();
        assert_eq!(h[504..], [0, 0, 0, 0, 0, 0, 0xA1, 0x1A]);
        let plain = des_cbc_decrypt(&h[..504]);
        assert_eq!(plain[..12], [0x65, 0x00, 0x1A, 0x6D, 0xA1, 0x86, 0x01, 0x00, 0x00, 0x01, 0x23, 0x45]);
        assert!(plain[12..500].iter().all(|&b| b == 0));
        assert_eq!(plain[500..], [4, 4, 4, 4]); // PKCS#7 : 4 octets de valeur 4
        let z = build_header(0x65, &0x0001_2345u32.to_be_bytes(), 100_001, Padding::Zero).unwrap();
        assert_eq!(des_cbc_decrypt(&z[..504])[500..], [0, 0, 0, 0]); // Python : zéros
        assert_eq!(h[..496], z[..496]); // CBC : seuls les 8 derniers octets chiffrés diffèrent
        assert_ne!(h[496..504], z[496..504]);
        assert!(build_header(1, &[0u8; 493], 0, Padding::Zero).is_err());
    }

    #[test]
    fn catalog_matches_the_documented_resolutions() {
        let dims = |p| model(p).map(|m| (m.width, m.height));
        assert_eq!(dims(0x0028), Some((480, 480)));
        assert_eq!(dims(0x0046), Some((320, 960)));
        assert_eq!(dims(0x0050), Some((720, 1280)));
        assert_eq!(dims(0x0080), Some((800, 1280)));
        assert_eq!(dims(0x0088), Some((480, 1920)));
        assert_eq!(dims(0x0092), Some((462, 1920)));
        assert_eq!(dims(0x0123), Some((720, 1920)));
        assert_eq!(dims(0xA021), Some((480, 480)));
        assert_eq!(dims(0xA065), Some((720, 1472)));
        assert_eq!(dims(0xA092), Some((464, 1920)));
        assert_eq!(dims(0xA018), Some((400, 400)));
        assert_eq!(dims(0xA068), None); // OLED Curve : non géré
    }

    #[test]
    fn get_ver_parsing_is_strict() {
        assert_eq!(parse_get_ver(&ver_reply()).unwrap(), "V1.7");
        assert!(parse_get_ver(&[]).is_err());
        assert!(parse_get_ver(&[0x0B; 20]).is_err());
        let mut r = ver_reply();
        r[8] = 0x01; // octet de contrôle
        assert!(parse_get_ver(&r).is_err());
        let mut r = ver_reply();
        r[8..12].fill(0);
        assert!(parse_get_ver(&r).is_err());
    }

    #[test]
    fn connect_runs_stop_play_then_get_ver_for_lian_li() {
        let (s, log) = screen(0xA088, vec![vec![0x7B], ver_reply()]);
        assert_eq!(s.firmware(), Some("V1.7"));
        assert_eq!((s.info().width, s.info().height, s.info().vendor.as_str()), (480, 1920, "Lian Li"));
        assert!(!s.info().verified && !s.info().resolution_confirmed);
        let l = log.lock().unwrap();
        assert_eq!(l.writes.len(), 2);
        assert!(l.writes.iter().all(|w| w.len() == 512 && w[510..] == [0xA1, 0x1A]));
        assert_eq!(des_cbc_decrypt(&l.writes[0][..504])[0], 0x7B);
        assert_eq!(des_cbc_decrypt(&l.writes[1][..504])[0], 0x0A);
    }

    #[test]
    fn connect_rejects_bad_get_ver_except_where_documented_optional() {
        let (t, _) = MockTransport::new(vec![vec![0x7B], vec![0x55; 20]]);
        let e = TurzxScreen::connect(t, "x".into(), 0xA088).err().unwrap().to_string();
        assert!(e.contains("L-Connect 3"), "{e}");
        let (t, _) = MockTransport::new(vec![]);
        assert!(TurzxScreen::connect(t, "x".into(), 0x0088).is_err());
        // H2 : GetVer sans réponse toléré (commentaire de h2_control_init).
        let (t, _) = MockTransport::new(vec![]);
        assert!(TurzxScreen::connect(t, "x".into(), 0xA021).is_ok());
        let (t, _) = MockTransport::new(vec![]);
        assert!(TurzxScreen::connect(t, "x".into(), 0x7777).is_err());
    }

    #[test]
    fn push_frame_writes_header_and_jpeg_in_one_transfer() {
        let (mut s, log) = screen(0xA018, vec![ver_reply(), level_reply(0)]);
        s.push_frame(&solid(400, 400)).unwrap();
        let l = log.lock().unwrap();
        let w = &l.writes[1]; // A018 : pas de StopPlay, écriture 0 = GetVer
        let plain = des_cbc_decrypt(&w[..504]);
        assert_eq!(plain[0], 0x65);
        let size = u32::from_be_bytes([plain[8], plain[9], plain[10], plain[11]]) as usize;
        assert_eq!(w.len(), 512 + size);
        assert_eq!(w[512..514], [0xFF, 0xD8]);
        assert_eq!(w[w.len() - 2..], [0xFF, 0xD9]);
    }

    #[test]
    fn push_frame_validates_input_and_transport() {
        let (mut s, _) = screen(0xA018, vec![ver_reply()]);
        assert!(s.push_frame(&solid(100, 100)).is_err());
        let mut bad = solid(400, 400);
        bad.rgba.pop();
        assert!(s.push_frame(&bad).is_err());
        // Aucune réponse préparée : l'absence d'acquittement est une erreur.
        assert!(s.push_frame(&solid(400, 400)).is_err());
        let (mut s, _) = screen(0xA018, vec![ver_reply(), vec![1]]);
        s.transport.short_write = Some(100);
        assert!(s.push_frame(&solid(400, 400)).unwrap_err().to_string().contains("courte"));
        let (mut s, _) = screen(0xA018, vec![ver_reply(), vec![]]);
        assert!(s.push_frame(&solid(400, 400)).unwrap_err().to_string().contains("vide"));
    }

    #[test]
    fn full_buffer_triggers_query_block_polling() {
        let (mut s, log) = screen(0xA018, vec![ver_reply(), level_reply(9), level_reply(9), level_reply(1)]);
        s.push_frame(&solid(400, 400)).unwrap();
        let l = log.lock().unwrap();
        assert_eq!(l.writes.len(), 4); // GetVer, image, 2 x QueryBlock
        assert_eq!(des_cbc_decrypt(&l.writes[2][..504])[0], 0x7A);
        drop(l);
        let mut replies = vec![ver_reply()];
        replies.extend((0..25).map(|_| level_reply(9)));
        let (mut s, _) = screen(0xA018, replies);
        assert!(s.push_frame(&solid(400, 400)).unwrap_err().to_string().contains("saturé"));
    }

    #[test]
    fn brightness_scales_per_family() {
        let (mut s, log) = screen(0xA018, vec![ver_reply(), vec![1]]);
        assert!(s.set_brightness(50).unwrap());
        assert!(s.set_brightness(101).is_err());
        assert_eq!(des_cbc_decrypt(&log.lock().unwrap().writes[1][..504])[8], 50);
        // Turing : int(50 / 100 * 102) = 51 ; 100 -> 102 ; 33 -> 33.
        let (mut t, log) = screen(0x0088, vec![ver_reply(), vec![1], vec![1], vec![1]]);
        t.set_brightness(50).unwrap();
        t.set_brightness(100).unwrap();
        t.set_brightness(33).unwrap();
        let l = log.lock().unwrap();
        let vals: Vec<u8> = l.writes[1..].iter().map(|w| des_cbc_decrypt(&w[..504])[8]).collect();
        assert_eq!(vals, [51, 102, 33]);
    }

    #[test]
    fn timestamps_strictly_increase() {
        let (mut s, _) = screen(0xA018, vec![ver_reply()]);
        let (a, b) = (s.next_ts(), s.next_ts());
        assert!(b > a);
    }

    #[test]
    fn desktop_mode_ids_are_recognised() {
        assert!(is_desktop_mode(0x1A86, 0xAD20) && is_desktop_mode(0x1A86, 0xACD1) && is_desktop_mode(0x1A86, 0xACE1));
        assert!(!is_desktop_mode(0x1A86, 0x7523) && !is_desktop_mode(0x1CBE, 0xAD20));
    }
}
