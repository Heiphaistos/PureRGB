//! Pilote ASUS ROG Ryujin III (VID 0B05 ; PID 1AA2 / 1BCB / 1ADA / 1ADE), écran 320x240.
//!
//! Source (DONNEES relevées, aucun code copié) : `zaclanzon/ryujin-iii-lcd-linux` @d52214e (MIT),
//! `docs/protocol.md` + `capture/` (capture USBPcap d'Armoury Crate, AacAIOFanHal 1.5.10.0, firmware
//! `AURJ2-S750-0108`, rejouée par l'auteur sur un Ryujin III 360). Fiche : `docs/screens-protocols/asus.md`.
//!
//! IMPLEMENTE (décrit « verified » dans la source) : lecture du firmware (`82`), luminosité du rétroéclairage
//! (`DC`/`5C`, octet 7), état du stockage (`71`/`F1`), envoi d'un JPEG dans un emplacement du stockage de
//! l'AIO (`72`/`73`/`7F` + bulk), suppression d'un emplacement, affichage d'un JPEG stocké (`60`/`51 1F`).
//!
//! NON IMPLEMENTE (INCONNU) : l'envoi d'images libres en temps réel. La source ne décrit PAS de flux vidéo ;
//! la seule voie vérifiée est l'écriture en flash du stockage (16 emplacements, ~0,5 s par opération, usure
//! flash non documentée). `push_frame` renvoie donc une erreur explicite. Les variantes White/Extreme/EVA et
//! la résolution réelle ne sont pas confirmées (`resolution_confirmed: false`). Ryujin II (0B05:1988) : exclu.
//!
//! Le périphérique est composite : interface 1 = HID (rapport EC de 65 o, réponses de 64 o), interface 0 =
//! bulk vendeur (EP 0x01 OUT) pour les données de fichier. Il n'est pas ouvert à la détection.

use super::transport::{NusbTransport, Transport};
use crate::screens::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::{anyhow, bail, Result};
use std::collections::VecDeque;
use std::time::Duration;

const VID: u16 = 0x0B05;
/// (PID, nom). Source : asus.md [V] (liquidctl, killryujin) ; White/Extreme/EVA = [S] même famille.
const MODELS: [(u16, &str); 4] =
    [(0x1AA2, "ROG Ryujin III 360"), (0x1BCB, "ROG Ryujin III Extreme"), (0x1ADA, "ROG Ryujin III White Edition"), (0x1ADE, "ROG Ryujin III EVA")];

const REPORT_ID: u8 = 0xEC;
const EVENT_ID: u8 = 0xEE;
const PACKET: usize = 65;
const REPLY: usize = 64;
const CHUNK: usize = 4096;
const MAX_SLOT: u8 = 9; // Armoury Crate utilise 0..9 (le firmware a 16 bits de bitmap)
/// Emplacement JPEG réservé aux images envoyées par PureRGB.
const PUREGB_SLOT: u8 = MAX_SLOT;
const MAX_JPEG: usize = 0xFFFF; // champ taille `7F 02` vérifié sur 2 octets seulement
const SLOT_COST_KB: u32 = 8; // « each stored file cost 8 KB »
const MAX_READS: usize = 16;
pub const SIZE: (u32, u32) = (320, 240);

/// Commande HID : `EC <cmd> <args…>` complétée de zéros à 65 octets.
pub fn command(cmd: u8, args: &[u8]) -> Result<Vec<u8>> {
    if args.len() + 2 > PACKET {
        bail!("Commande ASUS trop longue ({} octets).", args.len() + 2);
    }
    let mut p = vec![0u8; PACKET];
    p[0] = REPORT_ID;
    p[1] = cmd;
    p[2..2 + args.len()].copy_from_slice(args);
    Ok(p)
}

/// Commande « écrire l'état d'affichage » : recopie la réponse `DC` (octets 2..16), octet 2 = 01, octet 7 = luminosité.
/// Layout : protocol.md « Display status » (octet 7 = SEUL octet qui change le rétroéclairage).
pub fn status_write(reply: &[u8], percent: u8) -> Result<Vec<u8>> {
    if reply.len() < 16 || reply[0] != REPORT_ID || reply[1] != 0x5C {
        bail!("État d'affichage ASUS illisible (réponse inattendue).");
    }
    let mut args = reply[2..16].to_vec();
    args[0] = 0x01;
    args[5] = percent.min(100); // octet 7 absolu = indice 5 après les deux premiers octets
    command(0x5C, &args)
}

