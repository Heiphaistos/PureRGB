//! Pilote Thermaltake LCD (VID 264A) : AIO LCD rond 480x480 (264A:2328), RC Pro 480x128 (264A:232A)
//! et TH420 V2 Ultra EX 480x480 (264A:233C). JPEG sur HID, sans chiffrement.
//!
//! Sources (DONNEES relevées, aucun code copié ; lus hors dépôt) :
//! - `pcmx1/thermaltake-lcd-linux` @9f1423d (MIT) : README « Chunk header format » et « init sequence »,
//!   `tt-lcd-aio.py` / `tt-lcd-rc-pro.py` (charges des feature reports, en-tête de morceau). Format « Legacy ».
//! - `messiahlap/th420-display` @6999fcc (sans licence déclarée : on ne reprend que les OCTETS du protocole
//!   décrits dans son README et `src/device.rs`, pas une ligne de code). Format « TH420 ».
//!
//! Tout est issu de deux projets d'un seul auteur chacun, jamais recoupés : `verified: false` partout.
//! Les contenus de réponse de l'appareil sont INCONNU (aucune doc) : on exige seulement une réponse non vide
//! (et l'identifiant de rapport attendu pour les feature reports Legacy).
//! Non gérés (INCONNU) : Tower 500 (264A:233D), luminosité, rotation matérielle, lecture de température.
//!
//! L'appareil n'est PAS ouvert à la détection : l'ouverture se fait au premier envoi (et après une erreur),
//! car l'init du RC Pro doit être suivie immédiatement des morceaux sur le même handle.

use super::encode;
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Result};
use std::time::Duration;

const VID: u16 = 0x264A;
const LEGACY_CHUNK: usize = 1016;
const LEGACY_PACKET: usize = LEGACY_CHUNK + 8;
const TH420_CTRL: usize = 440;
const TH420_IMG_PACKET: usize = 1024;
const TH420_IMG_DATA: usize = 1020;
/// Premier octet d'index = nombre total de morceaux sur 1 octet (README th420-display) : 255 max.
const TH420_MAX_CHUNKS: usize = 255;
const MAX_JPEG: usize = 1 << 20;

/// Famille de trame selon le PID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// 264A:2328, AIO rond 480x480 (feature reports + morceaux de 1016 o).
    Aio,
    /// 264A:232A, RC Pro 480x128 (idem + `18` et `1D`).
    RcPro,
    /// 264A:233C, TH420 V2 Ultra EX 480x480 (deux interfaces HID, paquets de 1024 o).
    Th420,
}

impl Model {
    pub fn from_pid(pid: u16) -> Option<Model> {
        match pid {
            0x2328 => Some(Model::Aio),
            0x232A => Some(Model::RcPro),
            0x233C => Some(Model::Th420),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Model::Aio => "AIO LCD (rond)",
            Model::RcPro => "RC Pro (Tower 500/900)",
            Model::Th420 => "TH420 V2 Ultra EX",
        }
    }

    pub fn size(self) -> (u32, u32) {
        match self {
            Model::Aio | Model::Th420 => (480, 480),
            Model::RcPro => (480, 128),
        }
    }
}

// ---------------------------------------------------------------------------
// Trames (fonctions pures, testées octet par octet)
// ---------------------------------------------------------------------------

/// Feature report Legacy : identifiant 0x03, commande, 62 octets de charge, complété à 64.
fn feature(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; 64];
    p[0] = 0x03;
    p[1] = cmd;
    p[2..2 + payload.len()].copy_from_slice(payload);
    p
}

/// `0C` de configuration : 64 00 00 00 00, largeur LE, hauteur LE, 04 05 (README : 480x480 = `e001 e001`,
/// RC Pro 480x128 = `e001 8000`). Le sens de `64`, `04 05` est INCONNU (recopié).
pub fn cfg_dims(w: u16, h: u16) -> Vec<u8> {
    let mut pl = vec![0x64, 0, 0, 0, 0];
    pl.extend_from_slice(&w.to_le_bytes());
    pl.extend_from_slice(&h.to_le_bytes());
    pl.extend_from_slice(&[0x04, 0x05]);
    feature(0x0C, &pl)
}

