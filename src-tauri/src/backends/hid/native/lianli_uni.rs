//! Famille « lianli_uni » : hubs ENE 6K77 Uni Fan SL (VID 0x0CF2, PID 7750, A100), SL V2 (A103) et SL V2 « V2A » (A105). Rapport 0xE0.
//! Source : docs/rgb-protocols/lianli.md (section « Famille ENE 6K77 »).
//!
//! - RGB : couleurs en output report `E0 <0x30|groupe>` + triplets R,B,G ; effet statique en feature `E0 <0x10|groupe> 01 00 00 00`
//!   (mode 01 = statique, vitesse normale, direction G->D, luminosité 100 %) ; commit `E0 60 00 <1|4>` (4 pour SL V2/V2A).
//!   Ordre d'envoi = ordre de la note (couleurs, effet, commit) ; INCONNU si le matériel l'exige ainsi.
//! - Ventilateurs : `set_fan_duty` REFUSE. La note ne tranche pas entre deux formules de consigne (pourcentage direct d'après lian-li-linux,
//!   octet calculé d'après liquidctl) ; `Model.fans` = 0. Un test matériel avec lecture RPM tranchera.
//! - Non revendiqués : AL (A101), SL-Infinity (A102), AL V2 (A104) (tables de modes intérieur/extérieur non recopiées dans la note),
//!   SL Redragon (A106, modes non documentés), Strimer (A200, octets INCONNUS), TL/Trinity/sans-fil (autres familles).
//! - INCONNU : longueur exacte des output reports (envoyés sans remplissage), déclaration du nombre de ventilos (`E0 10 <sub> ...`, non envoyée :
//!   3 ventilos par groupe supposés, valeur par défaut de la note), retour au mode matériel (`release` ne fait rien).

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::Duration;

const VID: u16 = 0x0CF2;
const REPORT_ID: u8 = 0xE0;
const GROUPS: usize = 4;
const LEDS_PER_FAN: u32 = 16;
const DEFAULT_FANS: u32 = 3;
const CMD_DELAY: Duration = Duration::from_millis(20);
const MODE_STATIC: u8 = 0x01;

/// (nom, ventilos max par groupe, valeur de commit)
fn variant(pid: u16) -> Option<(&'static str, u32, u8)> {
    match pid {
        0x7750 | 0xA100 => Some(("Lian Li Uni Hub SL", 4, 1)),
        0xA103 => Some(("Lian Li Uni Hub SL V2", 6, 4)),
        0xA105 => Some(("Lian Li Uni Hub SL V2 (V2A)", 6, 4)),
        _ => None,
    }
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (name, max_fans, _) = variant(pid)?;
    let zones = (1..=GROUPS)
        .map(|g| ZoneInfo {
            name: format!("Groupe {g}"),
            led_count: DEFAULT_FANS * LEDS_PER_FAN,
            zone_type: 1,
            leds_min: LEDS_PER_FAN,
            leds_max: max_fans * LEDS_PER_FAN,
        })
        .collect();
    let mut m = Model::new(name, DeviceType::Fan, zones);
    m.usage_page = Some(0xFF72);
    Some(m)
}

struct UniHub {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
    commit: u8,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    // Le PID n'est pas dans `Model` : la valeur de commit se retrouve par le nom (même table que `variant`).
    let commit = [0x7750u16, 0xA103, 0xA105]
        .iter()
        .filter_map(|p| variant(*p))
        .find(|(n, _, _)| *n == model.name)
        .map(|(_, _, c)| c);
    match commit {
        Some(commit) => Ok(Box::new(UniHub { hid, zones: model.zones.clone(), commit })),
        None => bail!("Uni Hub : modèle « {} » inconnu du driver", model.name),
    }
}

impl NativeRgb for UniHub {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        for (g, leds) in split_by_zone(colors, &self.zones).iter().enumerate() {
            let group = g as u8;
            let mut out = vec![REPORT_ID, 0x30 | group];
            for c in leds {
                out.extend_from_slice(&[c.r, c.b, c.g]); // ordre R,B,G
            }
            self.hid.write(&out)?;
            std::thread::sleep(CMD_DELAY);
            self.hid.send_feature_report(&[REPORT_ID, 0x10 | group, MODE_STATIC, 0x00, 0x00, 0x00])?;
            std::thread::sleep(CMD_DELAY);
        }
        self.hid.send_feature_report(&[REPORT_ID, 0x60, 0x00, self.commit])
    }

    fn set_fan_duty(&mut self, _channel: u8, _percent: u8) -> Result<()> {
        bail!("Uni Hub : pilotage des ventilateurs désactivé, la note lianli.md ne tranche pas entre la consigne en % (lian-li-linux) et l'octet calculé (liquidctl) ; à valider sur matériel")
    }
}

pub const FAMILY: Family = Family { id: "lianli_uni", doc: "docs/rgb-protocols/lianli.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn dev(pid: u16) -> (Box<dyn NativeRgb>, super::super::Log, super::super::Log) {
        let (hid, sent, feats) = MockHid::new(vec![]);
        (open(Box::new(hid), &probe(VID, pid).unwrap()).unwrap(), sent, feats)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        for pid in [0x7750, 0xA100, 0xA103, 0xA105] {
            let m = probe(VID, pid).unwrap();
            assert_eq!((m.fans, m.zones.len(), m.led_count(), m.usage_page), (0, 4, 192, Some(0xFF72)));
        }
        for pid in [0xA101, 0xA102, 0xA104, 0xA106, 0xA200] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x1234, 0xA100).is_none());
    }

    #[test]
    fn colors_are_rbg_then_effect_then_commit_sl() {
        let (mut d, sent, feats) = dev(0xA100);
        let mut cols = vec![Color::BLACK; 192];
        cols[0] = Color::new(1, 2, 3);
        cols[48] = Color::new(7, 8, 9); // groupe 2
        d.set_colors(&cols).unwrap();
        let s = sent.lock();
        let f = feats.lock();
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].len(), 2 + 48 * 3);
        assert_eq!(&s[0][..5], &[0xE0, 0x30, 1, 3, 2]); // R,B,G
        assert_eq!(&s[1][..5], &[0xE0, 0x31, 7, 9, 8]);
        assert_eq!(s[3][1], 0x33);
        assert_eq!(f.len(), 5);
        assert_eq!(f[0], [0xE0, 0x10, 0x01, 0, 0, 0]);
        assert_eq!(f[3], [0xE0, 0x13, 0x01, 0, 0, 0]);
        assert_eq!(f[4], [0xE0, 0x60, 0x00, 0x01]);
    }

    #[test]
    fn v2_commit_value_is_4() {
        let (mut d, _, feats) = dev(0xA103);
        d.set_colors(&[Color::new(1, 1, 1)]).unwrap();
        assert_eq!(feats.lock().last().unwrap(), &[0xE0, 0x60, 0x00, 0x04]);
    }

    #[test]
    fn fan_duty_is_refused_with_explicit_message() {
        let (mut d, sent, feats) = dev(0xA100);
        let e = d.set_fan_duty(0, 50).unwrap_err().to_string();
        assert!(e.contains("ne tranche pas"));
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }
}