/// Taille en KB libre et bitmap des emplacements JPG, d'après la réponse `F1`.
/// Layout (protocol.md « GetDiskInfo ») : total u32 LE @4, libre u32 LE @8, 0x10 @12/@17/@22, bitmap JPG @18.
pub fn parse_disk_info(r: &[u8]) -> Result<(u32, u32, u32)> {
    if r.len() < 27 || r[0] != REPORT_ID || r[1] != 0x71 || r[2] != 0 || r[3] != 1 || r[12] != 0x10 || r[17] != 0x10 || r[22] != 0x10 {
        bail!("Table de stockage ASUS invalide (réponse inattendue).");
    }
    let u = |i: usize| u32::from_le_bytes([r[i], r[i + 1], r[i + 2], r[i + 3]]);
    Ok((u(4), u(8), u(18)))
}

/// Premier rapport `EC` du firmware : `EC 02 00 "AURJ…"` -> la chaîne de version.
pub fn parse_firmware(r: &[u8]) -> Result<String> {
    if r.len() < 4 || r[0] != REPORT_ID || r[1] != 0x02 {
        bail!("Réponse de version ASUS inattendue.");
    }
    let s: String = r[3..].iter().take_while(|&&b| b != 0).map(|&b| char::from(b)).collect();
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_graphic()) {
        bail!("Version de firmware ASUS illisible.");
    }
    Ok(s)
}

/// Canal HID avec mise en réserve des évènements `EE` arrivés avant d'être attendus.
/// Les autres rapports (réponses du pilote hwmon `19/1A/20/21`…) sont ignorés (protocol.md « Transport »).
pub struct Hid<'a> {
    t: &'a mut dyn Transport,
    stash: VecDeque<Vec<u8>>,
}

impl<'a> Hid<'a> {
    pub fn new(t: &'a mut dyn Transport) -> Self {
        Hid { t, stash: VecDeque::new() }
    }

    fn send(&mut self, cmd: u8, args: &[u8]) -> Result<()> {
        self.raw(&command(cmd, args)?)
    }

    fn raw(&mut self, packet: &[u8]) -> Result<()> {
        let n = self.t.write(packet)?;
        if n < packet.len().saturating_sub(1) || n == 0 {
            bail!("Écriture HID ASUS courte ({n}/{} octets).", packet.len());
        }
        Ok(())
    }

    /// Attend un rapport vérifiant `ok` ; ignore les autres (garde les évènements EE).
    fn wait(&mut self, what: &str, ok: impl Fn(&[u8]) -> bool) -> Result<Vec<u8>> {
        if let Some(i) = self.stash.iter().position(|r| ok(r)) {
            return self.stash.remove(i).ok_or_else(|| anyhow!("Rapport ASUS perdu."));
        }
        for _ in 0..MAX_READS {
            let r = self.t.read(REPLY)?;
            if ok(&r) {
                return Ok(r);
            }
            if r.first() == Some(&EVENT_ID) {
                self.stash.push_back(r);
            }
        }
        bail!("L'AIO ASUS n'a pas répondu ({what}). Fermez Armoury Crate / Aura / OpenRGB, puis réessayez ; si l'écran reste bloqué, coupez l'alimentation de l'AIO environ 30 s.")
    }

    fn reply(&mut self, cmd: u8, what: &str) -> Result<Vec<u8>> {
        self.wait(what, |r| r.len() >= 2 && r[0] == REPORT_ID && r[1] == cmd)
    }

    fn event(&mut self, kind: u8, b2: u8, b3: u8, what: &str) -> Result<()> {
        self.wait(what, |r| r.len() >= 4 && r[0] == EVENT_ID && r[1] == kind && r[2] == b2 && r[3] == b3).map(|_| ())
    }

