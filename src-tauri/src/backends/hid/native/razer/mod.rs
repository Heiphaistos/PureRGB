//! Famille « razer » (VID 0x1532) : trame feature 90 octets, CRC XOR, matrice « custom frame » ligne par ligne.
//! Source : docs/rgb-protocols/razer.md et docs/rgb-protocols/signalrgb.md (§ 6.1, 1.1 à 1.4).
//!
//! Seul le chemin « matrice moderne » (classe 0x0F) est implémenté. Un appareil qui répond autrement à la détection
//! (claviers anciens, classe 0x03 / cmd 0x0B : taille de paquet contradictoire entre les deux notes) est refusé à l'ouverture.
//! Rien n'est écrit en mémoire flash. Effet statique de repli : non implémenté (octets RVB de l'effet 0x01 marqués (R)
//! dans razer.md, sans recoupement).
//! INCONNU : variante « NewWriteLighting » (compteur 0x80..0x9F à la place du txid, seuil de firmware par modèle) ;
//! pause inter-paquet par modèle ; cadence maximale.

mod table;

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, ZoneInfo};
use anyhow::{bail, Context, Result};
use std::time::Duration;

const VID: u16 = 0x1532;
const REPORT_LEN: usize = 91; // 1 octet report-id (0) + 90 octets de trame
const MAX_ARGS: usize = 80;
/// txid candidats : 0x1F, 0x2F, ... 0x9F (signalrgb.md 1.1).
const TXIDS: [u8; 9] = [0x1F, 0x2F, 0x3F, 0x4F, 0x5F, 0x6F, 0x7F, 0x8F, 0x9F];
/// La note prévoit 5 boucles au plus (pause 400 ms entre essais) ; 2 suffisent, sinon refus.
const PROBE_ROUNDS: usize = 2;

fn pause(ms: u64) {
    if !cfg!(test) {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

/// Construit le rapport feature : `[0]` report-id, puis la trame de 90 octets (razer.md « Trame »).
fn frame(txid: u8, size: u8, class: u8, cmd: u8, args: &[u8]) -> Result<[u8; REPORT_LEN]> {
    if args.len() > MAX_ARGS {
        bail!("trame Razer : {} arguments (max {MAX_ARGS})", args.len());
    }
    let mut b = [0u8; REPORT_LEN];
    b[2] = txid;
    b[6] = size;
    b[7] = class;
    b[8] = cmd;
    b[9..9 + args.len()].copy_from_slice(args);
    // CRC : XOR des octets 2..=87 de la trame = indices 3..=88 du rapport.
    b[89] = b[3..89].iter().fold(0, |a, x| a ^ x);
    Ok(b)
}

struct Razer {
    hid: Box<dyn Hid>,
    txid: u8,
    rows: u8,
    cols: u8,
    /// Retour matériel à la fermeture : clavier = effet spectre ; souris = mode appareil matériel.
    keyboard: bool,
}

impl Razer {
    /// Envoie une commande et lit la réponse (rapport complet, report-id compris).
    fn request(&self, txid: u8, size: u8, class: u8, cmd: u8, args: &[u8]) -> Result<Vec<u8>> {
        self.hid.send_feature_report(&frame(txid, size, class, cmd, args)?)?;
        pause(10);
        let mut buf = vec![0u8; REPORT_LEN];
        let n = self.hid.get_feature_report(&mut buf)?;
        if n < REPORT_LEN {
            bail!("réponse Razer trop courte ({n} octets)");
        }
        Ok(buf)
    }

    fn command(&self, size: u8, class: u8, cmd: u8, args: &[u8]) -> Result<()> {
        let r = self.request(self.txid, size, class, cmd, args)?;
        if r[1] != 0x02 {
            bail!("commande Razer {class:02X}/{cmd:02X} refusée (statut {})", r[1]);
        }
        Ok(())
    }

    /// Cherche le txid pour lequel la commande « numéro de série » (classe 0, cmd 0x82) répond un serial non nul.
    fn find_txid(&self) -> Result<u8> {
        for _ in 0..PROBE_ROUNDS {
            for &t in &TXIDS {
                if let Ok(r) = self.request(t, 2, 0x00, 0x82, &[0, 0]) {
                    // serial = octets 8..=22 de la trame = indices 9..=23 du rapport
                    if r[9..24].iter().any(|&x| x != 0) {
                        return Ok(t);
                    }
                }
                pause(400);
            }
        }
        bail!("Razer : aucun transaction_id n'a répondu (Synapse tient peut-être l'appareil)")
    }
}

impl NativeRgb for Razer {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let (rows, cols) = (self.rows as usize, self.cols as usize);
        if colors.len() > rows * cols {
            bail!("Razer : {} couleurs pour {} cellules", colors.len(), rows * cols);
        }
        for row in 0..rows {
            let mut args = vec![0, 0, row as u8, 0, (cols - 1) as u8];
            for col in 0..cols {
                let c = colors.get(row * cols + col).copied().unwrap_or(Color::BLACK);
                args.extend_from_slice(&[c.r, c.g, c.b]);
            }
            // Ni réponse lue ni commit (signalrgb.md 1.3).
            self.hid.send_feature_report(&frame(self.txid, args.len() as u8, 0x0F, 0x03, &args)?)?;
        }
        Ok(())
    }

    fn release(&mut self) {
        let f = if self.keyboard {
            frame(self.txid, 6, 0x0F, 0x02, &[0, 0, 0x03])
        } else {
            frame(self.txid, 2, 0x00, 0x04, &[0x00, 0x00])
        };
        // Fermeture : une erreur d'envoi n'a rien à rattraper.
        if let Ok(f) = f {
            let _ = self.hid.send_feature_report(&f);
        }
    }
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let e = table::by_pid(pid)?;
    let mut zone = ZoneInfo::fixed(if e.rows > 1 { "Matrice" } else { "LEDs" }, e.rows as u32 * e.cols as u32);
    if e.rows > 1 {
        zone.zone_type = 2;
    }
    let mut m = Model::new(e.name, e.device_type.clone(), vec![zone]);
    m.interface = Some(e.interface);
    m.usage_page = Some(e.usage_page);
    Some(m)
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let e = table::by_name(&model.name).context("modèle Razer inconnu")?;
    let mut dev = Razer { hid, txid: 0, rows: e.rows, cols: e.cols, keyboard: e.rows > 1 };
    dev.txid = dev.find_txid()?;
    // Détection de la matrice moderne : classe 0x0F cmd 0x82, args 00 ; statut OK = moderne (signalrgb.md 1.3).
    let r = dev.request(dev.txid, 6, 0x0F, 0x82, &[0])?;
    if r[1] != 0x02 {
        bail!("Razer : matrice moderne absente (chemin classe 0x03 non implémenté)");
    }
    // Mode logiciel = effet « custom frame » (pas de sauvegarde).
    dev.command(6, 0x0F, 0x02, &[0, 0, 0x08, 0, 0x01])?;
    Ok(Box::new(dev))
}

pub const FAMILY: Family = Family { id: "razer", doc: "docs/rgb-protocols/razer.md", probe, open };

#[cfg(test)]
mod tests;
