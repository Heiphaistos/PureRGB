//! Famille « logitech_centurion » : casque Logitech G PRO X 2 LIGHTSPEED (VID 0x046D, PID 0x0AF7), protocole Centurion sur la page d'usage 0xFFA0.
//!
//! SOURCES : (1) définition G HUB de l'appareil (`pro_x2_gaming_wireless_headset`, déchiffrée en local pour interopérabilité) : AUCUN
//! `lightingSupport` ; capacités audio (acoustique, DTS, BlueVoice, EQ embarqué Centurion, réduction de bruit), minuterie d'extinction, batterie
//! => le casque n'a PAS d'éclairage, il est pilotable seulement pour ses réglages ; (2) trames Centurion : projet libre HeadsetControl
//! (`logitech_centurion_protocol.hpp`, `logitech_gpro_x2_lightspeed.hpp`, GPL-3, lu pour les FAITS de protocole, rien copié), niveau [S] : non confirmé
//! dans G HUB, jamais exécuté ici sur le vrai casque. Voir docs/rgb-protocols/analyse-logitech-ghub.md §13.
//!
//! Trame (64 octets, ID de rapport 0x51) : `51 <n> 00 <charge utile>` avec n = taille de la charge + 1. Requête directe : `<index feature> <fn|1> <params>`.
//! Le casque est derrière un « pont » (feature 0x0003) : `<pont> 11 <taille hi> <taille lo> 00 <index sous-feature> <fn|1> <params>`.
//! Index des sous-features cherchés à la demande avec Root.getFeature (feature 0x0000, fn 0x00).
//!
//! RÉGLAGE IMPLÉMENTÉ : retour du micro (sidetone, feature 0x0604, fn 0x10) uniquement. NON implémentés : EQ embarqué (0x0636) et minuterie
//! d'extinction (0x0108) car stockés dans le casque (persistants), batterie (lecture seule mais sans emplacement d'affichage).
//! Jamais envoyé : mise à jour du micrologiciel, écriture de profil, nom embarqué, EQ.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType};
use anyhow::{bail, Result};

const VID: u16 = 0x046D;
const PID: u16 = 0x0AF7;
const REPORT_ID: u8 = 0x51;
const FRAME: usize = 64;
const SOFTWARE_ID: u8 = 0x01;
const POLL_ATTEMPTS: usize = 8;
const TIMEOUT_MS: i32 = 500;
const FEATURE_BRIDGE: u16 = 0x0003;
const FEATURE_SIDETONE: u16 = 0x0604;
/// Identifiant du micro dans la charge du retour micro.
const SIDETONE_MIC: u8 = 0x01;
const SIDETONE_MAX: i32 = 100;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || pid != PID {
        return None;
    }
    let mut m = Model::new("Logitech G PRO X 2 LIGHTSPEED", DeviceType::Headset, Vec::new());
    m.usage_page = Some(0xFFA0);
    m.usage = Some(0x0001);
    Some(m)
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Centurion { hid, bridge: None, sidetone_index: None }))
}

struct Centurion {
    hid: Box<dyn Hid>,
    bridge: Option<u8>,
    sidetone_index: Option<u8>,
}

/// `51 <n> 00 <charge>` complété à 64 octets.
fn frame(payload: &[u8]) -> [u8; FRAME] {
    let mut f = [0u8; FRAME];
    f[0] = REPORT_ID;
    f[1] = (payload.len() + 1) as u8;
    f[3..3 + payload.len()].copy_from_slice(payload);
    f
}

/// Charge utile d'une trame reçue (après `51 <n> 00`).
fn payload_of(r: &[u8]) -> Option<&[u8]> {
    if r.len() < 4 || r[0] != REPORT_ID {
        return None;
    }
    let n = r[1] as usize;
    (n > 1 && n + 2 <= r.len()).then(|| &r[3..n + 2])
}