    /// Commande puis attente de son écho `EC <cmd>`.
    fn call(&mut self, cmd: u8, args: &[u8], what: &str) -> Result<Vec<u8>> {
        self.send(cmd, args)?;
        self.reply(cmd, what)
    }

    /// Version du firmware (`82` -> `EC 02`).
    pub fn firmware(&mut self) -> Result<String> {
        self.send(0x82, &[])?;
        parse_firmware(&self.wait("version", |r| r.len() >= 2 && r[0] == REPORT_ID && r[1] == 0x02)?)
    }

    fn display_status(&mut self) -> Result<Vec<u8>> {
        self.send(0xDC, &[])?;
        self.reply(0x5C, "état d'affichage")
    }

    /// Luminosité du rétroéclairage 0-100 : Get (`DC`), Set (`5C`, octet 2 = 1, octet 7), Get de contrôle.
    pub fn set_brightness(&mut self, percent: u8) -> Result<()> {
        let p = percent.min(100);
        let st = self.display_status()?;
        self.raw(&status_write(&st, p)?)?;
        self.reply(0x5C, "écriture de la luminosité")?;
        let after = self.display_status()?;
        if after.len() < 8 || after[7] != p {
            bail!("L'AIO ASUS n'a pas appliqué la luminosité demandée ({p}).");
        }
        Ok(())
    }

    /// (KB libres, bitmap des emplacements JPG occupés).
    pub fn storage(&mut self) -> Result<(u32, u32)> {
        self.call(0x71, &[0x01, 0x01], "mise à jour du stockage")?;
        self.event(0x12, 0x00, 0x00, "fin de mise à jour du stockage")?;
        self.send(0xF1, &[])?;
        let r = self.wait("table de stockage", |r| r.len() >= 4 && r[0] == REPORT_ID && r[1] == 0x71 && r[3] == 0x01)?;
        parse_disk_info(&r).map(|(_, free, jpg)| (free, jpg))
    }

    /// Affiche le JPEG stocké dans `slot` : fond `60 00 01 10 <slot>` puis mode diaporama `51 1F`.
    pub fn show_jpeg_slot(&mut self, slot: u8) -> Result<()> {
        check_slot(slot)?;
        self.call(0x60, &[0x00, 0x01, 0x10, slot], "fond d'écran")?;
        self.call(0x51, &[0x1F], "mode diaporama")?;
        Ok(())
    }

    /// Supprime le JPEG de `slot` (`72 01 01 <slot>` puis `73 03`, acquittement `EE 13 xx 03`).
    pub fn delete_jpeg_slot(&mut self, slot: u8) -> Result<()> {
        check_slot(slot)?;
        self.call(0x72, &[0x01, 0x01, slot], "sélection de l'emplacement")?;
        self.send(0x73, &[0x03])?;
        self.reply(0x73, "suppression")?;
        self.wait("acquittement de suppression", |r| r.len() >= 4 && r[0] == EVENT_ID && r[1] == 0x13 && r[3] == 0x03).map(|_| ())
    }

    /// Écrit `jpeg` (320x240 attendu, validé par l'appelant) dans l'emplacement libre `slot` via le bulk.
    /// ATTENTION : écriture en flash. Pas de nouvelle tentative en boucle (désynchronise la poignée de main).
    pub fn upload_jpeg(&mut self, bulk: &mut dyn Transport, slot: u8, jpeg: &[u8]) -> Result<()> {
        check_slot(slot)?;
        check_jpeg(jpeg)?;
        let (free_kb, used) = self.storage()?;
        if used & (1 << slot) != 0 {
            bail!("L'emplacement JPEG {slot} de l'AIO ASUS est déjà occupé : supprimez-le d'abord.");
        }
        if free_kb < SLOT_COST_KB {
            bail!("Stockage de l'AIO ASUS plein ({free_kb} Ko libres).");
        }
        self.call(0x72, &[0x01, 0x01, slot], "sélection de l'emplacement")?;
        self.call(0x73, &[0x01], "début d'écriture")?;
        let res = self.write_body(bulk, jpeg);
        if res.is_err() {
            // Fin d'écriture au mieux, sans attente ni nouvelle tentative (source : « tool retries once » à la main).
            let _ = self.send(0x73, &[0xFF]);
            return res;
        }
        self.call(0x73, &[0xFF], "fin d'écriture")?;
        self.event(0x13, 0x00, 0xFF, "fin d'écriture (évènement)")
    }