/// `0C` de début de trame suivante : 64 FF FF EA FF FF FF (sens INCONNU, recopié).
pub fn cfg_next() -> Vec<u8> {
    feature(0x0C, &[0x64, 0xFF, 0xFF, 0xEA, 0xFF, 0xFF, 0xFF])
}

/// `1D` (RC Pro) : 00 FF FF EA FF FF FF.
pub fn cmd_1d() -> Vec<u8> {
    feature(0x1D, &[0x00, 0xFF, 0xFF, 0xEA, 0xFF, 0xFF, 0xFF])
}

/// Découpe Legacy : en-tête `02 09 00 <fin> <taille LE16> <index LE16>` + 1016 o (dernier complété de zéros).
/// Normal : fin=0, taille=1016 ; dernier : fin=1, taille=octets utiles restants.
pub fn legacy_chunks(jpeg: &[u8]) -> Result<Vec<Vec<u8>>> {
    if jpeg.is_empty() || jpeg.len() > MAX_JPEG {
        bail!("Taille de JPEG invalide : {} octets.", jpeg.len());
    }
    let total = jpeg.len().div_ceil(LEGACY_CHUNK);
    let mut out = Vec::with_capacity(total);
    for (i, part) in jpeg.chunks(LEGACY_CHUNK).enumerate() {
        let last = i == total - 1;
        let idx = u16::try_from(i)?;
        let size = if last { part.len() } else { LEGACY_CHUNK } as u16;
        let mut p = vec![0u8; LEGACY_PACKET];
        p[..4].copy_from_slice(&[0x02, 0x09, 0x00, u8::from(last)]);
        p[4..6].copy_from_slice(&size.to_le_bytes());
        p[6..8].copy_from_slice(&idx.to_le_bytes());
        p[8..8 + part.len()].copy_from_slice(part);
        out.push(p);
    }
    Ok(out)
}

/// Commande de contrôle TH420 : 440 octets, zéros après la commande.
pub fn th420_ctrl(cmd: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; TH420_CTRL];
    p[..cmd.len()].copy_from_slice(cmd);
    p
}

/// Séquence d'init TH420 (README th420-display / device.rs) : 85 87 85 87 84 81, chacune `<op> 01 00 80`.
pub const TH420_INIT_OPS: [u8; 6] = [0x85, 0x87, 0x85, 0x87, 0x84, 0x81];
/// Début de trame TH420 : `12 01 00 80 64` (sens de 0x64 INCONNU).
pub const TH420_FRAME_START: [u8; 5] = [0x12, 0x01, 0x00, 0x80, 0x64];