impl Centurion {
    fn read_payload(&self) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; FRAME];
        let n = self.hid.read_timeout(&mut buf, TIMEOUT_MS)?;
        if n == 0 {
            bail!("casque G PRO X 2 : pas de réponse (éteint ou hors de portée du récepteur ?)");
        }
        Ok(payload_of(&buf[..n]).map(<[u8]>::to_vec))
    }

    /// Requête directe au récepteur ; rend les octets après `<index> <fn|sw>`.
    fn direct(&self, index: u8, function: u8, params: &[u8]) -> Result<Vec<u8>> {
        let mut p = vec![index, (function & 0xF0) | SOFTWARE_ID];
        p.extend_from_slice(params);
        self.hid.write(&frame(&p))?;
        for _ in 0..POLL_ATTEMPTS {
            if let Some(r) = self.read_payload()? {
                if r.len() >= 2 && r[0] == index {
                    return Ok(r[2..].to_vec());
                }
            }
        }
        bail!("casque G PRO X 2 : réponse du récepteur absente")
    }

    /// Requête vers le casque à travers le pont ; rend les données de la sous-feature.
    fn bridged(&self, sub_index: u8, function: u8, params: &[u8]) -> Result<Vec<u8>> {
        let bridge = self.bridge.ok_or_else(|| anyhow::anyhow!("pont Centurion non initialisé"))?;
        let mut sub = vec![0x00, sub_index, (function & 0xF0) | SOFTWARE_ID];
        sub.extend_from_slice(params);
        let mut layer = vec![bridge, 0x10 | SOFTWARE_ID, ((sub.len() >> 8) & 0x0F) as u8, (sub.len() & 0xFF) as u8];
        layer.extend_from_slice(&sub);
        if layer.len() > 60 {
            bail!("message Centurion trop long");
        }
        self.hid.write(&frame(&layer))?;
        for _ in 0..POLL_ATTEMPTS {
            let Some(r) = self.read_payload()? else { continue };
            if r.len() < 2 || r[0] != bridge {
                continue;
            }
            // Événement de connexion du pont : [pont, 00, 00, connecté].
            if r[1] == 0x00 && r.len() >= 4 {
                if r[3] == 0 {
                    bail!("casque G PRO X 2 éteint ou non connecté");
                }
                continue;
            }
            if r[1] >> 4 != 1 || r[1] & 0x0F == SOFTWARE_ID {
                continue; // accusé de réception ou autre fonction
            }
            if r[1] & 0x0F != 0 || r.len() < 6 || r[4] != 0x00 {
                continue;
            }
            if r[5] == 0xFF {
                bail!("le casque a refusé la requête");
            }
            if r[5] == sub_index && r.len() >= 7 {
                return Ok(r[7..].to_vec());
            }
        }
        bail!("casque G PRO X 2 : réponse absente")
    }

    fn ensure_bridge(&mut self) -> Result<()> {
        if self.bridge.is_none() {
            let r = self.direct(0x00, 0x00, &[(FEATURE_BRIDGE >> 8) as u8, FEATURE_BRIDGE as u8])?;
            match r.first() {
                Some(&i) if i != 0 => self.bridge = Some(i),
                _ => bail!("pont Centurion introuvable sur ce récepteur"),
            }
        }
        Ok(())
    }

    fn sidetone_index(&mut self) -> Result<u8> {
        if let Some(i) = self.sidetone_index {
            return Ok(i);
        }
        self.ensure_bridge()?;
        let r = self.bridged(0x00, 0x00, &[(FEATURE_SIDETONE >> 8) as u8, FEATURE_SIDETONE as u8])?;
        match r.first() {
            Some(&i) if i != 0 => {
                self.sidetone_index = Some(i);
                Ok(i)
            }
            _ => bail!("ce casque n'expose pas le retour du micro"),
        }
    }

    fn set_sidetone(&mut self, level: i32) -> Result<()> {
        if !(0..=SIDETONE_MAX).contains(&level) {
            bail!("retour du micro : valeur {level} hors de 0 à {SIDETONE_MAX}");
        }
        let index = self.sidetone_index()?;
        // Lecture de l'état : une réponse de 4 octets ou plus = version 2 de la feature (charge `micro, FF, niveau`).
        let state = self.bridged(index, 0x00, &[])?;
        let level = level as u8;
        let payload: Vec<u8> = if state.len() >= 4 { vec![SIDETONE_MIC, 0xFF, level] } else { vec![SIDETONE_MIC, level] };
        self.bridged(index, 0x10, &payload)?;
        Ok(())
    }
}

