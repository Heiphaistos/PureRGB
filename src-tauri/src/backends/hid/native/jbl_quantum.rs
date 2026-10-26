//! Famille « jbl_quantum » : récepteurs USB des casques JBL Quantum « V1 » (Harman, VID 0x0ECB), réglage d'effet local (sidetone).
//!
//! SOURCE : logiciel officiel JBL QuantumENGINE 2.8.6 (installeur NSIS du site JBL, SHA-256 f3af6884…8723), assemblages .NET non obfusqués
//! lus avec ILSpy (docs/rgb-protocols/aqua-jbl-arctic.md). Aucun code ni asset repris : seules les valeurs de trame sont relevées.
//! [V] Sidetone V1 : collection HID de page d'usage 0xFF13 (65299) ; lecture = rapport de fonctionnalité `5C` (+ 1 octet),
//! écriture = rapport de fonctionnalité `5D <niveau>`, niveau 0 Off, 1 Faible, 2 Moyen, 3 Fort.
//! [S] PERSISTANCE : le logiciel ne dit pas si le casque garde le niveau après extinction. Garde-fou : le niveau d'origine est lu AVANT la
//! première écriture (pas de lecture = pas d'écriture) puis remis à la fermeture du pilote. Rien d'autre n'est envoyé (éclairage = animation
//! envoyée au casque, REFUSÉ ; EQ, profils, firmware : jamais).

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{DeviceSetting, DeviceType, SettingKind};
use anyhow::{bail, Result};

const VID: u16 = 0x0ECB;
const USAGE_PAGE: u16 = 0xFF13;
const GET_SIDETONE: u8 = 0x5C;
const SET_SIDETONE: u8 = 0x5D;
const LEVELS: u8 = 3;

/// Récepteurs (PID de `QuantumUSBAudio.inf` + `new VID_PID` du serveur) dont le logiciel utilise la collection 0xFF13 et le sidetone V1.
const MODELS: [(u16, &str); 6] = [
    (0x205C, "JBL Quantum 610"),
    (0x2069, "JBL Quantum 810"),
    (0x2088, "JBL Quantum 910"),
    (0x209D, "JBL Quantum 910X"),
    (0x20A2, "JBL Quantum 910P"),
    (0x2147, "JBL Quantum 910X Noir"),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let (_, name) = MODELS.iter().find(|(p, _)| vid == VID && *p == pid)?;
    let mut m = Model::new(*name, DeviceType::Headset, Vec::new());
    m.usage_page = Some(USAGE_PAGE);
    Some(m)
}

pub fn settings() -> Vec<DeviceSetting> {
    vec![DeviceSetting {
        id: "sidetone".into(),
        label: "Retour micro (sidetone)".into(),
        kind: SettingKind::Choice { options: vec!["Désactivé".into(), "Faible".into(), "Moyen".into(), "Fort".into()] },
        default: 0,
    }]
}

struct Quantum {
    hid: Box<dyn Hid>,
    /// Niveau lu avant notre première écriture ; `Some` = quelque chose est à remettre.
    original: Option<u8>,
}

impl Quantum {
    fn read_level(&self) -> Result<u8> {
        let mut buf = [GET_SIDETONE, 0];
        let n = self.hid.get_feature_report(&mut buf)?;
        if n < 2 || buf[0] != GET_SIDETONE || buf[1] > LEVELS {
            bail!("JBL Quantum : lecture du sidetone illisible, aucune écriture");
        }
        Ok(buf[1])
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Quantum { hid, original: None }))
}

impl NativeRgb for Quantum {
    fn set_colors(&mut self, _colors: &[crate::core::Color]) -> Result<()> {
        Ok(()) // aucune LED pilotée (voir l'en-tête)
    }

    fn set_setting(&mut self, id: &str, value: i32) -> Result<()> {
        if id != "sidetone" {
            bail!("réglage inconnu : {id}");
        }
        if !(0..=LEVELS as i32).contains(&value) {
            bail!("sidetone : niveau {value} hors 0..=3");
        }
        if self.original.is_none() {
            self.original = Some(self.read_level()?);
        }
        self.hid.send_feature_report(&[SET_SIDETONE, value as u8])
    }

    fn release(&mut self) {
        if let Some(level) = self.original.take() {
            let _ = self.hid.send_feature_report(&[SET_SIDETONE, level]);
        }
    }
}

impl Drop for Quantum {
    fn drop(&mut self) {
        self.release();
    }
}

pub const FAMILY: Family = Family { id: "jbl_quantum", doc: "docs/rgb-protocols/aqua-jbl-arctic.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    type Frames = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn dev(replies: Vec<Vec<u8>>) -> (Box<dyn NativeRgb>, Frames) {
        let (mock, _, feats) = MockHid::new(replies);
        (open(Box::new(mock), &probe(VID, 0x2069).unwrap()).unwrap(), feats)
    }

    #[test]
    fn claims_only_the_listed_receivers_on_page_ff13() {
        for (pid, _) in MODELS {
            assert_eq!(probe(VID, pid).unwrap().usage_page, Some(0xFF13));
        }
        assert!(probe(VID, 0x2129).is_none(), "calibration 950 : pas de collection de contrôle V1");
        assert!(probe(0x046D, 0x2069).is_none());
    }

    #[test]
    fn reads_the_original_level_then_writes_5d_and_restores_it() {
        let (mut d, feats) = dev(vec![vec![0x5C, 0x02]]);
        d.set_setting("sidetone", 3).unwrap();
        d.set_setting("sidetone", 1).unwrap();
        assert_eq!(feats.lock().as_slice(), [vec![0x5D, 3], vec![0x5D, 1]]);
        drop(d);
        assert_eq!(feats.lock().last().unwrap(), &vec![0x5D, 2], "niveau d'origine remis à la fermeture");
    }

    #[test]
    fn no_readable_level_means_no_write() {
        let (mut d, feats) = dev(vec![]);
        assert!(d.set_setting("sidetone", 2).is_err());
        let (mut d2, feats2) = dev(vec![vec![0x5C, 0x09]]);
        assert!(d2.set_setting("sidetone", 2).is_err());
        drop((d, d2));
        assert!(feats.lock().is_empty() && feats2.lock().is_empty());
    }

    #[test]
    fn rejects_bad_ids_and_levels_without_touching_the_device() {
        let (mut d, feats) = dev(vec![vec![0x5C, 0x00]]);
        assert!(d.set_setting("sidetone", 4).is_err());
        assert!(d.set_setting("sidetone", -1).is_err());
        assert!(d.set_setting("eq", 1).is_err());
        drop(d);
        assert!(feats.lock().is_empty());
    }
}
