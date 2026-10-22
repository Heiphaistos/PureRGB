//! Famille « steelseries » : claviers Apex, souris Rival 3 / Rival 5 / Aerox 3-5-9 filaires, tapis QCK.
//! Sources : docs/rgb-protocols/steelseries.md et docs/rgb-protocols/analyse-steelseries-gg.md (table des touches : `layout.bin`
//! du logiciel officiel ; trames : source tierce, à valider sur matériel).
//!
//! Refusés (non revendiqués, aucun octet envoyé) car INCONNU :
//! - Apex Pro TKL 1614 : livré en firmware 4.16.8, la règle « Gen3 si version >= 1.19.7 » le classerait Gen3 mais la note
//!   signale que le matériel n'est pas vérifié ; Apex Pro 2024 (1640), Apex 5 2024 (1650), Apex 7 2024 (1652), Pro Mini
//!   (161E, 1648), Apex 3 / 3 TKL / M750 / OG : génération non donnée par la note ;
//! - Apex sans fil (1630 / 1632 / 1644 / 1646) : longueur du rapport direct non confirmée, transport sans fil non décrit ;
//! - Aerox / Rival sans fil 2,4 GHz (dongles 1838, 1878, 1858, 1852, 185C, 1860, Rival 3 Wireless 1830) : le OU 0x40 est
//!   décrit mais pas la taille ni le contenu de l'accusé de lecture ;
//! - Rival 600 (longueur 7 octets avec ou sans identifiant de rapport, zone de la molette), Rival 650 (séquence de quatre
//!   écritures, zones partielles), Rival 700 (identifiants de zone non donnés) ; zone « Reactive » des Aerox (octet [2] de la
//!   commande 0x26 non donné) ;
//! - table des touches des Apex 9 / Pro TKL : pas de `layout.bin`, la table des 114 codes confirmée pour les autres Apex est
//!   réutilisée (le nombre de touches envoyé tient dans 513 octets) ; 0xFB (Apex Pro 2024 seulement) non utilisé.
//!
//! Jamais envoyé : commandes de sauvegarde (0x09 Rival, 0x11 Aerox), luminosité, profil matériel.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::Duration;

const VID: u16 = 0x1038;
const CTRL_LEN: usize = 65;
const APEX_INIT: u8 = 0x4B;
const BRIGHTNESS_MAX: u8 = 0x64;
const WRITE_GAP: Duration = Duration::from_millis(2);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// Clavier : `gen1` = id direct 0x3A / reset 0x3B sans init ; sinon 0x40 / 0x41 avec init 0x4B. `len` = taille du rapport direct.
    Apex { gen1: bool, len: usize },
    /// Rival 3 : effet direct puis couleur par zone (écritures de 4 et 8 octets).
    Rival3,
    /// Rival 5 : un rapport feature de 64 octets pour les 10 zones.
    Rival5,
    /// Aerox 3 / 5 filaires : 65 octets, masque de zone.
    Aerox65,
    /// Aerox « sans fil » en mode filaire : 64 octets, zone en octet 3.
    Aerox64,
    /// Tapis QCK : rapport feature de 525 octets puis validation de 65.
    Qck,
}

struct Entry {
    pids: &'static [u16],
    name: &'static str,
    device_type: DeviceType,
    kind: Kind,
    zones: &'static [&'static str],
    interface: i32,
    usage_page: Option<u16>,
}

const AEROX_ZONES: &[&str] = &["Avant", "Milieu", "Arrière"];

fn kb(pids: &'static [u16], name: &'static str, gen1: bool, len: usize) -> Entry {
    Entry { pids, name, device_type: DeviceType::Keyboard, kind: Kind::Apex { gen1, len }, zones: &[], interface: 1, usage_page: None }
}

fn aerox(pids: &'static [u16], name: &'static str, kind: Kind) -> Entry {
    Entry { pids, name, device_type: DeviceType::Mouse, kind, zones: AEROX_ZONES, interface: 3, usage_page: Some(0xFFC0) }
}

