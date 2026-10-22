//! Famille « msi_mystic » : MSI Mystic Light USB, variante feature report 0x52 de 185 octets, mode « par zone » (effet statique).
//! Source : docs/rgb-protocols/msi-mystic-light.md
//!
//! Périmètre volontairement restreint : le protocole ne se choisit pas par PID mais par la longueur du rapport 0x52 ; `open`
//! refuse tout contrôleur dont le rapport n'a pas EXACTEMENT 185 octets (familles 112/162/761/Super I/O : hors périmètre,
//! offsets non documentés ou pilote noyau requis). Les PID « désactivés par défaut » (risque de brick) et la famille 761
//! (7Exx, B926) ne sont pas revendiqués, ni 0DB0:0076 (page d'usage ambiguë).
//! Lecture-modification-écriture : l'état courant est lu, seuls `effect` (=1 statique) et R,G,B de chaque zone changent,
//! tout le reste (vitesse, luminosité, flags de synchro) est conservé. `save_data` (octet 184) reste TOUJOURS à 0 :
//! jamais d'écriture en flash. Deux rapports consécutifs par mise à jour (ancien état, puis nouvel état), comme le fait MSI Center.
//! INCONNU : table `board_config` par PID (nombre de LEDs embarquées, connecteurs présents) absente de la note, donc le mode
//! direct LED à LED (rapport 0x53) n'est PAS implémenté ; les 8 zones sont exposées sans savoir lesquelles existent
//! sur la carte (les zones absentes reçoivent la couleur mais leur effet est INCONNU). `release()` ne fait rien.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const REPORT_ID: u8 = 0x52;
const LEN: usize = 185;
const SAVE_OFFSET: usize = 184;
const EFFECT_STATIC: u8 = 1;

/// Zones d'une trame 185 : (nom, offset du `ZoneData` : effet @+0, R,G,B @+1..+3).
const ZONES: [(&str, usize); 8] = [
    ("JRGB 1", 1),
    ("JPIPE 1", 11),
    ("JPIPE 2", 21),
    ("JRAINBOW 1", 31),
    ("JRAINBOW 2", 42),
    ("JCORSAIR", 53),
    ("LED carte", 74),
    ("JRGB 2", 174),
];

fn is_supported_pid(pid: u16) -> bool {
    matches!(pid,
        0x7B12 | 0x7B16 | 0x7B50 | 0x7B92 | 0x7B93 | 0x7C02
        | 0x7C34..=0x7C37 | 0x7C56 | 0x7C59 | 0x7C60 | 0x7C67 | 0x7C71 | 0x7C73 | 0x7C75..=0x7C77
        | 0x7C79..=0x7C84 | 0x7C86 | 0x7C87 | 0x7C90..=0x7C92 | 0x7C94 | 0x7C95 | 0x7C98
        | 0x7D03 | 0x7D04 | 0x7D06..=0x7D09 | 0x7D13..=0x7D15 | 0x7D17..=0x7D20 | 0x7D25 | 0x7D27..=0x7D33
        | 0x7D36..=0x7D38 | 0x7D40..=0x7D43 | 0x7D46 | 0x7D50..=0x7D54 | 0x7D59 | 0x7D67 | 0x7D69 | 0x7D70
        | 0x7D73..=0x7D78 | 0x7D86 | 0x7D88..=0x7D91 | 0x7D93 | 0x7D96..=0x7D99)
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != 0x1462 || !is_supported_pid(pid) {
        return None;
    }
    let zones = ZONES.iter().map(|(n, _)| ZoneInfo::fixed(*n, 1)).collect();
    let mut m = Model::new("MSI Mystic Light (carte mère)", DeviceType::Motherboard, zones);
    m.usage_page = Some(0x0001);
    Some(m)
}

struct Mystic {
    hid: Box<dyn Hid>,
    /// Dernier état envoyé (ou lu à l'ouverture), `save_data` = 0.
    state: [u8; LEN],
    zones: Vec<ZoneInfo>,
}

