//! Famille « corsair_link » : iCUE LINK System Hub (0C3F), VID 0x1B1C. RGB seulement.
//! Sources : docs/rgb-protocols/corsair.md (famille B) ; docs/rgb-protocols/analyse-corsair-icue.md §5 et §12 (recoupement iCUE :
//! identifiants de sous-appareils, nombre de LED, 476 LED max, 12 appareils par côté, tampon couleur) ; cadre OpenRGB (faits seulement).
//!
//! Écrit uniquement : `05 01 01 <ressource>` / `0D 01 <ressource>` / `0D 00 <ressource>` (fermer/ouvrir), `08 01` (lecture), `02 13` (firmware)
//! et les couleurs directes (ressource `22`, type `12 00`, morceaux `06 00` puis `07 00` de 508 octets).
//!
//! REFUSÉ volontairement (écritures possiblement persistantes, voir analyse §12.4) : ressources `1D`/`1E` (géométrie, nombre de LED,
//! mode ventilateurs), `CoolersMode`, `CoolersFixRotateSpeed`, `CurveRPM`, et le changement de mode `01 03 00 01|02` (jamais envoyé : le
//! hub n'est donc ni réveillé en mode logiciel ni rendu en mode matériel par ce driver). Aucune consigne de pompe ni de ventilateur.
//!
//! Énumération (lecture seule) : ressource `36`, type `21 00` ; octet 6 = nombre de canaux, puis par canal une fiche de 8 octets (octet 2 = type,
//! octet 3 = modèle, octet 7 = longueur de l'identifiant) suivie de l'identifiant. Tout appareil inconnu, ou dont le nombre de LED diffère entre
//! OpenRGB et iCUE, fait REFUSER l'ouverture : un nombre de LED faux décalerait les couleurs de tous les appareils suivants.
//!
//! Cadre : les zones du modèle sont fixées avant l'ouverture et la chaîne n'est connue qu'après énumération ; le modèle expose donc UNE zone
//! redimensionnable (« Chaîne iCUE LINK », 1 à 476 LED, dans l'ordre des canaux) ; l'excédent est ignoré, le manque complété en noir.
//! Limites assumées : pas de keep-alive autonome (le hub exige un ping toutes les 30 s selon iCUE : l'appelant doit rappeler `set_colors`
//! régulièrement) ; réduction de luminosité d'iCUE (`BrightnessReductionMap`, unité INCONNUE) non appliquée ; délai de repli du firmware INCONNU.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Context, Result};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

const VID: u16 = 0x1B1C;
const PID: u16 = 0x0C3F;
const USAGE_PAGE: u16 = 0xFF42;
const WRITE_LEN: usize = 513;
const READ_TIMEOUT_MS: i32 = 1000;
/// Relectures max en attendant le bon type de données (note : 5).
const READ_RETRIES: usize = 5;
/// Octets de charge utile max par requête de couleur.
const CHUNK: usize = 508;
/// `MaxLedsCountAcrossAllChannels` (iCUE).
const MAX_LEDS: u32 = 476;
/// 2 côtés x 12 appareils (`MaxSubdevicesOnSide`, iCUE).
const MAX_CHANNELS: usize = 24;
const MIN_FRAME_GAP: Duration = Duration::from_millis(33);

const RES_DEVICES: u8 = 0x36;
const RES_COLORS: u8 = 0x22;
const TYPE_DEVICES: [u8; 2] = [0x21, 0x00];
const TYPE_COLORS: [u8; 2] = [0x12, 0x00];

