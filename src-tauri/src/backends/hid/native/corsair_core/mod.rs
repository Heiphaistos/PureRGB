//! Famille « corsair_core » : Commander Core (0C1C), Commander ST (0C32) et Commander Core XT (0C2A), VID 0x1B1C.
//! Source : docs/rgb-protocols/corsair.md (section « Famille A »).
//!
//! - RGB : couleurs directes (endpoint `22 00`, type `12 00`, 3 octets R,G,B par LED).
//! - Ventilateurs : REFUSÉ. Écrire une consigne exige de relire l'objet de vitesse puis de le réécrire en entier (sinon les autres canaux sont
//!   cassés), or l'offset des données dans une réponse de lecture est INCONNU dans la note (seul le type, octets 3..4, est donné).
//! - Famille B (iCUE LINK 0C3F) : voir `corsair_link/` (RGB seulement, zone de chaîne redimensionnable).
//! - Famille D (Hydro Platinum / Pro XT / Elite) NON revendiquée : paquets d'activation de l'éclairage = constante INCONNUE, profil de
//!   refroidissement trop lacunaire pour piloter une pompe sans risque.
//!
//! Limites assumées : taille de rapport sous Windows à valider (97 / 65 / 385, 1025 pour un 0C1C en firmware majeur 1) ; pas de keepalive
//! (le firmware retombe en mode matériel après ~10 s sans trame : l'appelant doit renvoyer les couleurs en continu).

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Context, Result};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

const VID: u16 = 0x1B1C;
const USAGE_PAGE: u16 = 0xFF42;
const READ_TIMEOUT_MS: i32 = 200;
const READ_TRIES: usize = 8;
/// Cadence minimale entre deux trames de couleurs (~33 ms selon la note).
const MIN_FRAME_GAP: Duration = Duration::from_millis(33);
const FAN_LEDS: u32 = 34;
const FANS: usize = 6;
/// Report d'un 0C1C en firmware majeur 1 (détecté via la réponse à `02 13`).
const REPORT_FW1: usize = 1025;

const CMD_WAKE: [u8; 4] = [0x01, 0x03, 0x00, 0x02];
const CMD_SLEEP: [u8; 4] = [0x01, 0x03, 0x00, 0x01];
const CMD_FIRMWARE: [u8; 2] = [0x02, 0x13];
const CMD_CLOSE: [u8; 3] = [0x05, 0x01, 0x00];

struct Spec {
    pid: u16,
    name: &'static str,
    /// Taille du rapport d'écriture (octet d'ID inclus).
    report: usize,
    /// Taille fixe du tampon de couleurs (octets).
    color_bytes: usize,
    /// LED de la pompe/AIO (0 = pas de pompe). Valeur déduite de 699 = 3 x (29 + 6 x 34) ; le nombre réel est INCONNU (lu en direct par la source).
    pump_leds: u32,
    /// Octets 2..3 du paquet de mode « 6 QL » : `01 08` = Core AIO ; `00 00` = XT sans port externe (le port externe `01 01` est INCONNU ici).
    port_mode: [u8; 2],
    /// Report à utiliser si le firmware est en version majeure 1.
    fw1_report: Option<usize>,
}

const SPECS: [Spec; 3] = [
    Spec { pid: 0x0C1C, name: "Corsair Commander Core", report: 97, color_bytes: 699, pump_leds: 29, port_mode: [0x01, 0x08], fw1_report: Some(REPORT_FW1) },
    Spec { pid: 0x0C32, name: "Corsair Commander ST", report: 65, color_bytes: 699, pump_leds: 29, port_mode: [0x01, 0x08], fw1_report: None },
    // INCONNU : disposition des 1224 octets du XT (12 x 34 LED) ; seuls les 6 ventilos sont placés, au pas de 34 LED.
    Spec { pid: 0x0C2A, name: "Corsair Commander Core XT", report: 385, color_bytes: 1224, pump_leds: 0, port_mode: [0x00, 0x00], fw1_report: None },
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let s = SPECS.iter().find(|s| s.pid == pid)?;
    let mut zones = Vec::new();
    if s.pump_leds > 0 {
        zones.push(ZoneInfo::fixed("Pompe", s.pump_leds));
    }
    zones.extend((1..=FANS).map(|i| ZoneInfo::fixed(format!("Ventilateur {i}"), FAN_LEDS)));
    let mut m = Model::new(s.name, DeviceType::Hub, zones);
    m.usage_page = Some(USAGE_PAGE);
    Some(m)
}

struct Core {
    hid: Box<dyn Hid>,
    report: usize,
    color_bytes: usize,
    led_total: usize,
    last_frame: Option<Instant>,
}