impl NativeRgb for Centurion {
    fn set_colors(&mut self, _colors: &[Color]) -> Result<()> {
        Ok(()) // aucun éclairage (définition G HUB sans lightingSupport)
    }

    fn set_setting(&mut self, id: &str, value: i32) -> Result<()> {
        match id {
            "sidetone" => self.set_sidetone(value),
            _ => bail!("réglage inconnu : {id}"),
        }
    }
}

pub static FAMILY: Family = Family { id: "logitech_centurion", doc: "docs/rgb-protocols/analyse-logitech-ghub.md §13", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    /// Trame reçue : `51 <n> 00 <charge>`.
    fn reply(payload: &[u8]) -> Vec<u8> {
        frame(payload).to_vec()
    }

    #[test]
    fn only_the_documented_receiver_is_claimed() {
        let m = probe(VID, PID).expect("0AF7");
        assert_eq!((m.usage_page, m.usage, m.led_count()), (Some(0xFFA0), Some(1), 0));
        assert!(probe(VID, 0x0ABA).is_none() && probe(0x1234, PID).is_none());
    }

    #[test]
    fn frames_follow_the_documented_layout() {
        let f = frame(&[0x00, 0x01, 0x00, 0x03]);
        assert_eq!(&f[..7], &[0x51, 5, 0, 0, 1, 0, 3]);
        assert_eq!(f.len(), 64);
        assert_eq!(payload_of(&f), Some(&[0x00, 0x01, 0x00, 0x03][..]));
    }

    #[test]
    fn sidetone_flow_sends_exactly_the_documented_frames() {
        let replies = vec![
            reply(&[0x00, 0x01, 0x02]),                                          // Root.getFeature(0x0003) -> pont à l'index 2
            reply(&[0x02, 0x11]),                                                // accusé du pont
            reply(&[0x02, 0x10, 0x00, 0x05, 0x00, 0x00, 0x01, 0x07]),            // sous-Root : 0x0604 à l'index 7
            reply(&[0x02, 0x11]),
            reply(&[0x02, 0x10, 0x00, 0x07, 0x00, 0x07, 0x01, 1, 0xFF, 0, 0]),   // état (4 octets : version 2)
            reply(&[0x02, 0x11]),
            reply(&[0x02, 0x10, 0x00, 0x04, 0x00, 0x07, 0x11]),                  // accusé d'écriture
        ];
        let (mock, sent, _) = MockHid::new(replies);
        let mut d = open(Box::new(mock), &probe(VID, PID).unwrap()).unwrap();
        d.set_setting("sidetone", 40).unwrap();
        let s = sent.lock();
        assert_eq!(s.len(), 4);
        assert_eq!(&s[0][..7], &[0x51, 5, 0, 0x00, 0x01, 0x00, 0x03]);
        assert_eq!(&s[1][..12], &[0x51, 10, 0, 2, 0x11, 0, 5, 0, 0, 0x01, 0x06, 0x04]);
        assert_eq!(&s[2][..10], &[0x51, 8, 0, 2, 0x11, 0, 3, 0, 7, 0x01]);
        assert_eq!(&s[3][..13], &[0x51, 11, 0, 2, 0x11, 0, 6, 0, 7, 0x11, 1, 0xFF, 40]);
    }

    #[test]
    fn bad_values_unknown_settings_and_a_powered_off_headset_are_refused() {
        let (mock, sent, _) = MockHid::new(vec![]);
        let mut d = open(Box::new(mock), &probe(VID, PID).unwrap()).unwrap();
        assert!(d.set_setting("sidetone", 101).is_err());
        assert!(d.set_setting("eq", 1).is_err());
        assert!(sent.lock().is_empty());
        let (mock, _, _) = MockHid::new(vec![reply(&[0x00, 0x01, 0x02]), reply(&[0x02, 0x00, 0x00, 0x00])]);
        let mut d = open(Box::new(mock), &probe(VID, PID).unwrap()).unwrap();
        let e = d.set_setting("sidetone", 10).unwrap_err().to_string();
        assert!(e.contains("éteint"), "{e}");
    }
}
