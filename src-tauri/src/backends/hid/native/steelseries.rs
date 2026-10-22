//! Famille « steelseries » : Apex Pro TKL 2023 filaire (trame directe 0x40) et Aerox 3 (trame couleur 0x21 par zone).
//! Source : docs/rgb-protocols/steelseries.md (sections « Claviers Apex » et « Souris »).
//!
//! INCONNU (donc refusé / non revendiqué) :
//! - Apex 3 / 3 TKL / 5 / 7 / Pro / Pro TKL (1614) / Apex 9 / M750 / OG : génération (0x3A Gen1, 0x40 Gen2, Gen3) dépendante de la version
//!   firmware, dont le format de réponse (id 0x90) n'est pas décrit ; Apex 3 et M750 ont un contrôleur « NON lu » ;
//! - Apex sans fil (1630 / 1632 / 1644 / 1646) : longueur du rapport direct (id 0x61) non précisée ; Apex Pro TKL Gen3 filaire (1642) : id direct non précisé ;
//! - Rival 3 (valeurs d'effet et tailles de tampon inconnues), Rival 5/600, Aerox 5/9 (contrôleurs « NON lus »), tapis QCK, casques ;
//! - table exacte des codes touche Apex (~106, « non transcrite ») : LED i -> usages HID 0x04-0x64 puis 0xE0-0xE7 (approximation), les codes
//!   absents du clavier sont envoyés tels quels ; taille du rapport de reset 0x41 (65 octets supposés comme l'init) ;
//! - commandes de sauvegarde (0x09 Rival, 0x11 Aerox) : jamais envoyées ; luminosité (0x23, plage Aerox inconnue) : non pilotée.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x1038;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// Apex Pro TKL 2023 filaire : init 0x4B, direct 0x40, reset 0x41.
    Apex2023,
    /// Aerox 3 : paquet 65 octets, commande 0x21 avec masque de zone.
    Aerox3,
}

const APEX_PACKET_LEN: usize = 643;
const APEX_CTRL_LEN: usize = 65;
const APEX_INIT: u8 = 0x4B;
const APEX_DIRECT: u8 = 0x40;
const APEX_RESET: u8 = 0x41;
const APEX_LEDS: usize = 0x61 + 8;

const AEROX_LEN: usize = 65;
const AEROX_COLOR: u8 = 0x21;
const AEROX_ZONES: usize = 3;

fn apex_keys() -> impl Iterator<Item = u8> {
    (0x04u8..=0x64).chain(0xE0..=0xE7)
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    match pid {
        0x1628 => {
            let mut m = Model::new("SteelSeries Apex Pro TKL 2023", DeviceType::Keyboard, vec![ZoneInfo::fixed("Touches (codes HID)", APEX_LEDS as u32)]);
            m.interface = Some(1);
            Some(m)
        }
        0x1836 | 0x1838 | 0x183A | 0x1878 | 0x187A => {
            let zones = (1..=AEROX_ZONES).map(|i| ZoneInfo::fixed(format!("Zone {i}"), 1)).collect();
            let mut m = Model::new("SteelSeries Aerox 3", DeviceType::Mouse, zones);
            m.interface = Some(3);
            m.usage_page = Some(0xFFC0);
            Some(m)
        }
        _ => None,
    }
}

struct SteelSeries {
    hid: Box<dyn Hid>,
    kind: Kind,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let kind = match model.device_type {
        DeviceType::Keyboard => Kind::Apex2023,
        DeviceType::Mouse => Kind::Aerox3,
        other => bail!("SteelSeries : type {other:?} non pris en charge"),
    };
    if kind == Kind::Apex2023 {
        let mut init = [0u8; APEX_CTRL_LEN];
        init[1] = APEX_INIT;
        hid.send_feature_report(&init)?;
    }
    Ok(Box::new(SteelSeries { hid, kind }))
}