fn entries() -> Vec<Entry> {
    vec![
        kb(&[0x1610], "SteelSeries Apex Pro", true, 643),
        kb(&[0x1612], "SteelSeries Apex 7", true, 643),
        kb(&[0x1618], "SteelSeries Apex 7 TKL", true, 643),
        kb(&[0x161C], "SteelSeries Apex 5", true, 643),
        kb(&[0x1628], "SteelSeries Apex Pro TKL (2022)", false, 643),
        kb(&[0x1642], "SteelSeries Apex Pro TKL (2024)", false, 643),
        kb(&[0x1634], "SteelSeries Apex 9 TKL", false, 513),
        kb(&[0x1620], "SteelSeries Apex 9 Mini", false, 513),
        Entry {
            pids: &[0x1824, 0x184C],
            name: "SteelSeries Rival 3",
            device_type: DeviceType::Mouse,
            kind: Kind::Rival3,
            zones: &["Avant", "Milieu", "Arrière", "Logo"],
            interface: 3,
            usage_page: None,
        },
        Entry {
            pids: &[0x183C],
            name: "SteelSeries Rival 5",
            device_type: DeviceType::Mouse,
            kind: Kind::Rival5,
            zones: &["Molette", "Corps G1", "Corps D1", "Corps G2", "Corps D2", "Corps G3", "Corps D3", "Corps G4", "Corps D4", "Logo"],
            interface: 0,
            usage_page: Some(0xFFC0),
        },
        aerox(&[0x1836], "SteelSeries Aerox 3", Kind::Aerox65),
        aerox(&[0x1850], "SteelSeries Aerox 5", Kind::Aerox65),
        aerox(&[0x183A, 0x187A], "SteelSeries Aerox 3 Wireless (filaire)", Kind::Aerox64),
        aerox(&[0x1854, 0x185E, 0x1862], "SteelSeries Aerox 5 Wireless (filaire)", Kind::Aerox64),
        aerox(&[0x185A], "SteelSeries Aerox 9 Wireless (filaire)", Kind::Aerox64),
        Entry {
            pids: &[0x150A, 0x150D, 0x151E, 0x1514, 0x151C, 0x1516, 0x1518, 0x151A, 0x1520],
            name: "SteelSeries QCK Prism",
            device_type: DeviceType::Mousemat,
            kind: Kind::Qck,
            zones: &["Couleur A", "Couleur B"],
            interface: 0,
            usage_page: None,
        },
    ]
}

/// Usages HID des LEDs d'un Apex (table confirmée par `layout.bin`) : 114 codes.
fn apex_keys() -> impl Iterator<Item = u8> {
    (0x04u8..=0x64).chain([0x85]).chain(0x87..=0x8B).chain([0x90, 0x91]).chain(0xE0..=0xE7).chain([0xF0])
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let e = entries().into_iter().find(|e| e.pids.contains(&pid))?;
    let zones = match e.kind {
        Kind::Apex { .. } => vec![ZoneInfo::fixed("Touches (usages HID)", apex_keys().count() as u32)],
        _ => e.zones.iter().map(|n| ZoneInfo::fixed(*n, 1)).collect(),
    };
    let mut m = Model::new(e.name, e.device_type, zones);
    m.interface = Some(e.interface);
    m.usage_page = e.usage_page;
    Some(m)
}

struct SteelSeries {
    hid: Box<dyn Hid>,
    kind: Kind,
    /// Nombre de LEDs (Apex : touches) ou de zones acceptées par `set_colors`.
    capacity: usize,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let Some(e) = entries().into_iter().find(|e| e.name == model.name) else {
        bail!("SteelSeries : modèle « {} » non pris en charge", model.name);
    };
    match e.kind {
        Kind::Apex { gen1: false, .. } => {
            let mut init = [0u8; CTRL_LEN];
            init[1] = APEX_INIT;
            hid.send_feature_report(&init)?;
        }
        Kind::Rival3 => {
            hid.write(&[0x00, 0x06, 0x00, 0x04])?; // effet 0x04 = couleurs directes
        }
        _ => {}
    }
    Ok(Box::new(SteelSeries { hid, kind: e.kind, capacity: model.led_count() as usize }))
}

impl SteelSeries {
    /// Écrit une trame puis attend 2 ms (intervalle conseillé entre écritures de zone).
    fn write(&self, frame: &[u8]) -> Result<()> {
        self.hid.write(frame)?;
        std::thread::sleep(WRITE_GAP);
        Ok(())
    }

