//! Famille « litra » : Logitech Litra Glow (C900), Beam (C901, B901) et Beam LX (C903) : lumière blanche (marche, luminosité, température)
//! et, sur la Beam LX seulement, 7 zones couleur sur la face arrière. Tout est en RAM de l'appareil, réversible.
//! Source : docs/rgb-protocols/marques-groupe-c.md ; projet public litra-rs (MIT, `src/lib.rs`, consulté 2026-10-26), réimplémenté
//! d'après les faits de format (aucun code copié).
//!
//! Lumière blanche : rapports 20 octets `11 FF <idx> <cmd> …`, idx `04` (Glow/Beam) ou `06` (Beam LX) : marche `1C <0|1>`, luminosité `4C <u16 lumens
//! grand-boutiste>` (Glow 20..250, Beam/LX 30..400), température `9C <u16 K grand-boutiste>` (2700..6500, multiples de 100).
//! Correspondance couleur -> blanc (PureRGB ne connaît que des couleurs) : noir = éteint ; sinon luminosité = max(R,V,B) rapportée à la plage de
//! lumens, température = 2700 K + 3800 K x B/(R+B) arrondie à 100 K (rouge = chaud, bleu = froid, blanc = 4600 K).
//!
//! Face arrière (Beam LX) : `11 FF 0C 1B <zone 1..7> R G B FF 00 00 00 FF 00 00 00 FF 00 00 00`, puis validation `11 FF 0C 7B 00 00 01 00 00`
//! (9 octets chez litra-rs ; complétée de zéros jusqu'à 20). La source ramène chaque composante à 1 minimum (« l'appareil se comporte mal à 0 »).
//! Collection HID : page d'usage 0xFF43. Non touchés : luminosité/marche de la face arrière.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceSetting, DeviceType, SettingKind, ZoneInfo};
use anyhow::{bail, Result};

const VID_LOGITECH: u16 = 0x046D;
const PID_BEAM_LX: u16 = 0xC903;
const ZONES: usize = 7;
const REPORT_LEN: usize = 20;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Glow,
    Beam,
    BeamLx,
}

impl Kind {
    /// Octet d'indice de fonction des rapports de lumière blanche.
    fn idx(self) -> u8 {
        if self == Kind::BeamLx {
            0x06
        } else {
            0x04
        }
    }

    /// Plage de luminosité en lumens.
    fn lumens(self) -> (u16, u16) {
        if self == Kind::Glow {
            (20, 250)
        } else {
            (30, 400)
        }
    }
}

fn kind_of(model_name: &str) -> Kind {
    if model_name.ends_with("LX") {
        Kind::BeamLx
    } else if model_name.ends_with("Glow") {
        Kind::Glow
    } else {
        Kind::Beam
    }
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    let (name, back) = match (vid, pid) {
        (VID_LOGITECH, 0xC900) => ("Logitech Litra Glow", false),
        (VID_LOGITECH, 0xC901 | 0xB901) => ("Logitech Litra Beam", false),
        (VID_LOGITECH, PID_BEAM_LX) => ("Logitech Litra Beam LX", true),
        _ => return None,
    };
    let mut zones = vec![ZoneInfo::fixed("Lumière blanche", 1)];
    if back {
        zones.push(ZoneInfo::fixed("Face arrière", ZONES as u32));
    }
    let mut m = Model::new(name, DeviceType::Light, zones);
    m.usage_page = Some(0xFF43);
    Some(m)
}

struct Litra {
    hid: Box<dyn Hid>,
    kind: Kind,
    last_white: Option<(bool, u16, u16)>,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Litra { hid, kind: kind_of(&model.name), last_white: None }))
}

fn white_report(idx: u8, cmd: u8, hi: u8, lo: u8) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[..6].copy_from_slice(&[0x11, 0xFF, idx, cmd, hi, lo]);
    r
}

/// Couleur -> (allumée, lumens, kelvins) ; voir l'en-tête du fichier.
fn white_of(kind: Kind, c: Color) -> (bool, u16, u16) {
    let (lo, hi) = kind.lumens();
    let m = u32::from(c.r.max(c.g).max(c.b));
    let lumens = lo + ((u32::from(hi - lo) * m + 127) / 255) as u16;
    let (r, b) = (u32::from(c.r), u32::from(c.b));
    let k = if r + b == 0 { 4600 } else { 2700 + 3800 * b / (r + b) };
    (m > 0, lumens, ((k + 50) / 100 * 100).clamp(2700, 6500) as u16)
}