    fn write_body(&mut self, bulk: &mut dyn Transport, jpeg: &[u8]) -> Result<()> {
        self.event(0x13, 0x00, 0x01, "début d'écriture (évènement)")?;
        let size = u16::try_from(jpeg.len())?.to_le_bytes();
        let r = self.call(0x7F, &[0x02, size[0], size[1]], "annonce de taille")?;
        if r.len() < 5 || u16::from_le_bytes([r[3], r[4]]) as usize != CHUNK {
            bail!("Taille de bloc inattendue annoncée par l'AIO ASUS (attendu {CHUNK}).");
        }
        for part in jpeg.chunks(CHUNK) {
            let mut block = part.to_vec();
            block.resize(CHUNK, 0);
            if bulk.write(&block)? < CHUNK {
                bail!("Écriture bulk ASUS courte.");
            }
            self.event(0x14, 0x00, 0x00, "acquittement de bloc")?;
        }
        Ok(())
    }
}

fn check_slot(slot: u8) -> Result<()> {
    if slot > MAX_SLOT {
        bail!("Emplacement ASUS invalide : {slot} (0 à {MAX_SLOT}).");
    }
    Ok(())
}

fn check_jpeg(jpeg: &[u8]) -> Result<()> {
    if jpeg.len() < 4 || jpeg[..2] != [0xFF, 0xD8] || jpeg[jpeg.len() - 2..] != [0xFF, 0xD9] {
        bail!("Le fichier n'est pas un JPEG valide.");
    }
    if jpeg.len() > MAX_JPEG {
        bail!("JPEG trop gros pour l'AIO ASUS ({} octets, maximum {MAX_JPEG}).", jpeg.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Matériel réel
// ---------------------------------------------------------------------------

/// Transport HID : le premier octet écrit est déjà l'identifiant de rapport (0xEC).
struct RyujinHid(hidapi::HidDevice);

impl Transport for RyujinHid {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.0.write(data).map_err(|e| anyhow!("Écriture HID ASUS échouée : {e}"))
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let n = self.0.read_timeout(&mut buf, 1500).map_err(|e| anyhow!("Lecture HID ASUS échouée : {e}"))?;
        if n == 0 {
            bail!("Aucune réponse de l'AIO ASUS (délai dépassé).");
        }
        buf.truncate(n);
        Ok(buf)
    }
}

type Opener = Box<dyn FnMut() -> Result<Box<dyn Transport>> + Send>;

pub struct RyujinScreen {
    info: ScreenInfo,
    hid: Option<Box<dyn Transport>>,
    open: Opener,
}

impl RyujinScreen {
    pub fn new(id: String, model: &str, open: Opener) -> Self {
        let info = ScreenInfo {
            id,
            vendor: "ASUS".into(),
            model: model.into(),
            protocol: Protocol::AsusRyujin,
            width: SIZE.0,
            height: SIZE.1,
            rotation: 0,
            resolution_confirmed: false,
            verified: false,
        };
        RyujinScreen { info, hid: None, open }
    }

    fn with_hid<R>(&mut self, f: impl FnOnce(&mut Hid) -> Result<R>) -> Result<R> {
        if self.hid.is_none() {
            self.hid = Some((self.open)()?);
        }
        let Some(t) = self.hid.as_mut() else { bail!("Canal HID ASUS indisponible.") };
        let res = f(&mut Hid::new(t.as_mut()));
        if res.is_err() {
            self.hid = None; // réouverture au prochain appel
        }
        res
    }

    /// Version du firmware lue sur l'appareil.
    pub fn firmware(&mut self) -> Result<String> {
        self.with_hid(|h| h.firmware())
    }

    /// Envoie un JPEG 320x240 dans un emplacement libre puis l'affiche (écriture flash, usage ponctuel :
    /// fond d'écran fixe, jamais en boucle).
    pub fn upload_and_show_jpeg(&mut self, slot: u8, jpeg: &[u8]) -> Result<()> {
        let mut bulk = open_bulk()?;
        self.with_hid(|h| {
            h.upload_jpeg(&mut bulk, slot, jpeg)?;
            h.show_jpeg_slot(slot)
        })
    }
}

impl ScreenDriver for RyujinScreen {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, _frame: &Frame) -> Result<()> {
        // INCONNU : aucune source ne décrit un flux d'images en direct pour le Ryujin III.
        bail!(
            "Envoi d'image non validé pour l'AIO ASUS Ryujin III : le flux temps réel n'est pas documenté \
             (seuls l'affichage de capteurs et les images stockées en mémoire flash le sont)."
        )
    }

    fn set_brightness(&mut self, percent: u8) -> Result<bool> {
        self.with_hid(|h| h.set_brightness(percent))?;
        Ok(true)
    }

    fn live_frames(&self) -> bool {
        false
    }

    /// Image fixe dans l'emplacement réservé à PureRGB (le précédent contenu est remplacé).
    fn upload_still(&mut self, frame: &Frame) -> Result<()> {
        let mut quality = 90u8;
        let jpeg = loop {
            let j = super::encode::rgba_to_jpeg(&frame.rgba, frame.width, frame.height, quality)?;
            if j.len() <= MAX_JPEG || quality <= 30 {
                break j;
            }
            quality -= 10;
        };
        if jpeg.len() > MAX_JPEG {
            bail!("Image trop lourde pour la mémoire de l'AIO ASUS ({} octets, max {MAX_JPEG}).", jpeg.len());
        }
        // Emplacement vide : l'AIO peut refuser la suppression, sans conséquence.
        let _ = self.with_hid(|h| h.delete_jpeg_slot(PUREGB_SLOT));
        self.upload_and_show_jpeg(PUREGB_SLOT, &jpeg)
    }
}

/// Bulk OUT 0x01 / IN 0x81 de l'interface 0. Sur un périphérique composite le pilote lié peut ne pas être
/// WinUSB : le message d'erreur le dit (à valider sur matériel).
fn open_bulk() -> Result<NusbTransport> {
    let info = nusb::list_devices()
        .map_err(|e| anyhow!("Énumération USB impossible : {e}"))?
        .into_iter()
        .find(|d| d.vendor_id() == VID && MODELS.iter().any(|(p, _)| *p == d.product_id()))
        .ok_or_else(|| anyhow!("AIO ASUS introuvable sur le bus USB. Fermez Armoury Crate puis rebranchez l'AIO."))?;
    NusbTransport::open(&info, 0x01, 0x81, Duration::from_secs(5))
}

pub struct RyujinProvider;

impl ScreenProvider for RyujinProvider {
    fn name(&self) -> &'static str {
        "ASUS ROG Ryujin"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
        // Énumération seule : interface 1 (HID) de chaque AIO ; un seul chemin par (PID, série).
        let mut seen: Vec<(u16, String)> = Vec::new();
        let mut out: Vec<Box<dyn ScreenDriver>> = Vec::new();
        for d in api.device_list().filter(|d| d.vendor_id() == VID && d.interface_number() <= 1) {
            let Some((pid, name)) = MODELS.iter().find(|(p, _)| *p == d.product_id()).copied() else { continue };
            let serial = d.serial_number().unwrap_or("").to_string();
            if seen.contains(&(pid, serial.clone())) {
                continue;
            }
            seen.push((pid, serial.clone()));
            let path = d.path().to_owned();
            let tag = if serial.is_empty() { path.to_string_lossy().into_owned() } else { serial };
            let opener: Opener = Box::new(move || {
                let api = hidapi::HidApi::new().map_err(|e| anyhow!("Initialisation HID impossible : {e}"))?;
                let dev = api.open_path(&path).map_err(|e| {
                    anyhow!("Impossible d'ouvrir l'AIO ASUS ({e}). Fermez Armoury Crate, Aura et OpenRGB (ils utilisent la même interface), puis réessayez.")
                })?;
                Ok(Box::new(RyujinHid(dev)) as Box<dyn Transport>)
            });
            out.push(Box::new(RyujinScreen::new(format!("{VID:04x}:{pid:04x}:{tag}"), name, opener)));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::transport::mock::MockTransport;

    fn rep(cmd: u8, rest: &[u8]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0xEC;
        r[1] = cmd;
        r[2..2 + rest.len()].copy_from_slice(rest);
        r
    }

    fn ev(kind: u8, rest: &[u8]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0xEE;
        r[1] = kind;
        r[2..2 + rest.len()].copy_from_slice(rest);
        r
    }

    #[test]
    fn command_bytes_per_protocol_md() {
        // « A command is one 65-byte output report: report id 0xEC, then 64 bytes » ; `EC 71 01 01` ; `EC 7F 02 16 08` (2070 o).
        let c = command(0x71, &[1, 1]).unwrap();
        assert_eq!((c.len(), c[..4].to_vec()), (65, vec![0xEC, 0x71, 0x01, 0x01]));
        assert!(c[4..].iter().all(|&b| b == 0));
        assert_eq!(command(0x7F, &[0x02, 0x16, 0x08]).unwrap()[..5], [0xEC, 0x7F, 0x02, 0x16, 0x08]);
        assert!(command(0x01, &[0u8; 64]).is_err());
    }

    #[test]
    fn status_write_sets_byte2_and_byte7_only() {
        // Réponse de la doc : idx 2..15 = 00 00 64 00 00 64 10 01 04 00 1E 10 01 04.
        let st = rep(0x5C, &[0x00, 0x00, 0x64, 0x00, 0x00, 0x64, 0x10, 0x01, 0x04, 0x00, 0x1E, 0x10, 0x01, 0x04]);
        let w = status_write(&st, 40).unwrap();
        assert_eq!(w[..16], [0xEC, 0x5C, 0x01, 0x00, 0x64, 0x00, 0x00, 40, 0x10, 0x01, 0x04, 0x00, 0x1E, 0x10, 0x01, 0x04]);
        assert_eq!(status_write(&st, 250).unwrap()[7], 100);
        assert!(status_write(&[0xEC, 0x5C], 10).is_err());
        assert!(status_write(&rep(0x71, &[0; 20]), 10).is_err());
    }

    #[test]
    fn firmware_and_disk_info_parsers() {
        let mut fw = rep(0x02, &[0x00]);
        fw[3..3 + 15].copy_from_slice(b"AURJ2-S750-0108");
        assert_eq!(parse_firmware(&fw).unwrap(), "AURJ2-S750-0108");
        assert!(parse_firmware(&rep(0x02, &[0x00])).is_err()); // chaîne vide
        assert!(parse_firmware(&rep(0x03, &[0])).is_err());
        // Exemple de la doc : total 0x7EA8 KB = 32424 ; libre 21880 = 0x5578 ; JPG 0x203 (emplacements 0, 1, 9).
        let mut d = rep(0x71, &[0x00, 0x01]);
        d[4..8].copy_from_slice(&[0xA8, 0x7E, 0, 0]);
        d[8..12].copy_from_slice(&[0x78, 0x55, 0, 0]);
        d[12] = 0x10;
        d[17] = 0x10;
        d[18..22].copy_from_slice(&[0x03, 0x02, 0, 0]);
        d[22] = 0x10;
        assert_eq!(parse_disk_info(&d).unwrap(), (32424, 21880, 0x203));
        d[17] = 0;
        assert!(parse_disk_info(&d).is_err());
    }

    #[test]
    fn brightness_get_set_get_sequence_and_verification() {
        let st = |b: u8| rep(0x5C, &[0x00, 0x00, 0x64, 0x00, 0x00, b, 0x10, 0x01, 0x04, 0x00, 0x1E, 0x10, 0x01, 0x04]);
        // Une réponse hwmon (0x19) intercalée doit être ignorée.
        let (mut t, log) = MockTransport::new(vec![rep(0x19, &[1]), st(0x64), st(0x64), st(30)]);
        Hid::new(&mut t).set_brightness(30).unwrap();
        let w = log.lock().unwrap().writes.clone();
        assert_eq!(w.iter().map(|p| p[1]).collect::<Vec<_>>(), vec![0xDC, 0x5C, 0xDC]);
        assert_eq!(w[1][2], 0x01);
        assert_eq!(w[1][7], 30);
        // La relecture ne confirme pas : erreur.
        let (mut t, _) = MockTransport::new(vec![st(0x64), st(0x64), st(0x64)]);
        assert!(Hid::new(&mut t).set_brightness(30).is_err());
    }

    fn disk(free_kb: u32, jpg: u32) -> Vec<u8> {
        let mut d = rep(0x71, &[0x00, 0x01]);
        d[4..8].copy_from_slice(&32424u32.to_le_bytes());
        d[8..12].copy_from_slice(&free_kb.to_le_bytes());
        d[12] = 0x10;
        d[17] = 0x10;
        d[18..22].copy_from_slice(&jpg.to_le_bytes());
        d[22] = 0x10;
        d
    }

    /// Réponses d'une session d'envoi complète de `chunks` blocs.
    fn upload_script(chunks: usize, jpg_bitmap: u32) -> Vec<Vec<u8>> {
        let mut v = vec![rep(0x71, &[]), ev(0x12, &[0, 0]), disk(21880, jpg_bitmap), rep(0x72, &[]), rep(0x73, &[]), ev(0x13, &[0, 1]), rep(0x7F, &[0x00, 0x00, 0x10])];
        v.extend((0..chunks).map(|_| ev(0x14, &[0, 0, 0x55])));
        v.extend([rep(0x73, &[]), ev(0x13, &[0, 0xFF])]);
        v
    }

    fn fake_jpeg(n: usize) -> Vec<u8> {
        let mut j = vec![0x11u8; n];
        j[..2].copy_from_slice(&[0xFF, 0xD8]);
        j[n - 2..].copy_from_slice(&[0xFF, 0xD9]);
        j
    }

    #[test]
    fn upload_sequence_matches_capture() {
        // protocol.md « Media upload » : 5785 o = 2 écritures bulk de 4096 avec un EE 14 entre elles (slot JPG 9 ici).
        let jpeg = fake_jpeg(5785);
        let (mut hid, hlog) = MockTransport::new(upload_script(2, 0x003));
        let (mut bulk, blog) = MockTransport::new(vec![]);
        Hid::new(&mut hid).upload_jpeg(&mut bulk, 9, &jpeg).unwrap();
        let w = hlog.lock().unwrap().writes.clone();
        let seq: Vec<(u8, Vec<u8>)> = w.iter().map(|p| (p[1], p[2..6].to_vec())).collect();
        assert_eq!(
            seq,
            vec![
                (0x71, vec![1, 1, 0, 0]),
                (0xF1, vec![0, 0, 0, 0]),
                (0x72, vec![1, 1, 9, 0]),          // type 01 = JPG, slot 9
                (0x73, vec![1, 0, 0, 0]),          // début
                (0x7F, vec![2, 0x99, 0x16, 0]),    // 5785 = 0x1699 LE
                (0x73, vec![0xFF, 0, 0, 0]),       // fin
            ]
        );
        let b = blog.lock().unwrap().writes.clone();
        assert_eq!((b.len(), b[0].len(), b[1].len()), (2, 4096, 4096));
        assert_eq!(b[0][..4], jpeg[..4]);
        assert_eq!(b[1][..5], jpeg[4096..4101]);
        assert!(b[1][5785 - 4096..].iter().all(|&x| x == 0)); // bourrage de zéros
    }

    #[test]
    fn upload_refuses_bad_input_and_occupied_slot() {
        let (mut hid, hlog) = MockTransport::new(upload_script(1, 1 << 3));
        let (mut bulk, blog) = MockTransport::new(vec![]);
        let j = fake_jpeg(100);
        assert!(Hid::new(&mut hid).upload_jpeg(&mut bulk, 3, &j).is_err()); // occupé
        assert!(hlog.lock().unwrap().writes.iter().all(|p| p[1] != 0x73)); // rien d'écrit en flash
        let mut h = Hid::new(&mut hid);
        assert!(h.upload_jpeg(&mut bulk, 10, &j).is_err()); // hors plage
        assert!(h.upload_jpeg(&mut bulk, 0, &[1, 2, 3, 4]).is_err()); // pas un JPEG
        assert!(h.upload_jpeg(&mut bulk, 0, &fake_jpeg(70_000)).is_err()); // > 65535
        assert!(blog.lock().unwrap().writes.is_empty());
    }

    #[test]
    fn upload_failure_ends_write_once_without_retry() {
        // Pas d'évènement EE 13 00 01 : erreur FR, un seul `73 FF` best-effort, aucun octet bulk.
        let mut script = upload_script(1, 0);
        script.truncate(5); // s'arrête après `73` : plus aucune réponse
        let (mut hid, hlog) = MockTransport::new(script);
        let (mut bulk, blog) = MockTransport::new(vec![]);
        assert!(Hid::new(&mut hid).upload_jpeg(&mut bulk, 0, &fake_jpeg(100)).is_err());
        let w = hlog.lock().unwrap().writes.clone();
        assert_eq!(w.iter().filter(|p| p[1] == 0x73 && p[2] == 0xFF).count(), 1);
        assert!(blog.lock().unwrap().writes.is_empty());
    }

    #[test]
    fn unexpected_chunk_size_is_refused() {
        let mut s = upload_script(1, 0);
        s[6] = rep(0x7F, &[0x00, 0x00, 0x08]); // 0x0800
        let (mut hid, _) = MockTransport::new(s);
        let (mut bulk, blog) = MockTransport::new(vec![]);
        assert!(Hid::new(&mut hid).upload_jpeg(&mut bulk, 0, &fake_jpeg(100)).is_err());
        assert!(blog.lock().unwrap().writes.is_empty());
    }

    #[test]
    fn show_and_delete_bytes() {
        let (mut hid, hlog) = MockTransport::new(vec![rep(0x60, &[]), rep(0x51, &[])]);
        Hid::new(&mut hid).show_jpeg_slot(9).unwrap();
        let w = hlog.lock().unwrap().writes.clone();
        assert_eq!(w[0][..6], [0xEC, 0x60, 0x00, 0x01, 0x10, 0x09]); // `EC 60 00 01 10 <slot>`
        assert_eq!(w[1][..3], [0xEC, 0x51, 0x1F]);
        // Suppression : `72 01 01 00` puis `73 03`, ack `EE 13 00 03`.
        let (mut hid, hlog) = MockTransport::new(vec![rep(0x72, &[]), rep(0x73, &[]), ev(0x13, &[0, 3])]);
        Hid::new(&mut hid).delete_jpeg_slot(0).unwrap();
        let w = hlog.lock().unwrap().writes.clone();
        assert_eq!((w[0][1..5].to_vec(), w[1][1..3].to_vec()), (vec![0x72, 1, 1, 0], vec![0x73, 3]));
    }

    #[test]
    fn push_frame_is_refused_in_french_and_info_is_unverified() {
        let mut s = RyujinScreen::new("x".into(), "ROG Ryujin III 360", Box::new(|| Err(anyhow!("jamais ouvert"))));
        let err = s.push_frame(&Frame::new(320, 240)).unwrap_err().to_string();
        assert!(err.contains("Envoi d'image non validé"));
        let i = s.info();
        assert_eq!((i.width, i.height, i.verified, i.resolution_confirmed, i.protocol), (320, 240, false, false, Protocol::AsusRyujin));
        assert_eq!(MODELS.iter().map(|m| m.0).collect::<Vec<_>>(), vec![0x1AA2, 0x1BCB, 0x1ADA, 0x1ADE]);
    }

    #[test]
    fn events_arriving_early_are_kept() {
        // L'évènement EE 12 arrive AVANT la réponse EC 71 : il doit être retrouvé ensuite.
        let (mut t, _) = MockTransport::new(vec![ev(0x12, &[0, 0]), rep(0x71, &[])]);
        let mut h = Hid::new(&mut t);
        h.send(0x71, &[1, 1]).unwrap();
        h.reply(0x71, "x").unwrap();
        h.event(0x12, 0, 0, "x").unwrap();
    }
}