/// Découpe TH420 : paquets de 1024 o `08 <n> 00 <80|00>` + 1020 o ; premier paquet : n = nombre total
/// de morceaux et drapeau 0x80 ; les suivants : n = index, drapeau 0.
pub fn th420_chunks(jpeg: &[u8]) -> Result<Vec<Vec<u8>>> {
    if jpeg.is_empty() {
        bail!("JPEG vide.");
    }
    let total = jpeg.len().div_ceil(TH420_IMG_DATA);
    if total > TH420_MAX_CHUNKS {
        bail!("JPEG trop gros pour le TH420 : {total} morceaux (maximum {TH420_MAX_CHUNKS}).");
    }
    Ok(jpeg
        .chunks(TH420_IMG_DATA)
        .enumerate()
        .map(|(i, part)| {
            let mut p = vec![0u8; TH420_IMG_PACKET];
            p[0] = 0x08;
            p[1] = if i == 0 { total as u8 } else { i as u8 };
            p[3] = if i == 0 { 0x80 } else { 0x00 };
            p[4..4 + part.len()].copy_from_slice(part);
            p
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Accès HID (trait mockable)
// ---------------------------------------------------------------------------

/// Un handle HID : rapports de sortie, d'entrée et feature reports. Le premier octet des tampons
/// est l'identifiant de rapport (convention hidapi).
pub trait TtHid: Send {
    fn send_feature(&mut self, data: &[u8]) -> Result<()>;
    fn get_feature(&mut self, id: u8, len: usize) -> Result<Vec<u8>>;
    fn write_output(&mut self, data: &[u8]) -> Result<usize>;
    fn read_input(&mut self, len: usize) -> Result<Vec<u8>>;
}

struct RealHid {
    dev: hidapi::HidDevice,
}

impl TtHid for RealHid {
    fn send_feature(&mut self, data: &[u8]) -> Result<()> {
        self.dev.send_feature_report(data).map_err(|e| anyhow!("Feature report HID refusé : {e}"))
    }

    fn get_feature(&mut self, id: u8, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        buf[0] = id;
        let n = self.dev.get_feature_report(&mut buf).map_err(|e| anyhow!("Lecture de feature report HID échouée : {e}"))?;
        buf.truncate(n);
        Ok(buf)
    }

    fn write_output(&mut self, data: &[u8]) -> Result<usize> {
        self.dev.write(data).map_err(|e| anyhow!("Écriture HID échouée : {e}"))
    }

    fn read_input(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let n = self.dev.read_timeout(&mut buf, 1000).map_err(|e| anyhow!("Lecture HID échouée : {e}"))?;
        if n == 0 {
            bail!("Aucune réponse de l'écran Thermaltake (délai dépassé).");
        }
        buf.truncate(n);
        Ok(buf)
    }
}

fn write_all(h: &mut dyn TtHid, data: &[u8]) -> Result<()> {
    // Windows compte parfois l'octet d'identifiant : on exige au moins `len - 1`... sauf écriture courte franche.
    let n = h.write_output(data)?;
    if n < data.len().saturating_sub(1) || n == 0 {
        bail!("Écriture HID courte ({n}/{} octets) : l'écran n'a pas reçu toute l'image.", data.len());
    }
    Ok(())
}

/// Session ouverte : une ou deux interfaces HID selon le modèle.
pub enum Session {
    Legacy { hid: Box<dyn TtHid>, initialised: bool },
    Th420 { ctrl: Box<dyn TtHid>, image: Box<dyn TtHid>, initialised: bool },
}

fn expect_feature(reply: &[u8], id: u8) -> Result<()> {
    if reply.len() < 2 || reply[0] != id {
        bail!("Réponse inattendue de l'écran Thermaltake (feature report {id:#04X}).");
    }
    Ok(())
}

fn legacy_init(h: &mut dyn TtHid, model: Model) -> Result<()> {
    let (w, hh) = model.size();
    if model == Model::RcPro {
        h.send_feature(&feature(0x18, &[]))?;
    }
    h.send_feature(&feature(0x1A, &[]))?;
    // Les deux lectures sont tolérantes dans la source (try/except) : un échec de lecture est ignoré,
    // une réponse présente doit en revanche porter le bon identifiant.
    if let Ok(r) = h.get_feature(0x07, 64) {
        expect_feature(&r, 0x07)?;
    }
    let cfg = cfg_dims(u16::try_from(w)?, u16::try_from(hh)?);
    for _ in 0..if model == Model::RcPro { 4 } else { 3 } {
        h.send_feature(&cfg)?;
    }
    if let Ok(r) = h.get_feature(0x0F, 64) {
        expect_feature(&r, 0x0F)?;
    }
    if model == Model::RcPro {
        h.send_feature(&cmd_1d())?; // les morceaux doivent suivre IMMEDIATEMENT sur le même handle
    }
    Ok(())
}

fn th420_cmd(ctrl: &mut dyn TtHid, cmd: &[u8], reply_required: bool) -> Result<()> {
    write_all(ctrl, &th420_ctrl(cmd))?;
    match ctrl.read_input(TH420_CTRL) {
        Ok(r) if r.is_empty() => bail!("Réponse vide du contrôleur TH420."),
        Ok(_) => Ok(()),
        Err(e) if reply_required => Err(e),
        Err(_) => Ok(()), // l'acquittement du début de trame est optionnel (device.rs)
    }
}

impl Session {
    fn send(&mut self, model: Model, jpeg: &[u8]) -> Result<()> {
        match self {
            Session::Legacy { hid, initialised } => {
                let chunks = legacy_chunks(jpeg)?;
                if *initialised {
                    if let Ok(r) = hid.get_feature(0x0F, 64) {
                        expect_feature(&r, 0x0F)?;
                    }
                    hid.send_feature(&cfg_next())?;
                    std::thread::sleep(Duration::from_millis(20));
                } else {
                    legacy_init(hid.as_mut(), model)?;
                    *initialised = true;
                }
                chunks.iter().try_for_each(|c| write_all(hid.as_mut(), c))
            }
            Session::Th420 { ctrl, image, initialised } => {
                let chunks = th420_chunks(jpeg)?;
                if !*initialised {
                    for op in TH420_INIT_OPS {
                        th420_cmd(ctrl.as_mut(), &[op, 0x01, 0x00, 0x80], true)?;
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    *initialised = true;
                }
                th420_cmd(ctrl.as_mut(), &TH420_FRAME_START, false)?;
                chunks.iter().try_for_each(|c| write_all(image.as_mut(), c))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Écran
// ---------------------------------------------------------------------------

type Opener = Box<dyn FnMut() -> Result<Session> + Send>;

pub struct ThermaltakeScreen {
    info: ScreenInfo,
    model: Model,
    session: Option<Session>,
    open: Opener,
}

impl ThermaltakeScreen {
    pub fn new(id: String, model: Model, open: Opener) -> Self {
        let (width, height) = model.size();
        let info = ScreenInfo {
            id,
            vendor: "Thermaltake".into(),
            model: model.name().into(),
            protocol: Protocol::ThermaltakeLcd,
            width,
            height,
            rotation: 0, // INCONNU : le montage inversé de l'AIO du dépôt source est propre à son auteur
            resolution_confirmed: false,
            verified: false,
        };
        ThermaltakeScreen { info, model, session: None, open }
    }
}

impl ScreenDriver for ThermaltakeScreen {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        if !frame.is_valid() || (frame.width, frame.height) != (self.info.width, self.info.height) {
            bail!("L'image fait {}x{} mais l'écran attend {}x{}.", frame.width, frame.height, self.info.width, self.info.height);
        }
        let (rgba, w, h) = encode::rotate_rgba(&frame.rgba, frame.width, frame.height, self.info.rotation)?;
        // Qualité décroissante jusqu'à tenir dans la limite du format (TH420 : 255 morceaux).
        let limit = if self.model == Model::Th420 { TH420_MAX_CHUNKS * TH420_IMG_DATA } else { MAX_JPEG };
        let mut jpeg = Vec::new();
        for q in [90u8, 75, 60, 40, 25] {
            jpeg = encode::rgba_to_jpeg(&rgba, w, h, q)?;
            if jpeg.len() <= limit {
                break;
            }
        }
        if jpeg.len() > limit {
            bail!("Image JPEG trop lourde ({} octets) même à qualité réduite.", jpeg.len());
        }
        if self.session.is_none() {
            self.session = Some((self.open)()?);
        }
        let model = self.model;
        let res = self.session.as_mut().map_or(Ok(()), |s| s.send(model, &jpeg));
        if res.is_err() {
            self.session = None; // réouverture + ré-initialisation au prochain envoi
        }
        res
    }
}

// ---------------------------------------------------------------------------
// Détection
// ---------------------------------------------------------------------------

const BUSY_HINT: &str = "Fermez TT RGB PLUS (ou TT PlayLink) puis rebranchez l'écran.";

fn open_path(api: &hidapi::HidApi, path: &std::ffi::CStr) -> Result<Box<dyn TtHid>> {
    api.open_path(path)
        .map(|dev| Box::new(RealHid { dev }) as Box<dyn TtHid>)
        .map_err(|e| anyhow!("Impossible d'ouvrir l'écran Thermaltake ({e}). {BUSY_HINT}"))
}

pub struct ThermaltakeProvider;

impl ScreenProvider for ThermaltakeProvider {
    fn name(&self) -> &'static str {
        "Thermaltake LCD"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
        // (pid, série, [chemin interface 0, chemin interface 1]) ; énumération seule, aucun handle.
        let mut found: Vec<(u16, String, [Option<std::ffi::CString>; 2])> = Vec::new();
        for d in api.device_list().filter(|d| d.vendor_id() == VID && Model::from_pid(d.product_id()).is_some()) {
            let serial = d.serial_number().unwrap_or("").to_string();
            let slot = usize::try_from(d.interface_number()).ok().filter(|&n| n < 2).unwrap_or(0);
            let pos = found.iter().position(|(p, s, paths)| *p == d.product_id() && *s == serial && paths[slot].is_none());
            let entry = match pos {
                Some(i) => &mut found[i],
                None => {
                    found.push((d.product_id(), serial, [None, None]));
                    let last = found.len() - 1;
                    &mut found[last]
                }
            };
            entry.2[slot] = Some(d.path().to_owned());
        }
        let mut out: Vec<Box<dyn ScreenDriver>> = Vec::new();
        for (pid, serial, paths) in found {
            let Some(model) = Model::from_pid(pid) else { continue };
            let [p0, p1] = paths;
            let Some(p0) = p0 else { continue };
            if model == Model::Th420 && p1.is_none() {
                log::warn!("TH420 {VID:04X}:{pid:04X} : interface image (:1.1) absente, écran ignoré.");
                continue;
            }
            let tag = if serial.is_empty() { p0.to_string_lossy().into_owned() } else { serial };
            let id = format!("{VID:04x}:{pid:04x}:{tag}");
            let opener: Opener = Box::new(move || {
                let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
                if model == Model::Th420 {
                    let image = p1.as_deref().ok_or_else(|| anyhow!("Interface image du TH420 absente."))?;
                    Ok(Session::Th420 { ctrl: open_path(&api, &p0)?, image: open_path(&api, image)?, initialised: false })
                } else {
                    Ok(Session::Legacy { hid: open_path(&api, &p0)?, initialised: false })
                }
            });
            out.push(Box::new(ThermaltakeScreen::new(id, model, opener)));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, PartialEq)]
    enum Ev {
        Feature(Vec<u8>),
        GetFeature(u8),
        Write(Vec<u8>),
        Read,
    }

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<Ev>>>);

    struct Fake {
        log: Shared,
        feat_reply: bool,
        inputs: VecDeque<Result<Vec<u8>>>,
    }

    impl Fake {
        fn new(log: &Shared) -> Self {
            Fake { log: log.clone(), feat_reply: true, inputs: VecDeque::new() }
        }
        fn push(&self, e: Ev) {
            self.log.0.lock().unwrap().push(e);
        }
    }

    impl TtHid for Fake {
        fn send_feature(&mut self, d: &[u8]) -> Result<()> {
            self.push(Ev::Feature(d.to_vec()));
            Ok(())
        }
        fn get_feature(&mut self, id: u8, len: usize) -> Result<Vec<u8>> {
            self.push(Ev::GetFeature(id));
            if !self.feat_reply {
                bail!("pas de réponse");
            }
            let mut v = vec![0u8; len];
            v[0] = id;
            Ok(v)
        }
        fn write_output(&mut self, d: &[u8]) -> Result<usize> {
            self.push(Ev::Write(d.to_vec()));
            Ok(d.len())
        }
        fn read_input(&mut self, _len: usize) -> Result<Vec<u8>> {
            self.push(Ev::Read);
            self.inputs.pop_front().unwrap_or_else(|| Ok(vec![1; 8]))
        }
    }

    fn solid(w: u32, h: u32) -> Frame {
        Frame { width: w, height: h, rgba: [30u8, 60, 90, 255].repeat((w * h) as usize) }
    }

    #[test]
    fn feature_payloads_match_documented_hex() {
        // README/tt-lcd-aio.py : CMD_1A = '031a' + 62 zéros ; CMD_0C_480 = '030c6400000000e001e0010405000000'+48 zéros.
        let mut e1a = vec![0x03, 0x1A];
        e1a.resize(64, 0);
        assert_eq!(feature(0x1A, &[]), e1a);
        let mut dims = vec![0x03, 0x0C, 0x64, 0, 0, 0, 0, 0xE0, 0x01, 0xE0, 0x01, 0x04, 0x05, 0, 0, 0];
        dims.resize(64, 0);
        assert_eq!(cfg_dims(480, 480), dims);
        // RC Pro : '030c6400000000e00180000405000000'.
        assert_eq!(cfg_dims(480, 128)[..16], [0x03, 0x0C, 0x64, 0, 0, 0, 0, 0xE0, 0x01, 0x80, 0x00, 0x04, 0x05, 0, 0, 0]);
        // '030c64ffffeaffffff' ; '031d00ffffeaffffff'.
        assert_eq!(cfg_next()[..9], [0x03, 0x0C, 0x64, 0xFF, 0xFF, 0xEA, 0xFF, 0xFF, 0xFF]);
        assert_eq!(cmd_1d()[..9], [0x03, 0x1D, 0x00, 0xFF, 0xFF, 0xEA, 0xFF, 0xFF, 0xFF]);
        assert!(cfg_next().len() == 64 && cmd_1d().len() == 64);
    }

    #[test]
    fn legacy_chunk_headers() {
        // README : normal = 02 09 00 00 F8 03 idx ; dernier = 02 09 00 01 <reste> idx.
        let jpeg: Vec<u8> = (0..2100u32).map(|i| (i % 251) as u8 + 1).collect(); // 1016 + 1016 + 68
        let c = legacy_chunks(&jpeg).unwrap();
        assert_eq!(c.len(), 3);
        assert!(c.iter().all(|p| p.len() == 1024));
        assert_eq!(c[0][..8], [0x02, 0x09, 0x00, 0x00, 0xF8, 0x03, 0x00, 0x00]);
        assert_eq!(c[1][..8], [0x02, 0x09, 0x00, 0x00, 0xF8, 0x03, 0x01, 0x00]);
        assert_eq!(c[2][..8], [0x02, 0x09, 0x00, 0x01, 68, 0x00, 0x02, 0x00]);
        assert_eq!(c[2][8 + 68], 0); // zéros de bourrage
        assert_eq!(c[1][8], jpeg[1016]);
        // Taille multiple exact : le dernier annonce 1016.
        let c = legacy_chunks(&vec![7u8; 2032]).unwrap();
        assert_eq!((c.len(), c[1][3], c[1][4..6].to_vec()), (2, 1, vec![0xF8, 0x03]));
        assert!(legacy_chunks(&[]).is_err());
    }

    #[test]
    fn th420_chunk_headers_and_limits() {
        // README th420-display : 08 <n> 00 80 + 1020 o ; premier = total, suivants = index.
        let c = th420_chunks(&vec![9u8; 2500]).unwrap(); // 1020+1020+460
        assert_eq!(c.len(), 3);
        assert!(c.iter().all(|p| p.len() == 1024));
        assert_eq!(c[0][..5], [0x08, 3, 0x00, 0x80, 9]);
        assert_eq!(c[1][..4], [0x08, 1, 0x00, 0x00]);
        assert_eq!(c[2][..4], [0x08, 2, 0x00, 0x00]);
        assert_eq!(c[2][4 + 459], 9);
        assert_eq!(c[2][4 + 460], 0);
        assert!(th420_chunks(&vec![1u8; 256 * 1020]).is_err());
        assert_eq!(th420_chunks(&vec![1u8; 255 * 1020]).unwrap().len(), 255);
        assert_eq!(th420_ctrl(&TH420_FRAME_START)[..6], [0x12, 0x01, 0x00, 0x80, 0x64, 0x00]);
        assert_eq!(th420_ctrl(&[0x85]).len(), 440);
    }

    fn legacy_screen(model: Model, log: &Shared) -> ThermaltakeScreen {
        let log = log.clone();
        ThermaltakeScreen::new(
            "t".into(),
            model,
            Box::new(move || Ok(Session::Legacy { hid: Box::new(Fake::new(&log)), initialised: false })),
        )
    }

    #[test]
    fn aio_init_sequence_then_next_frame_handshake() {
        let log = Shared::default();
        let mut s = legacy_screen(Model::Aio, &log);
        s.push_frame(&solid(480, 480)).unwrap();
        let ev = log.0.lock().unwrap();
        // AIO : 1A, GET 07, 0C x3, GET 0F, puis les morceaux (pas de 1D).
        assert_eq!(ev[0], Ev::Feature(feature(0x1A, &[])));
        assert_eq!(ev[1], Ev::GetFeature(0x07));
        assert!((2..5).all(|i| ev[i] == Ev::Feature(cfg_dims(480, 480))));
        assert_eq!(ev[5], Ev::GetFeature(0x0F));
        let Ev::Write(first) = &ev[6] else { panic!("morceau attendu, {:?}", ev[6]) };
        assert_eq!(first[..4], [0x02, 0x09, 0x00, 0x00]);
        let Some(Ev::Write(last)) = ev.last() else { panic!() };
        assert_eq!(last[3], 1); // drapeau « dernier »
        drop(ev);
        // 2e image sur la même session : GET 0F + 0C « next », sans ré-init.
        let n = log.0.lock().unwrap().len();
        s.push_frame(&solid(480, 480)).unwrap();
        let ev = log.0.lock().unwrap();
        assert_eq!(ev[n], Ev::GetFeature(0x0F));
        assert_eq!(ev[n + 1], Ev::Feature(cfg_next()));
        assert!(matches!(ev[n + 2], Ev::Write(_)));
    }

    #[test]
    fn rc_pro_init_has_18_four_0c_and_1d_right_before_chunks() {
        let log = Shared::default();
        let mut s = legacy_screen(Model::RcPro, &log);
        s.push_frame(&solid(480, 128)).unwrap();
        let ev = log.0.lock().unwrap();
        assert_eq!(ev[0], Ev::Feature(feature(0x18, &[])));
        assert_eq!(ev[1], Ev::Feature(feature(0x1A, &[])));
        assert_eq!(ev.iter().filter(|e| **e == Ev::Feature(cfg_dims(480, 128))).count(), 4);
        let i = ev.iter().position(|e| *e == Ev::Feature(cmd_1d())).unwrap();
        assert!(matches!(ev[i + 1], Ev::Write(_)));
    }

    #[test]
    fn bad_feature_reply_id_is_an_error_and_session_is_reset() {
        struct BadId;
        impl TtHid for BadId {
            fn send_feature(&mut self, _: &[u8]) -> Result<()> {
                Ok(())
            }
            fn get_feature(&mut self, _: u8, len: usize) -> Result<Vec<u8>> {
                Ok(vec![0x55; len])
            }
            fn write_output(&mut self, d: &[u8]) -> Result<usize> {
                Ok(d.len())
            }
            fn read_input(&mut self, _: usize) -> Result<Vec<u8>> {
                Ok(vec![1])
            }
        }
        let opens = Arc::new(Mutex::new(0));
        let o = opens.clone();
        let mut s = ThermaltakeScreen::new(
            "t".into(),
            Model::Aio,
            Box::new(move || {
                *o.lock().unwrap() += 1;
                Ok(Session::Legacy { hid: Box::new(BadId), initialised: false })
            }),
        );
        assert!(s.push_frame(&solid(480, 480)).is_err());
        assert!(s.push_frame(&solid(480, 480)).is_err());
        assert_eq!(*opens.lock().unwrap(), 2); // rouverte après l'erreur
    }

    fn th420_screen(log: &Shared, ctrl_inputs: Vec<Result<Vec<u8>>>) -> ThermaltakeScreen {
        let log = log.clone();
        let mut inputs = Some(ctrl_inputs);
        ThermaltakeScreen::new(
            "t".into(),
            Model::Th420,
            Box::new(move || {
                let mut ctrl = Fake::new(&log);
                ctrl.inputs = inputs.take().unwrap_or_default().into();
                Ok(Session::Th420 { ctrl: Box::new(ctrl), image: Box::new(Fake::new(&log)), initialised: false })
            }),
        )
    }

    #[test]
    fn th420_init_frame_start_and_packets() {
        let log = Shared::default();
        let mut s = th420_screen(&log, vec![]);
        s.push_frame(&solid(480, 480)).unwrap();
        let ev = log.0.lock().unwrap();
        let writes: Vec<&Vec<u8>> = ev.iter().filter_map(|e| if let Ev::Write(w) = e { Some(w) } else { None }).collect();
        let ctrl: Vec<&&Vec<u8>> = writes.iter().filter(|w| w.len() == 440).collect();
        let ops: Vec<u8> = ctrl.iter().map(|w| w[0]).collect();
        assert_eq!(ops, vec![0x85, 0x87, 0x85, 0x87, 0x84, 0x81, 0x12]);
        assert_eq!(ctrl[0][..4], [0x85, 0x01, 0x00, 0x80]);
        assert_eq!(ctrl[6][..5], TH420_FRAME_START);
        let imgs: Vec<&&Vec<u8>> = writes.iter().filter(|w| w.len() == 1024).collect();
        assert_eq!(imgs[0][0], 0x08);
        assert_eq!(imgs[0][1] as usize, imgs.len()); // premier paquet = nombre total
        assert_eq!(imgs[0][3], 0x80);
        assert_eq!(imgs[0][4..6], [0xFF, 0xD8]); // début du JPEG
        assert_eq!(imgs[1][..4], [0x08, 1, 0, 0]);
        // Aucune ré-init à la 2e image : seul `12` repart.
        let n = ev.len();
        drop(ev);
        s.push_frame(&solid(480, 480)).unwrap();
        let ev = log.0.lock().unwrap();
        let again: Vec<u8> = ev[n..].iter().filter_map(|e| if let Ev::Write(w) = e { Some(w) } else { None }).filter(|w| w.len() == 440).map(|w| w[0]).collect();
        assert_eq!(again, vec![0x12]);
    }

    #[test]
    fn th420_empty_init_reply_fails_but_missing_frame_ack_is_ok() {
        let log = Shared::default();
        let mut s = th420_screen(&log, vec![Ok(vec![])]);
        assert!(s.push_frame(&solid(480, 480)).is_err());
        let log = Shared::default();
        // 6 réponses d'init valides puis erreur sur l'ACK du début de trame : tolérée.
        let mut replies: Vec<Result<Vec<u8>>> = (0..6).map(|_| Ok(vec![1u8; 8])).collect();
        replies.push(Err(anyhow!("timeout")));
        let mut s = th420_screen(&log, replies);
        assert!(s.push_frame(&solid(480, 480)).is_ok());
        // Init : réponse manquante = erreur.
        let log = Shared::default();
        let mut s = th420_screen(&log, vec![Err(anyhow!("timeout"))]);
        assert!(s.push_frame(&solid(480, 480)).is_err());
    }

    #[test]
    fn wrong_frame_size_and_pid_mapping() {
        let log = Shared::default();
        let mut s = legacy_screen(Model::Aio, &log);
        assert!(s.push_frame(&solid(480, 128)).is_err());
        assert!(s.push_frame(&Frame { width: 480, height: 480, rgba: vec![0; 4] }).is_err());
        assert!(log.0.lock().unwrap().is_empty()); // rien n'est ouvert ni envoyé
        assert_eq!(Model::from_pid(0x233C), Some(Model::Th420));
        assert_eq!(Model::from_pid(0x233D), None); // Tower 500 : INCONNU
        assert_eq!(Model::RcPro.size(), (480, 128));
        assert!(!s.info().verified);
    }
}