    fn set_apex(&self, gen1: bool, len: usize, colors: &[Color]) -> Result<()> {
        let mut f = vec![0u8; len];
        f[1] = if gen1 { 0x3A } else { 0x40 };
        f[2] = self.capacity as u8;
        for (i, k) in apex_keys().enumerate() {
            let c = colors.get(i).copied().unwrap_or(Color::BLACK);
            f[3 + 4 * i..7 + 4 * i].copy_from_slice(&[k, c.r, c.g, c.b]);
        }
        self.hid.send_feature_report(&f)
    }

    fn set_qck(&self, colors: &[Color]) -> Result<()> {
        let a = colors.first().copied().unwrap_or(Color::BLACK);
        let b = colors.get(1).copied().unwrap_or(Color::BLACK);
        let mut f = vec![0u8; 525];
        f[1] = 0x0E;
        f[3] = 0x02;
        f[5..8].copy_from_slice(&[a.r, a.g, a.b]);
        f[8..11].copy_from_slice(&[0xFF, 0x32, 0xC8]);
        f[14] = 1;
        f[0x11..0x14].copy_from_slice(&[b.r, b.g, b.b]);
        f[0x14..0x17].copy_from_slice(&[0xFF, 0x32, 0xC8]);
        f[0x19] = 1;
        f[0x1A] = 1;
        f[0x1C] = 1;
        self.hid.send_feature_report(&f)?;
        let mut apply = [0u8; CTRL_LEN];
        apply[1] = 0x0D; // validation de l'image (pas une sauvegarde en mémoire)
        self.hid.write(&apply)?;
        Ok(())
    }
}