/// Réglages de la lumière blanche (RAM de l'appareil, rien n'est envoyé avant une action de l'utilisateur).
pub fn settings(model: &Model) -> Vec<DeviceSetting> {
    let (lo, hi) = kind_of(&model.name).lumens();
    let slider = |min, max, step, unit: &str| SettingKind::Slider { min, max, step, unit: unit.into() };
    vec![
        DeviceSetting { id: "white_on".into(), label: "Lumière allumée".into(), kind: SettingKind::Toggle, default: 1 },
        DeviceSetting { id: "white_lumens".into(), label: "Luminosité".into(), kind: slider(i32::from(lo), i32::from(hi), 1, "lm"), default: i32::from(lo) },
        DeviceSetting { id: "white_kelvin".into(), label: "Température de couleur".into(), kind: slider(2700, 6500, 100, "K"), default: 4600 },
    ]
}

fn zone_report(zone: u8, c: Color) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[..4].copy_from_slice(&[0x11, 0xFF, 0x0C, 0x1B]);
    r[4] = zone;
    r[5..8].copy_from_slice(&[c.r.max(1), c.g.max(1), c.b.max(1)]);
    r[8] = 0xFF;
    r[12] = 0xFF;
    r[16] = 0xFF;
    r
}

fn commit_report() -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[..9].copy_from_slice(&[0x11, 0xFF, 0x0C, 0x7B, 0, 0, 1, 0, 0]);
    r
}