impl NativeRgb for Mystic {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let parts = split_by_zone(colors, &self.zones);
        let mut next = self.state;
        for ((_, off), part) in ZONES.iter().zip(&parts) {
            let c = part[0];
            next[*off..*off + 4].copy_from_slice(&[EFFECT_STATIC, c.r, c.g, c.b]);
        }
        if next == self.state {
            return Ok(());
        }
        self.hid.send_feature_report(&self.state)?;
        self.hid.send_feature_report(&next)?;
        self.state = next;
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.zones.len() != ZONES.len() {
        bail!("MSI Mystic : disposition de modèle inattendue");
    }
    let mut buf = [0u8; 200];
    buf[0] = REPORT_ID;
    let n = hid.get_feature_report(&mut buf)?;
    if n != LEN || buf[0] != REPORT_ID {
        bail!("MSI Mystic : rapport 0x52 de {n} octets, seule la variante 185 octets est supportée");
    }
    let mut state = [0u8; LEN];
    state.copy_from_slice(&buf[..LEN]);
    state[SAVE_OFFSET] = 0;
    Ok(Box::new(Mystic { hid, state, zones: model.zones.clone() }))
}

pub const FAMILY: Family = Family { id: "msi_mystic", doc: "docs/rgb-protocols/msi-mystic-light.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn state185() -> Vec<u8> {
        let mut v = vec![0xAA; LEN];
        v[0] = 0x52;
        v[SAVE_OFFSET] = 1; // la carte annonce save=1 : on ne doit jamais le renvoyer
        v
    }

    #[test]
    fn probe_claims_only_185_candidates() {
        assert!(probe(0x1462, 0x7C95).is_some());
        assert!(probe(0x1462, 0x7D99).is_some());
        assert!(probe(0x1462, 0x7C80).is_some());
        assert!(probe(0x1462, 0x7E01).is_none()); // 761
        assert!(probe(0x1462, 0x7C42).is_none()); // « untested »
        assert!(probe(0x1462, 0x1720).is_none()); // 162
        assert!(probe(0x0DB0, 0x0076).is_none());
        assert!(probe(0x1463, 0x7C95).is_none());
    }

    #[test]
    fn open_refuses_other_report_lengths() {
        let m = probe(0x1462, 0x7C95).unwrap();
        for len in [162usize, 112, 186, 0] {
            let mut r = vec![0u8; len];
            if len > 0 {
                r[0] = 0x52;
            }
            let (mock, _, _) = MockHid::new(vec![r]);
            assert!(open(Box::new(mock), &m).is_err(), "len {len}");
        }
        let mut wrong_id = state185();
        wrong_id[0] = 0x50;
        let (mock, _, _) = MockHid::new(vec![wrong_id]);
        assert!(open(Box::new(mock), &m).is_err());
    }

    #[test]
    fn set_colors_sends_old_then_new_state_without_save() {
        let (mock, _, features) = MockHid::new(vec![state185()]);
        let m = probe(0x1462, 0x7C95).unwrap();
        let mut d = open(Box::new(mock), &m).unwrap();
        let colors: Vec<Color> = (1..=8).map(|i| Color::new(i, i + 10, i + 20)).collect();
        d.set_colors(&colors).unwrap();
        let f = features.lock();
        assert_eq!(f.len(), 2);
        let mut old = vec![0xAA; LEN];
        old[0] = 0x52;
        old[184] = 0;
        assert_eq!(f[0], old);
        let mut new = old.clone();
        // JRGB1@1, JPIPE1@11, JPIPE2@21, JRAINBOW1@31, JRAINBOW2@42, JCORSAIR@53, LED carte@74, JRGB2@174
        for (i, off) in [1usize, 11, 21, 31, 42, 53, 74, 174].iter().enumerate() {
            let i = i as u8 + 1;
            new[*off..*off + 4].copy_from_slice(&[1, i, i + 10, i + 20]);
        }
        assert_eq!(f[1], new);
        assert_eq!(f[1].len(), 185);
    }

    #[test]
    fn unchanged_colors_write_nothing() {
        let (mock, _, features) = MockHid::new(vec![state185()]);
        let m = probe(0x1462, 0x7C95).unwrap();
        let mut d = open(Box::new(mock), &m).unwrap();
        let colors = vec![Color::new(3, 4, 5); 8];
        d.set_colors(&colors).unwrap();
        d.set_colors(&colors).unwrap();
        assert_eq!(features.lock().len(), 2);
    }
}