impl NativeRgb for SteelSeries {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > self.capacity {
            bail!("SteelSeries : {} couleurs pour {} LED/zones", colors.len(), self.capacity);
        }
        match self.kind {
            Kind::Apex { gen1, len } => self.set_apex(gen1, len, colors),
            Kind::Qck => self.set_qck(colors),
            Kind::Rival5 => {
                let mut f = [0u8; 64];
                f[1] = 0x21;
                f[2] = 0xFF;
                f[3] = 0x03;
                for (i, c) in colors.iter().enumerate() {
                    f[4 + 3 * i..7 + 3 * i].copy_from_slice(&[c.r, c.g, c.b]);
                }
                self.hid.send_feature_report(&f)
            }
            Kind::Rival3 => {
                for (z, c) in colors.iter().enumerate() {
                    self.write(&[0x00, 0x05, 0x00, z as u8 + 1, c.r, c.g, c.b, BRIGHTNESS_MAX])?;
                }
                Ok(())
            }
            Kind::Aerox65 => {
                for (z, c) in colors.iter().enumerate() {
                    let mut f = [0u8; CTRL_LEN];
                    f[1] = 0x21;
                    f[2] = 1 << z;
                    f[3 + 3 * z..6 + 3 * z].copy_from_slice(&[c.r, c.g, c.b]);
                    self.write(&f)?;
                }
                Ok(())
            }
            Kind::Aerox64 => {
                for (z, c) in colors.iter().enumerate() {
                    let mut f = [0u8; 64];
                    f[1] = 0x21;
                    f[2] = 0x01;
                    f[3] = z as u8;
                    f[4..7].copy_from_slice(&[c.r, c.g, c.b]);
                    self.write(&f)?;
                }
                Ok(())
            }
        }
    }

    fn release(&mut self) {
        // Seuls les Apex documentent un retour au mode matériel.
        if let Kind::Apex { gen1, .. } = self.kind {
            let mut reset = [0u8; CTRL_LEN];
            reset[1] = if gen1 { 0x3B } else { 0x41 };
            // Échec de fermeture : rien à rattraper, l'appareil reprend son mode au débranchement.
            let _ = self.hid.write(&reset);
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

    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::new(r, g, b)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        let want: [(u16, &str, u32, i32); 12] = [
            (0x1610, "SteelSeries Apex Pro", 114, 1),
            (0x1612, "SteelSeries Apex 7", 114, 1),
            (0x1618, "SteelSeries Apex 7 TKL", 114, 1),
            (0x161C, "SteelSeries Apex 5", 114, 1),
            (0x1628, "SteelSeries Apex Pro TKL (2022)", 114, 1),
            (0x1642, "SteelSeries Apex Pro TKL (2024)", 114, 1),
            (0x1634, "SteelSeries Apex 9 TKL", 114, 1),
            (0x1620, "SteelSeries Apex 9 Mini", 114, 1),
            (0x1824, "SteelSeries Rival 3", 4, 3),
            (0x184C, "SteelSeries Rival 3", 4, 3),
            (0x183C, "SteelSeries Rival 5", 10, 0),
            (0x150A, "SteelSeries QCK Prism", 2, 0),
        ];
        for (pid, name, leds, itf) in want {
            let m = probe(VID, pid).unwrap_or_else(|| panic!("{pid:04x}"));
            assert_eq!((m.name.as_str(), m.led_count(), m.interface), (name, leds, Some(itf)), "{pid:04x}");
        }
        for pid in [0x1836, 0x1850, 0x183A, 0x187A, 0x1854, 0x185E, 0x1862, 0x185A] {
            let m = probe(VID, pid).expect("pid");
            assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFFC0), 3), "{pid:04x}");
        }
        for pid in [0x150D, 0x151E, 0x1514, 0x151C, 0x1516, 0x1518, 0x151A, 0x1520] {
            assert_eq!(probe(VID, pid).map(|m| m.device_type), Some(DeviceType::Mousemat), "{pid:04x}");
        }
        // Refusés : génération / longueur / accusé sans fil / zones INCONNUS, ou hors périmètre (casques).
        let refused = [
            0x1614, 0x1630, 0x1632, 0x1644, 0x1646, 0x1640, 0x1650, 0x1652, 0x161E, 0x1648, 0x161A, 0x1622, 0x0616, 0x1838, 0x1878, 0x1858, 0x1852,
            0x185C, 0x1860, 0x1830, 0x1724, 0x172E, 0x172B, 0x1726, 0x1700, 0x1730, 0x183E, 0x1890, 0x12EC, 0x1250,
        ];
        for pid in refused {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x046D, 0x1628).is_none());
    }

    #[test]
    fn open_refuses_unknown_model_and_sends_nothing() {
        let (mock, sent, feats) = MockHid::new(vec![]);
        let m = Model::new("Autre", DeviceType::Keyboard, vec![]);
        assert!(open(Box::new(mock), &m).is_err());
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }

    #[test]
    fn apex_key_table_has_114_confirmed_codes() {
        let k: Vec<u8> = apex_keys().collect();
        assert_eq!(k.len(), 114);
        assert_eq!([k[0], k[96], k[97], k[98], k[102], k[103], k[104], k[105], k[112], k[113]], [0x04, 0x64, 0x85, 0x87, 0x8B, 0x90, 0x91, 0xE0, 0xE7, 0xF0]);
        assert!(k.windows(2).all(|w| w[0] < w[1]));
    }

    /// Trame directe attendue, indices de touches écrits à la main : (index, usage).
    fn check_direct(f: &[u8], len: usize, id: u8) {
        assert_eq!(f.len(), len);
        assert_eq!(&f[..3], &[0x00, id, 114]);
        for (i, k) in [(0usize, 0x04u8), (96, 0x64), (97, 0x85), (103, 0x90), (112, 0xE7), (113, 0xF0)] {
            assert_eq!(f[3 + 4 * i], k, "touche {i}");
        }
        assert_eq!(&f[3..7], &[0x04, 1, 2, 3]);
        assert_eq!(&f[3 + 4 * 113..7 + 4 * 113], &[0xF0, 7, 8, 9]);
        assert!(f[3 + 4 * 114..].iter().all(|b| *b == 0));
    }

    fn sample_colors() -> Vec<Color> {
        let mut c = vec![Color::BLACK; 114];
        c[0] = rgb(1, 2, 3);
        c[113] = rgb(7, 8, 9);
        c
    }

    #[test]
    fn apex_gen1_has_no_init_direct_0x3a_and_reset_0x3b() {
        for pid in [0x1610, 0x1612, 0x1618, 0x161C] {
            let (mut d, sent, feats) = driver(pid);
            assert!(feats.lock().is_empty(), "pas d'init Gen1");
            d.set_colors(&sample_colors()).expect("set");
            check_direct(&feats.lock()[0], 643, 0x3A);
            assert!(sent.lock().is_empty());
            d.release();
            let mut reset = vec![0u8; 65];
            reset[1] = 0x3B;
            assert_eq!(sent.lock()[0], reset);
            assert_eq!(feats.lock().len(), 1);
        }
    }

    #[test]
    fn apex_gen2_gen3_init_direct_0x40_and_reset_0x41() {
        for pid in [0x1628, 0x1642] {
            let (mut d, sent, feats) = driver(pid);
            let mut init = vec![0u8; 65];
            init[1] = 0x4B;
            assert_eq!(feats.lock()[0], init);
            d.set_colors(&sample_colors()).expect("set");
            check_direct(&feats.lock()[1], 643, 0x40);
            d.release();
            let mut reset = vec![0u8; 65];
            reset[1] = 0x41;
            assert_eq!(sent.lock()[0], reset);
        }
    }

    #[test]
    fn apex_9_uses_513_byte_report() {
        for pid in [0x1634, 0x1620] {
            let (mut d, _, feats) = driver(pid);
            d.set_colors(&sample_colors()).expect("set");
            check_direct(&feats.lock()[1], 513, 0x40);
        }
    }

    #[test]
    fn apex_short_input_is_padded_black_and_too_many_rejected_without_sending() {
        let (mut d, sent, feats) = driver(0x1628);
        d.set_colors(&[rgb(9, 9, 9)]).expect("set");
        let f = feats.lock()[1].clone();
        assert_eq!(&f[3..7], &[0x04, 9, 9, 9]);
        assert_eq!(&f[7..11], &[0x05, 0, 0, 0]);
        assert!(d.set_colors(&vec![Color::BLACK; 115]).is_err());
        assert_eq!(feats.lock().len(), 2); // init + 1 trame
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn rival3_effect_then_one_8_byte_color_per_zone() {
        let (mut d, sent, feats) = driver(0x1824);
        assert_eq!(sent.lock()[0], vec![0x00, 0x06, 0x00, 0x04]);
        d.set_colors(&[rgb(1, 2, 3), rgb(4, 5, 6), rgb(7, 8, 9), rgb(10, 11, 12)]).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 5);
        assert_eq!(s[1], vec![0x00, 0x05, 0x00, 0x01, 1, 2, 3, 0x64]);
        assert_eq!(s[2], vec![0x00, 0x05, 0x00, 0x02, 4, 5, 6, 0x64]);
        assert_eq!(s[3], vec![0x00, 0x05, 0x00, 0x03, 7, 8, 9, 0x64]);
        assert_eq!(s[4], vec![0x00, 0x05, 0x00, 0x04, 10, 11, 12, 0x64]);
        assert!(feats.lock().is_empty());
    }

    #[test]
    fn rival3_rejects_fifth_zone_without_sending() {
        let (mut d, sent, _) = driver(0x184C);
        assert!(d.set_colors(&[Color::BLACK; 5]).is_err());
        assert_eq!(sent.lock().len(), 1); // seulement l'effet d'ouverture
    }

    #[test]
    fn rival5_single_64_byte_feature_with_ten_zones() {
        let (mut d, sent, feats) = driver(0x183C);
        assert!(feats.lock().is_empty() && sent.lock().is_empty());
        let c: Vec<Color> = (0..10u8).map(|i| rgb(i * 10 + 1, i * 10 + 2, i * 10 + 3)).collect();
        d.set_colors(&c).expect("set");
        let f = feats.lock()[0].clone();
        assert_eq!(f.len(), 64);
        assert_eq!(&f[..4], &[0x00, 0x21, 0xFF, 0x03]);
        assert_eq!(&f[4..7], &[1, 2, 3]);
        assert_eq!(&f[13..16], &[31, 32, 33]); // zone 3
        assert_eq!(&f[31..34], &[91, 92, 93]); // zone 9 = logo
        assert!(f[34..].iter().all(|b| *b == 0));
        assert!(sent.lock().is_empty());
        assert!(d.set_colors(&[Color::BLACK; 11]).is_err());
        assert_eq!(feats.lock().len(), 1);
    }

    #[test]
    fn aerox_65_one_packet_per_zone_with_mask_and_offset() {
        for pid in [0x1836, 0x1850] {
            let (mut d, sent, feats) = driver(pid);
            d.set_colors(&[rgb(10, 20, 30), rgb(40, 50, 60), rgb(70, 80, 90)]).expect("set");
            let s = sent.lock();
            assert_eq!(s.len(), 3);
            let mut z0 = vec![0u8; 65];
            z0[1..6].copy_from_slice(&[0x21, 0x01, 10, 20, 30]);
            let mut z1 = vec![0u8; 65];
            z1[1..3].copy_from_slice(&[0x21, 0x02]);
            z1[6..9].copy_from_slice(&[40, 50, 60]);
            let mut z2 = vec![0u8; 65];
            z2[1..3].copy_from_slice(&[0x21, 0x04]);
            z2[9..12].copy_from_slice(&[70, 80, 90]);
            assert_eq!((&s[0], &s[1], &s[2]), (&z0, &z1, &z2));
            assert!(feats.lock().is_empty());
        }
    }

    #[test]
    fn aerox_64_wired_mode_zone_in_byte_3_color_at_offset_4() {
        for pid in [0x183A, 0x187A, 0x1854, 0x185E, 0x1862, 0x185A] {
            let (mut d, sent, feats) = driver(pid);
            d.set_colors(&[rgb(10, 20, 30), rgb(40, 50, 60), rgb(70, 80, 90)]).expect("set");
            let s = sent.lock();
            assert_eq!(s.len(), 3);
            for (z, c) in [[10u8, 20, 30], [40, 50, 60], [70, 80, 90]].iter().enumerate() {
                let mut want = vec![0u8; 64];
                want[1..4].copy_from_slice(&[0x21, 0x01, z as u8]);
                want[4..7].copy_from_slice(c);
                assert_eq!(s[z], want, "{pid:04x} zone {z}"); // filaire : ni OU 0x40 ni lecture
            }
            assert!(feats.lock().is_empty());
        }
    }

    #[test]
    fn aerox_rejects_extra_zone_and_release_sends_nothing() {
        for pid in [0x1836, 0x185A] {
            let (mut d, sent, feats) = driver(pid);
            assert!(d.set_colors(&[Color::BLACK; 4]).is_err());
            d.release();
            assert!(sent.lock().is_empty() && feats.lock().is_empty());
        }
    }

    #[test]
    fn qck_525_byte_feature_then_65_byte_apply() {
        let (mut d, sent, feats) = driver(0x150D);
        d.set_colors(&[rgb(1, 2, 3), rgb(4, 5, 6)]).expect("set");
        let mut want = vec![0u8; 525];
        want[1] = 0x0E;
        want[3] = 0x02;
        want[5..8].copy_from_slice(&[1, 2, 3]);
        want[8..11].copy_from_slice(&[0xFF, 0x32, 0xC8]);
        want[14] = 0x01;
        want[17..20].copy_from_slice(&[4, 5, 6]);
        want[20..23].copy_from_slice(&[0xFF, 0x32, 0xC8]);
        want[25] = 0x01;
        want[26] = 0x01;
        want[28] = 0x01;
        assert_eq!(feats.lock()[0], want);
        let mut apply = vec![0u8; 65];
        apply[1] = 0x0D;
        assert_eq!(sent.lock()[0], apply);
        d.set_colors(&[rgb(9, 9, 9)]).expect("set"); // une seule couleur : la seconde est noire
        assert_eq!(&feats.lock()[1][17..20], &[0, 0, 0]);
    }

    #[test]
    fn qck_rejects_three_colors_without_sending() {
        let (mut d, sent, feats) = driver(0x150A);
        assert!(d.set_colors(&[Color::BLACK; 3]).is_err());
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }

    #[test]
    fn no_save_command_is_ever_sent() {
        for pid in [0x1610, 0x1628, 0x1634, 0x1824, 0x183C, 0x1836, 0x1850, 0x185A, 0x150A] {
            let (mut d, sent, feats) = driver(pid);
            let max = probe(VID, pid).map_or(0, |m| m.led_count() as usize);
            d.set_colors(&vec![rgb(1, 1, 1); max]).expect("set");
            d.release();
            let bad = |f: &Vec<u8>| f.get(1).is_some_and(|b| *b == 0x11 || *b == 0x09);
            assert!(!sent.lock().iter().any(bad) && !feats.lock().iter().any(bad), "{pid:04x}");
        }
    }
}