/// (type, modèle, nombre de LED). Source : table OpenRGB de la note, corrigée par iCUE là où la note dit « retenir iCUE »
/// (5000T 204, pompe LCD 40). 0 = appareil sans LED (ou interne, ignoré). Absents car OpenRGB (0) et iCUE (>0) divergent :
/// `14 00` ORIGIN OA (0 / 32) et `0A 00` XG3 Hybrid (0 / 18) -> INCONNU, refus.
const DEVICES: &[(u8, u8, u32)] = &[
    (0x05, 0x02, 204), // 5000T RGB
    (0x05, 0x01, 22),  // 9000D RGB Airflow (châssis seul)
    (0x05, 0x00, 0),   // adaptateur
    (0x06, 0x00, 40),  // Cooler Pump LCD
    (0x11, 0x00, 20), (0x11, 0x04, 20), (0x11, 0x01, 20), (0x11, 0x02, 20), (0x11, 0x05, 20), (0x11, 0x03, 20), // TITAN
    (0x07, 0x00, 20), (0x07, 0x04, 20), (0x07, 0x01, 20), (0x07, 0x02, 20), (0x07, 0x05, 20), (0x07, 0x03, 20), // H100i..H170i
    (0x10, 0x00, 0),   // VRM cooler
    (0x02, 0x00, 18),  // LX RGB
    (0x01, 0x00, 34),  // QX RGB
    (0x13, 0x00, 0),   // RX
    (0x04, 0x00, 0),   // RX MAX
    (0x0F, 0x00, 8),   // RX RGB
    (0x03, 0x00, 8),   // RX RGB MAX
    (0x09, 0x00, 24),  // XC7 Elite
    (0x0C, 0x00, 22),  // XD5 Elite
    (0x0E, 0x00, 0),   // XD5 Elite LCD : interne, doublon de XD5 (ignoré)
    (0x19, 0x00, 22),  // XD6 Elite
    (0x0D, 0x00, 16),  // XG7 RGB
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if (vid, pid) != (VID, PID) {
        return None;
    }
    let zone = ZoneInfo { name: "Chaîne iCUE LINK".into(), led_count: MAX_LEDS, zone_type: 1, leds_min: 1, leds_max: MAX_LEDS };
    let mut m = Model::new("Corsair iCUE LINK System Hub", DeviceType::Hub, vec![zone]);
    m.usage_page = Some(USAGE_PAGE);
    Some(m)
}

/// Nombre total de LED de la chaîne d'après la réponse à la lecture de la ressource `36` (octets bruts du rapport).
fn parse_chain(reply: &[u8]) -> Result<u32> {
    if reply.len() < 7 {
        bail!("iCUE LINK : réponse d'énumération tronquée ({} octets)", reply.len());
    }
    let count = reply[6] as usize;
    if count > MAX_CHANNELS {
        bail!("iCUE LINK : {count} canaux annoncés, {MAX_CHANNELS} au maximum");
    }
    let (mut pos, mut total) = (7usize, 0u32);
    for _ in 0..count {
        let Some(card) = reply.get(pos..pos + 8) else {
            bail!("iCUE LINK : fiche de canal tronquée à l'octet {pos}");
        };
        let id_len = card[7] as usize;
        if pos + 8 + id_len > reply.len() {
            bail!("iCUE LINK : identifiant de canal tronqué à l'octet {pos}");
        }
        pos += 8 + id_len;
        if id_len == 0 {
            continue; // canal vide
        }
        let (typ, model) = (card[2], card[3]);
        let leds = DEVICES.iter().find(|d| d.0 == typ && d.1 == model).map(|d| d.2);
        total += leds.with_context(|| format!("iCUE LINK : appareil de type {typ:02X} modèle {model:02X} inconnu (nombre de LED INCONNU), ouverture refusée"))?;
    }
    if total == 0 {
        bail!("iCUE LINK : aucun appareil lumineux dans la chaîne");
    }
    if total > MAX_LEDS {
        bail!("iCUE LINK : {total} LED dans la chaîne, {MAX_LEDS} au maximum");
    }
    Ok(total)
}

struct Link {
    hid: Box<dyn Hid>,
    chain_leds: usize,
    last_frame: Option<Instant>,
}

impl Link {
    fn send(&self, cmd: &[u8]) -> Result<()> {
        if cmd.len() + 3 > WRITE_LEN {
            bail!("iCUE LINK : commande de {} octets trop longue", cmd.len());
        }
        let mut buf = vec![0u8; WRITE_LEN];
        buf[2] = 0x01;
        buf[3..3 + cmd.len()].copy_from_slice(cmd);
        self.hid.write(&buf)?;
        Ok(())
    }