impl NativeRgb for SteelSeries {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        match self.kind {
            Kind::Apex2023 => {
                if colors.len() > APEX_LEDS {
                    bail!("SteelSeries : {} couleurs pour {APEX_LEDS} touches", colors.len());
                }
                let mut f = vec![0u8; APEX_PACKET_LEN];
                f[1] = APEX_DIRECT;
                f[2] = APEX_LEDS as u8;
                for (i, k) in apex_keys().enumerate() {
                    let c = colors.get(i).copied().unwrap_or(Color::BLACK);
                    f[3 + 4 * i..7 + 4 * i].copy_from_slice(&[k, c.r, c.g, c.b]);
                }
                self.hid.send_feature_report(&f)
            }
            Kind::Aerox3 => {
                if colors.len() > AEROX_ZONES {
                    bail!("SteelSeries : {} couleurs pour {AEROX_ZONES} zones", colors.len());
                }
                for (z, c) in colors.iter().enumerate() {
                    let mut f = [0u8; AEROX_LEN];
                    f[1] = AEROX_COLOR;
                    f[2] = 1 << z;
                    f[3 + 3 * z..6 + 3 * z].copy_from_slice(&[c.r, c.g, c.b]);
                    self.hid.write(&f)?;
                }
                Ok(())
            }
        }
    }

    fn release(&mut self) {
        // Aerox : aucun retour au mode matériel documenté.
        if self.kind == Kind::Apex2023 {
            let mut reset = [0u8; APEX_CTRL_LEN];
            reset[1] = APEX_RESET;
            // Échec de fermeture : rien à rattraper, l'appareil reprend son mode au débranchement.
            let _ = self.hid.send_feature_report(&reset);
        }
    }
}

pub const FAMILY: Family = Family { id: "steelseries", doc: "docs/rgb-protocols/steelseries.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Log = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn driver(pid: u16) -> (Box<dyn NativeRgb>, Log, Log) {
        let model = probe(VID, pid).expect("pid");
        let (mock, sent, feats) = MockHid::new(vec![]);
        (open(Box::new(mock), &model).expect("open"), sent, feats)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert_eq!(probe(VID, 0x1628).map(|m| m.led_count()), Some(105));
        for pid in [0x1836, 0x1838, 0x183A, 0x1878, 0x187A] {
            let m = probe(VID, pid).expect("pid");
            assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFFC0), 3));
        }
        for pid in [0x161A, 0x1612, 0x1614, 0x1630, 0x1642, 0x184C, 0x1850, 0x1724, 0x150A] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x046D, 0x1628).is_none());
    }

    #[test]
    fn apex_init_direct_frame_and_reset() {
        let (mut d, sent, feats) = driver(0x1628);
        let mut init = vec![0u8; 65];
        init[1] = 0x4B;
        assert_eq!(feats.lock()[0], init);
        let mut c = vec![Color::BLACK; 105];
        c[0] = Color::new(1, 2, 3);
        c[1] = Color::new(4, 5, 6);
        c[104] = Color::new(7, 8, 9);
        d.set_colors(&c).expect("set");
        let f = feats.lock()[1].clone();
        assert_eq!(f.len(), 643);
        assert_eq!(&f[..11], &[0x00, 0x40, 105, 0x04, 1, 2, 3, 0x05, 4, 5, 6]);
        assert_eq!(&f[3 + 4 * 104..7 + 4 * 104], &[0xE7, 7, 8, 9]);
        assert!(f[3 + 4 * 105..].iter().all(|b| *b == 0));
        assert!(sent.lock().is_empty()); // couleurs en feature report, jamais en write
        d.release();
        let mut reset = vec![0u8; 65];
        reset[1] = 0x41;
        assert_eq!(feats.lock()[2], reset);
    }

    #[test]
    fn apex_rejects_too_many_colors_without_sending() {
        let (mut d, _, feats) = driver(0x1628);
        assert!(d.set_colors(&vec![Color::BLACK; 106]).is_err());
        assert_eq!(feats.lock().len(), 1); // seulement l'init
    }

    #[test]
    fn aerox_one_packet_per_zone_with_mask_and_offset() {
        let (mut d, sent, feats) = driver(0x1836);
        d.set_colors(&[Color::new(10, 20, 30), Color::new(40, 50, 60), Color::new(70, 80, 90)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 3);
        for (z, rgb) in [[10u8, 20, 30], [40, 50, 60], [70, 80, 90]].iter().enumerate() {
            let mut want = vec![0u8; 65];
            want[1] = 0x21;
            want[2] = 1 << z;
            want[3 + 3 * z..6 + 3 * z].copy_from_slice(rgb);
            assert_eq!(s[z], want);
        }
        assert!(feats.lock().is_empty()); // pas d'init, pas de sauvegarde (0x11)
    }

    #[test]
    fn aerox_rejects_extra_zone_and_release_sends_nothing() {
        let (mut d, sent, feats) = driver(0x1836);
        assert!(d.set_colors(&[Color::BLACK; 4]).is_err());
        d.release();
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }
}
