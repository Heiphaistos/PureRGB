//! Famille « corsair_commander » : Commander Pro (VID 0x1B1C, PID 0C10) et hub de l'Obsidian 1000D (1D00, même protocole selon la note).
//! Source : docs/rgb-protocols/corsair.md (section « Famille C »). Rapports : ID 00 + 64 octets, réponse de 16 octets.
//!
//! - Ventilateurs : consigne fixe `23 <ventilo> <%>` (6 ventilos).
//! - RGB : REFUSÉ. Le mode direct par pixel est INCONNU, l'ordre des couleurs de l'effet matériel `35` n'est pas donné et la note
//!   ne décrit que des effets matériels (jusqu'à 8 cumulés).
//! - Non revendiqués : Commander Core / Core XT / ST (0C1C, 0C2A, 0C32...) et iCUE LINK (0C3F) : taille de rapport selon firmware (97/65/385/1025),
//!   écriture lire-modifier-réécrire et fiabilité « broken » chez la source = note pas assez sûre ; AIO Hydro (famille D) hors périmètre.
//!
//! La note ne décrit aucun retour au mode matériel pour cette famille : `release` ne fait rien.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType};
use anyhow::{bail, Result};

const VID: u16 = 0x1B1C;
const PIDS: [u16; 2] = [0x0C10, 0x1D00];
const FANS: u8 = 6;
const REPORT: usize = 65;
const REPLY: usize = 16;
const READ_TIMEOUT_MS: i32 = 200;
const CMD_FIRMWARE: u8 = 0x02;
const CMD_FAN_FIXED: u8 = 0x23;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || !PIDS.contains(&pid) {
        return None;
    }
    let name = if pid == 0x0C10 { "Corsair Commander Pro" } else { "Corsair Obsidian 1000D (hub)" };
    let mut m = Model::new(name, DeviceType::Hub, vec![]);
    m.fans = FANS;
    Some(m)
}

struct Commander {
    hid: Box<dyn Hid>,
}

impl Commander {
    /// Envoie une commande de 64 o (octet 1 = commande) et renvoie la réponse lue juste après.
    fn command(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut buf = [0u8; REPORT];
        buf[1..=payload.len()].copy_from_slice(payload);
        self.hid.write(&buf)?;
        let mut reply = [0u8; REPLY];
        let n = self.hid.read_timeout(&mut reply, READ_TIMEOUT_MS)?;
        Ok(reply[..n].to_vec())
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    let dev = Commander { hid };
    // Contrôle de vie : la commande firmware doit répondre, sinon ce n'est pas la bonne interface.
    if dev.command(&[CMD_FIRMWARE])?.is_empty() {
        bail!("Commander Pro : pas de réponse à la commande firmware (iCUE ouvert ? mauvaise interface ?)");
    }
    Ok(Box::new(dev))
}

impl NativeRgb for Commander {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.is_empty() {
            return Ok(());
        }
        bail!("Commander Pro : RGB non supporté (mode direct INCONNU dans docs/rgb-protocols/corsair.md)")
    }

    fn set_fan_duty(&mut self, channel: u8, percent: u8) -> Result<()> {
        if channel >= FANS {
            bail!("Commander Pro : ventilateur {channel} inexistant (0..{})", FANS - 1);
        }
        if percent > 100 {
            bail!("Commander Pro : vitesse {percent} % hors plage 0-100");
        }
        self.command(&[CMD_FAN_FIXED, channel, percent]).map(|_| ())
    }
}

pub const FAMILY: Family = Family { id: "corsair_commander", doc: "docs/rgb-protocols/corsair.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn dev(replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, super::super::Log) {
        let (hid, sent, _) = MockHid::new(replies);
        (open(Box::new(hid), &probe(VID, 0x0C10).unwrap()), sent)
    }

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; REPORT];
        v[1..=bytes.len()].copy_from_slice(bytes);
        v
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert!(probe(VID, 0x0C10).is_some() && probe(VID, 0x1D00).is_some());
        for pid in [0x0C0B, 0x0C1A, 0x0C1C, 0x0C3F, 0x0C2A] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert_eq!(probe(VID, 0x0C10).unwrap().fans, 6);
    }

    #[test]
    fn open_checks_firmware_reply() {
        let (r, sent) = dev(vec![]);
        assert!(r.is_err());
        assert_eq!(sent.lock()[0], frame(&[0x02]));
        assert!(dev(vec![vec![0, 1, 2, 3]]).0.is_ok());
    }

    #[test]
    fn fan_frames_and_validation() {
        let (d, sent) = dev(vec![vec![0, 1, 2, 3]]);
        let mut d = d.unwrap();
        d.set_fan_duty(0, 50).unwrap();
        d.set_fan_duty(5, 100).unwrap();
        assert!(d.set_fan_duty(6, 10).is_err());
        assert!(d.set_fan_duty(0, 101).is_err());
        let s = sent.lock();
        assert_eq!(s.len(), 3);
        assert_eq!(s[1], frame(&[0x23, 0x00, 0x32]));
        assert_eq!(s[2], frame(&[0x23, 0x05, 0x64]));
    }

    #[test]
    fn rgb_refused_but_empty_frame_ok() {
        let mut d = dev(vec![vec![0, 1]]).0.unwrap();
        assert!(d.set_colors(&[Color::BLACK]).is_err());
        assert!(d.set_colors(&[]).is_ok());
    }

    #[test]
    fn open_and_release_never_write_a_fan_speed() {
        let (d, sent) = dev(vec![vec![1u8; 16]]);
        let mut d = d.unwrap();
        d.release_fans();
        d.release();
        assert_eq!(sent.lock().as_slice(), [frame(&[CMD_FIRMWARE])], "seule la lecture firmware part");
    }
}