    fn recv(&self, cmd0: u8) -> Result<Vec<u8>> {
        let mut reply = vec![0u8; 512];
        let n = self.hid.read_timeout(&mut reply, READ_TIMEOUT_MS)?;
        if n == 0 {
            bail!("iCUE LINK : pas de réponse à la commande {cmd0:02X} (iCUE ouvert ? mauvaise interface ?)");
        }
        reply.truncate(n);
        Ok(reply)
    }

    fn transact(&self, cmd: &[u8]) -> Result<Vec<u8>> {
        self.send(cmd)?;
        self.recv(cmd[0])
    }

    /// Lecture d'une ressource : fermer, ouvrir, `08 01`, relire jusqu'au bon type (octets 4..5), refermer (même en cas d'échec).
    fn read_resource(&self, res: u8, typ: [u8; 2]) -> Result<Vec<u8>> {
        self.transact(&[0x05, 0x01, 0x01, res])?;
        self.transact(&[0x0D, 0x01, res])?;
        let got = self.transact(&[0x08, 0x01]).and_then(|first| {
            let mut r = first;
            for _ in 0..READ_RETRIES {
                if r.get(4..6) == Some(&typ[..]) {
                    return Ok(r);
                }
                r = self.recv(0x08)?;
            }
            if r.get(4..6) == Some(&typ[..]) {
                Ok(r)
            } else {
                bail!("iCUE LINK : type de données {typ:02X?} jamais reçu pour la ressource {res:02X}")
            }
        });
        let closed = self.transact(&[0x05, 0x01, 0x01, res]).map(|_| ());
        let r = got?;
        closed?;
        Ok(r)
    }

    fn write_colors(&self, rgb: &[u8]) -> Result<()> {
        let len = u16::try_from(rgb.len() + 2).context("iCUE LINK : tampon de couleurs trop grand")?;
        let mut payload = vec![(len & 0xFF) as u8, (len >> 8) as u8, 0x00, 0x00, TYPE_COLORS[0], TYPE_COLORS[1]];
        payload.extend_from_slice(rgb);
        self.transact(&[0x05, 0x01, 0x01, RES_COLORS])?;
        let sent = self.transact(&[0x0D, 0x00, RES_COLORS]).and_then(|_| {
            for (i, chunk) in payload.chunks(CHUNK).enumerate() {
                let mut cmd = vec![if i == 0 { 0x06 } else { 0x07 }, 0x00];
                cmd.extend_from_slice(chunk);
                self.transact(&cmd)?;
            }
            Ok(())
        });
        let closed = self.transact(&[0x05, 0x01, 0x01, RES_COLORS]).map(|_| ());
        sent.and(closed)
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    let mut dev = Link { hid, chain_leds: 0, last_frame: None };
    let fw = dev.transact(&[0x02, 0x13])?;
    if fw.len() < 8 {
        bail!("iCUE LINK : réponse firmware trop courte ({} octets)", fw.len());
    }
    let devices = dev.read_resource(RES_DEVICES, TYPE_DEVICES)?;
    let total = parse_chain(&devices)?;
    log::info!("iCUE LINK : firmware {}.{}.{}, chaîne de {total} LED", fw[4], fw[5], u16::from_le_bytes([fw[6], fw[7]]));
    dev.chain_leds = total as usize;
    Ok(Box::new(dev))
}

impl NativeRgb for Link {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let mut rgb = vec![0u8; self.chain_leds * 3];
        for (i, c) in colors.iter().take(self.chain_leds).enumerate() {
            rgb[i * 3..i * 3 + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        if let Some(wait) = self.last_frame.and_then(|t| MIN_FRAME_GAP.checked_sub(t.elapsed())) {
            std::thread::sleep(wait);
        }
        self.last_frame = Some(Instant::now());
        self.write_colors(&rgb)
    }

    fn set_fan_duty(&mut self, _channel: u8, _percent: u8) -> Result<()> {
        bail!("iCUE LINK : consigne de ventilateur/pompe refusée (formats de CoolersMode/CurveRPM non lus, écriture possiblement persistante)")
    }
}

pub const FAMILY: Family = Family { id: "corsair_link", doc: "docs/rgb-protocols/corsair.md", probe, open };