impl Core {
    /// Écrit `[00][08][cmd…]` et lit la réponse : premier rapport d'octet 0 = 00, dont l'octet 1 doit répéter `cmd[0]`.
    fn transact(&self, cmd: &[u8]) -> Result<Vec<u8>> {
        if cmd.len() + 2 > self.report {
            bail!("Commander Core : commande de {} octets trop longue pour un rapport de {}", cmd.len(), self.report);
        }
        let mut buf = vec![0u8; self.report];
        buf[1] = 0x08;
        buf[2..2 + cmd.len()].copy_from_slice(cmd);
        self.hid.write(&buf)?;
        let mut reply = vec![0u8; self.report];
        for _ in 0..READ_TRIES {
            let n = self.hid.read_timeout(&mut reply, READ_TIMEOUT_MS)?;
            if n == 0 {
                bail!("Commander Core : pas de réponse à la commande {:02X} (iCUE ouvert ? mauvaise interface ?)", cmd[0]);
            }
            if reply[0] != 0x00 {
                continue;
            }
            if n < 2 || reply[1] != cmd[0] {
                bail!("Commander Core : réponse invalide à la commande {:02X}", cmd[0]);
            }
            return Ok(reply[..n].to_vec());
        }
        bail!("Commander Core : aucune réponse exploitable à la commande {:02X}", cmd[0])
    }

    /// Ouvre un endpoint, y écrit `type + données` (1er paquet `06 00`, suivants `07 00`), puis le referme.
    fn write_object(&self, endpoint: [u8; 2], typ: [u8; 2], data: &[u8]) -> Result<()> {
        let sent = self.send_object(endpoint, typ, data);
        let closed = self.transact(&CMD_CLOSE).map(|_| ());
        sent.and(closed)
    }

    fn send_object(&self, endpoint: [u8; 2], typ: [u8; 2], data: &[u8]) -> Result<()> {
        let mut payload = typ.to_vec();
        payload.extend_from_slice(data);
        let len = u16::try_from(payload.len()).context("Commander Core : objet trop grand")?;
        self.transact(&[0x0D, 0x00, endpoint[0], endpoint[1]])?;
        let first_cap = self.report - 8; // ID, 08, 06 00, len LE16, 00 00
        let (head, mut rest) = payload.split_at(payload.len().min(first_cap));
        let mut first = vec![0x06, 0x00, (len & 0xFF) as u8, (len >> 8) as u8, 0x00, 0x00];
        first.extend_from_slice(head);
        self.transact(&first)?;
        let cont_cap = self.report - 4; // ID, 08, 07 00
        while !rest.is_empty() {
            let (chunk, tail) = rest.split_at(rest.len().min(cont_cap));
            let mut cmd = vec![0x07, 0x00];
            cmd.extend_from_slice(chunk);
            self.transact(&cmd)?;
            rest = tail;
        }
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let spec = SPECS.iter().find(|s| s.name == model.name).context("Commander Core : modèle inconnu")?;
    let mut dev = Core { hid, report: spec.report, color_bytes: spec.color_bytes, led_total: model.led_count() as usize, last_frame: None };
    dev.transact(&CMD_WAKE)?;
    let fw = dev.transact(&CMD_FIRMWARE)?;
    if fw.len() < 6 {
        bail!("Commander Core : réponse firmware trop courte ({} octets)", fw.len());
    }
    if let (Some(big), 1) = (spec.fw1_report, fw[3]) {
        dev.report = big;
    }
    // Mode « 6 QL » (34 LED/port). Le 0C1C/0C32 suppose une AIO branchée (`01 08`).
    let mut mode = vec![0x07, spec.port_mode[0], spec.port_mode[1]];
    mode.extend(std::iter::repeat([0x01, 0x06]).take(FANS).flatten());
    dev.write_object([0x1E, 0x00], [0x0D, 0x00], &mode)?;
    dev.transact(&CMD_WAKE)?;
    Ok(Box::new(dev))
}

impl NativeRgb for Core {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() != self.led_total {
            bail!("Commander Core : {} couleurs reçues, {} attendues", colors.len(), self.led_total);
        }
        let mut buf = vec![0u8; self.color_bytes];
        for (i, c) in colors.iter().enumerate() {
            buf[i * 3..i * 3 + 3].copy_from_slice(&[c.r, c.g, c.b]);
        }
        if let Some(wait) = self.last_frame.and_then(|t| MIN_FRAME_GAP.checked_sub(t.elapsed())) {
            std::thread::sleep(wait);
        }
        self.last_frame = Some(Instant::now());
        self.write_object([0x22, 0x00], [0x12, 0x00], &buf)
    }

    fn set_fan_duty(&mut self, _channel: u8, _percent: u8) -> Result<()> {
        bail!("Commander Core : vitesse ventilateur non supportée (offset des données de lecture INCONNU dans docs/rgb-protocols/corsair.md)")
    }

    fn release(&mut self) {
        if let Err(e) = self.transact(&CMD_SLEEP) {
            log::warn!("Commander Core : retour au mode matériel impossible : {e}");
        }
    }
}

pub const FAMILY: Family = Family { id: "corsair_core", doc: "docs/rgb-protocols/corsair.md", probe, open };