impl NativeRgb for Litra {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let max = if self.kind == Kind::BeamLx { 1 + ZONES } else { 1 };
        if colors.len() > max {
            bail!("Litra : {} couleurs pour {max} LED", colors.len());
        }
        let white = white_of(self.kind, colors.first().copied().unwrap_or(Color::BLACK));
        if self.last_white != Some(white) {
            let (on, lumens, kelvin) = white;
            let idx = self.kind.idx();
            self.hid.write(&white_report(idx, 0x1C, u8::from(on), 0))?;
            if on {
                self.hid.write(&white_report(idx, 0x4C, (lumens >> 8) as u8, lumens as u8))?;
                self.hid.write(&white_report(idx, 0x9C, (kelvin >> 8) as u8, kelvin as u8))?;
            }
            self.last_white = Some(white);
        }
        if self.kind == Kind::BeamLx {
            for zone in 0..ZONES {
                let c = colors.get(1 + zone).copied().unwrap_or(Color::BLACK);
                self.hid.write(&zone_report(zone as u8 + 1, c))?;
            }
            self.hid.write(&commit_report())?;
        }
        Ok(())
    }

    fn set_setting(&mut self, id: &str, value: i32) -> Result<()> {
        let idx = self.kind.idx();
        let (lo, hi) = self.kind.lumens();
        let report = match id {
            "white_on" => white_report(idx, 0x1C, u8::from(value != 0), 0),
            "white_lumens" => {
                let v = u16::try_from(value).ok().filter(|v| (lo..=hi).contains(v));
                let Some(v) = v else { bail!("luminosité {value} lm hors plage {lo}..{hi}") };
                white_report(idx, 0x4C, (v >> 8) as u8, v as u8)
            }
            "white_kelvin" => {
                let v = u16::try_from(value).ok().filter(|v| (2700..=6500).contains(v) && v % 100 == 0);
                let Some(v) = v else { bail!("température {value} K invalide (2700..6500, multiples de 100)") };
                white_report(idx, 0x9C, (v >> 8) as u8, v as u8)
            }
            _ => bail!("réglage inconnu : {id}"),
        };
        self.hid.write(&report)?;
        self.last_white = None; // l'état réel a changé : la prochaine couleur réécrit le blanc
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "litra", doc: "docs/rgb-protocols/marques-groupe-c.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    fn driver(pid: u16) -> (Box<dyn NativeRgb>, super::super::Log) {
        let model = probe(VID_LOGITECH, pid).expect("pid");
        let (mock, sent, _) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent)
    }

    #[test]
    fn probe_claims_the_four_pids_and_only_lx_has_a_back_zone() {
        for pid in [0xC900, 0xC901, 0xB901] {
            assert_eq!(probe(VID_LOGITECH, pid).expect("blanc").led_count(), 1);
        }
        assert_eq!(probe(VID_LOGITECH, 0xC903).expect("lx").led_count(), 8);
        assert!(probe(VID_LOGITECH, 0xC902).is_none());
    }

    #[test]
    fn white_mapping_follows_the_documented_ranges() {
        assert_eq!(white_of(Kind::Glow, Color::new(255, 255, 255)), (true, 250, 4600));
        assert_eq!(white_of(Kind::Beam, Color::new(255, 0, 0)), (true, 400, 2700));
        assert_eq!(white_of(Kind::Beam, Color::new(0, 0, 255)), (true, 400, 6500));
        assert_eq!(white_of(Kind::Glow, Color::new(1, 1, 1)).1, 21);
        assert!(!white_of(Kind::Glow, Color::BLACK).0);
    }

    #[test]
    fn glow_sends_on_brightness_temperature_once_and_off_on_black() {
        let (mut d, sent) = driver(0xC900);
        d.set_colors(&[Color::new(255, 255, 255)]).expect("set");
        d.set_colors(&[Color::new(255, 255, 255)]).expect("même valeur : rien");
        {
            let s = sent.lock();
            assert_eq!(s.len(), 3);
            assert_eq!(&s[0][..6], &[0x11, 0xFF, 0x04, 0x1C, 1, 0]);
            assert_eq!(&s[1][..6], &[0x11, 0xFF, 0x04, 0x4C, 0, 250]);
            assert_eq!(&s[2][..6], &[0x11, 0xFF, 0x04, 0x9C, 0x11, 0xF8]); // 4600 K
            assert!(s.iter().all(|r| r.len() == 20));
        }
        d.set_colors(&[Color::BLACK]).expect("off");
        let s = sent.lock();
        assert_eq!((s.len(), &s[3][..6]), (4, &[0x11, 0xFF, 0x04, 0x1C, 0, 0][..]));
    }

    #[test]
    fn beam_lx_uses_idx_6_and_seven_back_zones_with_floor_of_one() {
        let (mut d, sent) = driver(0xC903);
        d.set_colors(&[Color::new(255, 255, 255), Color::new(0xAA, 0, 0x10)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 3 + 8, "3 blanc + 7 zones + validation");
        assert_eq!(&s[0][..4], &[0x11, 0xFF, 0x06, 0x1C]);
        let mut z1 = vec![0x11, 0xFF, 0x0C, 0x1B, 1, 0xAA, 1, 0x10, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0];
        assert_eq!(s[3], z1);
        z1[4] = 2;
        z1[5..8].copy_from_slice(&[1, 1, 1]);
        assert_eq!(s[4], z1);
        assert_eq!(&s[10][..9], &[0x11, 0xFF, 0x0C, 0x7B, 0, 0, 1, 0, 0]);
    }

    #[test]
    fn settings_write_one_report_each_and_reject_bad_values() {
        let (mut d, sent) = driver(0xC901);
        d.set_setting("white_lumens", 400).expect("lm");
        d.set_setting("white_kelvin", 6500).expect("K");
        d.set_setting("white_on", 0).expect("off");
        assert!(d.set_setting("white_lumens", 29).is_err());
        assert!(d.set_setting("white_kelvin", 4650).is_err());
        assert!(d.set_setting("autre", 1).is_err());
        let s = sent.lock();
        assert_eq!(s.len(), 3);
        assert_eq!(&s[0][..6], &[0x11, 0xFF, 0x04, 0x4C, 0x01, 0x90]);
        assert_eq!(&s[1][..6], &[0x11, 0xFF, 0x04, 0x9C, 0x19, 0x64]);
        assert_eq!(&s[2][..6], &[0x11, 0xFF, 0x04, 0x1C, 0, 0]);
        let model = probe(VID_LOGITECH, 0xC900).expect("glow");
        assert_eq!(settings(&model).len(), 3);
    }

    #[test]
    fn too_many_colors_are_refused() {
        let (mut d, sent) = driver(0xC903);
        assert!(d.set_colors(&[Color::BLACK; 9]).is_err());
        let (mut g, _) = driver(0xC900);
        assert!(g.set_colors(&[Color::BLACK; 2]).is_err());
        assert!(sent.lock().is_empty());
    }
}
